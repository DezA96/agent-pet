import CoreGraphics
import Foundation

/// Where the pet is allowed to sit, and how it behaves as it changes size.
///
/// Deliberately free functions over `CGRect` with no `NSWindow`, no `NSScreen`
/// and no FFI: this is the only arithmetic in the surface that can be wrong in a
/// way you would not see immediately, and keeping it independent of AppKit is
/// what makes it testable at all. Screens enter as their visible frames.
///
/// The surface is a creature with a speech bubble over it. The window's frame is
/// the two together: the bubble's width is the frame's width, the frame's top is
/// the bubble's top, and the frame's bottom is the creature's bottom. The creature
/// sits under one of the bubble's bottom corners, so the frame is pinned to the
/// creature at whichever end the bubble is *not* extending toward.

/// Which vertical edge the pet holds still as session rows come and go.
enum VerticalAnchor: String {
    case top, bottom
}

/// Which way the bubble extends from the creature.
///
/// Named for the bubble rather than the creature because it is the bubble that
/// moves: `.left` means the bubble runs off to the creature's left, which is the
/// same thing as the creature sitting under the bubble's *right* corner.
enum BubbleSide: String {
    case left, right

    var flipped: BubbleSide { self == .left ? .right : .left }
}

/// A remembered position, stored against the edge the pet is anchored to.
///
/// The height changes with the number of live sessions, so remembering the
/// bottom-left origin alone would drift: quit with two rows, relaunch with none,
/// and a top-anchored pet would come back lower than it was left. Storing the
/// anchored edge keeps the edge the user actually aligned against.
///
/// `x` is the *creature's* left edge, not the frame's. The creature is the part
/// the user put somewhere; the bubble is whatever hangs off it, and which way it
/// hangs can change between one launch and the next.
struct StoredPosition: Equatable {
    var x: CGFloat
    var edgeY: CGFloat
    var anchor: VerticalAnchor
    var side: BubbleSide
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

// MARK: - The creature inside the frame

/// Where the creature sits inside a frame, given which way the bubble runs.
func creatureRect(in frame: CGRect, side: BubbleSide, size: CGSize) -> CGRect {
    CGRect(
        x: side == .left ? frame.maxX - size.width : frame.minX,
        y: frame.minY,
        width: size.width,
        height: size.height
    )
}

/// The frame holding a creature at `creature` with a bubble of this height above
/// it. `bubbleHeight` includes the tail.
func petFrame(
    creature: CGRect,
    side: BubbleSide,
    bubbleWidth: CGFloat,
    bubbleHeight: CGFloat
) -> CGRect {
    CGRect(
        x: side == .left ? creature.maxX - bubbleWidth : creature.minX,
        y: creature.minY,
        width: bubbleWidth,
        height: creature.height + bubbleHeight
    )
}

/// Which side the bubble takes.
///
/// The rule is *stay unless you would fall off*, never *swap at the midpoint*:
/// midpoint switching makes the bubble jump sides in the middle of the screen
/// where both sides fit perfectly well, which reads as the pet twitching. Neither
/// side fitting keeps the current one, so a screen narrower than the bubble gets a
/// pet that stays put rather than one that oscillates.
func bubbleSide(
    creature: CGRect,
    current: BubbleSide,
    bubbleWidth: CGFloat,
    in visible: CGRect
) -> BubbleSide {
    func fits(_ side: BubbleSide) -> Bool {
        let frame = petFrame(creature: creature, side: side, bubbleWidth: bubbleWidth, bubbleHeight: 0)
        return frame.minX >= visible.minX && frame.maxX <= visible.maxX
    }
    if fits(current) { return current }
    return fits(current.flipped) ? current.flipped : current
}

// MARK: - Remembering across a relaunch

/// Reduce a frame to what gets remembered.
func stored(
    _ frame: CGRect,
    side: BubbleSide,
    creatureSize: CGSize,
    in visible: CGRect
) -> StoredPosition {
    let anchor = anchor(for: frame, in: visible)
    return StoredPosition(
        x: creatureRect(in: frame, side: side, size: creatureSize).minX,
        edgeY: anchor == .top ? frame.maxY : frame.minY,
        anchor: anchor,
        side: side
    )
}

/// Rebuild a frame from what was remembered, at whatever size the pet is now.
func frame(for stored: StoredPosition, creatureSize: CGSize, bubbleWidth: CGFloat, bubbleHeight: CGFloat) -> CGRect {
    let height = creatureSize.height + bubbleHeight
    let y: CGFloat
    switch stored.anchor {
    case .top: y = stored.edgeY - height
    case .bottom: y = stored.edgeY
    }
    let creature = CGRect(x: stored.x, y: y, width: creatureSize.width, height: creatureSize.height)
    return petFrame(
        creature: creature,
        side: stored.side,
        bubbleWidth: bubbleWidth,
        bubbleHeight: bubbleHeight
    )
}

/// Is this rect still usable on the screens that exist right now?
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

/// The visible frame showing most of `rect`, falling back to `fallback`.
func nearestVisible(to rect: CGRect, among visibleFrames: [CGRect], fallback: CGRect) -> CGRect {
    func overlapArea(_ screen: CGRect) -> CGFloat {
        let i = rect.intersection(screen)
        return i.isNull ? 0 : i.width * i.height
    }
    guard let best = visibleFrames.max(by: { overlapArea($0) < overlapArea($1) }),
          overlapArea(best) > 0
    else { return fallback }
    return best
}

/// Top-right of the given screen, clear of its edges. Where the pet starts when
/// it has never been moved, and where it is sent when its remembered position no
/// longer lands on any screen. The bubble runs left from there, so the creature
/// sits under its right corner.
func defaultFrame(size: CGSize, in visible: CGRect) -> CGRect {
    CGRect(
        x: visible.maxX - size.width - defaultInset,
        y: visible.maxY - size.height - defaultInset,
        width: size.width,
        height: size.height
    )
}

let defaultSide = BubbleSide.left

/// Where the pet belongs, given what was remembered and what screens exist.
///
/// One entry point so launch and a live display change cannot drift apart: both
/// ask this same question. The remembered side is honoured where it still fits
/// and re-picked where it does not — used from then on, but never written back,
/// because the user did not choose it.
///
/// Validity is judged on the *creature*, not on the frame: the creature is the
/// pet, and a bubble hanging off a screen edge is a thing the side rule fixes on
/// its own. A frame test would throw away a perfectly reachable creature because
/// its bubble had drifted.
/// `honoured` says whether the remembered position was actually used. A caller
/// that writes anything down needs it: a position rejected as unusable must not
/// be written back, or the pet remembers a place it just refused to go.
func placement(
    remembered: StoredPosition?,
    creatureSize: CGSize,
    bubbleWidth: CGFloat,
    bubbleHeight: CGFloat,
    visibleFrames: [CGRect],
    primary: CGRect
) -> (frame: CGRect, side: BubbleSide, honoured: Bool) {
    let size = CGSize(width: bubbleWidth, height: creatureSize.height + bubbleHeight)

    func fallback() -> (CGRect, BubbleSide, Bool) {
        (defaultFrame(size: size, in: primary), defaultSide, false)
    }

    guard let remembered else { return fallback() }
    let candidate = frame(for: remembered, creatureSize: creatureSize, bubbleWidth: bubbleWidth, bubbleHeight: bubbleHeight)
    let creature = creatureRect(in: candidate, side: remembered.side, size: creatureSize)
    guard isRestorable(creature, on: visibleFrames) else { return fallback() }

    let screen = nearestVisible(to: creature, among: visibleFrames, fallback: primary)
    let side = bubbleSide(creature: creature, current: remembered.side, bubbleWidth: bubbleWidth, in: screen)
    return (
        petFrame(creature: creature, side: side, bubbleWidth: bubbleWidth, bubbleHeight: bubbleHeight),
        side,
        true
    )
}

/// Where the creature goes when only story 002's frame was remembered.
///
/// That frame was the bubble alone, so its corners are the only thing to go on,
/// and the creature goes under the right one: the bubble then lands exactly where
/// the user last saw it. The left corner is not a real alternative — the bubble is
/// as wide as story 002's whole frame, so either corner puts the bubble in the
/// same place and only the creature would move. Where that bubble no longer fits,
/// `placement` re-picks the side on the way in, without moving the creature.
///
/// Only the horizontal half of the old position is reinterpreted. The anchored
/// edge keeps its meaning, so it is carried across untouched rather than derived.
///
/// Runs once, on the first launch after this story. The old key is read and left
/// alone; nothing is ever written back to it.
func derivedCreature(fromLegacyFrame legacy: CGRect, creatureWidth: CGFloat) -> (x: CGFloat, side: BubbleSide) {
    (legacy.maxX - creatureWidth, .left)
}
