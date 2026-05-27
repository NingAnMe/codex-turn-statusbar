import CodexTurnStatusCore
import Foundation

try missingStatusFileLoadsIdleState()
try needsAttentionStatusIncludesCodexEventDetails()
try malformedStatusFileLoadsErrorState()
try markHandledWritesIdleStatus()
try menuBarPresentationUsesIconOnly()
try handledStatePolicyOnlyClearsWhenCodexActivates()
try handledStatePolicyClearsNeedsAttentionWhenCodexIsAlreadyFrontmost()
print("CodexTurnStatusCoreSmokeTests passed")

private func missingStatusFileLoadsIdleState() throws {
    let directory = try temporaryDirectory()
    let store = StatusStore(
        statusURL: directory.appendingPathComponent("status.json"),
        eventURL: directory.appendingPathComponent("event.json")
    )

    let status = store.loadDisplayStatus()

    expect(status.state == .idle, "missing status file should load idle state")
    expect(status.title == "Codex idle", "idle state should have idle title")
    expect(
        status.detail == "No completed turn needs attention.",
        "idle state should have default detail"
    )
}

private func needsAttentionStatusIncludesCodexEventDetails() throws {
    let directory = try temporaryDirectory()
    let statusURL = directory.appendingPathComponent("status.json")
    let eventURL = directory.appendingPathComponent("event.json")
    try """
    {"state":"needs_attention","updatedAt":"2026-05-25T22:48:00+08:00","eventPath":"\(eventURL.path)"}
    """.write(to: statusURL, atomically: true, encoding: .utf8)
    try """
    {
      "thread-id": "thread-123",
      "turn-id": "turn-456",
      "cwd": "/Users/ning/Documents/research",
      "last-assistant-message": "Codex finished the turn."
    }
    """.write(to: eventURL, atomically: true, encoding: .utf8)
    let store = StatusStore(statusURL: statusURL, eventURL: eventURL)

    let status = store.loadDisplayStatus()

    expect(status.state == .needsAttention, "status should need attention")
    expect(status.title == "Codex needs attention", "needs attention title should be present")
    expect(status.detail == "Codex finished the turn.", "last assistant message should be detail")
    expect(status.cwd == "/Users/ning/Documents/research", "cwd should be decoded")
    expect(status.threadID == "thread-123", "thread id should be decoded")
    expect(status.turnID == "turn-456", "turn id should be decoded")
}

private func malformedStatusFileLoadsErrorState() throws {
    let directory = try temporaryDirectory()
    let statusURL = directory.appendingPathComponent("status.json")
    try "{not json".write(to: statusURL, atomically: true, encoding: .utf8)
    let store = StatusStore(
        statusURL: statusURL,
        eventURL: directory.appendingPathComponent("event.json")
    )

    let status = store.loadDisplayStatus()

    expect(status.state == .error, "malformed status should load error state")
    expect(status.title == "Codex status unavailable", "malformed status should have error title")
}

private func markHandledWritesIdleStatus() throws {
    let directory = try temporaryDirectory()
    let statusURL = directory.appendingPathComponent("status.json")
    let eventURL = directory.appendingPathComponent("event.json")
    let store = StatusStore(statusURL: statusURL, eventURL: eventURL)

    try store.markHandled()
    let status = store.loadDisplayStatus()

    expect(status.state == .idle, "mark handled should write idle state")
}

private func menuBarPresentationUsesIconOnly() throws {
    let status = DisplayStatus(
        state: .needsAttention,
        title: "Codex needs attention",
        detail: "Ready.",
        cwd: nil,
        threadID: nil,
        turnID: nil,
        updatedAt: nil
    )

    let presentation = MenuBarPresentation(status: status)

    expect(presentation.title.isEmpty, "menu bar title should be empty for icon-only display")
    expect(presentation.iconName == "checkmark.message.fill", "needs attention should use message icon")
    expect(presentation.tint == .attention, "needs attention should use attention tint")
    expect(presentation.usesOriginalColorImage, "needs attention should bypass menu bar template tinting")
}

private func handledStatePolicyOnlyClearsWhenCodexActivates() throws {
    let policy = HandledStatePolicy()

    expect(
        policy.shouldMarkHandledOnActivatedApplication(bundleIdentifier: "com.openai.codex"),
        "Codex activation should clear needs-attention state"
    )
    expect(
        !policy.shouldMarkHandledOnActivatedApplication(bundleIdentifier: "com.apple.finder"),
        "Finder activation should not clear needs-attention state"
    )
    expect(
        !policy.shouldMarkHandledOnActivatedApplication(bundleIdentifier: nil),
        "missing bundle id should not clear needs-attention state"
    )
}

private func handledStatePolicyClearsNeedsAttentionWhenCodexIsAlreadyFrontmost() throws {
    let policy = HandledStatePolicy()
    let needsAttention = DisplayStatus(
        state: .needsAttention,
        title: "Codex needs attention",
        detail: "Ready.",
        cwd: nil,
        threadID: nil,
        turnID: nil,
        updatedAt: nil
    )
    let idle = DisplayStatus.idle

    expect(
        policy.shouldMarkHandled(status: needsAttention, activeBundleIdentifier: "com.openai.codex"),
        "needs-attention should clear when Codex is already frontmost"
    )
    expect(
        !policy.shouldMarkHandled(status: idle, activeBundleIdentifier: "com.openai.codex"),
        "idle status should not rewrite handled state"
    )
    expect(
        !policy.shouldMarkHandled(status: needsAttention, activeBundleIdentifier: "com.apple.finder"),
        "needs-attention should stay visible when another app is frontmost"
    )
}

private func temporaryDirectory() throws -> URL {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString, isDirectory: true)
    try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    return url
}

private func expect(_ condition: @autoclosure () -> Bool, _ message: String) -> Void {
    if !condition() {
        fputs("Test failed: \(message)\n", stderr)
        exit(1)
    }
}
