import Foundation

/// How long the state on screen has been true, as the row says it.
///
/// In `PetState` rather than beside the row it draws, for the same reason the
/// state priority is: the tiers below are a rule a reader cannot check by looking
/// at the screen — a row reading `2m 05s` looks right whatever it means — so they
/// have to be somewhere tests can reach.
///
/// A unit is padded to two digits only when a larger unit precedes it. That is
/// what keeps a label's width fixed within a tier and stepping only when a unit is
/// gained, so a number ticking beside a project name never re-wraps the row it
/// sits in. There is deliberately no day unit and seconds never drop: an age that
/// coarsens is one the eye stops trusting for the short spans that matter most.
func ageText(seconds: Int) -> String {
    // An agent-supplied time in the future — a clock change, or skew between the
    // agent's clock and this one. Reads as `0s` rather than counting down or
    // showing a negative number, neither of which means anything on a row.
    let total = max(0, seconds)
    let s = total % 60
    let m = (total / 60) % 60
    let h = total / 3600

    if h > 0 {
        return "\(h)h \(pad(m))m \(pad(s))s"
    }
    if m > 0 {
        return "\(m)m \(pad(s))s"
    }
    return "\(s)s"
}

/// The elapsed age of a status, from its start to now.
///
/// Rounded rather than truncated so the first tick after a status begins reads
/// `0s` rather than flickering, and clamped by `ageText` when the start is ahead
/// of this machine's clock.
func ageText(since start: Date, now: Date = Date()) -> String {
    ageText(seconds: Int(now.timeIntervalSince(start).rounded()))
}

private func pad(_ n: Int) -> String {
    n < 10 ? "0\(n)" : "\(n)"
}
