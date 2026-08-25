import AppKit
import QuartzCore

/// The dot at the head of a session row that carries its state.
///
/// Every state is a circle and the colour is what tells them apart, with one
/// exception: `unknown` is drawn as a ring rather than a disc. Idle and unknown
/// are both grey — nothing else honest to colour them — and a filled grey dot for
/// each would make them the same dot. Story 001's rule is that an unreadable
/// state is never conflated with idle, so the ring is what keeps that true.
///
/// **Only the attention states breathe.** Working, idle and unknown hold still,
/// so movement anywhere on the surface means something is asking for the user
/// rather than merely reporting — which is what makes the pet readable from the
/// corner of an eye while another window is focused.
///
/// The breath is a Core Animation opacity pulse rather than a redraw on a timer.
/// It runs in the render server, so it costs no per-frame CPU in this process, it
/// keeps animating without the display clock, and it is smooth rather than the
/// one-second blink a timer could produce.
final class StateIndicatorView: NSView {
    static let size: CGFloat = 9

    private static let breathKey = "breath"
    private let dot = CAShapeLayer()
    private var state: SessionState

    override var intrinsicContentSize: NSSize {
        NSSize(width: Self.size, height: Self.size)
    }

    /// Sit the dot on the text baseline, so a row whose project name wraps to two
    /// lines keeps its dot beside the *first* line rather than centred against the
    /// whole block.
    override var firstBaselineOffsetFromTop: CGFloat { Self.size }

    init(state: SessionState) {
        self.state = state
        super.init(frame: NSRect(x: 0, y: 0, width: Self.size, height: Self.size))
        wantsLayer = true
        layer?.addSublayer(dot)
        setContentHuggingPriority(.required, for: .horizontal)
        setContentCompressionResistancePriority(.required, for: .horizontal)
        redraw()
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    func apply(_ state: SessionState) {
        guard state != self.state else { return }
        self.state = state
        redraw()
    }

    override func layout() {
        super.layout()
        redraw()
    }

    /// Appearance changes (light to dark, or the panel's material shifting) do not
    /// re-resolve a `CGColor` that was captured earlier, so the dot is rebuilt.
    override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance()
        redraw()
    }

    private func redraw() {
        let box = bounds.insetBy(dx: 0.5, dy: 0.5)
        dot.frame = bounds

        // Geometry changes must not animate: a state flip should read as an
        // instant change of colour, not a shape sliding into place.
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        if state == .unknown {
            dot.path = CGPath(ellipseIn: box.insetBy(dx: 0.75, dy: 0.75), transform: nil)
            dot.fillColor = nil
            dot.strokeColor = tint.cgColor
            dot.lineWidth = 1.5
        } else {
            dot.path = CGPath(ellipseIn: box, transform: nil)
            dot.fillColor = tint.cgColor
            dot.strokeColor = nil
            dot.lineWidth = 0
        }
        CATransaction.commit()

        setBreathing(state.wantsAttention)
    }

    private func setBreathing(_ on: Bool) {
        guard on else {
            dot.removeAnimation(forKey: Self.breathKey)
            dot.opacity = 1
            return
        }
        guard dot.animation(forKey: Self.breathKey) == nil else { return }
        let breath = CABasicAnimation(keyPath: "opacity")
        breath.fromValue = 1.0
        breath.toValue = 0.28
        breath.duration = 1.1
        breath.autoreverses = true
        breath.repeatCount = .infinity
        breath.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
        dot.add(breath, forKey: Self.breathKey)
    }

    /// With shape no longer distinguishing the states, colour is doing the whole
    /// job. System colours are used rather than fixed values so they follow the
    /// appearance the panel is drawn in.
    private var tint: NSColor {
        switch state {
        case .working: return .systemGreen
        case .idle: return .tertiaryLabelColor
        case .waiting: return .systemOrange
        case .errored: return .systemRed
        case .unknown: return .secondaryLabelColor
        }
    }
}
