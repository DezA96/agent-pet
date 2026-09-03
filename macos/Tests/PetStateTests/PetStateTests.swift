import Foundation
import Testing

@testable import PetState

// MARK: - What a surface full of sessions adds up to

@Test func theMostUrgentStateWins() {
    #expect(aggregate([.idle, .working, .waiting]) == .waiting)
    #expect(aggregate([.idle, .working, .waiting, .errored]) == .errored)
    #expect(aggregate([.idle, .working]) == .working)
}

@Test func urgencyIsAStrictOrderWhicheverWayTheListIsBuilt() {
    // Order in the list must not matter: the pet sorts by urgency, never by the
    // order the core happened to discover sessions in.
    let order: [SessionState] = [.errored, .waiting, .working, .unknown, .idle]
    for (i, more) in order.enumerated() {
        for less in order[(i + 1)...] {
            #expect(aggregate([more, less]) == more)
            #expect(aggregate([less, more]) == more)
        }
    }
}

@Test func anUnreadableSurfaceIsNeverReportedAsFinished() {
    // The load-bearing half of the order: a pet that cannot read anything must
    // not look like a pet whose work is done.
    #expect(aggregate([.unknown]) == .unknown)
    #expect(aggregate([.unknown, .idle]) == .unknown)
    #expect(aggregate([.idle, .unknown]) == .unknown)
}

@Test func onlyCodexStatesStillAggregate() {
    // Codex rows carry working, idle or unknown and nothing else; the creature
    // aggregates whatever the rows carry and invents nothing above them.
    #expect(aggregate([.working, .idle, .unknown]) == .working)
    #expect(aggregate([.idle, .unknown]) == .unknown)
}

@Test func noSessionsHasNoAggregate() {
    // Not idle: nothing running is a different thing to say than work finished,
    // and the surface says it with a sleeping creature.
    #expect(aggregate([]) == nil)
}

// MARK: - The rhythm both the dot and the creature read

@Test func theUrgentStatesBreatheFasterAndDeeperThanWorking() {
    let working = SessionState.working.breath!
    for urgent in [SessionState.waiting, .errored] {
        let breath = urgent.breath!
        #expect(breath.period < working.period)
        #expect(breath.floor < working.floor)
    }
}

@Test func theSettledStatesHoldStill() {
    #expect(SessionState.idle.breath == nil)
    #expect(SessionState.unknown.breath == nil)
}

@Test func onlyTheStatesThatWantSomethingAreTheUrgentPair() {
    for state in SessionState.byUrgency {
        #expect(state.wantsAttention == (state == .waiting || state == .errored))
    }
}

@Test func everyStateIsRankedExactlyOnce() {
    // A state missing from the order would drop out of every aggregate silently.
    for state in [SessionState.working, .idle, .waiting, .errored, .unknown] {
        #expect(SessionState.byUrgency.filter { $0 == state }.count == 1)
    }
}

// MARK: - How long the state on screen has been true

@Test func theAgeReadsInTiersThatOnlyWidenWhenAUnitIsGained() {
    #expect(ageText(seconds: 0) == "0s")
    #expect(ageText(seconds: 47) == "47s")
    #expect(ageText(seconds: 59) == "59s")
    #expect(ageText(seconds: 60) == "1m 00s")
    #expect(ageText(seconds: 125) == "2m 05s")
    #expect(ageText(seconds: 3599) == "59m 59s")
    #expect(ageText(seconds: 3600) == "1h 00m 00s")
    #expect(ageText(seconds: 3827) == "1h 03m 47s")
}

@Test func aVeryOldStatusKeepsCountingInHoursRatherThanGainingADayUnit() {
    // 51h 20m 14s, not "2d 3h". Seconds always show.
    #expect(ageText(seconds: 51 * 3600 + 20 * 60 + 14) == "51h 20m 14s")
}

@Test func aStatusTimeInTheFutureReadsAsZeroRatherThanCountingDown() {
    // Clock change, or skew between the agent's clock and this machine's.
    #expect(ageText(seconds: -1) == "0s")
    #expect(ageText(seconds: -90_000) == "0s")
    let now = Date()
    #expect(ageText(since: now.addingTimeInterval(3600), now: now) == "0s")
}

@Test func theAgeIsMeasuredFromTheStatusStartNotFromNow() {
    let now = Date()
    #expect(ageText(since: now.addingTimeInterval(-96 * 60), now: now) == "1h 36m 00s")
    #expect(ageText(since: now, now: now) == "0s")
}

@Test func aPartSecondIsRoundedRatherThanTruncated() {
    // Truncating would hold a row at `0s` for almost two seconds and then jump,
    // and would read every age a second short of the truth for most of its life.
    let now = Date()
    #expect(ageText(since: now.addingTimeInterval(-1.6), now: now) == "2s")
    #expect(ageText(since: now.addingTimeInterval(-1.4), now: now) == "1s")
    #expect(ageText(since: now.addingTimeInterval(-59.6), now: now) == "1m 00s")
}
