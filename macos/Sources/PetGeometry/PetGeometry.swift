import CoreGraphics
import Foundation

/// Where the pet is allowed to sit, and how it behaves as it changes size.
///
/// Deliberately free functions over `CGRect` with no `NSWindow`, no `NSScreen`
/// and no FFI: this is the only arithmetic in the surface that can be wrong in a
/// way you would not see immediately, and keeping it independent of AppKit is
/// what makes it testable at all. Screens enter as their visible frames.

/// Which vertical edge the pet holds still as session rows come and go.
enum VerticalAnchor: String {
    case top, bottom
}

/// A remembered position, stored against the edge the pet is anchored to.
///
/// The height changes with the number of live sessions, so remembering the
/// bottom-left origin alone would drift: quit with two rows, relaunch with none,
/// and a top-anchored pet would come back lower than it was left. Storing the
/// anchored edge keeps the edge the user actually aligned against.
struct StoredPosition: Equatable {
    var x: CGFloat
    var edgeY: CGFloat
    var anchor: VerticalAnchor
}

/// A remembered position is worth restoring only if this much of it lands on a
/// screen. Small enough that parking the pet against an edge is still allowed,
/// large enough that a pet left on a display that is now gone is not.
let minimumVisibleExtent = CGSize(width: 40, height: 40)

/// Distance from the screen's visible edges at the default position.
let defaultInset: CGFloat = 16

/// The pet grows away from whichever horizontal half it sits in.
///
/// Always pinning the top — correct when the pet was fixed to the top-right —
/// walks a bottom-placed pet off the screen as rows appear. Clamping back into
/// the screen instead would move the pet away from where it was put every time a
/// session starts, which is the thing this story exists to stop.
func anchor(for frame: CGRect, in visible: CGRect) -> VerticalAnchor {
    frame.midY >= visible.midY ? .top : .bottom
}

/// Resize in place, holding the anchored edge still.
func resized(_ frame: CGRect, to size: CGSize, anchoredAt anchor: VerticalAnchor) -> CGRect {
    let y: CGFloat
    switch anchor {
    case .top: y = frame.maxY - size.height
    case .bottom: y = frame.minY
    }
    return CGRect(x: frame.minX, y: y, width: size.width, height: size.height)
}

/// Reduce a frame to what gets remembered.
func stored(_ frame: CGRect, in visible: CGRect) -> StoredPosition {
    let anchor = anchor(for: frame, in: visible)
    return StoredPosition(
        x: frame.minX,
        edgeY: anchor == .top ? frame.maxY : frame.minY,
        anchor: anchor
    )
}

/// Rebuild a frame from what was remembered, at whatever size the pet is now.
func frame(for stored: StoredPosition, size: CGSize) -> CGRect {
    let y: CGFloat
    switch stored.anchor {
    case .top: y = stored.edgeY - size.height
    case .bottom: y = stored.edgeY
    }
    return CGRect(x: stored.x, y: y, width: size.width, height: size.height)
}

/// Is this frame still usable on the screens that exist right now?
///
/// Any single screen has to show enough of it. Summing overlaps across screens
/// would let two slivers on two displays count as a visible pet, which they are
/// not.
func isRestorable(_ frame: CGRect, on visibleFrames: [CGRect]) -> Bool {
    visibleFrames.contains { screen in
        let overlap = frame.intersection(screen)
        return !overlap.isNull
            && overlap.width >= minimumVisibleExtent.width
            && overlap.height >= minimumVisibleExtent.height
    }
}

/// Top-right of the given screen, clear of its edges. Where the pet starts when
/// it has never been moved, and where it is sent when its remembered position no
/// longer lands on any screen.
func defaultFrame(size: CGSize, in visible: CGRect) -> CGRect {
    CGRect(
        x: visible.maxX - size.width - defaultInset,
        y: visible.maxY - size.height - defaultInset,
        width: size.width,
        height: size.height
    )
}

/// Where the pet belongs, given what was remembered and what screens exist.
///
/// One entry point so launch and a live display change cannot drift apart: both
/// ask this same question.
func placement(
    remembered: StoredPosition?,
    size: CGSize,
    visibleFrames: [CGRect],
    primary: CGRect
) -> CGRect {
    guard let remembered else { return defaultFrame(size: size, in: primary) }
    let candidate = frame(for: remembered, size: size)
    return isRestorable(candidate, on: visibleFrames)
        ? candidate
        : defaultFrame(size: size, in: primary)
}
