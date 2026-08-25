import AppKit

/// The mark at the head of a session row that carries its state.
///
/// **Shape carries the state; colour only reinforces it.** Two reasons, both from
/// this surface rather than from general principle. The panel is translucent over
/// whatever window happens to be behind it, so no colour can be relied on to hold
/// its contrast — story 001 verified rows over both dark and light foregrounds.
/// And colour alone is unreadable to a viewer who cannot separate red from green.
/// A silhouette survives both: filled disc, hollow ring, triangle, cross, diamond
/// are told apart at a glance and at this size.
///
/// **Only the attention states move.** Working, idle and unknown are drawn still,
/// so any movement on the surface means something wants the user — which is what
/// makes the pet readable from the corner of an eye while another window is
/// focused. Motion costs nothing extra: it advances on the one-second display
/// timer the age counters already run on, so there is no second clock and no
/// measurable CPU beyond what the surface was spending anyway.
final class StateIndicatorView: NSView {
    static let size: CGFloat = 9

    private var state: SessionState = .unknown
    /// Which half of the two-frame pulse the attention states are showing.
    private var pulseOn = true

    override var intrinsicContentSize: NSSize {
        NSSize(width: Self.size, height: Self.size)
    }

    override var isFlipped: Bool { false }

    init(state: SessionState) {
        super.init(frame: NSRect(x: 0, y: 0, width: Self.size, height: Self.size))
        self.state = state
        setContentHuggingPriority(.required, for: .horizontal)
        setContentCompressionResistancePriority(.required, for: .horizontal)
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    func apply(_ state: SessionState) {
        guard state != self.state else { return }
        self.state = state
        // A state change restarts the pulse at full strength, so a row that has
        // just started asking for attention is never caught mid-fade.
        pulseOn = true
        needsDisplay = true
    }

    /// Advance the pulse one frame. Called from the surface's one-second timer.
    ///
    /// A still state redraws nothing: the great majority of rows are working or
    /// idle, and they must not pay for a feature they do not use.
    func advance() {
        guard state.wantsAttention else { return }
        pulseOn.toggle()
        needsDisplay = true
    }

    override func draw(_ dirtyRect: NSRect) {
        let box = bounds.insetBy(dx: 0.5, dy: 0.5)
        var colour = tint
        if state.wantsAttention && !pulseOn {
            colour = colour.withAlphaComponent(0.3)
        }
        colour.setFill()
        colour.setStroke()

        switch state {
        case .working:
            NSBezierPath(ovalIn: box).fill()
        case .idle:
            let ring = NSBezierPath(ovalIn: box.insetBy(dx: 0.75, dy: 0.75))
            ring.lineWidth = 1.5
            ring.stroke()
        case .waiting:
            triangle(in: box).fill()
        case .errored:
            cross(in: box).stroke()
        case .unknown:
            let diamond = self.diamond(in: box)
            diamond.lineWidth = 1.3
            diamond.stroke()
        }
    }

    /// The colour is a second channel, never the only one.
    ///
    /// System colours are used rather than fixed values so they follow the
    /// appearance the panel is drawn in, and the two calm states borrow the same
    /// label colours the rest of the row already uses.
    private var tint: NSColor {
        switch state {
        case .working: return .labelColor
        case .idle: return .tertiaryLabelColor
        case .waiting: return .systemOrange
        case .errored: return .systemRed
        case .unknown: return .tertiaryLabelColor
        }
    }

    /// Pointing right — the row is handing the turn back to the user.
    private func triangle(in box: NSRect) -> NSBezierPath {
        let p = NSBezierPath()
        p.move(to: NSPoint(x: box.minX + 0.5, y: box.maxY))
        p.line(to: NSPoint(x: box.maxX, y: box.midY))
        p.line(to: NSPoint(x: box.minX + 0.5, y: box.minY))
        p.close()
        return p
    }

    private func cross(in box: NSRect) -> NSBezierPath {
        let p = NSBezierPath()
        let inset = box.insetBy(dx: 1, dy: 1)
        p.move(to: NSPoint(x: inset.minX, y: inset.minY))
        p.line(to: NSPoint(x: inset.maxX, y: inset.maxY))
        p.move(to: NSPoint(x: inset.minX, y: inset.maxY))
        p.line(to: NSPoint(x: inset.maxX, y: inset.minY))
        p.lineWidth = 1.6
        p.lineCapStyle = .round
        return p
    }

    private func diamond(in box: NSRect) -> NSBezierPath {
        let p = NSBezierPath()
        p.move(to: NSPoint(x: box.midX, y: box.maxY))
        p.line(to: NSPoint(x: box.maxX, y: box.midY))
        p.line(to: NSPoint(x: box.midX, y: box.minY))
        p.line(to: NSPoint(x: box.minX, y: box.midY))
        p.close()
        return p
    }
}
