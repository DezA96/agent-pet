import CoreGraphics
import Testing

@testable import PetGeometry

/// A 1440×900 screen with the menu bar taken off the top, which is what
/// `NSScreen.visibleFrame` reports.
private let screen = CGRect(x: 0, y: 0, width: 1440, height: 875)
private let petSize = CGSize(width: 300, height: 84)

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
    let atQuit = CGRect(x: 1124, y: 691, width: 300, height: 168)
    let remembered = stored(atQuit, in: screen)
    let atLaunch = frame(for: remembered, size: CGSize(width: 300, height: 60))
    #expect(atLaunch.maxY == atQuit.maxY)
    #expect(atLaunch.minX == atQuit.minX)
}

@Test func aBottomAnchoredPetComesBackAtTheSameBottomEdgeWhenItsHeightChanged() {
    let atQuit = CGRect(x: 40, y: 16, width: 300, height: 168)
    let remembered = stored(atQuit, in: screen)
    let atLaunch = frame(for: remembered, size: CGSize(width: 300, height: 60))
    #expect(atLaunch.minY == atQuit.minY)
    #expect(atLaunch.minX == atQuit.minX)
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
    let f = placement(remembered: nil, size: petSize, visibleFrames: [screen], primary: screen)
    #expect(f == defaultFrame(size: petSize, in: screen))
}

@Test func anUnusableRememberedPositionFallsBackToTheDefault() {
    let stranded = StoredPosition(x: 3000, edgeY: 800, anchor: .top)
    let f = placement(remembered: stranded, size: petSize, visibleFrames: [screen], primary: screen)
    #expect(f == defaultFrame(size: petSize, in: screen))
}

@Test func aUsableRememberedPositionIsHonoured() {
    let remembered = StoredPosition(x: 200, edgeY: 500, anchor: .top)
    let f = placement(remembered: remembered, size: petSize, visibleFrames: [screen], primary: screen)
    #expect(f.minX == 200)
    #expect(f.maxY == 500)
}
