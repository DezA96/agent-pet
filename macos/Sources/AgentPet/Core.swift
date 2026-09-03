import Foundation

struct AgentSession: Decodable, Equatable {
    let agentId: String
    let sessionKey: String
    let projectPath: String
    let displayName: String
    let state: SessionState
    let activity: String?
    /// Unix milliseconds of when the displayed state began. The row counts up
    /// from here — the agent's own record of the change wherever it kept one, and
    /// only otherwise when the pet first saw the reading.
    let statusSince: Double

    var statusSinceDate: Date { Date(timeIntervalSince1970: statusSince / 1000) }
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
