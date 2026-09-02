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

/// The speech bubble: the rows, with a tail pointing down at the creature.
///
/// It catches its own clicks and does nothing with them. Passing them through
/// would put it back in the trap story 001 hit — it floats over controls the user
/// cannot see — and making it a drag handle would mean a press on a row moved the
/// pet, which is not what a row is for. The creature is the handle; the bubble is
/// a thing to read. What a click on a row *should* eventually do is C-021.
///
/// The tail is cut into the material with a mask rather than drawn on top of it,
/// so the blur, the corner radius and the tail are one shape and the tail carries
/// the same background as the body.
final class BubbleView: NSVisualEffectView {
    /// How far the tail hangs below the bubble's body, toward the creature.
    static let tail: CGFloat = 8
    private static let radius: CGFloat = 10
    private static let halfWidth: CGFloat = 9

    override init(frame: NSRect) {
        super.init(frame: frame)
        material = .hudWindow
        blendingMode = .behindWindow
        state = .active
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    /// Point the tail at `tipX`, in this view's own coordinates.
    func pointTail(at tipX: CGFloat) {
        let tail = Self.tail, half = Self.halfWidth, radius = Self.radius
        maskImage = NSImage(size: frame.size, flipped: false) { rect in
            let body = NSBezierPath(
                roundedRect: CGRect(x: 0, y: tail, width: rect.width, height: rect.height - tail),
                xRadius: radius,
                yRadius: radius
            )
            // Meeting the body a point above its edge, so the join has no seam.
            body.move(to: CGPoint(x: tipX - half, y: tail + 1))
            body.line(to: CGPoint(x: tipX, y: 0))
            body.line(to: CGPoint(x: tipX + half, y: tail + 1))
            body.close()
            NSColor.black.setFill()
            body.fill()
            return true
        }
    }
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
    /// The bubble's width, which is the whole surface's width: the creature tucks
    /// under one of its corners rather than sitting beside it.
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
