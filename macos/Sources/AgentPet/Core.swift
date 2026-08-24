import Foundation

/// Whether a session is currently doing work.
///
/// `unknown` is a state in its own right, never resolved into `working` or `idle`
/// by guessing. Any value the core sends that this build does not recognise also
/// lands here rather than being silently treated as idle.
enum SessionState: String, Decodable {
    case working, idle, unknown

    init(from decoder: Decoder) throws {
        let raw = try decoder.singleValueContainer().decode(String.self)
        self = SessionState(rawValue: raw) ?? .unknown
    }
}

struct AgentSession: Decodable, Equatable {
    let agentId: String
    let sessionKey: String
    let projectPath: String
    let displayName: String
    let state: SessionState
    let activity: String?
    /// Unix milliseconds. The row counts up from here.
    let observedAt: Double

    var observedDate: Date { Date(timeIntervalSince1970: observedAt / 1000) }
}

struct Poll: Decodable {
    let ok: Bool
    let sessions: [AgentSession]
    let error: String?
}

/// The whole boundary between the Swift surface and the Rust observation core:
/// one call, one JSON payload.
enum Core {
    static func poll() -> Poll {
        guard let raw = agentpet_poll() else {
            return Poll(ok: false, sessions: [], error: "core returned nothing")
        }
        defer { agentpet_free(raw) }
        let json = String(cString: raw)
        guard let data = json.data(using: .utf8) else {
            return Poll(ok: false, sessions: [], error: "core returned unreadable text")
        }
        do {
            return try JSONDecoder().decode(Poll.self, from: data)
        } catch {
            return Poll(ok: false, sessions: [], error: "cannot read core output: \(error)")
        }
    }
}
