import AppKit

/// The pet is an accessory: no dock icon, no menu bar, nothing that can be
/// switched to. It can be ignored entirely, which is the point.
final class AppDelegate: NSObject, NSApplicationDelegate {
    private var controller: PetController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        let controller = PetController()
        controller.start()
        self.controller = controller
    }
}

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let delegate = AppDelegate()
app.delegate = delegate
app.run()
