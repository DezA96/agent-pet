import AppKit
import QuartzCore

/// The creature under the bubble: what the whole surface adds up to, in one shape.
///
/// It is a **placeholder**, and knowingly so — the story records a properly drawn
/// creature as later work (C-022). What it is not a placeholder for is the
/// vocabulary: five expressions plus sleep, all told by pose and face, none told
/// by colour. Colour stays the row dots' job, so the creature reads the same over
/// a green working surface and a red errored one and never becomes a second,
/// blurrier copy of the dots.
///
/// Drawn as vector paths in Core Animation layers rather than images: `build.sh`
/// is swiftc and cargo, and an asset pipeline is not something it should grow for
/// a creature that is going to be redrawn anyway.

/// What the creature is doing. Five states plus the one that is not a state.
///
/// `asleep` is not a `SessionState` because no session is in it: it is the surface
/// having nothing to watch, which is a different thing to say than a session that
/// has finished. A sleeping pet and an idle one must not look the same.
enum Expression: Equatable {
    case errored, waiting, working, unknown, idle, asleep

    init(_ state: SessionState) {
        switch state {
        case .errored: self = .errored
        case .waiting: self = .waiting
        case .working: self = .working
        case .unknown: self = .unknown
        case .idle: self = .idle
        }
    }

    /// The same rhythm the row dots use, from the same definition; a sleeping
    /// creature holds still, as the settled states do.
    var breath: (period: Double, floor: Float)? {
        switch self {
        case .errored: return SessionState.errored.breath
        case .waiting: return SessionState.waiting.breath
        case .working: return SessionState.working.breath
        case .unknown, .idle, .asleep: return nil
        }
    }

    fileprivate var eyes: Eyes {
        switch self {
        case .errored: return .cross
        case .waiting: return .wide
        case .working: return .focus
        case .unknown: return .hollow
        case .idle: return .happy
        case .asleep: return .shut
        }
    }

    fileprivate var mouth: Mouth {
        switch self {
        case .errored: return .wavy
        case .waiting: return .o
        case .working, .unknown: return .flat
        case .idle: return .smile
        case .asleep: return .tiny
        }
    }
}

private enum Eyes { case focus, wide, cross, hollow, happy, shut }
private enum Mouth { case flat, o, wavy, smile, tiny }

// MARK: - Paths, in a 64 × 56 design space with y up and the feet on y = 2

private func pt(_ x: CGFloat, _ y: CGFloat) -> CGPoint { CGPoint(x: x, y: y) }
private func rad(_ degrees: CGFloat) -> CGFloat { degrees * .pi / 180 }

private func path(_ build: (CGMutablePath) -> Void) -> CGPath {
    let p = CGMutablePath()
    build(p)
    return p
}

private func circle(_ c: CGPoint, _ r: CGFloat) -> CGPath {
    CGPath(ellipseIn: CGRect(x: c.x - r, y: c.y - r, width: 2 * r, height: 2 * r), transform: nil)
}

private func arc(_ c: CGPoint, _ r: CGFloat, _ from: CGFloat, _ to: CGFloat) -> CGPath {
    path { $0.addArc(center: c, radius: r, startAngle: rad(from), endAngle: rad(to), clockwise: false) }
}

private func polyline(_ points: [CGPoint]) -> CGPath {
    path { p in
        p.move(to: points[0])
        points.dropFirst().forEach { p.addLine(to: $0) }
    }
}

private func line(_ a: CGPoint, _ b: CGPoint) -> CGPath { polyline([a, b]) }

/// Ink and paper for one appearance.
///
/// Every part of the body is a paper fill under an ink outline rather than a bare
/// stroke: the creature floats in a transparent gap over whatever window happens
/// to be behind it, and an unfilled outline would have that window showing through
/// its face. Both colours are system colours, so the creature follows the
/// appearance the panel is drawn in without being told about it.
private struct Pen {
    static let width: CGFloat = 1.6

    let ink: CGColor
    let paper: CGColor

    private func shape(_ p: CGPath, fill: CGColor?, stroke: CGColor?, width: CGFloat) -> CAShapeLayer {
        let l = CAShapeLayer()
        l.path = p
        l.fillColor = fill
        l.strokeColor = stroke
        l.lineWidth = width
        l.lineCap = .round
        l.lineJoin = .round
        return l
    }

    func stroke(_ p: CGPath, width: CGFloat = Pen.width) -> CALayer {
        shape(p, fill: nil, stroke: ink, width: width)
    }

    func body(_ p: CGPath) -> CALayer {
        shape(p, fill: paper, stroke: ink, width: Pen.width)
    }

    func dot(_ c: CGPoint, _ r: CGFloat) -> CALayer {
        shape(circle(c, r), fill: ink, stroke: nil, width: 0)
    }

    // MARK: The face

    func eye(_ kind: Eyes, at c: CGPoint, scale s: CGFloat) -> [CALayer] {
        switch kind {
        case .focus:
            // A pupil with a straight lid across its top: half-lidded, on the job.
            return [
                dot(c, 2 * s),
                stroke(line(pt(c.x - 2.6 * s, c.y + 1.3 * s), pt(c.x + 2.6 * s, c.y + 1.3 * s)), width: 1.5),
            ]
        case .wide:
            return [
                stroke(circle(c, 3.1 * s), width: 1.4),
                dot(c, 1.4 * s),
                stroke(arc(pt(c.x, c.y + 3.2 * s), 3 * s, 40, 140), width: 1.3),
            ]
        case .cross:
            let d = 2.4 * s
            return [
                stroke(line(pt(c.x - d, c.y - d), pt(c.x + d, c.y + d)), width: 1.8),
                stroke(line(pt(c.x - d, c.y + d), pt(c.x + d, c.y - d)), width: 1.8),
            ]
        case .hollow:
            // A ring with nothing in it: the dot's own mark for unknown.
            return [stroke(circle(c, 2.6 * s), width: 1.4)]
        case .happy:
            return [stroke(arc(pt(c.x, c.y - 0.8 * s), 2.8 * s, 20, 160), width: 1.6)]
        case .shut:
            return [stroke(arc(pt(c.x, c.y + 1.4 * s), 2.8 * s, 200, 340), width: 1.4)]
        }
    }

    func mouth(_ kind: Mouth, at c: CGPoint, scale s: CGFloat) -> [CALayer] {
        switch kind {
        case .flat:
            return [stroke(line(pt(c.x - 2.5 * s, c.y), pt(c.x + 2.5 * s, c.y)), width: 1.4)]
        case .tiny:
            return [stroke(line(pt(c.x - 1.2 * s, c.y), pt(c.x + 1.2 * s, c.y)), width: 1.3)]
        case .o:
            return [stroke(circle(c, 1.8 * s), width: 1.3)]
        case .smile:
            return [stroke(arc(pt(c.x, c.y + 1.5 * s), 3.2 * s, 205, 335), width: 1.5)]
        case .wavy:
            let w = 4 * s, a = 1.3 * s
            return [stroke(polyline([
                pt(c.x - w, c.y - a), pt(c.x - w / 2, c.y + a), pt(c.x, c.y - a),
                pt(c.x + w / 2, c.y + a), pt(c.x + w, c.y - a),
            ]), width: 1.4)]
        }
    }

    func face(_ e: Expression, at c: CGPoint, spread: CGFloat, scale s: CGFloat, eyeShift: CGPoint = .zero) -> [CALayer] {
        let left = pt(c.x - spread + eyeShift.x, c.y + eyeShift.y)
        let right = pt(c.x + spread + eyeShift.x, c.y + eyeShift.y)
        return eye(e.eyes, at: left, scale: s)
            + eye(e.eyes, at: right, scale: s)
            + mouth(e.mouth, at: pt(c.x, c.y - 6 * s), scale: s)
    }

    /// Two Zs floating up and away from a sleeper's head.
    func zs(at p: CGPoint) -> [CALayer] {
        func z(_ o: CGPoint, _ s: CGFloat) -> CGPath {
            polyline([pt(o.x, o.y + s), pt(o.x + s, o.y + s), pt(o.x, o.y), pt(o.x + s, o.y)])
        }
        return [stroke(z(p, 3.5), width: 1.3), stroke(z(pt(p.x + 5, p.y + 6), 5), width: 1.4)]
    }
}

/// A group of layers posed as one, pivoting on the feet.
private func group(
    _ children: [CALayer],
    pivot: CGPoint = pt(32, 2),
    transform: CATransform3D = CATransform3DIdentity
) -> CALayer {
    let g = CALayer()
    g.bounds = CGRect(origin: .zero, size: CreatureView.size)
    g.anchorPoint = CGPoint(x: pivot.x / CreatureView.size.width, y: pivot.y / CreatureView.size.height)
    g.position = pivot
    g.transform = transform
    children.forEach(g.addSublayer)
    return g
}

private func pose(rotate degrees: CGFloat = 0, sx: CGFloat = 1, sy: CGFloat = 1) -> CATransform3D {
    CATransform3DRotate(CATransform3DMakeScale(sx, sy, 1), rad(degrees), 0, 0, 1)
}

/// The Blob: a dome with two nub feet.
///
/// Everything is carried by the face and by squashing or tilting the one shape,
/// which is what makes it cheap enough to be a placeholder and still legible at 56
/// points — the prototype's finding, against a cat and an upright figure.
private func blob(_ e: Expression, pen: Pen) -> CALayer {
    let dome = path { p in
        p.move(to: pt(20, 2))
        p.addLine(to: pt(44, 2))
        p.addArc(center: pt(44, 10), radius: 8, startAngle: rad(-90), endAngle: 0, clockwise: false)
        // The dome tops out at 50, not 56: the waiting stretch needs the headroom,
        // or its crown runs into the bubble's tail.
        p.addLine(to: pt(52, 30))
        p.addArc(center: pt(32, 30), radius: 20, startAngle: 0, endAngle: rad(180), clockwise: false)
        p.addLine(to: pt(12, 10))
        p.addArc(center: pt(20, 10), radius: 8, startAngle: rad(180), endAngle: rad(270), clockwise: false)
        p.closeSubpath()
    }
    let feet = [pen.body(circle(pt(23, 3.5), 3.5)), pen.body(circle(pt(41, 3.5), 3.5))]
    let faceCentre = pt(32, 35)

    var parts = feet + [pen.body(dome)]
    var extras: [CALayer] = []
    let posed: CATransform3D

    switch e {
    case .working:
        posed = pose(rotate: 7, sy: 0.97)
        parts += pen.face(e, at: faceCentre, spread: 7, scale: 1.1, eyeShift: pt(-1.5, -2))
    case .waiting:
        posed = pose(sx: 0.94, sy: 1.08)
        parts += pen.face(e, at: faceCentre, spread: 7.5, scale: 1.1)
    case .errored:
        posed = pose(rotate: -6, sx: 1.12, sy: 0.84)
        parts += pen.face(e, at: faceCentre, spread: 7.5, scale: 1.1)
    case .unknown:
        posed = pose(rotate: 12)
        parts += pen.face(e, at: faceCentre, spread: 7, scale: 1.1)
        parts.append(pen.stroke(line(pt(36.5, 42), pt(42, 44)), width: 1.4)) // one raised brow
    case .idle:
        posed = CATransform3DIdentity
        parts += pen.face(e, at: faceCentre, spread: 7, scale: 1.1)
    case .asleep:
        posed = pose(rotate: -4, sx: 1.08, sy: 0.86)
        parts += pen.face(e, at: pt(32, 33), spread: 7, scale: 1.1)
        extras = pen.zs(at: pt(49, 38))
    }
    return group([group(parts, transform: posed)] + extras)
}

// MARK: - Hosting one drawn creature, breathing

/// The creature on the surface, and the pet's only drag handle.
///
/// The view is transparent apart from what the creature paints, which is what
/// makes the gap around it click through to the window beneath while the creature
/// itself catches a press. Nothing here decides where the pet goes: it reports the
/// drag and the controller does the arithmetic, so the side rule lives in one
/// place whether the pet is dragged, launched or rescued.
final class CreatureView: NSView {
    static let size = CGSize(width: 64, height: 56)

    /// A press landed on the creature. The controller notes where the pet is.
    var onDragBegin: (() -> Void)?
    /// Reports the pointer's movement since the press began, in screen points.
    var onDrag: ((CGSize) -> Void)?
    /// The press ended. The only moment the pet's position is written down.
    var onDragEnd: (() -> Void)?

    private var root = CALayer()
    private var shown: Expression?
    private var dragStart: NSPoint?

    override var intrinsicContentSize: NSSize { Self.size }

    init() {
        super.init(frame: CGRect(origin: .zero, size: Self.size))
        wantsLayer = true
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    /// An expression change is instant, exactly as a dot's colour change is —
    /// nothing tweens between two poses, because a state flip is not a journey.
    /// An unchanged expression is left alone so its breath is not restarted every
    /// poll, which would show as a stutter.
    func apply(_ expression: Expression) {
        guard expression != shown else { return }
        shown = expression
        redraw()
    }

    /// A `CGColor` captured earlier does not re-resolve when the appearance
    /// changes, so the creature is rebuilt rather than recoloured.
    override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance()
        redraw()
    }

    private func redraw() {
        guard let shown else { return }
        root.removeFromSuperlayer()

        // `NSColor.cgColor` resolves against whatever drawing appearance is current,
        // which inside a timer callback is ambient rather than this view's. Pinning
        // it makes the two colours a function of where the creature actually is;
        // getting it wrong would not fail loudly, it would just invert the creature.
        var ink = NSColor.labelColor.cgColor
        var paper = NSColor.windowBackgroundColor.cgColor
        effectiveAppearance.performAsCurrentDrawingAppearance {
            ink = NSColor.labelColor.cgColor
            paper = NSColor.windowBackgroundColor.cgColor
        }

        root = blob(shown, pen: Pen(ink: ink, paper: paper))
        root.anchorPoint = CGPoint(x: 0.5, y: 0)
        root.bounds = CGRect(origin: .zero, size: Self.size)
        root.position = CGPoint(x: Self.size.width / 2, y: 0)

        let backing = window?.backingScaleFactor ?? 2
        func crisp(_ l: CALayer) {
            l.contentsScale = backing
            l.sublayers?.forEach(crisp)
        }
        crisp(root)

        // Pose and geometry are set, not animated: only the breath moves.
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        layer?.addSublayer(root)
        CATransaction.commit()

        breathe(shown.breath)
    }

    /// The breath is a squash rather than a fade.
    ///
    /// The dot spends its `floor` as opacity, which works because a 9-point disc at
    /// 0.28 alpha is still a coloured disc. A line drawing at 0.28 over a busy
    /// window is very nearly nothing — and the urgent floor is exactly the moment
    /// the creature must not be missable. Squashing keeps it solid throughout while
    /// the rhythm, which is what a glance actually reads, is the same.
    ///
    /// Core Animation runs it in the render server, so it costs this process no
    /// per-frame work and keeps time without the display clock.
    private func breathe(_ breath: (period: Double, floor: Float)?) {
        root.removeAllAnimations()
        guard let breath else { return }
        // Depth follows the same urgency the dot's opacity floor does.
        let depth: CGFloat = breath.floor < 0.4 ? 0.07 : 0.035

        func pulse(_ keyPath: String, to: CGFloat) {
            let a = CABasicAnimation(keyPath: keyPath)
            a.fromValue = CGFloat(1)
            a.toValue = to
            a.duration = breath.period
            a.autoreverses = true
            a.repeatCount = .infinity
            a.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
            root.add(a, forKey: keyPath)
        }
        // Taller and narrower on the in-breath, pivoting on the feet, so the
        // creature swells in place rather than sliding.
        pulse("transform.scale.y", to: 1 + depth)
        pulse("transform.scale.x", to: 1 - depth / 2)
    }

    // MARK: - The drag handle

    /// Dragged by hand rather than by `performDrag`, which owns the frame for the
    /// length of the gesture and so cannot let the bubble flip sides mid-drag. Both
    /// this and `performDrag` move the window without activating the app, so the
    /// focus guarantee is unaffected.
    override func mouseDown(with event: NSEvent) {
        dragStart = NSEvent.mouseLocation
        onDragBegin?()
    }

    override func mouseDragged(with event: NSEvent) {
        guard let dragStart else { return }
        let now = NSEvent.mouseLocation
        onDrag?(CGSize(width: now.x - dragStart.x, height: now.y - dragStart.y))
    }

    override func mouseUp(with event: NSEvent) {
        guard dragStart != nil else { return }
        dragStart = nil
        onDragEnd?()
    }
}
