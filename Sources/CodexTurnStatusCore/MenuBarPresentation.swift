public enum MenuBarTint: Equatable, Sendable {
    case idle
    case attention
    case warning
}

public struct MenuBarPresentation: Equatable, Sendable {
    public let title: String
    public let iconName: String
    public let tint: MenuBarTint
    public let tooltip: String
    public let usesOriginalColorImage: Bool

    public init(status: DisplayStatus) {
        self.title = ""
        self.tooltip = status.title

        switch status.state {
        case .idle:
            self.iconName = "circle"
            self.tint = .idle
            self.usesOriginalColorImage = false
        case .needsAttention:
            self.iconName = "checkmark.message.fill"
            self.tint = .attention
            self.usesOriginalColorImage = true
        case .error:
            self.iconName = "exclamationmark.triangle.fill"
            self.tint = .warning
            self.usesOriginalColorImage = true
        }
    }
}
