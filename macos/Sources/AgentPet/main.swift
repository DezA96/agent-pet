import AppKit

/// The pet is an accessory: no dock icon, nothing in the app switcher. Its only
/// chrome is a menu bar item — enough to move it out of the way or quit it, which
/// an app with no dock icon otherwise has no visible way to do.
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
