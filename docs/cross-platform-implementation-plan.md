# Cross-Platform Implementation Plan

Goal: add a Rust/Tauri cross-platform tray app plus a cross-platform notify executable without removing the current Swift macOS app.

## Tasks

1. Add a Rust workspace with shared core crate, notify CLI crate, and Tauri tray crate.
2. Port the status protocol and presentation logic into Rust.
3. Add tests for default paths, status loading, notify writing, detail normalization, and handled-state policy.
4. Implement the notify CLI with stdin/argument payload support and macOS SkyComputerUseClient forwarding.
5. Implement the Tauri tray app with polling, menu actions, icon updates, and active Codex auto-clear.
6. Add macOS universal and Windows package scripts.
7. Update README/package docs with Swift legacy and Rust cross-platform instructions.
8. Verify with `cargo test`, `cargo build`, and the existing Swift smoke test.
