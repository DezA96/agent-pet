use serde::{Deserialize, Deserializer};
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
    /// `busy`, `idle`, `waiting` or `shell`. Absent for entrypoints that publish
    /// no status at all.
    ///
    /// Story 001 recorded this as `busy|idle`, which was wrong and cost the pet a
    /// state: the agent publishes `waiting` whenever a session is blocked on a
    /// dialog, and `shell` while the user drives a shell inside it. Both landed in
    /// `Unknown`, so a session sitting on a permission prompt read as "state
    /// unknown" rather than as the one thing on screen actually asking for the
    /// user.
    #[serde(default)]
    pub status: Option<String>,
    /// Why a `waiting` session is waiting, in the agent's own words — e.g.
    /// `sandbox request`, `input needed`, `worker request`, `dialog open`, or
    /// whatever the live dialog calls itself.
    ///
    /// Not a closed set: the agent passes its top dialog's own label straight
    /// through, so this is rendered as arbitrary text and truncated like any
    /// other line.
    #[serde(default)]
    pub waiting_for: Option<String>,
    /// Unix ms of when `status` last actually changed, as the agent recorded it.
    ///
    /// The whole point of story 006: the pet's row counts up from this rather than
    /// from the tick it happened to first read the file, so a session idle for
    /// ninety-six minutes says so even if the pet started thirty seconds ago.
    ///
    /// Read through `Value` rather than straight into `Option<u64>` because this
    /// field belongs to the agent: a build that wrote it as a string, or as a
    /// float, would fail the whole entry to parse and cost the row entirely.
    /// Anything that is not a whole number reads as absent, which falls back to
    /// first-seen — a weaker age, not a missing session.
    #[serde(default, deserialize_with = "lenient_ms")]
    pub status_updated_at: Option<u64>,
}

/// Read a millisecond timestamp without letting a surprise cost the whole entry.
fn lenient_ms<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
    Ok(match Option::<serde_json::Value>::deserialize(d)? {
        Some(serde_json::Value::Number(n)) => n.as_u64(),
        _ => None,
    })
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

/// `procStart` — `Mon Aug 24 04:16:25 2026`, in UTC — as unix ms.
///
/// Read so the age has something to be checked against. A session's status cannot
/// have changed before the process publishing it existed, and the process's own
/// start is the only bound available that is a fact rather than a threshold
/// somebody chose. `None` for anything unparseable, which simply means no bound
/// is applied rather than a session being refused.
pub fn proc_start_ms(raw: &str) -> Option<u64> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let normalised = normalise(raw);
    let mut f = normalised.split(' ');
    let _weekday = f.next()?;
    let month_name = f.next()?;
    let month = MONTHS.iter().position(|m| *m == month_name)? as u32 + 1;
    let day: u32 = f.next()?.parse().ok()?;
    let mut clock = f.next()?.split(':');
    let hour: u64 = clock.next()?.parse().ok()?;
    let minute: u64 = clock.next()?.parse().ok()?;
    let second: u64 = clock.next()?.parse().ok()?;
    let year: i64 = f.next()?.parse().ok()?;
    if clock.next().is_some() || f.next().is_some() {
        return None;
    }
    crate::session::civil_to_ms(year, month, day, hour, minute, second)
}

impl RegistryEntry {
    /// When this session's status began, unix ms — or `None` where the agent gave
    /// nothing usable to date it from.
    ///
    /// A `statusUpdatedAt` is usable only alongside a `status`, since otherwise it
    /// would time a state the pet never read, and only when it is not older than
    /// the process itself. That second test is what keeps a value in the wrong
    /// unit off the surface: `statusUpdatedAt` arrives as a bare number with no
    /// parsing to fail, so seconds where milliseconds were meant reads as 1970 and
    /// renders as an age of half a million hours — wide enough to push the project
    /// name out of its own row. Refused rather than clamped: the pet cannot tell
    /// what the agent meant, and an age counted from first-seen is honest where a
    /// clamp would assert a precision nobody has.
    pub fn status_began(&self) -> Option<u64> {
        let at = self.status_updated_at?;
        self.status.as_ref()?;
        match proc_start_ms(&self.proc_start) {
            Some(started) if at < started => None,
            _ => Some(at),
        }
    }
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
    fn the_status_change_time_is_read() {
        let e = parse(HEALTHY).unwrap();
        assert_eq!(e.status_updated_at, Some(1_787_594_586_507));
    }

    #[test]
    fn a_missing_or_unusable_status_time_reads_as_absent_and_keeps_the_entry() {
        // Absent: an older agent, or an entrypoint that publishes no status.
        let bare = r#"{"pid":1,"sessionId":"s","cwd":"/c","procStart":"x"}"#;
        assert_eq!(parse(bare).unwrap().status_updated_at, None);
        // Present but not a whole number. The row must survive; only its age is
        // weaker, falling back to first-seen.
        for odd in [r#""1787594586507""#, "null", "1787594586507.5", "[1]", "-1"] {
            let raw = format!(
                r#"{{"pid":1,"sessionId":"s","cwd":"/c","procStart":"x","statusUpdatedAt":{odd}}}"#
            );
            let e = parse(&raw).unwrap_or_else(|| panic!("entry lost over statusUpdatedAt {odd}"));
            assert_eq!(e.status_updated_at, None, "for {odd}");
        }
    }

    #[test]
    fn proc_start_reads_as_the_utc_moment_the_process_began() {
        // The same string the liveness rule compares, now also read as a time.
        assert_eq!(proc_start_ms("Mon Aug 24 04:16:25 2026"), Some(1_787_544_985_000));
        // `ps` pads its columns and the agent does not; both spell one moment.
        assert_eq!(proc_start_ms("  Mon Aug 24  04:16:25 2026  "), Some(1_787_544_985_000));
        // A single-digit day, which the agent pads to keep the column width — the
        // form actually on this machine, and the one that would quietly disable
        // the bound rather than fail anything if it stopped parsing.
        assert_eq!(proc_start_ms("Wed Sep  2 23:46:55 2026"), Some(1_788_392_815_000));
        for bad in ["", "Mon Aug 24 04:16:25", "Mon Xxx 24 04:16:25 2026", "not a time"] {
            assert_eq!(proc_start_ms(bad), None, "for {bad:?}");
        }
    }

    #[test]
    fn a_status_time_older_than_the_process_is_refused_rather_than_shown() {
        // The whole reachable case: `statusUpdatedAt` is a bare number, so a value
        // in the wrong unit fails no parse. Seconds where ms were meant reads as
        // 1970 and would render as an age of about 496,766 hours — a label wide
        // enough to squeeze the project name out of its own row.
        let with = |v: &str| {
            format!(
                r#"{{"pid":1,"sessionId":"s","cwd":"/c","procStart":"Mon Aug 24 04:16:25 2026",
                "status":"busy","statusUpdatedAt":{v}}}"#
            )
        };
        // Unix seconds, not milliseconds.
        assert_eq!(parse(&with("1787594586")).unwrap().status_began(), None);
        assert_eq!(parse(&with("0")).unwrap().status_began(), None);
        // A plausible value, after the process started, is kept.
        assert_eq!(
            parse(&with("1787594586507")).unwrap().status_began(),
            Some(1_787_594_586_507)
        );
        // Exactly the process's own start is not "before" it.
        assert_eq!(
            parse(&with("1787544985000")).unwrap().status_began(),
            Some(1_787_544_985_000)
        );
    }

    #[test]
    fn an_unreadable_proc_start_applies_no_bound_rather_than_dropping_the_age() {
        // No bound available is not evidence against the timestamp.
        let raw = r#"{"pid":1,"sessionId":"s","cwd":"/c","procStart":"who knows",
            "status":"busy","statusUpdatedAt":1787594586507}"#;
        assert_eq!(parse(raw).unwrap().status_began(), Some(1_787_594_586_507));
    }

    #[test]
    fn a_status_time_with_no_status_beside_it_is_not_used() {
        let raw = r#"{"pid":1,"sessionId":"s","cwd":"/c","procStart":"Mon Aug 24 04:16:25 2026",
            "statusUpdatedAt":1787594586507}"#;
        assert_eq!(parse(raw).unwrap().status_began(), None);
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
