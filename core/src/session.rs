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
    /// When the displayed state began, unix ms. The pet counts up from here.
    ///
    /// Taken from the agent's own record of when the status changed wherever the
    /// agent timestamped one — Claude's `statusUpdatedAt`, the transcript entry an
    /// error stopped on, a Codex turn boundary's `timestamp`. Only where the agent
    /// timestamped nothing does it fall back to when the pet first saw the reading,
    /// which is all it ever meant before story 006 and is why the field is no
    /// longer called `observed_at`: a name saying "when we looked" would now be
    /// wrong for every row that has a real answer.
    pub status_since: u64,
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

/// Parse an agent-written timestamp — `2026-08-25T00:41:35.372Z` — to unix ms.
///
/// Both agents write RFC 3339 in UTC with millisecond precision, and both are
/// read for the same purpose: the moment a status began. Parsed by hand rather
/// than by pulling in a date crate, because this crate has two dependencies on
/// purpose and this is the only date arithmetic in it.
///
/// Only UTC is accepted. A local-time rendering would be silently hours out, and
/// an age that is wrong by hours is worse than one that falls back to first-seen —
/// so anything not ending in `Z`, and anything otherwise malformed, is `None` and
/// takes the fallback path rather than a guess. The same reasoning, and the same
/// failure, as the `procStart` comparison in `claude::registry`.
pub fn parse_iso8601_ms(raw: &str) -> Option<u64> {
    let raw = raw.trim();
    let rest = raw.strip_suffix('Z').or_else(|| raw.strip_suffix('z'))?;
    let (date, time) = rest.split_once('T')?;

    let mut d = date.split('-');
    let year = digits(d.next()?)? as i64;
    let month = digits(d.next()?)? as u32;
    let day = digits(d.next()?)? as u32;
    // The year is bounded, not merely parsed. Rust's own integer parse accepts
    // `9223372036854775807`, and the civil-date arithmetic below overflows on it —
    // a panic in a debug build, across an `extern "C"` boundary. Bounding the year
    // here is what makes that arithmetic total rather than guarding each operation
    // in it, and no agent writes a year outside this range.
    if d.next().is_some() {
        return None;
    }

    let (clock, frac) = match time.split_once('.') {
        Some((c, f)) => (c, f),
        None => (time, ""),
    };
    let mut t = clock.split(':');
    let hour = digits(t.next()?)?;
    let minute = digits(t.next()?)?;
    let second = digits(t.next()?)?;
    if t.next().is_some() || second > 60 {
        return None;
    }
    // A leap second is clamped back onto :59 rather than rejected. One second
    // early beats losing the age of a real status over a second that only exists
    // on paper — no agent has written one, and neither would notice.
    let second = second.min(59);

    // Milliseconds, however many digits the agent wrote. Anything finer is
    // truncated, which is the right direction: an age must never round forward
    // into the future. A dot with nothing after it is malformed, not `.000`.
    //
    // Validated digit by digit rather than through `digits()`, which parses: a
    // nanosecond fraction is nine digits and a longer one is still a fraction,
    // but past nineteen it overflows a `u64` and the whole timestamp would be
    // refused over precision nobody asked to keep.
    if time.contains('.') && frac.is_empty() {
        return None;
    }
    if !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let millis: u64 = frac
        .chars()
        .chain(std::iter::repeat('0'))
        .take(3)
        .collect::<String>()
        .parse()
        .ok()?;

    Some(civil_to_ms(year, month, day, hour, minute, second)? + millis)
}

/// A validated civil date and time, in UTC, as unix ms.
///
/// Shared by the two timestamp formats the agents write — Codex's RFC 3339 lines
/// and Claude's `procStart` — so the range rules and the arithmetic exist once
/// rather than once per agent.
///
/// `None` outside `1970..=9999`, or on a date that month does not have. The year
/// bound is what makes the arithmetic total: Rust's integer parse accepts
/// `9223372036854775807`, on which the civil-date conversion overflows — a panic
/// in a debug build, across an `extern "C"` boundary.
pub fn civil_to_ms(year: i64, month: u32, day: u32, hour: u64, minute: u64, second: u64) -> Option<u64> {
    if !(1970..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || day < 1
        || day > days_in(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    // Every operand is bounded by the checks above — year at 9999, so days at
    // under 2.94 million and the whole result under 2.6e14 — which is why plain
    // arithmetic is sound here and the year bound is not cosmetic.
    let days = days_from_civil(year, month, day) as u64;
    Some((days * 86_400 + hour * 3600 + minute * 60 + second) * 1000)
}

/// One timestamp component: ASCII digits and nothing else.
///
/// Rust's integer parse accepts a leading sign, so `2026-08-25T+05:41:35Z` would
/// otherwise read as hour 5 — a plausible-looking time that is not the one the
/// agent wrote, which is the one failure mode worse than a rejection here.
fn digits(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// How many days that month has, leap years included.
fn days_in(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Days since 1970-01-01 for a proleptic Gregorian date. Howard Hinnant's
/// `days_from_civil`, which is exact and needs no tables.
///
/// Total for the range its only caller admits — `1970..=9999`, validated there —
/// so it returns a plain value rather than an `Option` it would never fill.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let m = month as i64;
    let d = day as i64;
    let y = if m <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
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
            status_since: 0,
        }
    }

    #[test]
    fn an_agent_timestamp_parses_to_unix_ms() {
        // Checked against `date -u -j -f '%Y-%m-%dT%H:%M:%S' 2026-08-25T00:41:35 +%s`.
        assert_eq!(parse_iso8601_ms("2026-08-25T00:41:35.372Z"), Some(1_787_618_495_372));
        assert_eq!(parse_iso8601_ms("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(parse_iso8601_ms("2026-09-02T14:04:08.964Z"), Some(1_788_357_848_964));
    }

    #[test]
    fn a_missing_or_short_fraction_is_padded_not_misread() {
        assert_eq!(parse_iso8601_ms("2026-08-25T00:41:35Z"), Some(1_787_618_495_000));
        assert_eq!(parse_iso8601_ms("2026-08-25T00:41:35.3Z"), Some(1_787_618_495_300));
        assert_eq!(parse_iso8601_ms("2026-08-25T00:41:35.37Z"), Some(1_787_618_495_370));
        // Finer than milliseconds truncates rather than rounding forward.
        assert_eq!(parse_iso8601_ms("2026-08-25T00:41:35.372999Z"), Some(1_787_618_495_372));
    }

    #[test]
    fn a_leap_second_is_clamped_back_rather_than_losing_the_timestamp() {
        // UTC's 61st second. One second early beats refusing a real status's age
        // over a second that only exists on paper.
        assert_eq!(
            parse_iso8601_ms("2026-08-25T00:41:60.000Z"),
            parse_iso8601_ms("2026-08-25T00:41:59.000Z"),
        );
        // Past the 61st, it is not a time at all.
        assert_eq!(parse_iso8601_ms("2026-08-25T00:41:61.000Z"), None);
    }

    #[test]
    fn a_fraction_longer_than_a_u64_still_truncates_to_milliseconds() {
        // Nine digits is a nanosecond fraction; twenty overflows a parse. Neither
        // is a reason to refuse the timestamp and fall back to first-seen.
        assert_eq!(parse_iso8601_ms("2026-08-25T00:41:35.372999999Z"), Some(1_787_618_495_372));
        assert_eq!(
            parse_iso8601_ms("2026-08-25T00:41:35.37299999999999999999Z"),
            Some(1_787_618_495_372),
        );
    }

    #[test]
    fn a_leap_day_is_not_off_by_one() {
        assert_eq!(parse_iso8601_ms("2024-02-29T00:00:00.000Z"), Some(1_709_164_800_000));
        assert_eq!(parse_iso8601_ms("2024-03-01T00:00:00.000Z"), Some(1_709_251_200_000));
    }

    #[test]
    fn anything_not_utc_or_not_a_timestamp_is_none() {
        // A local-time rendering would be hours out, silently. Refused.
        assert_eq!(parse_iso8601_ms("2026-08-25T00:41:35.372+02:00"), None);
        assert_eq!(parse_iso8601_ms("2026-08-25 00:41:35Z"), None);
        assert_eq!(parse_iso8601_ms("2026-08-25T00:41Z"), None);
        assert_eq!(parse_iso8601_ms("2026-13-01T00:00:00Z"), None);
        // A day the month does not have. Unvalidated, `days_from_civil` rolls it
        // forward silently — 2026-02-31 would read as 2026-03-03, an age three
        // days wrong rather than an honest fallback.
        assert_eq!(parse_iso8601_ms("2026-02-31T00:00:00Z"), None);
        assert_eq!(parse_iso8601_ms("2026-02-29T00:00:00Z"), None, "2026 is not a leap year");
        assert_eq!(parse_iso8601_ms("2026-04-31T00:00:00Z"), None);
        assert_eq!(parse_iso8601_ms("2026-01-00T00:00:00Z"), None);
        // Refused by the year floor, not by the century rule — `1900-01-01` is
        // refused too. The century rule below the floor is unreachable and is not
        // claimed here; the first year it could matter for is 2100.
        assert_eq!(parse_iso8601_ms("1900-01-01T00:00:00Z"), None, "below the year floor");
        assert_eq!(parse_iso8601_ms(""), None);
        assert_eq!(parse_iso8601_ms("not a date"), None);
        // A signed component parses as an integer but is not a timestamp. Left
        // accepted, `T+05:41:35Z` would read as hour 5 — plausible, and wrong.
        assert_eq!(parse_iso8601_ms("2026-08-25T+05:41:35Z"), None);
        assert_eq!(parse_iso8601_ms("2026-08-25T-05:41:35Z"), None);
        assert_eq!(parse_iso8601_ms("+2026-08-25T00:41:35Z"), None);
        // A dot with no fraction after it is malformed, not `.000`.
        assert_eq!(parse_iso8601_ms("2026-08-25T00:41:35.Z"), None);
        // Pre-epoch, and a year large enough to overflow the civil-date
        // arithmetic. Both refused rather than wrapped — the second panicked in a
        // debug build before the year was bounded.
        assert_eq!(parse_iso8601_ms("1969-12-31T23:59:59.999Z"), None);
        assert_eq!(parse_iso8601_ms("9223372036854775807-01-01T00:00:00Z"), None);
        assert_eq!(parse_iso8601_ms("10000-01-01T00:00:00Z"), None);
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
