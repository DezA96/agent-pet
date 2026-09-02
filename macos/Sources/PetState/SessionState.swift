import Foundation

/// What a session's state means, and what a surface full of them adds up to.
///
/// Deliberately free of AppKit and of the FFI, for the same reason the placement
/// arithmetic is: the priority order below is a rule a reader cannot check by
/// looking at the screen — a surface showing `working` while one session quietly
/// waits looks perfectly fine — so it has to be a thing tests can reach.

/// Whether a session is currently doing work.
///
/// `unknown` is a state in its own right, never resolved into `working` or `idle`
/// by guessing. Any value the core sends that this build does not recognise also
/// lands here rather than being silently treated as idle.
enum SessionState: String, Decodable {
    case working, idle, waiting, errored, unknown

    init(from decoder: Decoder) throws {
        let raw = try decoder.singleValueContainer().decode(String.self)
        self = SessionState(rawValue: raw) ?? .unknown
    }

    /// Whether this state wants something from the user.
    ///
    /// The whole visual vocabulary hangs off this one question: these two states
    /// move, the rest are still, so movement anywhere on the surface always means
    /// something is asking for the user rather than merely reporting.
    var wantsAttention: Bool { self == .waiting || self == .errored }

    /// How hard this state breathes, or `nil` for the states that hold still.
    ///
    /// The urgent pair is both faster and deeper than working, which is what keeps
    /// motion a real signal now that shape no longer distinguishes anything: a
    /// glance can tell a busy agent from one that is stuck without resolving the
    /// colour at all.
    ///
    /// One definition, read by both the row's dot and the creature, because the
    /// point of the creature's motion is that it means the *same thing* as a dot's.
    /// Two copies of these numbers would be two rhythms the moment one was tuned.
    /// The dot spends `floor` as an opacity; the creature spends it as the depth of
    /// a squash, so it never fades out over whatever it is floating above.
    var breath: (period: Double, floor: Float)? {
        switch self {
        case .working: return (2.4, 0.55)
        case .waiting, .errored: return (1.1, 0.28)
        case .idle, .unknown: return nil
        }
    }

    /// Most urgent first. The creature shows the first of these that is live.
    ///
    /// `unknown` above `idle` is the load-bearing part: a surface where nothing
    /// could be read must never show a creature that says everything is finished.
    static let byUrgency: [SessionState] = [.errored, .waiting, .working, .unknown, .idle]
}

/// What a whole surface of sessions adds up to, or `nil` when none are live.
///
/// `nil` rather than `idle`: no sessions at all is a different thing to say than a
/// session that has finished, and the surface says it differently — the creature
/// sleeps rather than sitting there content.
func aggregate(_ states: [SessionState]) -> SessionState? {
    SessionState.byUrgency.first { states.contains($0) }
}
