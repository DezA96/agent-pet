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
