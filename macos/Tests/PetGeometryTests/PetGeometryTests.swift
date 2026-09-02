import CoreGraphics
import Testing

@testable import PetGeometry

/// A 1440×900 screen with the menu bar taken off the top, which is what
/// `NSScreen.visibleFrame` reports.
private let screen = CGRect(x: 0, y: 0, width: 1440, height: 875)
private let petSize = CGSize(width: 300, height: 84)

/// The creature's box, the bubble's width, and a bubble of a couple of rows plus
/// its tail — the sizes the surface is actually laid out from.
private let creature = CGSize(width: 64, height: 56)
private let bubbleWidth: CGFloat = 300
private let bubbleHeight: CGFloat = 92
private let totalHeight = creature.height + bubbleHeight

// MARK: - Which edge stays put

@Test func aPetInTheUpperHalfAnchorsToItsTopEdge() {
    let frame = CGRect(x: 1124, y: 775, width: 300, height: 84)
    #expect(anchor(for: frame, in: screen) == .top)
}

@Test func aPetInTheLowerHalfAnchorsToItsBottomEdge() {
    let frame = CGRect(x: 1124, y: 16, width: 300, height: 84)
    #expect(anchor(for: frame, in: screen) == .bottom)
}

@Test func aPetStraddlingTheMiddleAnchorsToTheTop() {
    // Exactly centred: the tie has to break somewhere, and top keeps the
    // behaviour the pet had before it could be moved.
    let frame = CGRect(x: 0, y: screen.midY - 42, width: 300, height: 84)
    #expect(anchor(for: frame, in: screen) == .top)
}

@Test func aTopAnchoredPetGrowsDownwardAndKeepsItsTopEdge() {
    let frame = CGRect(x: 1124, y: 775, width: 300, height: 84)
    let grown = resized(frame, to: CGSize(width: 300, height: 148), anchoredAt: .top)
    #expect(grown.maxY == frame.maxY)
    #expect(grown.minY < frame.minY)
}

@Test func aBottomAnchoredPetGrowsUpwardAndKeepsItsBottomEdge() {
    let frame = CGRect(x: 1124, y: 16, width: 300, height: 84)
    let grown = resized(frame, to: CGSize(width: 300, height: 148), anchoredAt: .bottom)
    #expect(grown.minY == frame.minY)
    #expect(grown.maxY > frame.maxY)
}

@Test func aBottomAnchoredPetStaysOnScreenAsItGrows() {
    // The failure the anchor rule exists to prevent: at the bottom of the screen,
    // growing downward would walk the pet off the edge.
    let frame = CGRect(x: 1124, y: 16, width: 300, height: 84)
    let grown = resized(frame, to: CGSize(width: 300, height: 300), anchoredAt: .bottom)
    #expect(grown.minY >= screen.minY)
}

// MARK: - Remembering across a resize

@Test func aTopAnchoredPetComesBackAtTheSameTopEdgeWhenItsHeightChanged() {
    // Quit showing two sessions, relaunch showing none.
    let atQuit = CGRect(x: 1124, y: 691, width: bubbleWidth, height: totalHeight)
    let remembered = stored(atQuit, side: .left, creatureSize: creature, in: screen)
    let atLaunch = frame(
        for: remembered,
        creatureSize: creature,
        bubbleWidth: bubbleWidth,
        bubbleHeight: 40
    )
    #expect(atLaunch.maxY == atQuit.maxY)
    #expect(atLaunch.minX == atQuit.minX)
}

@Test func aBottomAnchoredPetComesBackAtTheSameBottomEdgeWhenItsHeightChanged() {
    let atQuit = CGRect(x: 40, y: 16, width: bubbleWidth, height: totalHeight)
    let remembered = stored(atQuit, side: .right, creatureSize: creature, in: screen)
    let atLaunch = frame(
        for: remembered,
        creatureSize: creature,
        bubbleWidth: bubbleWidth,
        bubbleHeight: 40
    )
    #expect(atLaunch.minY == atQuit.minY)
    #expect(atLaunch.minX == atQuit.minX)
}

@Test func whatIsRememberedIsTheCreatureNotTheFrame() {
    // The frame's left edge and the creature's are the same point only when the
    // bubble runs right. Remembering the frame would move the creature on any
    // relaunch that picked the other side.
    let f = CGRect(x: 400, y: 300, width: bubbleWidth, height: totalHeight)
    #expect(stored(f, side: .left, creatureSize: creature, in: screen).x == f.maxX - creature.width)
    #expect(stored(f, side: .right, creatureSize: creature, in: screen).x == f.minX)
}

// MARK: - Is a remembered position still usable

@Test func aFullyVisiblePositionIsRestored() {
    #expect(isRestorable(CGRect(x: 100, y: 100, width: 300, height: 84), on: [screen]))
}

@Test func aPositionOnADisconnectedDisplayIsNotRestored() {
    // Placed on a second display to the right, then undocked.
    let onSecondDisplay = CGRect(x: 1600, y: 400, width: 300, height: 84)
    #expect(!isRestorable(onSecondDisplay, on: [screen]))
    #expect(isRestorable(onSecondDisplay, on: [screen, CGRect(x: 1440, y: 0, width: 2560, height: 1415)]))
}

@Test func aPositionShowingLessThanTheMinimumIsNotRestored() {
    // 20 points of pet left on screen: present, but not findable.
    let slivered = CGRect(x: 1420, y: 400, width: 300, height: 84)
    #expect(!isRestorable(slivered, on: [screen]))
}

@Test func exactlyTheMinimumOverlapIsRestored() {
    let atTheLimit = CGRect(x: screen.maxX - 40, y: screen.maxY - 40, width: 300, height: 84)
    #expect(isRestorable(atTheLimit, on: [screen]))
}

@Test func aPetDeliberatelyTuckedAgainstAnEdgeIsRestored() {
    // Parking it mostly off-screen is a choice, not a fault, so long as enough
    // of it remains to grab.
    let tucked = CGRect(x: -160, y: 300, width: 300, height: 84)
    #expect(isRestorable(tucked, on: [screen]))
}

@Test func twoSliversOnTwoDisplaysDoNotAddUpToAVisiblePet() {
    let left = CGRect(x: 0, y: 0, width: 1000, height: 875)
    let right = CGRect(x: 1000, y: 0, width: 1000, height: 875)
    // 10 points on each side of the seam: 20 points wide in total, on neither.
    let straddling = CGRect(x: 990, y: 400, width: 20, height: 84)
    #expect(!isRestorable(straddling, on: [left, right]))
}

// MARK: - Where the pet goes

@Test func theDefaultPositionIsTheTopRightClearOfTheScreenEdges() {
    let f = defaultFrame(size: petSize, in: screen)
    #expect(f.maxX == screen.maxX - defaultInset)
    #expect(f.maxY == screen.maxY - defaultInset)
    #expect(screen.contains(f))
}

@Test func aPetThatHasNeverBeenMovedStartsAtTheDefault() {
    let placed = placement(
        remembered: nil,
        creatureSize: creature,
        bubbleWidth: bubbleWidth,
        bubbleHeight: bubbleHeight,
        visibleFrames: [screen],
        primary: screen
    )
    #expect(placed.frame == defaultFrame(size: CGSize(width: bubbleWidth, height: totalHeight), in: screen))
    // Top-right of the screen, so the creature sits under the bubble's right
    // corner and the bubble runs left away from the edge.
    #expect(placed.side == .left)
    #expect(creatureRect(in: placed.frame, side: placed.side, size: creature).maxX
        == screen.maxX - defaultInset)
}

@Test func anUnusableRememberedPositionFallsBackToTheDefault() {
    let stranded = StoredPosition(x: 3000, edgeY: 800, anchor: .top, side: .left)
    let placed = placement(
        remembered: stranded,
        creatureSize: creature,
        bubbleWidth: bubbleWidth,
        bubbleHeight: bubbleHeight,
        visibleFrames: [screen],
        primary: screen
    )
    #expect(placed.frame == defaultFrame(size: CGSize(width: bubbleWidth, height: totalHeight), in: screen))
}

@Test func aUsableRememberedPositionIsHonoured() {
    let remembered = StoredPosition(x: 1000, edgeY: 500, anchor: .top, side: .left)
    let placed = placement(
        remembered: remembered,
        creatureSize: creature,
        bubbleWidth: bubbleWidth,
        bubbleHeight: bubbleHeight,
        visibleFrames: [screen],
        primary: screen
    )
    #expect(placed.side == .left)
    #expect(placed.frame.maxY == 500)
    #expect(creatureRect(in: placed.frame, side: placed.side, size: creature).minX == 1000)
}

@Test func aRememberedSideThatNoLongerFitsIsRepickedWithoutMovingTheCreature() {
    // Left-hand edge of the screen: the bubble can only run right from here.
    let remembered = StoredPosition(x: 20, edgeY: 500, anchor: .top, side: .left)
    let placed = placement(
        remembered: remembered,
        creatureSize: creature,
        bubbleWidth: bubbleWidth,
        bubbleHeight: bubbleHeight,
        visibleFrames: [screen],
        primary: screen
    )
    #expect(placed.side == .right)
    #expect(creatureRect(in: placed.frame, side: placed.side, size: creature).minX == 20)
}

@Test func aCreatureIsKeptEvenWhereItsBubbleWouldHangOffTheEdge() {
    // Validity is judged on the creature, never on the frame: a bubble over an
    // edge is something the side rule fixes, not a reason to throw a reachable
    // creature back to the top-right.
    let remembered = StoredPosition(x: 0, edgeY: 500, anchor: .top, side: .left)
    let placed = placement(
        remembered: remembered,
        creatureSize: creature,
        bubbleWidth: bubbleWidth,
        bubbleHeight: bubbleHeight,
        visibleFrames: [screen],
        primary: screen
    )
    #expect(placed.frame != defaultFrame(size: CGSize(width: bubbleWidth, height: totalHeight), in: screen))
}

// MARK: - Which side the bubble takes

private func creatureAt(_ x: CGFloat, _ y: CGFloat = 400) -> CGRect {
    CGRect(x: x, y: y, width: creature.width, height: creature.height)
}

@Test func aBubbleThatStillFitsStaysOnTheSideItIsOn() {
    // Room on both sides: the pet must not twitch as it crosses the middle.
    let middle = creatureAt(700)
    #expect(bubbleSide(creature: middle, current: .left, bubbleWidth: bubbleWidth, in: screen) == .left)
    #expect(bubbleSide(creature: middle, current: .right, bubbleWidth: bubbleWidth, in: screen) == .right)
}

@Test func aBubbleThatWouldCrossTheLeftEdgeFlipsToTheCreaturesRight() {
    let nearLeft = creatureAt(10)
    #expect(bubbleSide(creature: nearLeft, current: .left, bubbleWidth: bubbleWidth, in: screen) == .right)
}

@Test func aBubbleThatWouldCrossTheRightEdgeFlipsToTheCreaturesLeft() {
    let nearRight = creatureAt(screen.maxX - creature.width - 10)
    #expect(bubbleSide(creature: nearRight, current: .right, bubbleWidth: bubbleWidth, in: screen) == .left)
}

@Test func theFlipIsDrivenByTheEdgeAndNotByTheMidpoint() {
    // Just past the middle, running toward the far edge and still fitting: the
    // rule that switches at the midpoint would flip here, and this one must not.
    let pastTheMiddle = creatureAt(screen.midX + 1)
    #expect(bubbleSide(creature: pastTheMiddle, current: .right, bubbleWidth: bubbleWidth, in: screen) == .right)
}

@Test func aScreenTooNarrowForEitherSideKeepsTheSideItIsOn() {
    // No display this pet runs on is this narrow, but oscillating would be worse
    // than sitting still if one were.
    let narrow = CGRect(x: 0, y: 0, width: 200, height: 875)
    let middle = CGRect(x: 68, y: 400, width: creature.width, height: creature.height)
    #expect(bubbleSide(creature: middle, current: .left, bubbleWidth: bubbleWidth, in: narrow) == .left)
    #expect(bubbleSide(creature: middle, current: .right, bubbleWidth: bubbleWidth, in: narrow) == .right)
}

// MARK: - The frame built around a creature

@Test func aFlipMovesTheFrameButNotTheCreature() {
    let c = creatureAt(700)
    let runningLeft = petFrame(creature: c, side: .left, bubbleWidth: bubbleWidth, bubbleHeight: bubbleHeight)
    let runningRight = petFrame(creature: c, side: .right, bubbleWidth: bubbleWidth, bubbleHeight: bubbleHeight)
    #expect(runningRight.minX - runningLeft.minX == bubbleWidth - creature.width)
    #expect(creatureRect(in: runningLeft, side: .left, size: creature) == c)
    #expect(creatureRect(in: runningRight, side: .right, size: creature) == c)
}

@Test func theFrameIsTheBubbleWideAndTheTwoOfThemTall() {
    let f = petFrame(creature: creatureAt(700), side: .left, bubbleWidth: bubbleWidth, bubbleHeight: bubbleHeight)
    #expect(f.width == bubbleWidth)
    #expect(f.height == totalHeight)
}

@Test func aTopAnchoredSurfaceHoldsItsTopAndCarriesTheCreatureDown() {
    // A row appears: the bubble's top stays where the user put it and the
    // creature descends with the bubble's bottom.
    let before = petFrame(creature: creatureAt(700, 400), side: .left, bubbleWidth: bubbleWidth, bubbleHeight: bubbleHeight)
    let after = resized(before, to: CGSize(width: bubbleWidth, height: totalHeight + 30), anchoredAt: .top)
    #expect(after.maxY == before.maxY)
    #expect(creatureRect(in: after, side: .left, size: creature).minY
        == creatureRect(in: before, side: .left, size: creature).minY - 30)
}

@Test func aBottomAnchoredSurfaceHoldsTheCreatureAndGrowsTheBubbleUpward() {
    let before = petFrame(creature: creatureAt(700, 20), side: .left, bubbleWidth: bubbleWidth, bubbleHeight: bubbleHeight)
    let after = resized(before, to: CGSize(width: bubbleWidth, height: totalHeight + 30), anchoredAt: .bottom)
    #expect(creatureRect(in: after, side: .left, size: creature) == creatureRect(in: before, side: .left, size: creature))
    #expect(after.maxY == before.maxY + 30)
}

// MARK: - Coming from story 002's keys

@Test func aLegacyFrameLeavesTheCreatureUnderItsRightCornerWithTheBubbleWhereItWas() {
    let legacy = CGRect(x: 600, y: 400, width: bubbleWidth, height: totalHeight)
    let derived = derivedCreature(fromLegacyFrame: legacy, creatureWidth: creature.width)
    #expect(derived.side == .left)
    #expect(derived.x == legacy.maxX - creature.width)
    // The whole point of the right corner: the bubble does not appear to move.
    #expect(petFrame(
        creature: CGRect(x: derived.x, y: legacy.minY, width: creature.width, height: creature.height),
        side: derived.side,
        bubbleWidth: bubbleWidth,
        bubbleHeight: bubbleHeight
    ).minX == legacy.minX)
}

@Test func aLegacyFrameTuckedOffTheLeftEdgeHasItsSideRepickedOnTheWayIn() {
    // Story 002 let the pet be parked mostly off-screen, so its bubble can arrive
    // somewhere the new one will not fit. The creature still lands under the old
    // right corner; the launch rule is what moves the bubble to the other side.
    let legacy = CGRect(x: -160, y: 400, width: bubbleWidth, height: totalHeight)
    let derived = derivedCreature(fromLegacyFrame: legacy, creatureWidth: creature.width)
    #expect(derived.x == legacy.maxX - creature.width)

    let placed = placement(
        remembered: StoredPosition(x: derived.x, edgeY: 500, anchor: .top, side: derived.side),
        creatureSize: creature,
        bubbleWidth: bubbleWidth,
        bubbleHeight: bubbleHeight,
        visibleFrames: [screen],
        primary: screen
    )
    #expect(placed.side == .right)
    #expect(creatureRect(in: placed.frame, side: placed.side, size: creature).minX == derived.x)
}
