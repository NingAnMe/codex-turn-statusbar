import Foundation

public enum StatusState: String, Codable, Equatable, Sendable {
    case idle
    case needsAttention = "needs_attention"
    case error
}

public struct DisplayStatus: Equatable, Sendable {
    public let state: StatusState
    public let title: String
    public let detail: String
    public let cwd: String?
    public let threadID: String?
    public let turnID: String?
    public let updatedAt: String?

    public init(
        state: StatusState,
        title: String,
        detail: String,
        cwd: String?,
        threadID: String?,
        turnID: String?,
        updatedAt: String?
    ) {
        self.state = state
        self.title = title
        self.detail = detail
        self.cwd = cwd
        self.threadID = threadID
        self.turnID = turnID
        self.updatedAt = updatedAt
    }
}

public struct StatusStore {
    private let statusURL: URL
    private let eventURL: URL
    private let fileManager: FileManager

    public init(
        statusURL: URL = StatusStore.defaultStatusURL,
        eventURL: URL = StatusStore.defaultEventURL,
        fileManager: FileManager = .default
    ) {
        self.statusURL = statusURL
        self.eventURL = eventURL
        self.fileManager = fileManager
    }

    public func loadDisplayStatus() -> DisplayStatus {
        guard fileManager.fileExists(atPath: statusURL.path) else {
            return .idle
        }

        do {
            let data = try Data(contentsOf: statusURL)
            let snapshot = try JSONDecoder().decode(StatusSnapshot.self, from: data)
            return displayStatus(from: snapshot)
        } catch {
            return .error
        }
    }

    public func markHandled() throws {
        let snapshot = StatusSnapshot(
            state: .idle,
            updatedAt: ISO8601DateFormatter().string(from: Date()),
            eventPath: nil
        )
        try write(snapshot)
    }

    private func displayStatus(from snapshot: StatusSnapshot) -> DisplayStatus {
        switch snapshot.state {
        case .idle:
            return .idle
        case .error:
            return .error
        case .needsAttention:
            let event = loadEvent(path: snapshot.eventPath)
            return DisplayStatus(
                state: .needsAttention,
                title: "Codex needs attention",
                detail: normalizedDetail(from: event?.lastAssistantMessage),
                cwd: event?.cwd,
                threadID: event?.threadID,
                turnID: event?.turnID,
                updatedAt: snapshot.updatedAt
            )
        }
    }

    private func loadEvent(path: String?) -> CodexNotifyEvent? {
        let url = path.map { URL(fileURLWithPath: $0) } ?? eventURL
        guard fileManager.fileExists(atPath: url.path) else {
            return nil
        }

        do {
            let data = try Data(contentsOf: url)
            return try JSONDecoder().decode(CodexNotifyEvent.self, from: data)
        } catch {
            return nil
        }
    }

    private func write(_ snapshot: StatusSnapshot) throws {
        let directory = statusURL.deletingLastPathComponent()
        try fileManager.createDirectory(at: directory, withIntermediateDirectories: true)
        let data = try JSONEncoder().encode(snapshot)
        try data.write(to: statusURL, options: .atomic)
    }

    private func normalizedDetail(from message: String?) -> String {
        guard let message else {
            return "A Codex turn completed."
        }

        let collapsed = message
            .split(whereSeparator: \.isWhitespace)
            .joined(separator: " ")

        if collapsed.isEmpty {
            return "A Codex turn completed."
        }

        if collapsed.count <= 180 {
            return collapsed
        }

        return String(collapsed.prefix(177)) + "..."
    }

    public static let defaultStatusURL = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent(".codex/codex-turn-status.json")

    public static let defaultEventURL = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent(".codex/codex-turn-status-event.json")
}

private struct StatusSnapshot: Codable {
    let state: StatusState
    let updatedAt: String?
    let eventPath: String?
}

private struct CodexNotifyEvent: Codable {
    let threadID: String?
    let turnID: String?
    let cwd: String?
    let lastAssistantMessage: String?

    enum CodingKeys: String, CodingKey {
        case threadID = "thread-id"
        case turnID = "turn-id"
        case cwd
        case lastAssistantMessage = "last-assistant-message"
    }
}

public extension DisplayStatus {
    static let idle = DisplayStatus(
        state: .idle,
        title: "Codex idle",
        detail: "No completed turn needs attention.",
        cwd: nil,
        threadID: nil,
        turnID: nil,
        updatedAt: nil
    )

    static let error = DisplayStatus(
        state: .error,
        title: "Codex status unavailable",
        detail: "The status file could not be read.",
        cwd: nil,
        threadID: nil,
        turnID: nil,
        updatedAt: nil
    )
}
