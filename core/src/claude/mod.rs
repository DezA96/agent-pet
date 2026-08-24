pub mod registry;
pub mod transcript;

use crate::adapter::Adapter;
use crate::procs::ProcessTable;
use crate::session::{now_ms, AgentSession, State};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Entrypoints that register a process but never become an observable session.
///
/// The VS Code extension registers once, publishes no `status`, writes no
/// transcript, and does not appear in peer enumeration — a live process that
/// cannot be reported on. Release 001 targets the CLI.
const UNOBSERVABLE_ENTRYPOINTS: &[&str] = &["claude-vscode"];

pub struct ClaudeAdapter {
    tailer: transcript::Tailer,
    /// Resolved transcript path per session, so the project directories are
    /// scanned once per session rather than once per tick.
    transcript_paths: HashMap<String, PathBuf>,
}

impl ClaudeAdapter {
    pub fn new() -> Self {
        Self {
            tailer: transcript::Tailer::new(),
            transcript_paths: HashMap::new(),
        }
    }

    /// Locate `<profile>/projects/<slug>/<sessionId>.jsonl`.
    ///
    /// The slug is derived from the session's cwd by a rule the agent owns, so it
    /// is tried first and then confirmed by scanning. Searching by session id is
    /// what makes this correct regardless of how the agent encodes a path.
    fn transcript_path(&mut self, profile: &Path, session_id: &str, cwd: &str) -> Option<PathBuf> {
        if let Some(p) = self.transcript_paths.get(session_id) {
            if p.exists() {
                return Some(p.clone());
            }
            self.transcript_paths.remove(session_id);
        }
        let projects = profile.join("projects");
        let guess = projects.join(slug(cwd)).join(format!("{session_id}.jsonl"));
        if guess.exists() {
            self.transcript_paths
                .insert(session_id.to_string(), guess.clone());
            return Some(guess);
        }
        for dir in std::fs::read_dir(&projects).ok()?.flatten() {
            let candidate = dir.path().join(format!("{session_id}.jsonl"));
            if candidate.exists() {
                self.transcript_paths
                    .insert(session_id.to_string(), candidate.clone());
                return Some(candidate);
            }
        }
        None
    }
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for ClaudeAdapter {
    fn agent_id(&self) -> &'static str {
        "claude"
    }

    fn live_sessions(&mut self, profiles: &[PathBuf], procs: &dyn ProcessTable) -> Vec<AgentSession> {
        let mut candidates: Vec<(PathBuf, registry::RegistryEntry)> = Vec::new();
        for profile in profiles {
            for entry in registry::read_dir(profile) {
                let unobservable = entry
                    .entrypoint
                    .as_deref()
                    .is_some_and(|e| UNOBSERVABLE_ENTRYPOINTS.contains(&e));
                if unobservable {
                    continue;
                }
                candidates.push((profile.clone(), entry));
            }
        }

        // One process lookup for every candidate at once, rather than per session.
        let pids: Vec<u32> = candidates.iter().map(|(_, e)| e.pid).collect();
        let starts = procs.start_times_utc(&pids);

        let observed = now_ms();
        let mut out = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for (profile, entry) in candidates {
            if !registry::is_live(&entry, &starts) {
                continue;
            }
            // The same session can be reachable through two configured paths to
            // one profile; it is still one session.
            if seen.contains(&entry.session_id) {
                continue;
            }
            seen.push(entry.session_id.clone());

            let state = match entry.status.as_deref() {
                Some("busy") => State::Working,
                Some("idle") => State::Idle,
                // Never inferred. A session whose state cannot be read says so.
                _ => State::Unknown,
            };

            // Only a working session gets an activity line: the transcript's last
            // tool call goes stale the moment the session stops, and an idle row
            // showing the last thing it did would read as busy at a glance.
            let activity = if state == State::Working {
                self.transcript_path(&profile, &entry.session_id, &entry.cwd)
                    .and_then(|p| self.tailer.activity(&p))
            } else {
                None
            };

            out.push(AgentSession {
                agent_id: "claude".into(),
                session_key: entry.session_id.clone(),
                project_path: entry.cwd.clone(),
                display_name: project_name(&entry.cwd),
                state,
                activity,
                observed_at: observed,
            });
        }

        self.transcript_paths
            .retain(|k, _| out.iter().any(|s| &s.session_key == k));
        let keep: Vec<PathBuf> = self.transcript_paths.values().cloned().collect();
        self.tailer.retain_only(&keep);
        out
    }
}

/// The project a session belongs to, as shown on its row.
fn project_name(cwd: &str) -> String {
    cwd.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(cwd)
        .to_string()
}

/// The agent's own encoding of a cwd into a directory name.
fn slug(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::procs::FakeProcessTable;
    use std::collections::HashMap;

    fn profile(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join("agentpet-claude-tests").join(name);
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(p.join("sessions")).unwrap();
        p
    }

    fn write_session(profile: &Path, pid: u32, id: &str, cwd: &str, start: &str, extra: &str) {
        let raw = format!(
            r#"{{"pid":{pid},"sessionId":"{id}","cwd":"{cwd}","procStart":"{start}"{extra}}}"#
        );
        std::fs::write(profile.join("sessions").join(format!("{pid}.json")), raw).unwrap();
    }

    fn table(pairs: &[(u32, &str)]) -> FakeProcessTable {
        FakeProcessTable {
            starts: pairs.iter().map(|(p, s)| (*p, s.to_string())).collect(),
            dirs: Vec::new(),
        }
    }

    #[test]
    fn discovers_live_sessions_across_two_directories_at_once() {
        let a = profile("dir-a");
        let b = profile("dir-b");
        write_session(&a, 100, "sess-a", "/Users/x/alpha", "Mon Aug 24 04:00:00 2026", r#","entrypoint":"cli","status":"busy""#);
        write_session(&b, 200, "sess-b", "/Users/x/beta", "Mon Aug 24 05:00:00 2026", r#","entrypoint":"cli","status":"idle""#);

        let procs = table(&[(100, "Mon Aug 24 04:00:00 2026"), (200, "Mon Aug 24 05:00:00 2026")]);
        let mut out = ClaudeAdapter::new().live_sessions(&[a, b], &procs);
        out.sort_by(|x, y| x.session_key.cmp(&y.session_key));

        assert_eq!(out.len(), 2);
        assert_eq!(out[0].display_name, "alpha");
        assert_eq!(out[0].state, State::Working);
        assert_eq!(out[1].display_name, "beta");
        assert_eq!(out[1].state, State::Idle);
    }

    #[test]
    fn a_configured_directory_that_is_absent_does_not_stop_the_others() {
        let a = profile("dir-present");
        write_session(&a, 101, "sess-p", "/Users/x/gamma", "Mon Aug 24 04:00:00 2026", r#","entrypoint":"cli","status":"busy""#);
        let missing = PathBuf::from("/nonexistent/profile-dir");

        let procs = table(&[(101, "Mon Aug 24 04:00:00 2026")]);
        let out = ClaudeAdapter::new().live_sessions(&[missing, a], &procs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].display_name, "gamma");
    }

    #[test]
    fn a_force_killed_session_produces_no_row() {
        let a = profile("dir-orphan");
        write_session(&a, 102, "sess-dead", "/Users/x/delta", "Mon Aug 24 04:00:00 2026", r#","entrypoint":"cli","status":"busy""#);
        // Registry file present, process gone.
        let procs = table(&[]);
        assert!(ClaudeAdapter::new().live_sessions(&[a], &procs).is_empty());
    }

    #[test]
    fn a_recycled_pid_produces_no_row() {
        let a = profile("dir-recycled");
        write_session(&a, 103, "sess-old", "/Users/x/eps", "Mon Aug 24 04:00:00 2026", r#","entrypoint":"cli","status":"busy""#);
        let procs = table(&[(103, "Mon Aug 24 11:22:33 2026")]);
        assert!(ClaudeAdapter::new().live_sessions(&[a], &procs).is_empty());
    }

    #[test]
    fn a_session_without_a_status_is_unknown_not_idle() {
        let a = profile("dir-nostatus");
        write_session(&a, 104, "sess-q", "/Users/x/zeta", "Mon Aug 24 04:00:00 2026", "");
        let procs = table(&[(104, "Mon Aug 24 04:00:00 2026")]);
        let out = ClaudeAdapter::new().live_sessions(&[a], &procs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].state, State::Unknown);
        assert_eq!(out[0].activity, None);
    }

    #[test]
    fn vscode_sessions_are_not_shown_even_when_live() {
        let a = profile("dir-vscode");
        write_session(&a, 105, "sess-vs", "/Users/x/eta", "Mon Aug 24 04:00:00 2026", r#","entrypoint":"claude-vscode""#);
        let procs = table(&[(105, "Mon Aug 24 04:00:00 2026")]);
        assert!(ClaudeAdapter::new().live_sessions(&[a], &procs).is_empty());
    }

    #[test]
    fn an_idle_session_shows_no_activity_line() {
        let a = profile("dir-idle");
        write_session(&a, 106, "sess-i", "/Users/x/theta", "Mon Aug 24 04:00:00 2026", r#","entrypoint":"cli","status":"idle""#);
        std::fs::create_dir_all(a.join("projects").join(slug("/Users/x/theta"))).unwrap();
        std::fs::write(
            a.join("projects").join(slug("/Users/x/theta")).join("sess-i.jsonl"),
            r#"{"message":{"content":[{"type":"tool_use","name":"Bash","input":{"description":"Doing a thing"}}]}}"#,
        )
        .unwrap();
        let procs = table(&[(106, "Mon Aug 24 04:00:00 2026")]);
        let out = ClaudeAdapter::new().live_sessions(&[a], &procs);
        assert_eq!(out[0].state, State::Idle);
        assert_eq!(out[0].activity, None, "an idle row must not show stale activity");
    }

    #[test]
    fn a_working_session_picks_up_its_transcript_activity() {
        let a = profile("dir-busy");
        let cwd = "/Users/x/iota";
        write_session(&a, 107, "sess-w", cwd, "Mon Aug 24 04:00:00 2026", r#","entrypoint":"cli","status":"busy""#);
        let pdir = a.join("projects").join(slug(cwd));
        std::fs::create_dir_all(&pdir).unwrap();
        std::fs::write(
            pdir.join("sess-w.jsonl"),
            "{\"message\":{\"content\":[{\"type\":\"tool_use\",\"name\":\"Read\",\"input\":{\"file_path\":\"/a/b/notes.md\"}}]}}\n",
        )
        .unwrap();
        let procs = table(&[(107, "Mon Aug 24 04:00:00 2026")]);
        let out = ClaudeAdapter::new().live_sessions(&[a], &procs);
        assert_eq!(out[0].activity.as_deref(), Some("Reading notes.md"));
    }

    #[test]
    fn a_just_started_session_renders_without_an_activity_line() {
        let a = profile("dir-fresh");
        write_session(&a, 108, "sess-new", "/Users/x/kappa", "Mon Aug 24 04:00:00 2026", r#","entrypoint":"cli","status":"busy""#);
        // No transcript on disk yet.
        let procs = table(&[(108, "Mon Aug 24 04:00:00 2026")]);
        let out = ClaudeAdapter::new().live_sessions(&[a], &procs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].state, State::Working);
        assert_eq!(out[0].activity, None);
    }

    #[test]
    fn the_same_profile_reached_twice_yields_one_row() {
        let a = profile("dir-dup");
        write_session(&a, 109, "sess-d", "/Users/x/lambda", "Mon Aug 24 04:00:00 2026", r#","entrypoint":"cli","status":"idle""#);
        let procs = table(&[(109, "Mon Aug 24 04:00:00 2026")]);
        let out = ClaudeAdapter::new().live_sessions(&[a.clone(), a], &procs);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn slug_matches_the_agents_encoding() {
        assert_eq!(
            slug("/Users/example/Projects/CODE/Engineering/agent-agnostic-pet"),
            "-Users-example-Projects-CODE-Engineering-agent-agnostic-pet"
        );
    }

    #[test]
    fn project_name_is_the_last_path_segment() {
        assert_eq!(project_name("/Users/x/Projects/pet"), "pet");
        assert_eq!(project_name("/Users/x/Projects/pet/"), "pet");
    }

    // Silence unused warnings for the HashMap import in this module's scope.
    #[allow(dead_code)]
    fn _unused(_: HashMap<u32, String>) {}
}
