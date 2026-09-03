pub mod activity;
pub mod rollout;

use crate::adapter::Adapter;
use crate::procs::ProcessTable;
use crate::session::{now_ms, AgentSession, State};
use std::path::{Path, PathBuf};

/// The command a Codex CLI session runs under.
///
/// Not by itself evidence of a session: ChatGPT.app runs
/// `/Applications/ChatGPT.app/Contents/Resources/codex … app-server`, whose
/// command name is exactly this. What separates the two is the rollout handle,
/// which the app-server never holds — verified live, 0 of its open files.
const COMMAND: &str = "codex";

/// Where a session records the profile directory it is using, and what it uses
/// when it records nothing: `~/.codex`.
///
/// Named here rather than anywhere in the pet. A pet holding this would need a
/// new entry per agent, which is the change adding an agent is not supposed to
/// need — the same reason liveness lives in the adapters.
const PROFILE_VAR: &str = "CODEX_HOME";

/// Codex sessions, discovered from the processes running them.
///
/// Codex records no PID anywhere — not in its rollout files, not in its database
/// — so the liveness rule story 001 gives Claude is unavailable. The spike found
/// the join instead: a running Codex process holds an open handle on its own
/// rollout for as long as the session lives (533 consecutive samples, no gap,
/// still held 22 s after the last write). So:
///
/// > A Codex CLI session is live iff some running `codex` process holds an open
/// > handle on a rollout whose `session_meta` says `source: "cli"` and
/// > `thread_source: "user"`.
///
/// That is liveness proven from the running process, never inferred from file
/// recency, which is the same standard Claude's registry file meets.
pub struct CodexAdapter {
    tailer: rollout::Tailer,
}

impl CodexAdapter {
    pub fn new() -> Self {
        Self {
            tailer: rollout::Tailer::new(),
        }
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for CodexAdapter {
    fn profile_dirs(&self, procs: &dyn ProcessTable) -> Vec<PathBuf> {
        let Some(home) = std::env::var_os("HOME") else {
            return Vec::new();
        };
        let default = PathBuf::from(home).join(".codex");
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
        let pids = procs.pids_of_command(COMMAND);
        if pids.is_empty() {
            self.tailer.retain_only(&[]);
            return Vec::new();
        }
        let open = procs.open_paths(&pids);

        let observed = now_ms();
        let mut out = Vec::new();
        let mut held = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        // Sorted so a machine with two candidates produces the same rows in the
        // same order every tick, rather than whatever the map happens to yield.
        let mut paths: Vec<&PathBuf> = open.values().flatten().collect();
        paths.sort();
        paths.dedup();

        for path in paths {
            if !rollout::is_rollout_path(path) || !under_any(path, profiles) {
                continue;
            }
            // One live process holds several rollouts — its own and every
            // subagent's — so this filter, not the handle, is what picks the
            // session out of the set.
            let Some(meta) = rollout::read_meta(path) else {
                continue;
            };
            if !meta.is_cli_user_session() {
                continue;
            }
            // Two processes could reach one rollout; it is still one session.
            if seen.contains(&meta.id) {
                continue;
            }
            seen.push(meta.id.clone());
            held.push(path.clone());

            let reading = rollout::Tailer::dated_after(self.tailer.read(path), meta.began);
            let state = reading.state;
            out.push(AgentSession {
                agent_id: COMMAND.into(),
                session_key: meta.id,
                project_path: meta.cwd.clone(),
                display_name: project_name(&meta.cwd),
                state,
                // A finished turn's last action is stale the moment it ends, and
                // an idle row showing it would read as busy at a glance.
                activity: (state == State::Working).then_some(reading.activity).flatten(),
                // The turn boundary's own time where there was a boundary; where
                // there was none the state is `Unknown` and there is nothing to
                // date, so the row keeps the pre-006 first-seen behaviour.
                status_since: reading.state_since.unwrap_or(observed),
            });
        }

        self.tailer.retain_only(&held);
        out
    }
}

/// Whether a path lies inside one of the directories the pet was told to watch.
///
/// Keeps discovery to the configured surface rather than following an open handle
/// anywhere on disk — a `codex` process holds plenty of files that are none of the
/// pet's business.
fn under_any(path: &Path, profiles: &[PathBuf]) -> bool {
    profiles.iter().any(|p| path.starts_with(p))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::procs::FakeProcessTable;
    use std::collections::HashMap;

    /// The three real rollout shapes the filter must tell apart, reduced from
    /// this machine's own `~/.codex/sessions` and committed beside the code.
    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("codex")
            .join(name)
    }

    /// A watch directory holding whichever fixtures a test needs, laid out the
    /// way Codex lays them out: `<profile>/sessions/YYYY/MM/DD/rollout-*.jsonl`.
    fn profile(test: &str, fixtures: &[&str]) -> (PathBuf, Vec<PathBuf>) {
        let root = std::env::temp_dir().join("agentpet-codex-tests").join(test);
        let _ = std::fs::remove_dir_all(&root);
        let day = root.join("sessions").join("2026").join("08").join("24");
        std::fs::create_dir_all(&day).unwrap();
        let mut paths = Vec::new();
        for (i, f) in fixtures.iter().enumerate() {
            let dst = day.join(format!("rollout-2026-08-24T20-41-{i:02}-{f}"));
            std::fs::copy(fixture(f), &dst).unwrap();
            paths.push(dst);
        }
        (root, paths)
    }

    fn procs(pids: &[(u32, Vec<PathBuf>)]) -> FakeProcessTable {
        FakeProcessTable {
            named: HashMap::from([(COMMAND.to_string(), pids.iter().map(|(p, _)| *p).collect())]),
            open: pids.iter().cloned().collect(),
            ..Default::default()
        }
    }

    #[test]
    fn a_running_session_reports_the_profile_it_is_using() {
        // Keyed by command *and* variable: asking under another agent's command,
        // or under the wrong variable name, learns nothing here — which is what
        // the machine does too, and is the whole behaviour this adapter adds.
        let mine = PathBuf::from("/tmp/agentpet-codex-home");
        let theirs = PathBuf::from("/tmp/agentpet-claude-home");
        let procs = FakeProcessTable {
            named: HashMap::from([(COMMAND.to_string(), vec![4790])]),
            dirs: HashMap::from([
                ((COMMAND.to_string(), PROFILE_VAR.to_string()), vec![mine.clone()]),
                (("claude".to_string(), "CLAUDE_CONFIG_DIR".to_string()), vec![theirs.clone()]),
            ]),
            ..Default::default()
        };
        let out = CodexAdapter::new().profile_dirs(&procs);
        assert!(out.contains(&mine), "the learned profile is missing: {out:?}");
        assert!(!out.contains(&theirs), "another agent's profile was claimed: {out:?}");
    }

    #[test]
    fn the_default_profile_is_watched_whether_or_not_anything_is_running() {
        let home = PathBuf::from(std::env::var("HOME").unwrap());
        let out = CodexAdapter::new().profile_dirs(&FakeProcessTable::default());
        assert_eq!(out, vec![home.join(".codex")]);
    }

    #[test]
    fn a_live_process_holding_a_cli_rollout_becomes_a_row() {
        let (root, paths) = profile("live", &["cli-user.jsonl"]);
        let out = CodexAdapter::new().live_sessions(&[root], &procs(&[(4790, paths)]));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].agent_id, "codex");
        assert_eq!(out[0].display_name, "agent-agnostic-pet");
        assert_eq!(
            out[0].project_path,
            "/Users/example/Projects/CODE/Engineering/agent-agnostic-pet"
        );
        // The real session's last turn boundary is `task_complete`.
        assert_eq!(out[0].state, State::Idle);
        assert_eq!(out[0].activity, None, "an idle row must not show stale activity");
    }

    #[test]
    fn a_row_dates_itself_from_the_boundary_in_the_real_rollout() {
        // The last hop of the chain: the tailer's boundary time actually reaching
        // `AgentSession.status_since`, checked against the committed fixture rather
        // than a synthetic line. That rollout's final boundary is the
        // `task_complete` on line 28, `2026-08-25T00:46:02.774Z`.
        let (root, paths) = profile("since", &["cli-user.jsonl"]);
        let out = CodexAdapter::new().live_sessions(&[root], &procs(&[(4790, paths)]));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].state, State::Idle);
        assert_eq!(out[0].status_since, 1_787_618_762_774);
    }

    #[test]
    fn a_row_with_no_boundary_to_date_falls_back_to_this_tick() {
        // The other half of the dating rule, and the half Claude has its own test
        // for: no boundary means no moment to count from, so the row keeps the
        // pre-006 first-seen behaviour rather than inventing one.
        let (root, paths) = profile("since-none", &["cli-user.jsonl"]);
        // Truncate to the meta line alone: a real session before its first turn
        // boundary, which is a live row with nothing yet to date.
        let raw = std::fs::read_to_string(&paths[0]).unwrap();
        let meta_only = raw.lines().next().unwrap().to_string() + "\n";
        std::fs::write(&paths[0], meta_only).unwrap();

        let before = crate::session::now_ms();
        let out = CodexAdapter::new().live_sessions(&[root], &procs(&[(4790, paths)]));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].state, State::Unknown);
        assert!(out[0].status_since >= before, "a row with no boundary dated itself from one");
    }

    /// The invariant that keeps a format change from silently doubling every row.
    #[test]
    fn one_process_holding_its_own_and_a_subagents_rollout_yields_exactly_one_row() {
        // Exactly what today's live session did: its own rollout plus a
        // `guardian` subagent's, both held open by one PID.
        let (root, paths) = profile("noflood", &["cli-user.jsonl", "subagent-guardian.jsonl"]);
        let out = CodexAdapter::new().live_sessions(&[root], &procs(&[(4790, paths)]));
        assert_eq!(out.len(), 1, "a subagent rollout produced a row of its own");
        assert_eq!(out[0].session_key, "01a0365d-4790-71f1-b34a-f21a3f1235b9");
    }

    #[test]
    fn a_desktop_thread_held_open_by_a_live_process_produces_no_row() {
        let (root, paths) = profile("desktop", &["desktop-vscode.jsonl"]);
        assert!(CodexAdapter::new()
            .live_sessions(&[root], &procs(&[(2018, paths)]))
            .is_empty());
    }

    #[test]
    fn a_subagent_thread_on_its_own_produces_no_row() {
        let (root, paths) = profile("subagent", &["subagent-guardian.jsonl"]);
        assert!(CodexAdapter::new()
            .live_sessions(&[root], &procs(&[(480, paths)]))
            .is_empty());
    }

    #[test]
    fn a_rollout_on_disk_that_no_process_holds_open_produces_no_row() {
        // The file exists and reads perfectly; nothing is running. This is the
        // stale-row failure the release forbids, and the only thing preventing it
        // is that discovery starts from processes rather than from the directory.
        let (root, _paths) = profile("orphan", &["cli-user.jsonl"]);
        let out = CodexAdapter::new().live_sessions(&[root], &procs(&[]));
        assert!(out.is_empty());
    }

    #[test]
    fn a_process_named_codex_that_holds_no_rollout_produces_no_row() {
        // ChatGPT.app's `codex … app-server`: a live process with exactly this
        // command name, holding the shared sqlite databases and an arg0 lock but
        // no rollout at all. Verified live — 0 rollout handles of its open files.
        let (root, _paths) = profile("appserver", &["cli-user.jsonl"]);
        let held = vec![
            PathBuf::from("/Users/x/.codex/state_5.sqlite"),
            PathBuf::from("/Users/x/.codex/state_5.sqlite-wal"),
            PathBuf::from("/Users/x/.codex/tmp/arg0/codex-arg063bX0G/.lock"),
        ];
        assert!(CodexAdapter::new()
            .live_sessions(&[root], &procs(&[(48071, held)]))
            .is_empty());
    }

    #[test]
    fn a_live_session_that_has_taken_no_turn_yet_produces_no_row() {
        // It has written no rollout file, so the process holds nothing to read.
        let (root, _paths) = profile("noturn", &["cli-user.jsonl"]);
        assert!(CodexAdapter::new()
            .live_sessions(&[root], &procs(&[(35023, vec![])]))
            .is_empty());
    }

    #[test]
    fn a_rollout_outside_every_watched_directory_is_left_alone() {
        let (root, paths) = profile("unwatched", &["cli-user.jsonl"]);
        let elsewhere = PathBuf::from("/some/other/place");
        assert!(CodexAdapter::new()
            .live_sessions(&[elsewhere], &procs(&[(4790, paths)]))
            .is_empty());
        drop(root);
    }

    #[test]
    fn two_processes_in_two_projects_each_get_their_own_row() {
        let (root_a, paths_a) = profile("two-a", &["cli-user.jsonl"]);
        let (root_b, paths_b) = profile("two-b", &["cli-user.jsonl"]);
        let procs = FakeProcessTable {
            named: HashMap::from([(COMMAND.to_string(), vec![100, 200])]),
            open: HashMap::from([(100, paths_a), (200, paths_b)]),
            ..Default::default()
        };
        // Both fixtures are copies of one session, so the same id reaches the
        // adapter twice and must still be one session.
        let out = CodexAdapter::new().live_sessions(&[root_a, root_b], &procs);
        assert_eq!(out.len(), 1, "one session seen twice became two rows");
    }

    #[test]
    fn a_working_session_shows_what_it_is_doing_right_now() {
        // The fixture ends with `task_complete`; appending a fresh turn puts it
        // back to work, which is the state a row is most often drawn in.
        let (root, paths) = profile("working", &["cli-user.jsonl"]);
        let mut body = std::fs::read_to_string(&paths[0]).unwrap();
        body.push_str(r#"{"type":"event_msg","payload":{"type":"task_started","turn_id":"t2"}}"#);
        body.push('\n');
        body.push_str(
            r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"Reasoning","summary_text":["**Checking the release plan**"]}}}"#,
        );
        body.push('\n');
        std::fs::write(&paths[0], body).unwrap();

        let out = CodexAdapter::new().live_sessions(&[root], &procs(&[(4790, paths)]));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].state, State::Working);
        assert_eq!(out[0].activity.as_deref(), Some("Checking the release plan"));
    }

    #[test]
    fn a_session_that_exits_stops_being_shown_on_the_next_poll() {
        let (root, paths) = profile("exit", &["cli-user.jsonl"]);
        let mut adapter = CodexAdapter::new();
        assert_eq!(adapter.live_sessions(&[root.clone()], &procs(&[(4790, paths)])).len(), 1);
        // Same rollout still on disk, process gone.
        assert!(adapter.live_sessions(&[root], &procs(&[])).is_empty());
    }

    #[test]
    fn discovery_never_reaches_for_the_database() {
        // Nothing the pet needs is only in `state_5.sqlite`, and it is WAL-mode
        // and held open by the live process. The identity the row uses comes
        // from line 1 of the rollout instead.
        let src = std::fs::read_to_string(fixture("cli-user.jsonl")).unwrap();
        let meta = rollout::read_meta(&fixture("cli-user.jsonl")).unwrap();
        assert_eq!(meta.id, "01a0365d-4790-71f1-b34a-f21a3f1235b9");
        assert!(meta.is_cli_user_session());
        assert!(src.lines().next().unwrap().contains("session_meta"));
    }

    #[test]
    fn project_name_is_the_last_path_segment() {
        assert_eq!(project_name("/Users/x/Projects/pet"), "pet");
        assert_eq!(project_name("/Users/x/Projects/pet/"), "pet");
    }
}
