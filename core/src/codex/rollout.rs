use crate::codex::activity::activity_of;
use crate::session::State;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// How far back from the end of an unseen rollout to read in full.
///
/// Story 001 reads a Claude transcript from offset 0, which is safe at its ~102 KB.
/// A Codex rollout on this disk is 74 MB and a single-turn session reached 1.3 MB,
/// so reading one whole would stall a poll for seconds at pet startup. Starting at
/// bare EOF was rejected as worse: a session already mid-turn would read `unknown`
/// until its next event, breaking the release's within-a-few-seconds spot check.
///
/// This window covers the activity line and, for a settled session, its state:
/// `task_complete` lands 214 to 4,799 bytes from EOF across every rollout on this
/// disk, because it is written as the turn ends. A session still *working* is the
/// other case entirely — see [`MAX_BACKSCAN`].
const FIRST_READ_WINDOW: u64 = 256 * 1024;

/// How far back to keep looking for a turn boundary when the window held none.
///
/// A working session's last boundary is its `task_started`, and that is however
/// far back the turn began — 987 KB into a live single-turn session measured
/// here, and 22.8 MB at the widest gap between two boundaries on this disk. No
/// fixed window reaches that, and a working session is precisely the one the pet
/// exists to show, so the first sight of a rollout falls back to scanning
/// backward for a boundary rather than reporting `unknown` for the whole turn.
///
/// Only boundaries are looked for here, and a cheap substring test rejects the
/// enormous `item_completed` lines before any JSON parse, so this stays a memory
/// scan rather than a parse of tens of megabytes. Past this cap the state is
/// honestly `unknown`; the next tick tails forward and picks up the next boundary
/// as it is written.
const MAX_BACKSCAN: u64 = 64 * 1024 * 1024;

/// How much to pull back per step while scanning for a boundary.
const BACKSCAN_CHUNK: u64 = 4 * 1024 * 1024;

/// How much of line 1 to read before giving up on it.
///
/// `session_meta` carries the model's whole system prompt; the largest on this
/// disk is 49 KB. The cap exists so a corrupt file with no newline in it cannot
/// pull an unbounded read into the poll.
const MAX_META_BYTES: u64 = 1024 * 1024;

/// Line 1 of a rollout: who this session is and what kind of thread it belongs to.
///
/// Taken from the rollout rather than from `~/.codex/state_5.sqlite`, which
/// records the same fields. The file is already being opened for state and
/// activity, so line 1 is free; the database is WAL-mode and held open by the
/// live process, would add a SQLite dependency, and has plainly churned — 36
/// columns, most added by `ALTER`, and the filename is already at version 5.
#[derive(Debug, Clone, PartialEq)]
pub struct Meta {
    pub id: String,
    pub cwd: String,
    pub source: Option<String>,
    pub thread_source: Option<String>,
    /// When the session itself began, unix ms, from line 1's own `timestamp`.
    ///
    /// Read for the same reason Claude's `procStart` is: a turn boundary cannot
    /// have been written before the session that wrote it, so this is the bound a
    /// nonsense timestamp is caught by — a fact rather than a threshold somebody
    /// chose. `None` where line 1 carries nothing readable, which applies no bound
    /// rather than refusing the session.
    pub began: Option<u64>,
}

impl Meta {
    /// Whether this is a CLI session a person is driving.
    ///
    /// The two other shapes sharing these directories are excluded by the same
    /// test: a subagent thread carries `thread_source: "subagent"`, and Codex
    /// Desktop carries `source: "vscode"`. `source` is not always a string —
    /// a subagent's is an object, `{"subagent": {"other": "guardian"}}` — which
    /// this reads as absent rather than as an error, and which excludes it
    /// either way.
    pub fn is_cli_user_session(&self) -> bool {
        self.source.as_deref() == Some("cli") && self.thread_source.as_deref() == Some("user")
    }
}

/// Whether a path is a rollout transcript, by the agent's own naming.
pub fn is_rollout_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("rollout-") && n.ends_with(".jsonl"))
}

/// Read line 1 and nothing else.
///
/// A rollout is created lazily at a session's first turn, so a missing or
/// unreadable file is the ordinary early case, not an error.
pub fn read_meta(path: &Path) -> Option<Meta> {
    let file = std::fs::File::open(path).ok()?;
    let mut line = Vec::new();
    BufReader::new(file.take(MAX_META_BYTES))
        .read_until(b'\n', &mut line)
        .ok()?;
    let v: Value = serde_json::from_slice(&line).ok()?;
    if v.get("type").and_then(Value::as_str) != Some("session_meta") {
        return None;
    }
    let p = v.get("payload")?;
    let text = |k: &str| p.get(k).and_then(Value::as_str).map(str::to_string);
    Some(Meta {
        // `id` is this thread's own; `session_id` is its parent's on a subagent,
        // so the two must never be treated as interchangeable.
        id: text("id")?,
        cwd: text("cwd")?,
        source: text("source"),
        thread_source: text("thread_source"),
        // The envelope's `timestamp`, not the payload's: the payload's is when the
        // session was created and the envelope's is when the line was written, and
        // the envelope's is the one every other line is dated by, so the bound and
        // the values it bounds come from one clock.
        began: v
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(crate::session::parse_iso8601_ms),
    })
}

#[derive(Default)]
struct Progress {
    offset: u64,
    state: State,
    activity: Option<String>,
    state_since: Option<u64>,
}

/// What one tick learned about a rollout.
///
/// A named struct rather than a tuple because story 006 added a third value and
/// three positional fields at a call site is where a reader starts guessing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reading {
    pub state: State,
    pub activity: Option<String>,
    /// When the boundary that decided the state was written, unix ms.
    ///
    /// `None` when no boundary was found — the state is `Unknown` then, and there
    /// is no moment to date. Codex timestamps every line it writes, so a boundary
    /// found by the backscan carries this exactly as one found by the forward tail.
    pub state_since: Option<u64>,
}

/// Reads rollouts forward from where it last stopped.
///
/// The first sight of a file reads backward from EOF over a bounded window; every
/// tick after that reads only what was appended. Nothing is ever re-read.
#[derive(Default)]
pub struct Tailer {
    seen: HashMap<PathBuf, Progress>,
}

impl Tailer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drop a boundary time that predates the session that wrote it.
    ///
    /// A rollout is self-consistent in practice, so this catches nothing today. It
    /// exists because the row's age is now agent-supplied data rather than the
    /// pet's own clock, and an age counted from a moment the session did not exist
    /// for is worse than one counted from first-seen — the same rule, and the same
    /// reasoning, as Claude's `procStart` bound.
    pub fn dated_after(reading: Reading, began: Option<u64>) -> Reading {
        match (reading.state_since, began) {
            (Some(since), Some(began)) if since < began => Reading {
                state_since: None,
                ..reading
            },
            _ => reading,
        }
    }

    /// This session's turn state and current activity.
    ///
    /// Turn state is the last boundary event seen, not a count: one rollout on
    /// this disk carries 9 `task_started` to 8 `task_complete`, so any pairing
    /// rule is already wrong on real data. A rollout holding no boundary within
    /// the window read stays `Unknown` — never inferred as idle or working.
    pub fn read(&mut self, path: &Path) -> Reading {
        let Ok(mut file) = std::fs::File::open(path) else {
            return self.remembered(path);
        };
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);

        let known = self.seen.get(path);
        // No offset yet, or one past the end because the file was replaced or
        // truncated: either way the remembered position means nothing.
        let fresh = known.is_none_or(|p| p.offset > len);
        let mut start = match known {
            Some(p) if !fresh => p.offset,
            _ => len.saturating_sub(FIRST_READ_WINDOW),
        };
        if fresh {
            self.seen.insert(path.to_path_buf(), Progress::default());
        }
        if start >= len {
            self.seen
                .entry(path.to_path_buf())
                .and_modify(|p| p.offset = len);
            return self.remembered(path);
        }

        if file.seek(SeekFrom::Start(start)).is_err() {
            return self.remembered(path);
        }
        let mut buf = Vec::new();
        if file.read_to_end(&mut buf).is_err() {
            return self.remembered(path);
        }

        // Landing mid-file lands mid-line; that fragment is not parseable JSON and
        // is dropped rather than guessed at.
        if fresh && start > 0 {
            match buf.iter().position(|b| *b == b'\n') {
                Some(i) => {
                    start += i as u64 + 1;
                    buf.drain(..=i);
                }
                // A window holding no line break at all yields nothing this tick.
                None => {
                    self.seen
                        .entry(path.to_path_buf())
                        .and_modify(|p| p.offset = len);
                    return self.remembered(path);
                }
            }
        }

        let text = String::from_utf8_lossy(&buf);
        // The agent may be mid-write; keep any trailing partial line for next tick.
        let complete_upto = match text.rfind('\n') {
            Some(i) => i + 1,
            None => 0,
        };

        let progress = self.seen.entry(path.to_path_buf()).or_default();
        for line in text[..complete_upto].lines() {
            apply(line, progress);
        }
        progress.offset = start + complete_upto as u64;

        // The window held no boundary and there is file behind it. For a settled
        // session that cannot happen — `task_complete` sits within a few KB of
        // EOF — so this is a session mid-turn, whose `task_started` is as far
        // back as the turn is long. The activity line already read stands: every
        // item in the window came after whatever boundary this finds.
        if fresh && progress.state == State::Unknown && start > 0 {
            if let Some((state, at)) = last_boundary_before(&mut file, start) {
                progress.state = state;
                progress.state_since = at;
            }
        }
        self.remembered(path)
    }

    fn remembered(&self, path: &Path) -> Reading {
        match self.seen.get(path) {
            Some(p) => Reading {
                state: p.state,
                activity: p.activity.clone(),
                state_since: p.state_since,
            },
            None => Reading::default(),
        }
    }

    pub fn retain_only(&mut self, keep: &[PathBuf]) {
        self.seen.retain(|k, _| keep.contains(k));
    }
}

/// The state named by the last turn boundary before `end`, if one is in reach.
///
/// Reads backward in chunks and takes the newest boundary found, which is the one
/// that decides the state. `None` means no boundary lies within [`MAX_BACKSCAN`] —
/// reported as `unknown` rather than guessed at.
fn last_boundary_before(file: &mut std::fs::File, end: u64) -> Option<(State, Option<u64>)> {
    let floor = end.saturating_sub(MAX_BACKSCAN);
    let mut hi = end;
    // The head of the chunk just examined: a line straddling the cut, whose start
    // lies in the chunk below. Carried down so it is read whole exactly once.
    let mut carry: Vec<u8> = Vec::new();

    while hi > floor {
        let lo = hi.saturating_sub(BACKSCAN_CHUNK).max(floor);
        file.seek(SeekFrom::Start(lo)).ok()?;
        let mut buf = vec![0u8; (hi - lo) as usize];
        file.read_exact(&mut buf).ok()?;
        buf.extend_from_slice(&carry);

        // Everything before the first newline began in the chunk below this one.
        let head_end = match buf.iter().position(|b| *b == b'\n') {
            Some(i) if lo > floor => i + 1,
            _ => 0,
        };
        carry = buf[..head_end].to_vec();

        let text = String::from_utf8_lossy(&buf[head_end..]);
        if let Some(found) = text.lines().rev().find_map(boundary_state) {
            return Some(found);
        }
        hi = lo;
    }
    None
}

/// The state one line names, if it is a turn boundary at all.
///
/// The substring test is what keeps the backscan cheap: `item_completed` lines
/// reach tens of KB each and make up nearly every byte of a rollout, and this
/// rejects them without handing any of it to a JSON parser.
fn boundary_state(line: &str) -> Option<(State, Option<u64>)> {
    if !line.contains("task_started") && !line.contains("task_complete") {
        return None;
    }
    let v: Value = serde_json::from_str(line.trim()).ok()?;
    if v.get("type").and_then(Value::as_str) != Some("event_msg") {
        return None;
    }
    let state = match v.get("payload")?.get("type").and_then(Value::as_str) {
        Some("task_started") => State::Working,
        Some("task_complete") => State::Idle,
        _ => return None,
    };
    Some((state, boundary_time(&v)))
}

/// The line's own `timestamp`, which every rollout line carries.
///
/// `task_started` also carries a `started_at` inside its payload; it is
/// deliberately unused. One field read the same way for both boundaries is one
/// rule, and two would be two rules that have to be kept agreeing.
fn boundary_time(v: &Value) -> Option<u64> {
    v.get("timestamp")
        .and_then(Value::as_str)
        .and_then(crate::session::parse_iso8601_ms)
}

/// Fold one rollout line into what is known about the session.
fn apply(line: &str, progress: &mut Progress) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    let Ok(v) = serde_json::from_str::<Value>(line) else {
        return;
    };
    if v.get("type").and_then(Value::as_str) != Some("event_msg") {
        return;
    }
    let Some(payload) = v.get("payload") else {
        return;
    };
    match payload.get("type").and_then(Value::as_str) {
        Some("task_started") => {
            progress.state = State::Working;
            progress.state_since = boundary_time(&v);
            // A new turn has not done anything yet. Carrying the previous turn's
            // last action forward would put a finished action under a live one.
            progress.activity = None;
        }
        Some("task_complete") => {
            progress.state = State::Idle;
            progress.state_since = boundary_time(&v);
        }
        Some("item_completed") => {
            if let Some(item) = payload.get("item") {
                if let Some(a) = activity_of(item) {
                    progress.activity = Some(a);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn dir() -> PathBuf {
        let d = std::env::temp_dir().join("agentpet-rollout-tests");
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write(name: &str, body: &str) -> PathBuf {
        let p = dir().join(name);
        std::fs::write(&p, body).unwrap();
        p
    }

    /// Whole lines, terminated the way a real rollout terminates them.
    ///
    /// The final newline matters: without one the tailer treats the last line as
    /// a write still in flight and holds it back, which is what it should do.
    fn write_lines(name: &str, lines: &[String]) -> PathBuf {
        write(name, &(lines.join("\n") + "\n"))
    }

    /// The committed rollouts, reduced from this machine's own `~/.codex/sessions`.
    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("codex")
            .join(name)
    }

    fn meta_line(id: &str, cwd: &str, source: &str, thread_source: &str) -> String {
        format!(
            r#"{{"type":"session_meta","payload":{{"id":"{id}","cwd":"{cwd}","source":{source},"thread_source":"{thread_source}"}}}}"#
        )
    }

    /// `2026-08-25T00:41:35.372Z`, the timestamp both boundary helpers carry.
    const BOUNDARY_MS: u64 = 1_787_618_495_372;

    fn started() -> String {
        // Every line the agent writes carries a `timestamp`, and `task_started`
        // additionally carries `started_at` inside its payload — as unix *seconds*,
        // an integer, which is how the committed fixture writes it
        // (`fixtures/codex/cli-user.jsonl` line 2: `"started_at":1787618495`).
        // A deliberately wrong value in the real shape, so a change that read the
        // payload field instead of the line's own `timestamp` fails loudly rather
        // than passing by luck or by type error.
        r#"{"timestamp":"2026-08-25T00:41:35.372Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1","started_at":1577836800}}"#.into()
    }

    fn complete() -> String {
        r#"{"timestamp":"2026-08-25T00:41:35.372Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"t1"}}"#.into()
    }

    fn reasoning(text: &str) -> String {
        format!(
            r#"{{"type":"event_msg","payload":{{"type":"item_completed","item":{{"type":"Reasoning","summary_text":["**{text}**"]}}}}}}"#
        )
    }

    // MARK: - Where the age counts from (story 006)

    #[test]
    fn the_session_start_is_read_from_line_one() {
        // The committed fixture, whose first line is dated 2026-08-25T00:41:35.372Z.
        let m = read_meta(&fixture_path("cli-user.jsonl")).unwrap();
        assert_eq!(m.began, Some(BOUNDARY_MS));
        // A line 1 with no envelope timestamp applies no bound rather than
        // refusing the session.
        let bare = read_meta(&write(
            "meta-no-ts.jsonl",
            &(meta_line("i", "/c", "\"cli\"", "user") + "\n"),
        ))
        .unwrap();
        assert_eq!(bare.began, None);
    }

    #[test]
    fn a_boundary_dated_before_its_own_session_is_refused_not_shown() {
        let reading = Reading {
            state: State::Working,
            activity: None,
            state_since: Some(1_000),
        };
        // Older than the session that wrote it: dropped, so the row falls back to
        // first-seen rather than counting from a moment the session did not exist.
        assert_eq!(Tailer::dated_after(reading.clone(), Some(2_000)).state_since, None);
        // The state itself is never touched by the bound.
        assert_eq!(Tailer::dated_after(reading.clone(), Some(2_000)).state, State::Working);
        // At or after the session's start, and with no bound available, it stands.
        assert_eq!(Tailer::dated_after(reading.clone(), Some(1_000)).state_since, Some(1_000));
        assert_eq!(Tailer::dated_after(reading.clone(), Some(500)).state_since, Some(1_000));
        assert_eq!(Tailer::dated_after(reading, None).state_since, Some(1_000));
    }

    #[test]
    fn a_boundary_dates_the_state_from_its_own_timestamp() {
        // Both boundaries, read the same way. `task_started`'s payload also holds
        // a `started_at`, deliberately unused: one field for both boundaries is
        // one rule rather than two that must be kept agreeing.
        for (name, boundary, state) in [
            ("since-started.jsonl", started(), State::Working),
            ("since-complete.jsonl", complete(), State::Idle),
        ] {
            let p = write(name, &format!("{}\n{boundary}\n", meta_line("i", "/c", "\"cli\"", "user")));
            let r = Tailer::new().read(&p);
            assert_eq!(r.state, state);
            assert_eq!(r.state_since, Some(BOUNDARY_MS), "for {name}");
        }
    }

    #[test]
    fn a_boundary_found_by_the_backscan_is_dated_too() {
        // The far-boundary path: the turn began outside the first-read window, so
        // the state comes from `last_boundary_before` rather than the forward
        // tail. The age must be just as real there — that path is precisely the
        // long-running turn whose age is worth knowing.
        let p = dir().join("since-backscan.jsonl");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(f, "{}", meta_line("i", "/c", "\"cli\"", "user")).unwrap();
        writeln!(f, "{}", started()).unwrap();
        let filler = reasoning("Padding written since the turn began");
        let mut written = 0u64;
        while written < FIRST_READ_WINDOW * 3 {
            writeln!(f, "{filler}").unwrap();
            written += filler.len() as u64 + 1;
        }
        f.sync_all().unwrap();

        let r = Tailer::new().read(&p);
        assert_eq!(r.state, State::Working);
        assert_eq!(r.state_since, Some(BOUNDARY_MS));
    }

    #[test]
    fn a_finished_turn_found_by_the_backscan_is_dated_too() {
        // The same far-boundary path as above, for `task_complete`. The two share
        // one code path, but a `task_started`-only test would not catch a change
        // that dated only the boundary it happened to be written against.
        let p = dir().join("since-backscan-idle.jsonl");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(f, "{}", meta_line("i", "/c", "\"cli\"", "user")).unwrap();
        writeln!(f, "{}", started()).unwrap();
        writeln!(f, "{}", complete()).unwrap();
        let filler = reasoning("Padding written after the turn ended");
        let mut written = 0u64;
        while written < FIRST_READ_WINDOW * 3 {
            writeln!(f, "{filler}").unwrap();
            written += filler.len() as u64 + 1;
        }
        f.sync_all().unwrap();

        let r = Tailer::new().read(&p);
        assert_eq!(r.state, State::Idle);
        assert_eq!(r.state_since, Some(BOUNDARY_MS));
    }

    #[test]
    fn a_rollout_with_no_boundary_has_no_time_to_date() {
        // `Unknown` and `None` together: there is no moment to count from, and the
        // adapter falls back to first-seen rather than inventing one.
        let p = write(
            "since-none.jsonl",
            &format!("{}\n{}\n", meta_line("i", "/c", "\"cli\"", "user"), reasoning("Thinking")),
        );
        let r = Tailer::new().read(&p);
        assert_eq!(r.state, State::Unknown);
        assert_eq!(r.state_since, None);
    }

    #[test]
    fn a_boundary_with_an_unreadable_timestamp_still_decides_the_state() {
        let line = r#"{"timestamp":"whenever","type":"event_msg","payload":{"type":"task_started"}}"#;
        let p = write(
            "since-bad-ts.jsonl",
            &format!("{}\n{line}\n", meta_line("i", "/c", "\"cli\"", "user")),
        );
        let r = Tailer::new().read(&p);
        assert_eq!(r.state, State::Working, "the state was lost with the time");
        assert_eq!(r.state_since, None);
    }

    #[test]
    fn a_rollout_is_recognised_by_the_agents_own_naming() {
        assert!(is_rollout_path(Path::new("/a/rollout-2026-08-24T20-41-17-abc.jsonl")));
        assert!(!is_rollout_path(Path::new("/a/state_5.sqlite")));
        assert!(!is_rollout_path(Path::new("/a/rollout-abc.json")));
        assert!(!is_rollout_path(Path::new("/a/notes.jsonl")));
    }

    #[test]
    fn a_cli_user_session_is_told_apart_from_the_shapes_beside_it() {
        let cli = read_meta(&write(
            "meta-cli.jsonl",
            &(meta_line("id-1", "/Users/x/alpha", "\"cli\"", "user") + "\n"),
        ))
        .unwrap();
        assert!(cli.is_cli_user_session());
        assert_eq!(cli.id, "id-1");
        assert_eq!(cli.cwd, "/Users/x/alpha");

        let desktop = read_meta(&write(
            "meta-vscode.jsonl",
            &(meta_line("id-2", "/Users/x/alpha", "\"vscode\"", "user") + "\n"),
        ))
        .unwrap();
        assert!(!desktop.is_cli_user_session());

        // A subagent's `source` is an object, not a string.
        let sub = read_meta(&write(
            "meta-subagent.jsonl",
            &(meta_line("id-3", "/Users/x/alpha", r#"{"subagent":{"other":"guardian"}}"#, "subagent") + "\n"),
        ))
        .unwrap();
        assert_eq!(sub.source, None, "a non-string source must read as absent");
        assert!(!sub.is_cli_user_session());
    }

    #[test]
    fn a_file_that_is_not_a_rollout_yields_no_meta() {
        assert_eq!(read_meta(&write("meta-other.jsonl", "{\"type\":\"world_state\"}\n")), None);
        assert_eq!(read_meta(&write("meta-garbage.jsonl", "not json at all\n")), None);
        assert_eq!(read_meta(Path::new("/nonexistent/rollout-x.jsonl")), None);
    }

    #[test]
    fn the_last_boundary_decides_the_state() {
        let p = write_lines(
            "state-working.jsonl",
            &[meta_line("i", "/c", "\"cli\"", "user"), started(), complete(), started()],
        );
        let mut t = Tailer::new();
        assert_eq!(t.read(&p).state, State::Working);

        let p = write_lines(
            "state-idle.jsonl",
            &[meta_line("i", "/c", "\"cli\"", "user"), started(), complete()],
        );
        assert_eq!(Tailer::new().read(&p).state, State::Idle);
    }

    #[test]
    fn more_starts_than_completes_still_reads_as_working() {
        // Observed on disk: one rollout carries 9 starts to 8 completes, so any
        // rule that pairs or counts them is already wrong on real data.
        let mut lines = vec![meta_line("i", "/c", "\"cli\"", "user")];
        for _ in 0..8 {
            lines.push(started());
            lines.push(complete());
        }
        lines.push(started());
        let p = write_lines("state-mismatch.jsonl", &lines);
        assert_eq!(Tailer::new().read(&p).state, State::Working);
    }

    #[test]
    fn a_rollout_with_no_boundary_is_unknown_not_idle() {
        let p = write_lines(
            "state-none.jsonl",
            &[meta_line("i", "/c", "\"cli\"", "user"), reasoning("Thinking")],
        );
        assert_eq!(Tailer::new().read(&p).state, State::Unknown);
    }

    #[test]
    fn an_unreadable_rollout_is_unknown_not_idle() {
        // Spelled out rather than compared against `Reading::default()`: both
        // sides would then derive from the same `#[default]`, and the test would
        // hold whatever that default became — including `Idle`, which is the one
        // answer it exists to forbid.
        let r = Tailer::new().read(Path::new("/nonexistent/rollout-x.jsonl"));
        assert_eq!(r.state, State::Unknown);
        assert_eq!(r.activity, None);
        assert_eq!(r.state_since, None);
    }

    #[test]
    fn the_newest_item_is_the_activity() {
        let p = write_lines(
            "activity-newest.jsonl",
            &[
                meta_line("i", "/c", "\"cli\"", "user"),
                started(),
                reasoning("Reading the config"),
                reasoning("Testing the connection"),
            ],
        );
        let Reading { state, activity, .. } = Tailer::new().read(&p);
        assert_eq!(state, State::Working);
        assert_eq!(activity.as_deref(), Some("Testing the connection"));
    }

    #[test]
    fn a_new_turn_does_not_inherit_the_previous_turns_activity() {
        let p = write_lines(
            "activity-fresh-turn.jsonl",
            &[
                meta_line("i", "/c", "\"cli\"", "user"),
                started(),
                reasoning("Reading the config"),
                complete(),
                started(),
            ],
        );
        let Reading { state, activity, .. } = Tailer::new().read(&p);
        assert_eq!(state, State::Working);
        assert_eq!(activity, None, "a live turn showed a finished turn's action");
    }

    #[test]
    fn only_what_is_new_is_read_and_the_previous_answer_stands() {
        let p = write_lines(
            "tail-append.jsonl",
            &[meta_line("i", "/c", "\"cli\"", "user"), started(), reasoning("One")],
        );
        let mut t = Tailer::new();
        assert_eq!(t.read(&p).activity.as_deref(), Some("One"));
        let after_first = t.seen.get(&p).unwrap().offset;

        // Nothing new: the offset does not move and the answer is unchanged.
        assert_eq!(t.read(&p).activity.as_deref(), Some("One"));
        assert_eq!(t.seen.get(&p).unwrap().offset, after_first);

        let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
        writeln!(f, "{}", reasoning("Two")).unwrap();
        assert_eq!(t.read(&p).activity.as_deref(), Some("Two"));
        assert!(t.seen.get(&p).unwrap().offset > after_first);
    }

    #[test]
    fn a_half_written_line_is_not_consumed_until_complete() {
        let head = [meta_line("i", "/c", "\"cli\"", "user"), started(), reasoning("Complete line")].join("\n");
        let p = write("tail-partial.jsonl", &format!("{head}\n{{\"type\":\"event_ms"));
        let mut t = Tailer::new();
        assert_eq!(t.read(&p).activity.as_deref(), Some("Complete line"));

        std::fs::write(&p, format!("{head}\n{}\n", reasoning("Finished line"))).unwrap();
        assert_eq!(t.read(&p).activity.as_deref(), Some("Finished line"));
    }

    #[test]
    fn a_replaced_shorter_file_is_read_again_from_scratch() {
        let long = [meta_line("i", "/c", "\"cli\"", "user"), started(), reasoning("Old and long")].join("\n") + "\n";
        let p = write("tail-truncated.jsonl", &long);
        let mut t = Tailer::new();
        assert_eq!(t.read(&p).activity.as_deref(), Some("Old and long"));

        std::fs::write(&p, format!("{}\n{}\n", started(), reasoning("New"))).unwrap();
        assert_eq!(t.read(&p).activity.as_deref(), Some("New"));
    }

    /// The bounded backward read, on a file far past the window.
    #[test]
    fn an_oversized_rollout_is_read_from_its_end_not_its_start() {
        let p = dir().join("tail-oversized.jsonl");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(f, "{}", meta_line("i", "/c", "\"cli\"", "user")).unwrap();
        // A stale boundary far outside the window, which must not be what is read.
        writeln!(f, "{}", complete()).unwrap();
        let filler = reasoning("Padding that is never the newest activity");
        let mut written = 0u64;
        while written < FIRST_READ_WINDOW * 2 {
            writeln!(f, "{filler}").unwrap();
            written += filler.len() as u64 + 1;
        }
        writeln!(f, "{}", started()).unwrap();
        writeln!(f, "{}", reasoning("The newest thing")).unwrap();
        f.sync_all().unwrap();
        let len = std::fs::metadata(&p).unwrap().len();
        assert!(len > FIRST_READ_WINDOW, "fixture is not oversized: {len}");

        let mut t = Tailer::new();
        let Reading { state, activity, .. } = t.read(&p);
        assert_eq!(state, State::Working);
        assert_eq!(activity.as_deref(), Some("The newest thing"));
        assert_eq!(t.seen.get(&p).unwrap().offset, len, "the read did not reach EOF");
    }

    /// A working session whose turn began far outside the window.
    ///
    /// This is the ordinary case, not an exotic one: a live session's own rollout
    /// reached 987 KB with its single `task_started` at the top, because one
    /// `item_completed` line runs to tens of KB. Reporting `unknown` for the whole
    /// turn would blank the state of exactly the session worth watching.
    #[test]
    fn a_working_session_is_found_even_when_its_turn_began_outside_the_window() {
        let p = dir().join("tail-far-boundary.jsonl");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(f, "{}", meta_line("i", "/c", "\"cli\"", "user")).unwrap();
        writeln!(f, "{}", started()).unwrap();
        let filler = reasoning("Padding written since the turn began");
        let mut written = 0u64;
        while written < FIRST_READ_WINDOW * 3 {
            writeln!(f, "{filler}").unwrap();
            written += filler.len() as u64 + 1;
        }
        writeln!(f, "{}", reasoning("The newest thing")).unwrap();
        f.sync_all().unwrap();

        let Reading { state, activity, .. } = Tailer::new().read(&p);
        assert_eq!(state, State::Working, "a live turn read as unknown");
        assert_eq!(activity.as_deref(), Some("The newest thing"));
    }

    /// The same, for a turn that finished long before the window opened.
    #[test]
    fn an_idle_session_is_found_the_same_way() {
        let p = dir().join("tail-far-boundary-idle.jsonl");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(f, "{}", meta_line("i", "/c", "\"cli\"", "user")).unwrap();
        writeln!(f, "{}", started()).unwrap();
        writeln!(f, "{}", complete()).unwrap();
        let filler = reasoning("Padding after the turn ended");
        let mut written = 0u64;
        while written < FIRST_READ_WINDOW * 3 {
            writeln!(f, "{filler}").unwrap();
            written += filler.len() as u64 + 1;
        }
        f.sync_all().unwrap();

        assert_eq!(Tailer::new().read(&p).state, State::Idle);
    }

    /// A boundary split across two backscan chunks is still read whole.
    #[test]
    fn a_boundary_straddling_a_backscan_chunk_is_not_lost() {
        let p = dir().join("tail-straddle.jsonl");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(f, "{}", meta_line("i", "/c", "\"cli\"", "user")).unwrap();
        // Pad so the boundary lands near a chunk edge, then far more than one
        // chunk of filler after it, forcing at least two backward steps.
        let filler = reasoning("Padding before the boundary");
        let mut written = 0u64;
        while written < BACKSCAN_CHUNK + 1024 {
            writeln!(f, "{filler}").unwrap();
            written += filler.len() as u64 + 1;
        }
        writeln!(f, "{}", started()).unwrap();
        let mut written = 0u64;
        while written < BACKSCAN_CHUNK + FIRST_READ_WINDOW * 2 {
            writeln!(f, "{filler}").unwrap();
            written += filler.len() as u64 + 1;
        }
        f.sync_all().unwrap();

        assert_eq!(Tailer::new().read(&p).state, State::Working);
    }

    /// A rollout with no boundary anywhere is still unknown, not guessed.
    #[test]
    fn an_oversized_rollout_with_no_boundary_at_all_is_unknown() {
        let p = dir().join("tail-oversized-noboundary.jsonl");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(f, "{}", meta_line("i", "/c", "\"cli\"", "user")).unwrap();
        let filler = reasoning("Padding in a rollout that never took a turn");
        let mut written = 0u64;
        while written < FIRST_READ_WINDOW * 3 {
            writeln!(f, "{filler}").unwrap();
            written += filler.len() as u64 + 1;
        }
        f.sync_all().unwrap();

        assert_eq!(Tailer::new().read(&p).state, State::Unknown);
    }

    #[test]
    fn forgetting_a_rollout_drops_what_was_remembered_about_it() {
        let p = write_lines(
            "tail-retain.jsonl",
            &[meta_line("i", "/c", "\"cli\"", "user"), started(), reasoning("One")],
        );
        let mut t = Tailer::new();
        t.read(&p);
        assert!(t.seen.contains_key(&p));
        t.retain_only(&[]);
        assert!(t.seen.is_empty());
    }
}
