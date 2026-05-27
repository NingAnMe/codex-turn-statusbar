# Codex Turn Status Bar

Quiet macOS menu bar indicator for Codex Desktop turn completion.

## Implementations

- `Package.swift` / `Sources/`: original Swift/AppKit macOS app.
- `Cargo.toml` / `crates/` / `src-tauri/`: Rust/Tauri cross-platform app for macOS and Windows.

Development notes and macOS packaging pitfalls are recorded in `docs/macos-development-notes.md`.

Both implementations use the same status protocol:

```text
~/.codex/codex-turn-status.json
~/.codex/codex-turn-status-event.json
```

On Windows the same files live under `%USERPROFILE%\.codex\`.

## Build Swift macOS App

```sh
./scripts/build-app.sh
```

The app bundle is created at:

```text
dist/CodexTurnStatusBar.app
```

Open it with:

```sh
open dist/CodexTurnStatusBar.app
```

## Build Cross-Platform Rust App

Run tests:

```sh
cargo test
```

Build the Rust/Tauri tray app and notify executable for the current platform:

```sh
cargo build --release -p codex-turn-statusbar-tauri -p codex-turn-notify
```

Package macOS as a Universal app for Apple Silicon and Intel:

```sh
./scripts/package-tauri-macos-universal.sh
```

This writes both a `.dmg` and a `.zip` package under `dist-cross/`.
The `.dmg` window shows a drag-to-Applications background, the app icon, and an `Applications` shortcut. The app configures Codex `notify` automatically on launch.

Package Windows from Windows PowerShell:

```powershell
.\scripts\package-tauri-windows.ps1
```

## Package Swift macOS App

```sh
./scripts/package-release.sh
```

This creates:

```text
dist/CodexTurnStatusBar-0.1.0.zip
```

The zip contains the app bundle, notify router, installer script, and a short README.

## Configure Codex Notify: Swift macOS Package

Install the notify router:

```sh
mkdir -p "$HOME/.codex/bin"
cp scripts/codex-notify-router.sh "$HOME/.codex/bin/codex-notify-router.sh"
chmod +x "$HOME/.codex/bin/codex-notify-router.sh"
```

Then set this in `~/.codex/config.toml`:

```toml
notify = ["/Users/ning/.codex/bin/codex-notify-router.sh"]
```

The router writes:

```text
~/.codex/codex-turn-status.json
~/.codex/codex-turn-status-event.json
```

It also forwards the event to the bundled SkyComputerUseClient when that binary exists, preserving the existing Codex Computer Use turn-ended behavior.

## Configure Codex Notify: Cross-Platform Package

For the macOS `.dmg`, open `CodexTurnStatusBar.app` once. The app contains the `codex-turn-notify` helper and configures Codex `notify` automatically.

For the macOS `.zip`, the app also configures Codex `notify` when opened. If you need to install the helper manually, run:

```sh
./scripts/install-cross-platform-notify.sh
```

On Windows:

```powershell
.\scripts\install-cross-platform-notify.ps1
```

These installers copy `codex-turn-notify` / `codex-turn-notify.exe` into Codex home and set:

```toml
notify = ["/path/to/codex-turn-notify"]
```

## Use

- Idle: white ring and center-dot icon in the menu bar.
- Needs attention: green unread/check icon.
- Error: yellow warning icon.
- On macOS, Codex Desktop unread/read IPC is preferred when available. The icon stays green while Codex has unread activity, even if Codex is already frontmost.
- `notify` remains installed as a fallback for completed turns. A matching Codex read-state event clears that fallback automatically.

Menu actions:

- `Open Codex`: opens Codex Desktop when you choose to return.
- `Mark Handled`: clears the fallback notify pending state.
- `Refresh`: reloads the status file immediately.
- `Quit`: exits the menu bar app.
