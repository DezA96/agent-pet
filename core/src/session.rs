use serde::Serialize;

/// Whether an agent session is currently doing work.
///
/// `Unknown` is a real, reportable state — never a placeholder to be resolved
/// into `Working` or `Idle` by inference. If an adapter cannot read a session's
/// working state, it says so.
///
/// `Waiting` and `Errored` are the two states that want something from the user;
/// the other three do not. That split, rather than the number of variants, is
/// what the surface draws: only these two move.
///
/// `Waiting` is deliberately not the same as `Idle`. An idle session finished its
/// turn cleanly and costs nothing to leave alone; a waiting one is blocked
/// mid-turn and will sit there until the user answers. Collapsing them would
/// leave every completed session demanding attention, which is how an ambient
/// surface turns into noise.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Working,
    Idle,
    /// Blocked mid-turn on something only the user can answer.
    Waiting,
    /// The session stopped on an error it did not recover from.
    Errored,
    #[default]
    Unknown,
}

impl State {
    /// Whether this state wants something from the user.
    pub fn wants_attention(self) -> bool {
        matches!(self, State::Waiting | State::Errored)
    }
}

/// One live agent session, already reduced to what the pet draws.
///
/// The pet never sees a PID, a registry file or a transcript — adapters keep all
/// of that inside themselves, so adding an agent changes no pet code.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    /// Which agent produced this session, e.g. `claude`.
    pub agent_id: String,
    /// Stable identity for this session, unique across all agents.
    pub session_key: String,
    /// Absolute working directory the session was launched in.
    pub project_path: String,
    /// What to show as the project, disambiguated if another row collides.
    pub display_name: String,
    pub state: State,
    /// Very short line under the project name, whenever the state has something
    /// specific to add: what a working session is doing, what a waiting one is
    /// blocked on, which code an errored one stopped with. `Idle` and `Unknown`
    /// have nothing to add and leave it empty rather than showing stale text.
    pub activity: Option<String>,
    /// When this observation was taken, unix ms. The pet counts up from here.
    pub observed_at: u64,
}

/// Make every displayed name unique.
///
/// Two sessions in the same project legitimately derive the same name — observed
/// on this machine, where two sessions both derived `agent-agnostic-pet-02`. The
/// displayed name is not an identifier, so where names collide each colliding row
/// gains a short suffix taken from its session key.
pub fn disambiguate(sessions: &mut [AgentSession]) {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for s in sessions.iter() {
        *counts.entry(s.display_name.clone()).or_insert(0) += 1;
    }
    for s in sessions.iter_mut() {
        if counts.get(&s.display_name).copied().unwrap_or(0) > 1 {
            let suffix: String = s.session_key.chars().take(4).collect();
            s.display_name = format!("{} ({})", s.display_name, suffix);
        }
    }
}

const MAX_ACTIVITY_CHARS: usize = 45;

/// Trim an activity line to the pet's width, on a word boundary where one is
/// close enough.
///
/// The width belongs to the pet's surface rather than to any one agent, so every
/// adapter's line is cut by this same rule and no row can out-grow another's.
pub fn truncate_activity(s: &str) -> String {
    let s = s.replace(['\n', '\r', '\t'], " ");
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= MAX_ACTIVITY_CHARS {
        return s;
    }
    let cut: String = s.chars().take(MAX_ACTIVITY_CHARS).collect();
    let trimmed = match cut.rfind(' ') {
        Some(i) if i >= MAX_ACTIVITY_CHARS / 2 => &cut[..i],
        _ => cut.as_str(),
    };
    format!("{}…", trimmed.trim_end())
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(name: &str, key: &str) -> AgentSession {
        AgentSession {
            agent_id: "claude".into(),
            session_key: key.into(),
            project_path: "/tmp/p".into(),
            display_name: name.into(),
            state: State::Idle,
            activity: None,
            observed_at: 0,
        }
    }

    #[test]
    fn colliding_names_each_gain_a_suffix() {
        let mut v = vec![s("pet-02", "32dd885c"), s("pet-02", "27bb0263")];
        disambiguate(&mut v);
        assert_eq!(v[0].display_name, "pet-02 (32dd)");
        assert_eq!(v[1].display_name, "pet-02 (27bb)");
        assert_ne!(v[0].display_name, v[1].display_name);
    }

    #[test]
    fn unique_names_are_left_alone() {
        let mut v = vec![s("alpha", "aaaa1111"), s("beta", "bbbb2222")];
        disambiguate(&mut v);
        assert_eq!(v[0].display_name, "alpha");
        assert_eq!(v[1].display_name, "beta");
    }
}
