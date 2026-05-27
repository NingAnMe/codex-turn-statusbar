#!/bin/zsh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PACKAGE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
NOTIFY_SOURCE="$PACKAGE_DIR/codex-turn-notify"
CODEX_HOME_DIR="${CODEX_HOME:-$HOME/.codex}"
NOTIFY_TARGET="$CODEX_HOME_DIR/bin/codex-turn-notify"
CONFIG_FILE="$CODEX_HOME_DIR/config.toml"

if [[ ! -f "$NOTIFY_SOURCE" ]]; then
  echo "Missing notify executable: $NOTIFY_SOURCE" >&2
  exit 1
fi

mkdir -p "$CODEX_HOME_DIR/bin"
install -m 755 "$NOTIFY_SOURCE" "$NOTIFY_TARGET"

if [[ -f "$CONFIG_FILE" ]]; then
  backup="$CONFIG_FILE.bak.codex-turn-statusbar.$(date -u '+%Y%m%dT%H%M%SZ')"
  cp "$CONFIG_FILE" "$backup"
else
  touch "$CONFIG_FILE"
fi

escaped_notify="${NOTIFY_TARGET//\\/\\\\}"
escaped_notify="${escaped_notify//\"/\\\"}"
notify_line="notify = [\"$escaped_notify\"]"

if grep -q '^notify = ' "$CONFIG_FILE"; then
  perl -0pi -e "s#^notify = \\[[^\\n]*\\]#$notify_line#m" "$CONFIG_FILE"
else
  tmp_file="$CONFIG_FILE.$$"
  {
    printf "%s\n\n" "$notify_line"
    cat "$CONFIG_FILE"
  } > "$tmp_file"
  mv "$tmp_file" "$CONFIG_FILE"
fi

echo "Installed notify executable: $NOTIFY_TARGET"
echo "Updated Codex config: $CONFIG_FILE"
echo "Restart Codex Desktop or start a new Codex session for notify changes to take effect."
