import AppKit

/// The pet's only chrome.
///
/// Controls live here rather than on the pet itself: the surface is small, it has
/// to stay readable at a glance, and buttons drawn on it would compete with the
/// status rows that are the whole point. It is also the only way to get an
/// accessory app back once hidden — there is no dock icon to click.
final class MenuBarController: NSObject {
    private let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private let toggleItem = NSMenuItem(title: "Hide Pet", action: nil, keyEquivalent: "")
    private let onToggle: () -> Void
    private let onQuit: () -> Void

    init(onToggle: @escaping () -> Void, onQuit: @escaping () -> Void) {
        self.onToggle = onToggle
        self.onQuit = onQuit
        super.init()

        item.button?.image = NSImage(
            systemSymbolName: "pawprint.fill",
            accessibilityDescription: "Agent Pet"
        )
        // Template images follow the menu bar through light and dark.
        item.button?.image?.isTemplate = true

        let menu = NSMenu()
        toggleItem.target = self
        toggleItem.action = #selector(toggleTapped)
        menu.addItem(toggleItem)
        menu.addItem(.separator())
        let quit = NSMenuItem(title: "Quit Agent Pet", action: #selector(quitTapped), keyEquivalent: "q")
        quit.target = self
        menu.addItem(quit)
        item.menu = menu
    }

    /// Say what the item will do next, not what the pet currently is.
    func showPetIsVisible(_ visible: Bool) {
        toggleItem.title = visible ? "Hide Pet" : "Show Pet"
    }

    @objc private func toggleTapped() { onToggle() }
    @objc private func quitTapped() { onQuit() }
}
