#!/bin/sh
set -eu

TOOL_NAME="Codex Turn Status Bar"
BASE_URL="${BASE_URL:-http://10.1.111.12:18000/codex-turn-statusbar}"
DMG_NAME="codex-turn-statusbar-latest.dmg"
EXPECTED_SHA256="1180f619a6126f5c0f8162abc8302ff13466efaf167c588721e0174ffbe11bba"
APP_NAME="CodexTurnStatusBar.app"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "$TOOL_NAME currently publishes a macOS DMG only." >&2
  exit 1
fi

tmp_dir="$(mktemp -d)"
mount_dir=""
cleanup() {
  if [ -n "$mount_dir" ]; then
    hdiutil detach "$mount_dir" -quiet >/dev/null 2>&1 || true
  fi
  rm -rf "$tmp_dir"
}
trap cleanup EXIT INT TERM

dmg_path="$tmp_dir/$DMG_NAME"
url="$BASE_URL/$DMG_NAME"

if command -v curl >/dev/null 2>&1; then
  curl -fL "$url" -o "$dmg_path"
elif command -v wget >/dev/null 2>&1; then
  wget -O "$dmg_path" "$url"
else
  echo "curl or wget is required." >&2
  exit 1
fi

actual_sha256="$(shasum -a 256 "$dmg_path" | awk '{print $1}')"
if [ "$actual_sha256" != "$EXPECTED_SHA256" ]; then
  echo "Checksum mismatch for $DMG_NAME" >&2
  echo "expected: $EXPECTED_SHA256" >&2
  echo "actual:   $actual_sha256" >&2
  exit 1
fi

mount_dir="$(hdiutil attach "$dmg_path" -readonly -nobrowse | awk '/\\/Volumes\\// {print $3; exit}')"
if [ -z "$mount_dir" ] || [ ! -d "$mount_dir/$APP_NAME" ]; then
  echo "Could not mount $DMG_NAME or find $APP_NAME." >&2
  exit 1
fi

install_dir="${CODEX_TURN_STATUSBAR_INSTALL_DIR:-/Applications}"
if [ ! -w "$install_dir" ]; then
  install_dir="$HOME/Applications"
  mkdir -p "$install_dir"
fi

target="$install_dir/$APP_NAME"
if [ -e "$target" ]; then
  backup="$target.backup.$(date -u '+%Y%m%dT%H%M%SZ')"
  mv "$target" "$backup"
  echo "Backed up existing app to $backup"
fi

cp -R "$mount_dir/$APP_NAME" "$install_dir/"
open "$target" || true

echo "Installed $TOOL_NAME to $target"
echo "The app configures Codex notify on launch."
echo "Restart Codex Desktop, or start a new Codex session, so Codex reloads notify."
