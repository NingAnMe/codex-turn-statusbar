import AppKit
import CodexTurnStatusCore

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private var controller: StatusBarController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        let controller = StatusBarController(store: StatusStore())
        self.controller = controller
        controller.start()
    }
}
