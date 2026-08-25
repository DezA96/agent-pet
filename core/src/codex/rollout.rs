use crate::codex::activity::activity_of;
use crate::session::State;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// How far back from the end of an unseen rollout to start reading.
///
/// Story 001 reads a Claude transcript from offset 0, which is safe at its ~102 KB.
/// A Codex rollout on this disk is 74 MB and a single-turn session reached 1.3 MB,
/// so reading one whole would stall a poll for seconds at pet startup. Starting at
/// bare EOF was rejected as worse: a session already mid-turn would read `unknown`
/// until its next event, breaking the release's within-a-few-seconds spot check.
const FIRST_READ_WINDOW: u64 = 256 * 1024;

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
    })
}

#[derive(Default)]
struct Progress {
    offset: u64,
    state: State,
    activity: Option<String>,
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

    /// This session's turn state and current activity.
    ///
    /// Turn state is the last boundary event seen, not a count: one rollout on
    /// this disk carries 9 `task_started` to 8 `task_complete`, so any pairing
    /// rule is already wrong on real data. A rollout holding no boundary within
    /// the window read stays `Unknown` — never inferred as idle or working.
    pub fn read(&mut self, path: &Path) -> (State, Option<String>) {
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
        self.remembered(path)
    }

    fn remembered(&self, path: &Path) -> (State, Option<String>) {
        match self.seen.get(path) {
            Some(p) => (p.state, p.activity.clone()),
            None => (State::Unknown, None),
        }
    }

    pub fn retain_only(&mut self, keep: &[PathBuf]) {
        self.seen.retain(|k, _| keep.contains(k));
    }
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
            // A new turn has not done anything yet. Carrying the previous turn's
            // last action forward would put a finished action under a live one.
            progress.activity = None;
        }
        Some("task_complete") => progress.state = State::Idle,
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

    fn meta_line(id: &str, cwd: &str, source: &str, thread_source: &str) -> String {
        format!(
            r#"{{"type":"session_meta","payload":{{"id":"{id}","cwd":"{cwd}","source":{source},"thread_source":"{thread_source}"}}}}"#
        )
    }

    fn started() -> String {
        r#"{"type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#.into()
    }

    fn complete() -> String {
        r#"{"type":"event_msg","payload":{"type":"task_complete","turn_id":"t1"}}"#.into()
    }

    fn reasoning(text: &str) -> String {
        format!(
            r#"{{"type":"event_msg","payload":{{"type":"item_completed","item":{{"type":"Reasoning","summary_text":["**{text}**"]}}}}}}"#
        )
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
        assert_eq!(t.read(&p).0, State::Working);

        let p = write_lines(
            "state-idle.jsonl",
            &[meta_line("i", "/c", "\"cli\"", "user"), started(), complete()],
        );
        assert_eq!(Tailer::new().read(&p).0, State::Idle);
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
        assert_eq!(Tailer::new().read(&p).0, State::Working);
    }

    #[test]
    fn a_rollout_with_no_boundary_is_unknown_not_idle() {
        let p = write_lines(
            "state-none.jsonl",
            &[meta_line("i", "/c", "\"cli\"", "user"), reasoning("Thinking")],
        );
        assert_eq!(Tailer::new().read(&p).0, State::Unknown);
    }

    #[test]
    fn an_unreadable_rollout_is_unknown_not_idle() {
        assert_eq!(
            Tailer::new().read(Path::new("/nonexistent/rollout-x.jsonl")),
            (State::Unknown, None)
        );
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
        let (state, activity) = Tailer::new().read(&p);
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
        let (state, activity) = Tailer::new().read(&p);
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
        assert_eq!(t.read(&p).1.as_deref(), Some("One"));
        let after_first = t.seen.get(&p).unwrap().offset;

        // Nothing new: the offset does not move and the answer is unchanged.
        assert_eq!(t.read(&p).1.as_deref(), Some("One"));
        assert_eq!(t.seen.get(&p).unwrap().offset, after_first);

        let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
        writeln!(f, "{}", reasoning("Two")).unwrap();
        assert_eq!(t.read(&p).1.as_deref(), Some("Two"));
        assert!(t.seen.get(&p).unwrap().offset > after_first);
    }

    #[test]
    fn a_half_written_line_is_not_consumed_until_complete() {
        let head = [meta_line("i", "/c", "\"cli\"", "user"), started(), reasoning("Complete line")].join("\n");
        let p = write("tail-partial.jsonl", &format!("{head}\n{{\"type\":\"event_ms"));
        let mut t = Tailer::new();
        assert_eq!(t.read(&p).1.as_deref(), Some("Complete line"));

        std::fs::write(&p, format!("{head}\n{}\n", reasoning("Finished line"))).unwrap();
        assert_eq!(t.read(&p).1.as_deref(), Some("Finished line"));
    }

    #[test]
    fn a_replaced_shorter_file_is_read_again_from_scratch() {
        let long = [meta_line("i", "/c", "\"cli\"", "user"), started(), reasoning("Old and long")].join("\n") + "\n";
        let p = write("tail-truncated.jsonl", &long);
        let mut t = Tailer::new();
        assert_eq!(t.read(&p).1.as_deref(), Some("Old and long"));

        std::fs::write(&p, format!("{}\n{}\n", started(), reasoning("New"))).unwrap();
        assert_eq!(t.read(&p).1.as_deref(), Some("New"));
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
        let (state, activity) = t.read(&p);
        assert_eq!(state, State::Working);
        assert_eq!(activity.as_deref(), Some("The newest thing"));
        assert_eq!(t.seen.get(&p).unwrap().offset, len, "the read did not reach EOF");
    }

    /// A window that opens after the last boundary reports unknown rather than guessing.
    #[test]
    fn an_oversized_rollout_whose_boundary_is_out_of_window_is_unknown() {
        let p = dir().join("tail-oversized-noboundary.jsonl");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(f, "{}", meta_line("i", "/c", "\"cli\"", "user")).unwrap();
        writeln!(f, "{}", started()).unwrap();
        let filler = reasoning("Padding after the only boundary in the file");
        let mut written = 0u64;
        while written < FIRST_READ_WINDOW * 2 {
            writeln!(f, "{filler}").unwrap();
            written += filler.len() as u64 + 1;
        }
        f.sync_all().unwrap();

        assert_eq!(Tailer::new().read(&p).0, State::Unknown);
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
