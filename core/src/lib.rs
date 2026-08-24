//! The pet's observation core.
//!
//! Watches agent sessions by reading files those agents already write, and hands
//! the frontend a plain list of what is running right now. Everything here is
//! read-only: no file owned by any agent is created, modified or deleted, and
//! nothing observed leaves the machine.

pub mod adapter;
pub mod claude;
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

/// Holds what must survive between ticks: transcript read offsets and resolved
/// transcript paths. Everything else is rebuilt each tick.
pub struct Observer {
    adapters: Vec<Box<dyn Adapter + Send>>,
    procs: SystemProcessTable,
    /// What each session last reported, and when that reading first appeared.
    /// Keyed by session, holding (state, activity, first seen).
    previous: std::collections::HashMap<String, (session::State, Option<String>, u64)>,
}

impl Observer {
    pub fn new() -> Self {
        Observer {
            adapters: vec![Box::new(claude::ClaudeAdapter::new())],
            procs: SystemProcessTable::new(),
            previous: std::collections::HashMap::new(),
        }
    }

    pub fn poll(&mut self, procs: &dyn ProcessTable) -> Poll {
        let Some(cfg_path) = config::config_path() else {
            return Poll::failed("cannot locate a home directory".into());
        };
        let cfg = match config::load(&cfg_path) {
            Ok(c) => c,
            Err(e) => return Poll::failed(e.to_string()),
        };

        let dirs = profiles::candidate_directories(&cfg, procs);
        if dirs.is_empty() {
            return Poll::failed("no directories to watch".into());
        }

        let mut sessions = Vec::new();
        for adapter in self.adapters.iter_mut() {
            sessions.extend(adapter.live_sessions(&dirs, procs));
        }

        self.age_unchanged_statuses(&mut sessions);

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
    /// Keep `observed_at` pinned to when a status first appeared.
    ///
    /// The row counts up from this, so re-stamping it every tick would reset the
    /// counter every couple of seconds and it could never show a status's real
    /// age. A reading is "the same" when both the state and the activity line are
    /// unchanged; either one moving is a new status and restarts the count.
    fn age_unchanged_statuses(&mut self, sessions: &mut [AgentSession]) {
        for s in sessions.iter_mut() {
            match self.previous.get(&s.session_key) {
                Some((state, activity, first_seen))
                    if *state == s.state && *activity == s.activity =>
                {
                    s.observed_at = *first_seen;
                }
                _ => {}
            }
            self.previous.insert(
                s.session_key.clone(),
                (s.state, s.activity.clone(), s.observed_at),
            );
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
            observed_at: at,
        }
    }

    #[test]
    fn an_unchanged_status_keeps_counting_up_instead_of_restarting() {
        let mut obs = Observer::new();
        let mut first = vec![sample("s1", State::Working, Some("Reading a.md"), 1_000)];
        obs.age_unchanged_statuses(&mut first);
        assert_eq!(first[0].observed_at, 1_000);

        // Next tick, same status, later clock. The age must not reset.
        let mut second = vec![sample("s1", State::Working, Some("Reading a.md"), 3_000)];
        obs.age_unchanged_statuses(&mut second);
        assert_eq!(second[0].observed_at, 1_000, "an unchanged status restarted its counter");
    }

    #[test]
    fn a_changed_activity_restarts_the_count() {
        let mut obs = Observer::new();
        obs.age_unchanged_statuses(&mut vec![sample("s1", State::Working, Some("Reading a.md"), 1_000)]);
        let mut next = vec![sample("s1", State::Working, Some("Editing b.md"), 3_000)];
        obs.age_unchanged_statuses(&mut next);
        assert_eq!(next[0].observed_at, 3_000);
    }

    #[test]
    fn a_changed_state_restarts_the_count() {
        let mut obs = Observer::new();
        obs.age_unchanged_statuses(&mut vec![sample("s1", State::Working, None, 1_000)]);
        let mut next = vec![sample("s1", State::Idle, None, 3_000)];
        obs.age_unchanged_statuses(&mut next);
        assert_eq!(next[0].observed_at, 3_000);
    }

    #[test]
    fn a_session_that_left_and_returned_starts_fresh() {
        let mut obs = Observer::new();
        obs.age_unchanged_statuses(&mut vec![sample("s1", State::Idle, None, 1_000)]);
        obs.age_unchanged_statuses(&mut vec![]);
        let mut back = vec![sample("s1", State::Idle, None, 9_000)];
        obs.age_unchanged_statuses(&mut back);
        assert_eq!(back[0].observed_at, 9_000);
    }

    #[test]
    fn a_successful_poll_with_nothing_running_is_ok_and_empty() {
        // Distinct from a failure: the pet shows "no agents running", not an error.
        let procs = FakeProcessTable {
            starts: Default::default(),
            dirs: vec![],
        };
        let p = Observer::new().poll(&procs);
        assert!(p.ok);
        assert!(p.error.is_none());
    }

    #[test]
    fn the_payload_shape_is_what_the_frontend_decodes() {
        let procs = FakeProcessTable {
            starts: Default::default(),
            dirs: vec![],
        };
        let p = Observer::new().poll(&procs);
        let json = serde_json::to_string(&p).unwrap();
        let back: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(back.get("ok").unwrap().is_boolean());
        assert!(back.get("sessions").unwrap().is_array());
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
