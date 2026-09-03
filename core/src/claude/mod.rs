pub mod registry;
pub mod transcript;

use crate::adapter::Adapter;
use crate::procs::ProcessTable;
use crate::session::{now_ms, truncate_activity, AgentSession, State};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The entrypoints this release can actually report on.
///
/// An allowlist, not a list of exclusions. It started as the latter, naming the
/// VS Code extension alone, and that is precisely how `claude-desktop` came to be
/// shown: the desktop app registers a process, publishes no `status` and writes no
/// transcript, exactly as the extension does, but nobody had thought to name it —
/// so it arrived as a row reading "state unknown" the day the app started writing
/// registry files. A list of what is known to work cannot fail that way. A new
/// entrypoint is invisible until someone teaches the pet to read it, which is the
/// safe direction: a session the pet cannot report on is worse than absent,
/// because it looks like a session in trouble.
///
/// Release 001 targets the CLI, which is why this list has one entry.
const OBSERVABLE_ENTRYPOINTS: &[&str] = &["cli"];

/// The command a Claude Code session runs under, and where it records the
/// profile it is using. Named here because it is this agent's fact, not the
/// pet's — `ProcessTable` is asked for both by name and knows neither.
const COMMAND: &str = "claude";

/// Where a session records the profile directory it is using, and what it uses
/// when it records nothing: `~/.claude`.
///
/// Named here rather than anywhere in the pet. A pet holding this would need a
/// new entry per agent, which is the change adding an agent is not supposed to
/// need — the same reason liveness lives in the adapters.
const PROFILE_VAR: &str = "CLAUDE_CONFIG_DIR";

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
    fn profile_dirs(&self, procs: &dyn ProcessTable) -> Vec<PathBuf> {
        let Some(home) = std::env::var_os("HOME") else {
            return Vec::new();
        };
        let default = PathBuf::from(home).join(".claude");
        // The default first, so it is watched whether or not a process is running
        // to report it, then whatever the running ones say instead.
        let mut dirs = vec![default.clone()];
        for dir in procs.profile_dirs_of_command(COMMAND, PROFILE_VAR, &default) {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
        dirs
    }

    fn live_sessions(&mut self, profiles: &[PathBuf], procs: &dyn ProcessTable) -> Vec<AgentSession> {
        let mut candidates: Vec<(PathBuf, registry::RegistryEntry)> = Vec::new();
        for profile in profiles {
            for entry in registry::read_dir(profile) {
                let observable = match entry.entrypoint.as_deref() {
                    // No entrypoint at all predates the field, from when the CLI
                    // was the only thing writing these files. Left observable so
                    // the allowlist cannot hide a real session on an older agent:
                    // the defect being fixed is named entrypoints slipping in, not
                    // unnamed ones.
                    None => true,
                    Some(e) => OBSERVABLE_ENTRYPOINTS.contains(&e),
                };
                if !observable {
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

            let status = entry.status.as_deref();
            let state = match status {
                Some("busy") => State::Working,
                Some("idle") => State::Idle,
                Some("waiting") => State::Waiting,
                // Shell mode is not an attention state: the user is driving a
                // shell inside the session, nothing is wrong and nothing is
                // wanted from them. Closer to working than to the `Unknown` it
                // used to fall into.
                Some("shell") => State::Working,
                // Never inferred. A session whose state cannot be read says so,
                // including a status this build has never heard of.
                _ => State::Unknown,
            };

            let transcript = self.transcript_path(&profile, &entry.session_id, &entry.cwd);

            // An error only counts while it is still the newest thing in the
            // transcript *and* the session is not busy. Either half alone is
            // wrong: a busy session is one that hit an error and carried on, and
            // an older error is one it already recovered from. Together they
            // separate "died on an error" from "hit one and kept going", which is
            // the distinction the row has to get right — a surface that lights up
            // for every retried blip is one the user learns to ignore.
            let failure = if status == Some("busy") {
                None
            } else {
                transcript
                    .as_ref()
                    .and_then(|p| self.tailer.error(p))
            };

            // Where the age counts from, before the pin in `lib.rs` has its say.
            //
            // Two sources, never mixed. An errored row dates the entry the session
            // stopped on; every other row dates the registry status — and only when
            // a `status` was actually read, since `statusUpdatedAt` beside no status
            // would be timing a state the pet never saw. Neither available leaves
            // `observed`, which is exactly the pre-006 behaviour.
            //
            // A status this build does not recognise counts as read: the row shows
            // `Unknown`, and its age is how long the agent has been in whatever it
            // is in. That is a different fact from Codex's `Unknown`, which means no
            // boundary was found at all and so has nothing to date. The two adapters
            // look inconsistent and are not — one read something it could not name,
            // the other read nothing.
            let status_since = match &failure {
                Some(err) => err.at,
                None => entry.status_began(),
            }
            .unwrap_or(observed);

            let (state, activity) = match (failure, state) {
                (Some(err), _) => (State::Errored, Some(err.line())),
                // The agent's own wording for what it is blocked on.
                (None, State::Waiting) => (
                    State::Waiting,
                    entry
                        .waiting_for
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(truncate_activity),
                ),
                // Only a working session gets an activity line: the transcript's
                // last tool call goes stale the moment the session stops, and an
                // idle row showing the last thing it did would read as busy.
                (None, State::Working) => (
                    State::Working,
                    transcript.as_ref().and_then(|p| self.tailer.activity(p)),
                ),
                (None, other) => (other, None),
            };

            out.push(AgentSession {
                agent_id: "claude".into(),
                session_key: entry.session_id.clone(),
                project_path: entry.cwd.clone(),
                display_name: project_name(&entry.cwd),
                state,
                activity,
                status_since,
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

    #[test]
    fn a_running_session_reports_the_profile_it_is_using() {
        let mine = PathBuf::from("/tmp/agentpet-claude-home");
        let theirs = PathBuf::from("/tmp/agentpet-codex-home");
        let procs = FakeProcessTable {
            named: std::collections::HashMap::from([(COMMAND.to_string(), vec![900])]),
            dirs: std::collections::HashMap::from([
                ((COMMAND.to_string(), PROFILE_VAR.to_string()), vec![mine.clone()]),
                (("codex".to_string(), "CODEX_HOME".to_string()), vec![theirs.clone()]),
            ]),
            ..Default::default()
        };
        let out = ClaudeAdapter::new().profile_dirs(&procs);
        assert!(out.contains(&mine), "the learned profile is missing: {out:?}");
        assert!(!out.contains(&theirs), "another agent's profile was claimed: {out:?}");
    }

    #[test]
    fn the_default_profile_is_watched_whether_or_not_anything_is_running() {
        let home = PathBuf::from(std::env::var("HOME").unwrap());
        let out = ClaudeAdapter::new().profile_dirs(&FakeProcessTable::default());
        assert_eq!(out, vec![home.join(".claude")]);
    }

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
            ..Default::default()
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
    fn desktop_app_sessions_are_not_shown_even_when_live() {
        // Observed live: the Claude desktop app writes `entrypoint: "claude-desktop"`
        // and no `status`, so before this it arrived as a row reading "state unknown".
        let a = profile("dir-desktop");
        write_session(&a, 110, "sess-desk", "/Users/x/mu", "Mon Aug 24 04:00:00 2026", r#","entrypoint":"claude-desktop""#);
        let procs = table(&[(110, "Mon Aug 24 04:00:00 2026")]);
        assert!(ClaudeAdapter::new().live_sessions(&[a], &procs).is_empty());
    }

    #[test]
    fn an_entrypoint_this_release_does_not_know_is_not_shown() {
        // The point of an allowlist: the next entrypoint nobody has heard of yet
        // stays out until it is taught, rather than arriving as an unreadable row.
        let a = profile("dir-future-entrypoint");
        write_session(&a, 111, "sess-future", "/Users/x/nu", "Mon Aug 24 04:00:00 2026", r#","entrypoint":"claude-something-new","status":"busy""#);
        let procs = table(&[(111, "Mon Aug 24 04:00:00 2026")]);
        assert!(ClaudeAdapter::new().live_sessions(&[a], &procs).is_empty());
    }

    #[test]
    fn a_session_with_no_entrypoint_at_all_is_still_shown() {
        // Older agents wrote no entrypoint. The allowlist must not hide those.
        let a = profile("dir-noentry");
        write_session(&a, 112, "sess-old", "/Users/x/xi", "Mon Aug 24 04:00:00 2026", r#","status":"busy""#);
        let procs = table(&[(112, "Mon Aug 24 04:00:00 2026")]);
        let out = ClaudeAdapter::new().live_sessions(&[a], &procs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].state, State::Working);
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

    /// Write a transcript for a session, so error and activity tests share one setup.
    fn write_transcript(profile: &Path, cwd: &str, session_id: &str, lines: &[&str]) {
        let dir = profile.join("projects").join(slug(cwd));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{session_id}.jsonl")),
            format!("{}\n", lines.join("\n")),
        )
        .unwrap();
    }

    // One line each: a transcript is JSONL, and a fixture that wraps would be read
    // as several unparseable fragments rather than one entry.
    // MARK: - Where the age counts from (story 006)

    #[test]
    fn a_status_dates_the_row_from_the_agents_own_status_change() {
        // Not from this tick. The whole defect C-017 was raised for: a session
        // idle since long before the pet started must say so.
        for (pid, status) in [(300u32, "busy"), (301, "idle"), (302, "waiting"), (303, "shell")] {
            let a = profile(&format!("dir-since-{status}"));
            let start = "Mon Aug 24 04:00:00 2026";
            write_session(
                &a,
                pid,
                &format!("sess-{status}"),
                "/Users/x/since",
                start,
                &format!(r#","entrypoint":"cli","status":"{status}","statusUpdatedAt":1787594586507"#),
            );
            let out = ClaudeAdapter::new().live_sessions(&[a], &table(&[(pid, start)]));
            assert_eq!(out.len(), 1, "no row for status {status}");
            assert_eq!(out[0].status_since, 1_787_594_586_507, "for status {status}");
        }
    }

    #[test]
    fn an_errored_row_dates_the_error_not_the_registry_status() {
        // Errored, then a dialog opened on it: the row still reads errored because
        // errored outranks every status, so its age must still date the error while
        // `statusUpdatedAt` has moved on to the dialog. The two sources are not
        // interchangeable, and this is the case that proves it.
        let a = profile("dir-errored-since");
        let cwd = "/Users/x/errored";
        let start = "Mon Aug 24 04:00:00 2026";
        write_session(
            &a,
            310,
            "sess-err",
            cwd,
            start,
            r#","entrypoint":"cli","status":"waiting","waitingFor":"dialog open","statusUpdatedAt":1787599999999"#,
        );
        write_transcript(
            &a,
            cwd,
            "sess-err",
            &[r#"{"type":"assistant","isApiErrorMessage":true,"apiErrorStatus":529,"timestamp":"2026-08-25T00:41:35.372Z","message":{"role":"assistant","content":[]}}"#],
        );

        let out = ClaudeAdapter::new().live_sessions(&[a], &table(&[(310, start)]));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].state, State::Errored);
        assert_eq!(out[0].status_since, 1_787_618_495_372, "the row dated the dialog, not the error");
    }

    #[test]
    fn an_errored_row_whose_error_carries_no_time_falls_back_to_this_tick() {
        // The error must still decide the state; only the age gives way. Dating it
        // from `statusUpdatedAt` instead would put the dialog's time on the error's
        // row, which is the exact confusion the two sources exist to avoid.
        let a = profile("dir-errored-no-time");
        let cwd = "/Users/x/errored-bare";
        let start = "Mon Aug 24 04:00:00 2026";
        write_session(&a, 311, "sess-err-bare", cwd, start,
            r#","entrypoint":"cli","status":"idle","statusUpdatedAt":1787599999999"#);
        write_transcript(&a, cwd, "sess-err-bare", &[API_ERROR_529]);

        let before = crate::session::now_ms();
        let out = ClaudeAdapter::new().live_sessions(&[a], &table(&[(311, start)]));
        assert_eq!(out[0].state, State::Errored);
        assert_eq!(out[0].activity.as_deref(), Some("Error: 529"));
        assert!(out[0].status_since >= before, "the row dated the registry status");
    }

    #[test]
    fn an_unrecognised_status_is_unknown_but_still_dated_from_the_agents_time() {
        // The agent published a status and said when it changed; this build simply
        // cannot name it. The row is honest about the state and honest about how
        // long it has held — unlike the no-status case below, where nothing was read.
        let a = profile("dir-unknown-status-time");
        let start = "Mon Aug 24 04:00:00 2026";
        write_session(&a, 321, "sess-hib", "/Users/x/hib", start,
            r#","entrypoint":"cli","status":"hibernating","statusUpdatedAt":1787594586507"#);
        let out = ClaudeAdapter::new().live_sessions(&[a], &table(&[(321, start)]));
        assert_eq!(out[0].state, State::Unknown);
        assert_eq!(out[0].status_since, 1_787_594_586_507);
    }

    #[test]
    fn a_session_the_agent_timestamped_nothing_for_falls_back_to_this_tick() {
        // `statusUpdatedAt` is read only where a `status` was read with it: beside
        // no status there is no status whose age it would be.
        let a = profile("dir-no-status-time");
        let start = "Mon Aug 24 04:00:00 2026";
        write_session(&a, 320, "sess-bare", "/Users/x/bare", start,
            r#","entrypoint":"cli","statusUpdatedAt":1787594586507"#);
        let before = crate::session::now_ms();
        let out = ClaudeAdapter::new().live_sessions(&[a], &table(&[(320, start)]));
        assert_eq!(out[0].state, State::Unknown);
        assert!(
            out[0].status_since >= before,
            "an unknown state dated itself from a status the pet never read"
        );
    }

    const API_ERROR_529: &str = r#"{"type":"assistant","isApiErrorMessage":true,"apiErrorStatus":529,"error":"server_error","message":{"role":"assistant","content":[{"type":"text","text":"API Error: 529 Overloaded."}]}}"#;
    const API_ERROR_NO_CODE: &str =
        r#"{"type":"assistant","isApiErrorMessage":true,"message":{"role":"assistant","content":[]}}"#;
    const ORDINARY_REPLY: &str =
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"ok"}]}}"#;

    /// One live session with a chosen status and transcript, reduced to its row.
    fn row_for(name: &str, status: &str, transcript: &[&str]) -> AgentSession {
        let p = profile(name);
        let cwd = "/Users/x/errs";
        let extra = format!(r#","entrypoint":"cli","status":"{status}""#);
        write_session(&p, 500, "sess-e", cwd, "Mon Aug 24 04:00:00 2026", &extra);
        if !transcript.is_empty() {
            write_transcript(&p, cwd, "sess-e", transcript);
        }
        let procs = table(&[(500, "Mon Aug 24 04:00:00 2026")]);
        let mut out = ClaudeAdapter::new().live_sessions(&[p], &procs);
        assert_eq!(out.len(), 1, "expected exactly one row");
        out.remove(0)
    }

    #[test]
    fn a_waiting_session_is_waiting_not_unknown() {
        // The bug this story exists to fix: `waiting` fell into the catch-all and
        // a session sitting on a permission prompt read as "state unknown".
        let a = profile("dir-waiting");
        write_session(
            &a,
            300,
            "sess-wait",
            "/Users/x/mu",
            "Mon Aug 24 04:00:00 2026",
            r#","entrypoint":"cli","status":"waiting","waitingFor":"input needed""#,
        );
        let procs = table(&[(300, "Mon Aug 24 04:00:00 2026")]);
        let out = ClaudeAdapter::new().live_sessions(&[a], &procs);
        assert_eq!(out[0].state, State::Waiting);
        assert_eq!(out[0].activity.as_deref(), Some("input needed"));
    }

    #[test]
    fn a_waiting_session_without_a_reason_still_waits() {
        let a = profile("dir-waiting-bare");
        write_session(
            &a,
            301,
            "sess-wb",
            "/Users/x/nu",
            "Mon Aug 24 04:00:00 2026",
            r#","entrypoint":"cli","status":"waiting""#,
        );
        let procs = table(&[(301, "Mon Aug 24 04:00:00 2026")]);
        let out = ClaudeAdapter::new().live_sessions(&[a], &procs);
        assert_eq!(out[0].state, State::Waiting);
        assert_eq!(out[0].activity, None);
    }

    #[test]
    fn shell_mode_is_working_rather_than_unknown() {
        // Nothing is wrong and nothing is wanted from the user, so it must not
        // sit in the attention vocabulary — nor in the unknown bucket.
        let a = profile("dir-shell");
        write_session(
            &a,
            302,
            "sess-sh",
            "/Users/x/xi",
            "Mon Aug 24 04:00:00 2026",
            r#","entrypoint":"cli","status":"shell""#,
        );
        let procs = table(&[(302, "Mon Aug 24 04:00:00 2026")]);
        let out = ClaudeAdapter::new().live_sessions(&[a], &procs);
        assert_eq!(out[0].state, State::Working);
    }

    #[test]
    fn a_status_this_build_has_never_heard_of_is_still_unknown() {
        let a = profile("dir-future");
        write_session(
            &a,
            303,
            "sess-f",
            "/Users/x/omicron",
            "Mon Aug 24 04:00:00 2026",
            r#","entrypoint":"cli","status":"hibernating""#,
        );
        let procs = table(&[(303, "Mon Aug 24 04:00:00 2026")]);
        let out = ClaudeAdapter::new().live_sessions(&[a], &procs);
        assert_eq!(out[0].state, State::Unknown);
    }

    #[test]
    fn a_session_that_died_on_an_error_is_errored() {
        let row = row_for("dir-err-died", "idle", &[ORDINARY_REPLY, API_ERROR_529]);
        assert_eq!(row.state, State::Errored);
        assert_eq!(row.activity.as_deref(), Some("Error: 529"));
    }

    #[test]
    fn an_error_without_a_status_code_still_reports_errored() {
        let row = row_for("dir-err-nocode", "idle", &[API_ERROR_NO_CODE]);
        assert_eq!(row.state, State::Errored);
        assert_eq!(row.activity.as_deref(), Some("Errored"));
    }

    #[test]
    fn an_error_the_session_carried_on_past_is_not_errored() {
        // Append-only transcripts keep every error forever. Only the newest entry
        // decides, or a session would stay marked dead for the rest of its life.
        let row = row_for("dir-err-past", "idle", &[API_ERROR_529, ORDINARY_REPLY]);
        assert_eq!(row.state, State::Idle);
        assert_eq!(row.activity, None);
    }

    #[test]
    fn a_busy_session_is_never_errored_even_with_a_trailing_error() {
        // Busy means the agent is already retrying through it.
        let row = row_for("dir-err-busy", "busy", &[API_ERROR_529]);
        assert_eq!(row.state, State::Working);
    }

    #[test]
    fn transcript_bookkeeping_does_not_clear_a_real_error() {
        // Snapshots and titles are written constantly and say nothing about
        // whether the session recovered.
        let row = row_for(
            "dir-err-noise",
            "idle",
            &[
                API_ERROR_529,
                r#"{"type":"file-history-snapshot","snapshot":{}}"#,
                r#"{"type":"ai-title","aiTitle":"something"}"#,
            ],
        );
        assert_eq!(row.state, State::Errored);
    }

    #[test]
    fn a_failed_tool_result_is_not_a_session_error() {
        // All sixteen in this project's transcripts are auto-mode classifier
        // denials the agent worked around.
        let row = row_for(
            "dir-err-tool",
            "idle",
            &[
                r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","is_error":true,"content":"Permission denied by classifier"}]}}"#,
            ],
        );
        assert_eq!(row.state, State::Idle);
    }

    #[test]
    fn an_errored_session_keeps_its_row() {
        // The process is still alive; hiding it would hide the error.
        let row = row_for("dir-err-visible", "idle", &[API_ERROR_529]);
        assert_eq!(row.display_name, "errs");
        assert_eq!(row.agent_id, "claude");
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
}
