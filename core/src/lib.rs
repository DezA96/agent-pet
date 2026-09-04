//! The pet's observation core.
//!
//! Watches agent sessions by reading files those agents already write, and hands
//! the frontend a plain list of what is running right now. Everything here is
//! read-only: no file owned by any agent is created, modified or deleted, and
//! nothing observed leaves the machine.

pub mod adapter;
pub mod claude;
pub mod codex;
pub mod config;
pub mod procs;
pub mod profiles;
pub mod session;

use adapter::Adapter;
use procs::{ProcessTable, SystemProcessTable};
use serde::Serialize;
use session::AgentSession;
use std::ffi::{c_char, CString};
use std::sync::{Mutex, OnceLock};

/// One tick's answer.
///
/// `ok: false` is how the pet distinguishes "discovery itself failed" from
/// "discovery worked and nothing is running" — two states the story requires be
/// tellable apart, which an empty list alone could not express.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Poll {
    pub ok: bool,
    pub sessions: Vec<AgentSession>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Poll {
    fn failed(reason: String) -> Self {
        Poll {
            ok: false,
            sessions: Vec::new(),
            error: Some(reason),
        }
    }
}

/// Holds what must survive between ticks — the adapters and their transcript
/// read offsets, the process table's caches, the state each row last showed —
/// and where the config file is. Everything else is rebuilt each tick.
pub struct Observer {
    adapters: Vec<Box<dyn Adapter + Send>>,
    procs: SystemProcessTable,
    /// Where the pet's own config file lives, or `None` where no home directory
    /// could be found to put it under. Resolved once: a home directory does not
    /// move while the pet runs, and a poll that resolved it itself could only be
    /// tested against whatever file the developer's machine happened to hold.
    config: Option<std::path::PathBuf>,
    /// The state each session last displayed, and the time the row was counting
    /// from while it displayed it. Keyed by session key.
    previous: std::collections::HashMap<String, (session::State, u64)>,
}

impl Observer {
    pub fn new() -> Self {
        Self::reading(config::config_path())
    }

    /// An observer over the config file at `config`: the machine's, or a test's.
    fn reading(config: Option<std::path::PathBuf>) -> Self {
        Observer {
            adapters: vec![
                Box::new(claude::ClaudeAdapter::new()),
                Box::new(codex::CodexAdapter::new()),
            ],
            procs: SystemProcessTable::new(),
            config,
            previous: std::collections::HashMap::new(),
        }
    }

    pub fn poll(&mut self, procs: &dyn ProcessTable) -> Poll {
        // Every question this tick asks is answered from one snapshot of the
        // machine, and this is where that snapshot starts.
        procs.begin_poll();
        let Some(cfg_path) = self.config.as_deref() else {
            return Poll::failed("cannot locate a home directory".into());
        };
        let cfg = match config::load(cfg_path) {
            Ok(c) => c,
            Err(e) => return Poll::failed(e.to_string()),
        };

        // Each adapter first, then the union: the pet asks who wants what
        // watched and never names an agent to find out.
        let learned: Vec<std::path::PathBuf> = self
            .adapters
            .iter()
            .flat_map(|a| a.profile_dirs(procs))
            .collect();
        let dirs = profiles::candidate_directories(&cfg, &learned);
        if dirs.is_empty() {
            return Poll::failed("no directories to watch".into());
        }

        let mut sessions = Vec::new();
        for adapter in self.adapters.iter_mut() {
            sessions.extend(adapter.live_sessions(&dirs, procs));
        }

        self.pin_unchanged_states(&mut sessions);

        // A displayed name is not an identifier; make sure two rows never look
        // like the same session.
        session::disambiguate(&mut sessions);
        sessions.sort_by(|a, b| {
            a.display_name
                .cmp(&b.display_name)
                .then_with(|| a.session_key.cmp(&b.session_key))
        });

        Poll {
            ok: true,
            sessions,
            error: None,
        }
    }
}

impl Observer {
    /// Hold `status_since` still for as long as the displayed state holds still.
    ///
    /// The age answers one question — how long has what is on screen been true —
    /// so it may only move when the screen does. Two things move underneath it
    /// that the user cannot see, and both are pinned here rather than in either
    /// adapter, because both are about the *displayed* state and the pet owns
    /// that: the agent's status changing without the pet's state changing
    /// (`busy` to `shell`, both drawn as working, whose `statusUpdatedAt` would
    /// otherwise snap the row to `0s`), and the activity line changing within a
    /// turn (which before story 006 restarted the count on every tool call, so
    /// the number measured the last tool rather than the turn).
    ///
    /// The same pin also carries the fallback: a session whose agent timestamped
    /// nothing arrives stamped `now` every tick, and this holds it at the first
    /// tick that saw the reading — exactly the pre-006 behaviour, which is what
    /// the story asks for wherever there is no real answer.
    fn pin_unchanged_states(&mut self, sessions: &mut [AgentSession]) {
        for s in sessions.iter_mut() {
            if let Some((state, since)) = self.previous.get(&s.session_key) {
                if *state == s.state {
                    s.status_since = *since;
                }
            }
            self.previous
                .insert(s.session_key.clone(), (s.state, s.status_since));
        }
        // A session that has gone starts fresh if it ever comes back.
        let live: Vec<String> = sessions.iter().map(|s| s.session_key.clone()).collect();
        self.previous.retain(|k, _| live.contains(k));
    }
}

impl Default for Observer {
    fn default() -> Self {
        Self::new()
    }
}

fn shared() -> &'static Mutex<Observer> {
    static OBSERVER: OnceLock<Mutex<Observer>> = OnceLock::new();
    OBSERVER.get_or_init(|| Mutex::new(Observer::new()))
}

/// Poll once and return the result as a JSON string.
///
/// The single function the frontend calls. Keeping the boundary to one JSON
/// payload means the same core can sit behind a Swift app, a separate process,
/// or a frontend on another platform without the contract changing.
///
/// # Safety
/// The returned pointer must be released with [`agentpet_free`].
#[no_mangle]
pub extern "C" fn agentpet_poll() -> *mut c_char {
    let result = match shared().lock() {
        Ok(mut obs) => {
            // Borrowed out first so the cached process table can be passed to a
            // method on the same value.
            let procs = std::mem::take(&mut obs.procs);
            let result = obs.poll(&procs);
            obs.procs = procs;
            result
        }
        Err(_) => Poll::failed("observer unavailable".into()),
    };
    let json = serde_json::to_string(&result).unwrap_or_else(|e| {
        format!(r#"{{"ok":false,"sessions":[],"error":"cannot encode result: {e}"}}"#)
    });
    match CString::new(json) {
        Ok(c) => c.into_raw(),
        Err(_) => CString::new(r#"{"ok":false,"sessions":[]}"#)
            .unwrap()
            .into_raw(),
    }
}

/// Release a string returned by [`agentpet_poll`].
///
/// # Safety
/// `ptr` must have come from [`agentpet_poll`] and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn agentpet_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use procs::FakeProcessTable;

    use session::{AgentSession, State};

    fn sample(key: &str, state: State, activity: Option<&str>, at: u64) -> AgentSession {
        AgentSession {
            agent_id: "claude".into(),
            session_key: key.into(),
            project_path: "/p".into(),
            display_name: "p".into(),
            state,
            activity: activity.map(str::to_string),
            status_since: at,
        }
    }

    /// A config path that holds no file, so a poll runs on defaults — never the
    /// file on the developer's machine, whose contents no test may depend on.
    fn no_config() -> Option<std::path::PathBuf> {
        Some(std::env::temp_dir().join("agentpet-no-such-config").join("config.json"))
    }

    #[test]
    fn a_poll_reads_the_config_it_was_given_not_the_machines() {
        // Before the path was injected, every poll test read
        // `~/.config/agent-pet/config.json` on whatever machine ran it, and a
        // malformed one there failed three tests about other things entirely.
        let dir = std::env::temp_dir().join("agentpet-poll-config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, "{ not json").unwrap();

        let p = Observer::reading(Some(path.clone())).poll(&FakeProcessTable::default());
        std::fs::remove_file(&path).ok();
        assert!(!p.ok);
        assert!(
            p.error.as_deref().unwrap_or("").contains(&path.display().to_string()),
            "the failure does not name the file it read: {:?}",
            p.error
        );

        let p = Observer::reading(None).poll(&FakeProcessTable::default());
        assert!(!p.ok, "no home directory must be a failed tick, not a silent default");
    }

    #[test]
    fn an_unchanged_status_keeps_counting_up_instead_of_restarting() {
        let mut obs = Observer::new();
        let mut first = vec![sample("s1", State::Working, Some("Reading a.md"), 1_000)];
        obs.pin_unchanged_states(&mut first);
        assert_eq!(first[0].status_since, 1_000);

        // Next tick, same status, later clock. The age must not reset.
        let mut second = vec![sample("s1", State::Working, Some("Reading a.md"), 3_000)];
        obs.pin_unchanged_states(&mut second);
        assert_eq!(second[0].status_since, 1_000, "an unchanged status restarted its counter");
    }

    #[test]
    fn a_changed_activity_does_not_restart_the_count() {
        // Story 006: the age measures the turn, not the time since the last tool
        // call. Before it, this was the behaviour that made a working row's number
        // reset every few seconds.
        let mut obs = Observer::new();
        obs.pin_unchanged_states(&mut vec![sample("s1", State::Working, Some("Reading a.md"), 1_000)]);
        let mut next = vec![sample("s1", State::Working, Some("Editing b.md"), 3_000)];
        obs.pin_unchanged_states(&mut next);
        assert_eq!(next[0].status_since, 1_000);
    }

    #[test]
    fn a_status_moving_under_an_unchanged_state_does_not_move_the_age() {
        // `busy` to `shell`: the agent's status changed and its `statusUpdatedAt`
        // with it, but both are drawn as working, so the user saw no change.
        let mut obs = Observer::new();
        obs.pin_unchanged_states(&mut vec![sample("s1", State::Working, None, 1_000)]);
        let mut shell = vec![sample("s1", State::Working, None, 8_000)];
        obs.pin_unchanged_states(&mut shell);
        assert_eq!(shell[0].status_since, 1_000);

        // The same for a waiting session whose `waitingFor` reason changes.
        let mut obs = Observer::new();
        obs.pin_unchanged_states(&mut vec![sample("s2", State::Waiting, Some("dialog open"), 1_000)]);
        let mut reason = vec![sample("s2", State::Waiting, Some("input needed"), 8_000)];
        obs.pin_unchanged_states(&mut reason);
        assert_eq!(reason[0].status_since, 1_000);
    }

    #[test]
    fn a_changed_state_takes_the_agents_new_time() {
        let mut obs = Observer::new();
        obs.pin_unchanged_states(&mut vec![sample("s1", State::Working, None, 1_000)]);
        let mut next = vec![sample("s1", State::Idle, None, 3_000)];
        obs.pin_unchanged_states(&mut next);
        assert_eq!(next[0].status_since, 3_000);
    }

    #[test]
    fn a_session_that_left_and_returned_starts_fresh() {
        let mut obs = Observer::new();
        obs.pin_unchanged_states(&mut vec![sample("s1", State::Idle, None, 1_000)]);
        obs.pin_unchanged_states(&mut vec![]);
        let mut back = vec![sample("s1", State::Idle, None, 9_000)];
        obs.pin_unchanged_states(&mut back);
        assert_eq!(back[0].status_since, 9_000);
    }

    #[test]
    fn a_successful_poll_with_nothing_running_is_ok_and_empty() {
        // Distinct from a failure: the pet shows "no agents running", not an error.
        let procs = FakeProcessTable::default();
        let p = Observer::reading(no_config()).poll(&procs);
        assert!(p.ok);
        assert!(p.error.is_none());
    }

    #[test]
    fn a_failing_codex_discovery_leaves_the_poll_and_the_claude_rows_intact() {
        // One healthy Claude session, and a `codex` process holding nothing a
        // rollout reader can make sense of. An adapter that cannot answer must
        // cost the tick nothing but its own rows.
        let profile = std::env::temp_dir().join("agentpet-poll-isolation");
        let _ = std::fs::remove_dir_all(&profile);
        std::fs::create_dir_all(profile.join("sessions")).unwrap();
        std::fs::write(
            profile.join("sessions").join("900.json"),
            r#"{"pid":900,"sessionId":"sess-live","cwd":"/Users/x/omega","procStart":"Mon Aug 24 04:00:00 2026","entrypoint":"cli","status":"busy"}"#,
        )
        .unwrap();

        let procs = FakeProcessTable {
            starts: std::collections::HashMap::from([(900, "Mon Aug 24 04:00:00 2026".to_string())]),
            polls: Default::default(),
            dirs: std::collections::HashMap::from([(
                ("claude".to_string(), "CLAUDE_CONFIG_DIR".to_string()),
                vec![profile],
            )]),
            named: std::collections::HashMap::from([
                ("claude".to_string(), vec![900]),
                ("codex".to_string(), vec![901]),
            ]),
            open: std::collections::HashMap::from([(
                901,
                vec![std::path::PathBuf::from("/nonexistent/rollout-gone.jsonl")],
            )]),
        };

        let p = Observer::reading(no_config()).poll(&procs);
        assert!(p.ok, "a Codex failure failed the whole poll");
        assert!(p.error.is_none());
        assert!(
            p.sessions.iter().any(|s| s.session_key == "sess-live"),
            "the Claude row was lost: {:?}",
            p.sessions
        );
        assert!(p.sessions.iter().all(|s| s.agent_id != "codex"));
    }

    #[test]
    fn a_codex_session_under_a_profile_the_pet_was_never_told_about_draws_a_row() {
        // The gap preflight run 1 found: `CODEX_HOME` set away from `~/.codex`,
        // nothing in the config file, so the rollout lies outside every watched
        // directory and `under_any` drops it. The row exists only because the
        // Codex adapter asks the process table for its own profile directories,
        // the way the Claude adapter does.
        let root = std::env::temp_dir().join("agentpet-codex-home");
        let _ = std::fs::remove_dir_all(&root);
        let day = root.join("sessions").join("2026").join("08").join("24");
        std::fs::create_dir_all(&day).unwrap();
        let rollout = day.join("rollout-2026-08-24T20-41-00-cli-user.jsonl");
        std::fs::copy(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures")
                .join("codex")
                .join("cli-user.jsonl"),
            &rollout,
        )
        .unwrap();

        // `lsof` reports the resolved path, and the candidate list is resolved
        // too, so the test hands over the same spelling the machine would.
        let rollout = std::fs::canonicalize(&rollout).unwrap();

        let procs = FakeProcessTable {
            dirs: std::collections::HashMap::from([(
                ("codex".to_string(), "CODEX_HOME".to_string()),
                vec![root],
            )]),
            named: std::collections::HashMap::from([("codex".to_string(), vec![7710])]),
            open: std::collections::HashMap::from([(7710, vec![rollout])]),
            ..Default::default()
        };

        let p = Observer::reading(no_config()).poll(&procs);
        assert!(p.ok);
        assert!(
            p.sessions.iter().any(|s| s.agent_id == "codex"),
            "a live Codex session under a learned profile drew no row: {:?}",
            p.sessions
        );
    }

    #[test]
    fn every_poll_tells_the_process_table_to_take_a_fresh_snapshot() {
        // The table serves one process listing to everything that asks inside a
        // tick; a poll that failed to say it had begun would let the next tick
        // answer from the last one, and an exited session would keep its row.
        let procs = FakeProcessTable::default();
        let mut obs = Observer::reading(no_config());
        obs.poll(&procs);
        obs.poll(&procs);
        assert_eq!(procs.polls.get(), 2);
    }

    #[test]
    fn the_payload_shape_is_what_the_frontend_decodes() {
        let procs = FakeProcessTable::default();
        let p = Observer::reading(no_config()).poll(&procs);
        let json = serde_json::to_string(&p).unwrap();
        let back: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(back.get("ok").unwrap().is_boolean());
        assert!(back.get("sessions").unwrap().is_array());
    }

    #[test]
    fn every_key_a_session_row_is_decoded_by_is_on_the_wire() {
        // The test above polls an empty machine, so it never sees a session object
        // and cannot catch a renamed key. Swift's `AgentSession` lives in the
        // `AgentPet` module, which `Package.swift` deliberately keeps out of
        // SwiftPM, so nothing on that side compiles against this contract either —
        // a key renamed on one side and not the other would reach the surface as
        // an empty pet with a decode error, and only a manual run would show it.
        let p = Poll {
            ok: true,
            sessions: vec![sample("s1", State::Working, Some("Reading a.md"), 1_000)],
            error: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        let row = &serde_json::from_str::<serde_json::Value>(&json).unwrap()["sessions"][0];
        for key in [
            "agentId",
            "sessionKey",
            "projectPath",
            "displayName",
            "state",
            "activity",
            "statusSince",
        ] {
            assert!(row.get(key).is_some(), "{key} is missing from {json}");
        }
        assert_eq!(row["statusSince"], 1_000);
        // The state's *vocabulary*, not just its presence. Swift maps any word it
        // does not recognise to `unknown` rather than throwing, so losing the
        // lowercase rename would turn every row grey with no decode error, no test
        // failure and no symptom to chase — a quieter failure than a missing key.
        assert_eq!(row["state"], "working");
        for (state, word) in [
            (State::Idle, "idle"),
            (State::Waiting, "waiting"),
            (State::Errored, "errored"),
            (State::Unknown, "unknown"),
        ] {
            let one = serde_json::to_value(sample("s", state, None, 0)).unwrap();
            assert_eq!(one["state"], word);
        }
    }

    #[test]
    fn ffi_round_trips_and_frees_cleanly() {
        let ptr = agentpet_poll();
        assert!(!ptr.is_null());
        let s = unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();
        unsafe { agentpet_free(ptr) };
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert!(v.get("ok").is_some());
        assert!(v.get("sessions").unwrap().is_array());
    }
}
