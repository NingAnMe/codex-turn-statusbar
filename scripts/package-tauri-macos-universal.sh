#!/bin/zsh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="CodexTurnStatusBar"
PACKAGE_NAME="CodexTurnStatusBar-0.2.1-macos-universal"
DIST_DIR="$ROOT_DIR/dist-cross"
PACKAGE_DIR="$DIST_DIR/$PACKAGE_NAME"
DMG_STAGING_DIR="$DIST_DIR/$PACKAGE_NAME-dmg"
ZIP_PATH="$DIST_DIR/$PACKAGE_NAME.zip"
DMG_PATH="$DIST_DIR/$PACKAGE_NAME.dmg"
RW_DMG_PATH="$DIST_DIR/$PACKAGE_NAME-rw.dmg"
APP_DIR="$PACKAGE_DIR/$APP_NAME.app"
MACOS_DIR="$APP_DIR/Contents/MacOS"
DMG_BACKGROUND_TOOL="$DIST_DIR/generate-dmg-background"
ICONSET_DIR="$DIST_DIR/$APP_NAME.iconset"

cd "$ROOT_DIR"
mkdir -p "$DIST_DIR"
node scripts/generate-tauri-icon.mjs >/dev/null
clang -fobjc-arc -framework AppKit \
  "$ROOT_DIR/scripts/generate-dmg-background.m" \
  -o "$DMG_BACKGROUND_TOOL"

rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin

cargo build --release -p codex-turn-statusbar-tauri -p codex-turn-notify --target aarch64-apple-darwin
cargo build --release -p codex-turn-statusbar-tauri -p codex-turn-notify --target x86_64-apple-darwin

rm -rf "$PACKAGE_DIR" "$DMG_STAGING_DIR" "$ZIP_PATH" "$DMG_PATH" "$RW_DMG_PATH" "$ICONSET_DIR"
mkdir -p "$MACOS_DIR" "$APP_DIR/Contents/Resources" "$PACKAGE_DIR/scripts"

lipo -create \
  "$ROOT_DIR/target/aarch64-apple-darwin/release/codex-turn-statusbar-tauri" \
  "$ROOT_DIR/target/x86_64-apple-darwin/release/codex-turn-statusbar-tauri" \
  -output "$MACOS_DIR/$APP_NAME"

lipo -create \
  "$ROOT_DIR/target/aarch64-apple-darwin/release/codex-turn-notify" \
  "$ROOT_DIR/target/x86_64-apple-darwin/release/codex-turn-notify" \
  -output "$PACKAGE_DIR/codex-turn-notify"

chmod +x "$MACOS_DIR/$APP_NAME" "$PACKAGE_DIR/codex-turn-notify"
cp "$ROOT_DIR/src-tauri/icons/icon.png" "$APP_DIR/Contents/Resources/icon.png"
sips -s format icns "$ROOT_DIR/src-tauri/icons/icon.png" --out "$APP_DIR/Contents/Resources/icon.icns" >/dev/null
"$DMG_BACKGROUND_TOOL" "$APP_DIR/Contents/Resources/dmg-background.png"
cp "$PACKAGE_DIR/codex-turn-notify" "$APP_DIR/Contents/Resources/codex-turn-notify"
chmod +x "$APP_DIR/Contents/Resources/codex-turn-notify"
cp "$ROOT_DIR/scripts/install-cross-platform-notify.sh" "$PACKAGE_DIR/scripts/install-cross-platform-notify.sh"
chmod +x "$PACKAGE_DIR/scripts/install-cross-platform-notify.sh"
cp "$ROOT_DIR/PACKAGE_README.md" "$PACKAGE_DIR/README.md"

cat > "$APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>$APP_NAME</string>
  <key>CFBundleIdentifier</key>
  <string>local.codex-turn-statusbar</string>
  <key>CFBundleName</key>
  <string>$APP_NAME</string>
  <key>CFBundleIconFile</key>
  <string>icon</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.2.1</string>
  <key>CFBundleVersion</key>
  <string>0.2.1</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>LSUIElement</key>
  <true/>
</dict>
</plist>
PLIST

(
  cd "$DIST_DIR"
  zip -qr "$PACKAGE_NAME.zip" "$PACKAGE_NAME"
)

mkdir -p "$DMG_STAGING_DIR"
cp -R "$APP_DIR" "$DMG_STAGING_DIR/$APP_NAME.app"
ln -s /Applications "$DMG_STAGING_DIR/Applications"

hdiutil create \
  -volname "$APP_NAME" \
  -srcfolder "$DMG_STAGING_DIR" \
  -ov \
  -format UDRW \
  "$RW_DMG_PATH" >/dev/null

hdiutil detach "/Volumes/$APP_NAME" -quiet >/dev/null 2>&1 || true
hdiutil attach "$RW_DMG_PATH" -readwrite -noverify -noautoopen >/dev/null

cleanup_mount() {
  hdiutil detach "/Volumes/$APP_NAME" -quiet >/dev/null 2>&1 || true
}
trap cleanup_mount EXIT

osascript <<APPLESCRIPT >/dev/null
tell application "Finder"
  tell disk "$APP_NAME"
    open
    delay 1
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set the bounds of container window to {160, 120, 820, 520}
    set viewOptions to the icon view options of container window
    set arrangement of viewOptions to not arranged
    set icon size of viewOptions to 104
    set background picture of viewOptions to POSIX file "/Volumes/$APP_NAME/$APP_NAME.app/Contents/Resources/dmg-background.png"
    set position of item "$APP_NAME.app" of container window to {185, 220}
    set position of item "Applications" of container window to {475, 220}
    update without registering applications
    delay 1
    close
  end tell
end tell
APPLESCRIPT

rm -rf "/Volumes/$APP_NAME/.fseventsd"
sync
cleanup_mount
trap - EXIT

hdiutil convert "$RW_DMG_PATH" -format UDZO -ov -o "$DMG_PATH" >/dev/null
rm -f "$RW_DMG_PATH"

echo "$ZIP_PATH"
echo "$DMG_PATH"
