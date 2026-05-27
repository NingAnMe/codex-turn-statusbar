# Codex Turn Status Bar

This package contains a quiet tray/status-bar indicator for Codex Desktop.

## Contents

- macOS `.dmg`: `CodexTurnStatusBar.app`, an `Applications` shortcut, and an installation hint background.
- macOS `.zip`: `CodexTurnStatusBar.app`, `codex-turn-notify`, and `scripts/install-cross-platform-notify.sh`.
- Windows package: `CodexTurnStatusBar.exe`, `codex-turn-notify.exe`, and `scripts/install-cross-platform-notify.ps1`.
- Legacy Swift macOS package: `codex-notify-router.sh` and `install-notify-router.sh`.

## Install

macOS:

1. Open the `.dmg`.
2. Drag `CodexTurnStatusBar.app` onto `Applications`.
3. Open `CodexTurnStatusBar.app` from Applications.
4. Restart Codex Desktop, or start a new Codex session, so Codex reloads `notify`.

The app configures Codex `notify` automatically on launch. For the `.zip` package, move `CodexTurnStatusBar.app` to `/Applications` and open it. The script `./scripts/install-cross-platform-notify.sh` remains available as a manual fallback.

Windows:

1. Run `CodexTurnStatusBar.exe`.
2. Run from the extracted package:

   ```powershell
   .\scripts\install-cross-platform-notify.ps1
   ```

3. Restart Codex Desktop, or start a new Codex session, so Codex reloads `notify`.

## Behavior

- White ring icon: no unread Codex activity is known.
- Green icon: Codex has unread activity, or `notify` reported a completed turn as a fallback.
- On macOS, the app listens to Codex Desktop unread/read IPC when available. It does not clear just because Codex is frontmost.

No sound, no focus stealing.
