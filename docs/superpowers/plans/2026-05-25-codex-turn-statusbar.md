# Codex Turn Status Bar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a quiet macOS menu bar app that shows when Codex Desktop has completed a turn and is waiting for the user.

**Architecture:** Codex `notify` invokes a shell router that writes a small status file under `~/.codex`. A Swift menu bar app polls that file, reads the most recent raw notify payload when present, and exposes "Open Codex" plus "Mark Handled" actions.

**Tech Stack:** Swift Package Manager, AppKit status item, XCTest, zsh scripts.

---

### Task 1: Core Status Model

**Files:**
- Create: `CodexTurnStatusBar/Package.swift`
- Create: `CodexTurnStatusBar/Tests/CodexTurnStatusCoreTests/StatusStoreTests.swift`
- Create: `CodexTurnStatusBar/Sources/CodexTurnStatusCore/StatusStore.swift`

- [ ] Write tests for loading idle, needs-attention, and malformed payload states.
- [ ] Run `swift test` and verify it fails because the core module is missing.
- [ ] Implement the core structs and file-backed store.
- [ ] Run `swift test` and verify the tests pass.

### Task 2: Menu Bar App

**Files:**
- Create: `CodexTurnStatusBar/Sources/CodexTurnStatusBar/main.swift`
- Create: `CodexTurnStatusBar/Sources/CodexTurnStatusBar/AppDelegate.swift`
- Create: `CodexTurnStatusBar/Sources/CodexTurnStatusBar/StatusBarController.swift`

- [ ] Implement an accessory AppKit app with an `NSStatusItem`.
- [ ] Show idle, needs-attention, and error states in menu labels.
- [ ] Add "Open Codex", "Mark Handled", "Refresh", and "Quit" menu actions.
- [ ] Run `swift build` and verify it compiles.

### Task 3: Notify Router And Bundle Script

**Files:**
- Create: `CodexTurnStatusBar/scripts/codex-notify-router.sh`
- Create: `CodexTurnStatusBar/scripts/build-app.sh`
- Create: `CodexTurnStatusBar/README.md`

- [ ] Implement a router that stores the raw Codex notify payload and marks status as `needs_attention`.
- [ ] Forward the same event to the existing SkyComputerUseClient command when present.
- [ ] Implement a build script that wraps the release binary in a `.app` bundle.
- [ ] Document install and configuration steps.
- [ ] Run tests and build script as final verification.
