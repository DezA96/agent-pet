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
    /// Directories this agent's own running processes say to watch this tick.
    ///
    /// Asked before `live_sessions`, and unioned into the profiles every adapter
    /// then receives. Here rather than in the pet for the same reason liveness is:
    /// the variable a session records its profile in is this agent's own fact —
    /// `CLAUDE_CONFIG_DIR` for one, `CODEX_HOME` for the next — and a pet that
    /// owned it would need a new pet-level method per agent, which is exactly
    /// what this release promises adding an agent does not require.
    ///
    /// Empty is a normal answer: an agent with nothing running, or one whose
    /// profile cannot be learned from a process, adds nothing to the defaults and
    /// whatever the user configured.
    fn profile_dirs(&self, procs: &dyn ProcessTable) -> Vec<PathBuf>;

    /// Sessions running right now, already reduced to what the pet draws.
    ///
    /// Directories that hold nothing this adapter understands are ignored
    /// silently — a profile is not an error just because another agent owns it.
    fn live_sessions(&mut self, profiles: &[PathBuf], procs: &dyn ProcessTable)
        -> Vec<AgentSession>;
}
