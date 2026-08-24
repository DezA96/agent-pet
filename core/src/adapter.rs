use crate::procs::ProcessTable;
use crate::session::AgentSession;
use std::path::PathBuf;

/// One agent's translator.
///
/// The whole contract is a single question asked once per tick: which of your
/// sessions are running right now, and what is each one doing? Everything the
/// answer depends on — how sessions are discovered, how liveness is proven, how
/// an activity line is worded — stays inside the implementation.
///
/// Liveness in particular belongs here rather than in the pet. Claude Code proves
/// it with a PID and a recorded start time; Codex cannot, because its rollout
/// files record no PID at all. A pet that owned the liveness rule would have to
/// change the first time an agent proved liveness differently, which is exactly
/// what this release promises it will not do.
pub trait Adapter {
    fn agent_id(&self) -> &'static str;

    /// Sessions running right now, already reduced to what the pet draws.
    ///
    /// Directories that hold nothing this adapter understands are ignored
    /// silently — a profile is not an error just because another agent owns it.
    fn live_sessions(&mut self, profiles: &[PathBuf], procs: &dyn ProcessTable)
        -> Vec<AgentSession>;
}
