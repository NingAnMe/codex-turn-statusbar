public struct HandledStatePolicy: Sendable {
    public static let codexBundleIdentifier = "com.openai.codex"

    public init() {}

    public func shouldMarkHandledOnActivatedApplication(bundleIdentifier: String?) -> Bool {
        bundleIdentifier == Self.codexBundleIdentifier
    }

    public func shouldMarkHandled(
        status: DisplayStatus,
        activeBundleIdentifier: String?
    ) -> Bool {
        status.state == .needsAttention &&
            shouldMarkHandledOnActivatedApplication(bundleIdentifier: activeBundleIdentifier)
    }
}
