#!/bin/zsh
set -euo pipefail

VERSION="0.1.0"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist"
PACKAGE_NAME="CodexTurnStatusBar-$VERSION"
PACKAGE_DIR="$DIST_DIR/$PACKAGE_NAME"
ZIP_PATH="$DIST_DIR/$PACKAGE_NAME.zip"

cd "$ROOT_DIR"
"$SCRIPT_DIR/build-app.sh" >/dev/null

rm -rf "$PACKAGE_DIR" "$ZIP_PATH"
mkdir -p "$PACKAGE_DIR"

cp -R "$DIST_DIR/CodexTurnStatusBar.app" "$PACKAGE_DIR/CodexTurnStatusBar.app"
cp "$ROOT_DIR/scripts/codex-notify-router.sh" "$PACKAGE_DIR/codex-notify-router.sh"
cp "$ROOT_DIR/scripts/install-notify-router.sh" "$PACKAGE_DIR/install-notify-router.sh"
cp "$ROOT_DIR/PACKAGE_README.md" "$PACKAGE_DIR/README.md"
chmod +x "$PACKAGE_DIR/codex-notify-router.sh" "$PACKAGE_DIR/install-notify-router.sh"

cd "$DIST_DIR"
/usr/bin/zip -qry -X "$ZIP_PATH" "$PACKAGE_NAME"

echo "$ZIP_PATH"
