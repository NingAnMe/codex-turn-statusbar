# Cross-Platform Design

## Goal

Build a tray/status-bar version of Codex Turn Status Bar that runs on macOS and Windows while preserving the existing Codex `notify` JSON protocol.

## Architecture

The Swift/AppKit app remains available as the current macOS implementation. The cross-platform implementation adds a Rust workspace:

- `codex-turn-status-core`: shared status-file protocol, event parsing, display presentation, handled-state policy, and notify writer.
- `codex-turn-notify`: cross-platform Codex `notify` executable that writes the status JSON files.
- `codex-turn-statusbar-tauri`: tray app using Tauri's Rust tray API.

Both implementations read and write the same files:

- `CODEX_HOME/codex-turn-status.json`
- `CODEX_HOME/codex-turn-status-event.json`

When `CODEX_HOME` is not set, the default is `$HOME/.codex` on macOS/Linux and `%USERPROFILE%\.codex` on Windows.

## Behavior

The tray UI is icon-only:

- idle: white ring with center dot
- needs attention: green unread/check icon
- error: yellow warning icon

The menu exposes status details plus:

- `Open Codex`
- `Mark Handled`
- `Refresh`
- `Quit`

The app polls the fallback status file once per second. On macOS it also connects to Codex Desktop's local unread/read IPC and keeps a small in-memory unread set from `thread-read-state-changed` and `thread-stream-state-changed` broadcasts. The tray shows attention when the unread set is non-empty, when the automation inbox has unread rows, or when the fallback notify status is pending. It no longer clears just because Codex is frontmost. Windows keeps the notify fallback until a compatible unread IPC transport is added.

## Packaging

macOS universal packaging builds both `aarch64-apple-darwin` and `x86_64-apple-darwin`, combines them with `lipo`, and creates a `.app` bundle. Windows packaging builds the two Rust executables and prepares a zip-oriented package layout. MSI/DMG signing and notarization stay out of scope for this local package.
