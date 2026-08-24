//! Exercises the real process-table path against a given profile directory.
//! Development aid, not shipped.
use agentpet_core::{adapter::Adapter, claude::ClaudeAdapter, procs::SystemProcessTable};
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("profile dir"));
    let out = ClaudeAdapter::new().live_sessions(&[dir], &SystemProcessTable::new());
    println!("{}", serde_json::to_string(&out).unwrap());
}
