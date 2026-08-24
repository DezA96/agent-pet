import AppKit

/// The floating surface.
///
/// `.nonactivatingPanel`, with `canBecomeKey` and `canBecomeMain` both false, is
/// what makes the pet never take keyboard focus — including when it is clicked.
/// That is an AppKit primitive here rather than behaviour emulated on top of a
/// window that would otherwise steal focus.
///
/// The pet deliberately does *not* pass clicks through. It used to, and that made
/// it an invisible trap: it sits above whatever is behind it, so a click landed on
/// controls the user could not see — once on a window's close button, ending a
/// live session. Catching its own clicks costs the small area it occupies and
/// removes the hazard, and it is what makes the surface draggable at all.
final class PetPanel: NSPanel {
    init(contentRect: NSRect) {
        super.init(
            contentRect: contentRect,
            styleMask: [.nonactivatingPanel, .borderless],
            backing: .buffered,
            defer: false
        )
        isFloatingPanel = true
        level = .statusBar
        // Clicks stop here rather than reaching whatever is underneath.
        ignoresMouseEvents = false
        isOpaque = false
        backgroundColor = .clear
        hasShadow = true
        hidesOnDeactivate = false
        // Visible over full-screen apps and on whatever Space is in front, so the
        // pet is readable whatever window is focused.
        collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary, .ignoresCycle]
    }

    // A borderless panel is not key or main by default; keep it that way.
    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}

extension String {
    /// Raise the first letter only, leaving the rest as the agent wrote it.
    ///
    /// Every line on the surface reads as a sentence, including activity text the
    /// agent supplied in lower case.
    var sentenceCased: String {
        guard let first = first else { return self }
        return first.uppercased() + dropFirst()
    }
}

/// Shared look for every line of text on the surface.
enum Style {
    static let rowFont = NSFont.systemFont(ofSize: 12, weight: .medium)
    static let detailFont = NSFont.monospacedDigitSystemFont(ofSize: 11, weight: .regular)
    static let primary = NSColor.labelColor
    static let secondary = NSColor.secondaryLabelColor
    static let padding: CGFloat = 12
    static let width: CGFloat = 300

    static func label(_ text: String, font: NSFont, color: NSColor) -> NSTextField {
        let l = NSTextField(labelWithString: text)
        l.font = font
        l.textColor = color
        l.lineBreakMode = .byTruncatingTail
        l.cell?.truncatesLastVisibleLine = true
        return l
    }
}
