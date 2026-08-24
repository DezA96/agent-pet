use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// One `<profile>/sessions/<pid>.json` file, as Claude Code writes it.
///
/// Deliberately tolerant of unknown fields: this is a file the agent owns and may
/// extend, and the pet must not break when it does.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEntry {
    pub pid: u32,
    pub session_id: String,
    pub cwd: String,
    /// Process start time as the agent recorded it — rendered in UTC.
    pub proc_start: String,
    #[serde(default)]
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    /// `busy` or `idle`. Absent for entrypoints that publish no status.
    #[serde(default)]
    pub status: Option<String>,
}

pub fn parse(raw: &str) -> Option<RegistryEntry> {
    serde_json::from_str(raw).ok()
}

/// Read every registry file in one profile's `sessions/` directory.
///
/// A directory that is absent or unreadable yields nothing rather than failing:
/// the pet is configured with directories that may legitimately not exist yet.
/// A single malformed file — most often one caught mid-write — is skipped, and
/// the rest of the directory still reports.
pub fn read_dir(profile: &Path) -> Vec<RegistryEntry> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(profile.join("sessions")) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Some(e) = parse(&raw) {
                out.push(e);
            }
        }
    }
    out
}

/// The three-part liveness rule.
///
/// A session counts as live only when all three hold: the registry file exists
/// (it was parsed, so it did), the PID it names is running, and that process's
/// actual start time matches the recorded `procStart`.
///
/// The third part is what makes the rule worth having. A force-killed session
/// cannot delete its own registry file, and PIDs are recycled — so "the file is
/// there and something with that PID is running" is not enough to conclude the
/// original session is alive.
///
/// `actual_starts` must be rendered in UTC, matching how the agent writes
/// `procStart`. `ps -o lstart` prints local time by default; comparing that
/// against the file unnormalised marks every live session dead.
pub fn is_live(entry: &RegistryEntry, actual_starts: &HashMap<u32, String>) -> bool {
    match actual_starts.get(&entry.pid) {
        Some(actual) => normalise(actual) == normalise(&entry.proc_start),
        None => false,
    }
}

/// `ps` pads its time column; the agent does not.
fn normalise(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEALTHY: &str = r#"{"pid":41173,"sessionId":"32dd885c-2a2a-4920-b9e4-cb00ef4ab5a2",
        "cwd":"/Users/a/Projects/pet","startedAt":1787544987907,
        "procStart":"Mon Aug 24 04:16:25 2026","version":"2.1.241","entrypoint":"cli",
        "name":"agent-agnostic-pet-02","status":"busy","statusUpdatedAt":1787594586507}"#;

    fn starts(pairs: &[(u32, &str)]) -> HashMap<u32, String> {
        pairs.iter().map(|(p, s)| (*p, s.to_string())).collect()
    }

    #[test]
    fn a_healthy_session_is_live() {
        let e = parse(HEALTHY).unwrap();
        let table = starts(&[(41173, "Mon Aug 24 04:16:25 2026")]);
        assert!(is_live(&e, &table));
    }

    #[test]
    fn an_orphaned_registry_file_is_not_live() {
        // Force-killed: the file survives, the process does not.
        let e = parse(HEALTHY).unwrap();
        let table = starts(&[]);
        assert!(!is_live(&e, &table));
    }

    #[test]
    fn a_reused_pid_with_a_different_start_time_is_not_live() {
        let e = parse(HEALTHY).unwrap();
        let table = starts(&[(41173, "Mon Aug 24 09:58:02 2026")]);
        assert!(!is_live(&e, &table));
    }

    #[test]
    fn ps_column_padding_does_not_break_the_match() {
        let e = parse(HEALTHY).unwrap();
        let table = starts(&[(41173, "  Mon Aug 24  04:16:25 2026   ")]);
        assert!(is_live(&e, &table));
    }

    #[test]
    fn a_local_time_rendering_must_not_be_accepted() {
        // The same live process, printed by `ps` without TZ=UTC (EDT is UTC-4).
        // If this ever passes, the timezone normalisation has been lost and every
        // live session would read as dead in the opposite direction.
        let e = parse(HEALTHY).unwrap();
        let table = starts(&[(41173, "Mon Aug 24 00:16:25 2026")]);
        assert!(!is_live(&e, &table));
    }

    #[test]
    fn unknown_fields_do_not_break_parsing() {
        let raw = r#"{"pid":1,"sessionId":"s","cwd":"/c","procStart":"x",
            "somethingTheAgentAddedLater":{"nested":true}}"#;
        assert!(parse(raw).is_some());
    }

    #[test]
    fn a_malformed_file_yields_nothing_rather_than_panicking() {
        assert!(parse("{ half written").is_none());
    }

    #[test]
    fn an_absent_directory_reads_as_empty() {
        assert!(read_dir(Path::new("/nonexistent/profile")).is_empty());
    }
}
