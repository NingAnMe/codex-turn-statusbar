import AppKit
import CodexTurnStatusCore

@MainActor
final class StatusBarController: NSObject {
    private let statusItem: NSStatusItem
    private let store: StatusStore
    private let handledStatePolicy = HandledStatePolicy()
    private var timer: Timer?
    private var currentStatus: DisplayStatus = .idle

    init(store: StatusStore) {
        self.store = store
        self.statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        super.init()
    }

    func start() {
        observeApplicationActivation()
        update()
        timer = Timer.scheduledTimer(
            timeInterval: 1.0,
            target: self,
            selector: #selector(refreshTimerFired),
            userInfo: nil,
            repeats: true
        )
    }

    @objc private func refreshTimerFired() {
        update()
    }

    @objc private func openCodex() {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/open")
        process.arguments = ["-a", "Codex"]
        try? process.run()
        markHandledIfNeeded()
    }

    @objc private func markHandled() {
        markHandledIfNeeded()
    }

    @objc private func refreshNow() {
        update()
    }

    @objc private func quit() {
        NSApplication.shared.terminate(nil)
    }

    @objc private func applicationDidActivate(_ notification: Notification) {
        guard
            let application = notification.userInfo?[NSWorkspace.applicationUserInfoKey]
                as? NSRunningApplication,
            handledStatePolicy.shouldMarkHandledOnActivatedApplication(
                bundleIdentifier: application.bundleIdentifier
            )
        else {
            return
        }

        markHandledIfNeeded()
    }

    private func update() {
        currentStatus = store.loadDisplayStatus()
        if handledStatePolicy.shouldMarkHandled(
            status: currentStatus,
            activeBundleIdentifier: NSWorkspace.shared.frontmostApplication?.bundleIdentifier
        ) {
            markHandledIfNeeded()
            return
        }

        render()
    }

    private func observeApplicationActivation() {
        NSWorkspace.shared.notificationCenter.addObserver(
            self,
            selector: #selector(applicationDidActivate),
            name: NSWorkspace.didActivateApplicationNotification,
            object: nil
        )
    }

    private func markHandledIfNeeded() {
        guard currentStatus.state == .needsAttention else {
            return
        }

        do {
            try store.markHandled()
            update()
        } catch {
            currentStatus = DisplayStatus(
                state: .error,
                title: "Codex status unavailable",
                detail: "Could not write the handled state.",
                cwd: nil,
                threadID: nil,
                turnID: nil,
                updatedAt: nil
            )
            render()
        }
    }

    private func render() {
        renderButton()
        renderMenu()
    }

    private func renderButton() {
        guard let button = statusItem.button else {
            return
        }
        let presentation = MenuBarPresentation(status: currentStatus)

        statusItem.length = NSStatusItem.squareLength
        button.title = presentation.title
        button.image = statusImage(for: presentation)
        button.imagePosition = .imageOnly
        button.contentTintColor = presentation.usesOriginalColorImage ? nil : tintColor(for: presentation.tint)
        button.toolTip = presentation.tooltip
    }

    private func renderMenu() {
        let menu = NSMenu()

        let titleItem = NSMenuItem(title: currentStatus.title, action: nil, keyEquivalent: "")
        titleItem.isEnabled = false
        menu.addItem(titleItem)

        let detailItem = NSMenuItem(title: currentStatus.detail, action: nil, keyEquivalent: "")
        detailItem.isEnabled = false
        menu.addItem(detailItem)

        if let cwd = currentStatus.cwd {
            let cwdItem = NSMenuItem(title: "Project: \(cwd)", action: nil, keyEquivalent: "")
            cwdItem.isEnabled = false
            menu.addItem(cwdItem)
        }

        if let updatedAt = currentStatus.updatedAt {
            let timeItem = NSMenuItem(title: "Updated: \(updatedAt)", action: nil, keyEquivalent: "")
            timeItem.isEnabled = false
            menu.addItem(timeItem)
        }

        menu.addItem(.separator())
        menu.addItem(actionItem(title: "Open Codex", action: #selector(openCodex)))

        if currentStatus.state == .needsAttention {
            menu.addItem(actionItem(title: "Mark Handled", action: #selector(markHandled)))
        }

        menu.addItem(actionItem(title: "Refresh", action: #selector(refreshNow)))
        menu.addItem(.separator())
        menu.addItem(actionItem(title: "Quit", action: #selector(quit)))

        statusItem.menu = menu
    }

    private func actionItem(title: String, action: Selector) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
        item.target = self
        return item
    }

    private func tintColor(for tint: MenuBarTint) -> NSColor {
        switch tint {
        case .idle:
            return .secondaryLabelColor
        case .attention:
            return .systemGreen
        case .warning:
            return .systemYellow
        }
    }

    private func statusImage(for presentation: MenuBarPresentation) -> NSImage? {
        if presentation.usesOriginalColorImage {
            return originalColorSymbolImage(
                systemName: presentation.iconName,
                color: tintColor(for: presentation.tint),
                accessibilityDescription: presentation.tooltip
            )
        }

        let image = NSImage(
            systemSymbolName: presentation.iconName,
            accessibilityDescription: presentation.tooltip
        )
        image?.isTemplate = true
        return image
    }

    private func originalColorSymbolImage(
        systemName: String,
        color: NSColor,
        accessibilityDescription: String
    ) -> NSImage? {
        let configuration = NSImage.SymbolConfiguration(pointSize: 16, weight: .semibold)
        guard let symbol = NSImage(
            systemSymbolName: systemName,
            accessibilityDescription: accessibilityDescription
        )?.withSymbolConfiguration(configuration) else {
            return nil
        }

        let size = NSSize(width: 18, height: 18)
        let image = NSImage(size: size)
        image.lockFocus()

        color.setFill()
        NSRect(origin: .zero, size: size).fill()

        let symbolRect = NSRect(
            x: (size.width - symbol.size.width) / 2.0,
            y: (size.height - symbol.size.height) / 2.0,
            width: symbol.size.width,
            height: symbol.size.height
        )
        symbol.draw(
            in: symbolRect,
            from: NSRect(origin: .zero, size: symbol.size),
            operation: .destinationIn,
            fraction: 1.0
        )

        image.unlockFocus()
        image.isTemplate = false
        image.accessibilityDescription = accessibilityDescription
        return image
    }
}
