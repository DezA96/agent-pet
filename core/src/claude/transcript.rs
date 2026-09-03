use crate::session::truncate_activity as truncate;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Reads transcripts forward from where it last stopped.
///
/// Transcripts are never re-read whole: this project's own is already 102 KB and
/// growing, and the largest Codex rollout on disk is 74 MB. Each tick seeks to the
/// remembered offset and reads only what is new.
#[derive(Default)]
pub struct Tailer {
    offsets: HashMap<PathBuf, u64>,
    last_activity: HashMap<PathBuf, String>,
    /// Whether the newest substantive entry in each transcript was an API error.
    ///
    /// Held as "the newest one, whatever it was" rather than "an error was seen",
    /// because a transcript is append-only: an error the agent retried through
    /// stays in the file forever, and a state that latched on first sight would
    /// mark a healthy session dead for the rest of its life.
    last_error: HashMap<PathBuf, Option<ApiError>>,
}

/// An API error exactly as the agent recorded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    /// The HTTP status the agent recorded, where it recorded one.
    pub status: Option<u64>,
    /// When the entry the session stopped on was written, unix ms.
    ///
    /// Not `statusUpdatedAt`: the two name different facts. Errored outranks every
    /// registry status, so a session that errors and then has a dialog opened on it
    /// still reads errored while `statusUpdatedAt` has moved on to the dialog. The
    /// age has to date the error, which is what this carries.
    pub at: Option<u64>,
}

impl ApiError {
    /// The line shown under the project name.
    ///
    /// The status code is passed through as fact and never interpreted. Observed
    /// live: a `429` that was a hard session limit ("resets 11:30pm"), not a rate
    /// limit to wait out — so the pet reports the number and lets the user judge.
    /// The agent's own error prose is deliberately not used: it is operator
    /// diagnostics aimed at a log, and this surface is not one.
    pub fn line(&self) -> String {
        match self.status {
            Some(code) => format!("Error: {code}"),
            None => "Errored".to_string(),
        }
    }
}

impl Tailer {
    pub fn new() -> Self {
        Self::default()
    }

    /// The newest activity description in this transcript, or `None` when the
    /// session has not used a tool yet.
    ///
    /// A session that just started has no tool activity, and its row must still
    /// render — so "nothing yet" is an ordinary answer, not a failure.
    pub fn activity(&mut self, path: &Path) -> Option<String> {
        self.scan(path);
        self.last_activity.get(path).cloned()
    }

    /// The API error the transcript ended on, or `None` if it did not.
    ///
    /// Separate from `activity` because the two are wanted in different states: a
    /// row asks for activity only while working, but must ask about errors
    /// whatever the registry says.
    pub fn error(&mut self, path: &Path) -> Option<ApiError> {
        self.scan(path);
        self.last_error.get(path).cloned().flatten()
    }

    /// Read whatever is new in this transcript and update both derived values.
    fn scan(&mut self, path: &Path) {
        let Ok(mut file) = std::fs::File::open(path) else {
            return;
        };
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        let mut offset = self.offsets.get(path).copied().unwrap_or(0);

        // A truncated or replaced file means our offset is meaningless.
        if offset > len {
            offset = 0;
            self.last_activity.remove(path);
            self.last_error.remove(path);
        }
        if offset == len {
            return;
        }
        if file.seek(SeekFrom::Start(offset)).is_err() {
            return;
        }

        let mut buf = Vec::new();
        if file.read_to_end(&mut buf).is_err() {
            return;
        }
        let text = String::from_utf8_lossy(&buf);

        // The agent may be mid-write; keep any trailing partial line for next tick.
        let complete_upto = match text.rfind('\n') {
            Some(i) => i + 1,
            None => 0,
        };
        for line in text[..complete_upto].lines() {
            if let Some(a) = activity_in_line(line) {
                self.last_activity.insert(path.to_path_buf(), a);
            }
            // Every substantive entry overwrites the verdict, so an error is only
            // reported while it is still the last thing the session managed to do.
            if let Some(verdict) = error_in_line(line) {
                self.last_error.insert(path.to_path_buf(), verdict);
            }
        }
        self.offsets
            .insert(path.to_path_buf(), offset + complete_upto as u64);
    }

    pub fn forget(&mut self, path: &Path) {
        self.offsets.remove(path);
        self.last_activity.remove(path);
        self.last_error.remove(path);
    }

    pub fn retain_only(&mut self, keep: &[PathBuf]) {
        self.offsets.retain(|k, _| keep.contains(k));
        self.last_activity.retain(|k, _| keep.contains(k));
        self.last_error.retain(|k, _| keep.contains(k));
    }
}

/// Read one transcript line as a verdict on whether the session is in error.
///
/// Returns `None` for lines that say nothing either way — the transcript is full
/// of bookkeeping (`file-history-snapshot`, `mode`, `ai-title`, attachments) that
/// must not clear an error the session has not actually recovered from.
///
/// `tool_result` failures are deliberately not errors. All sixteen in this
/// project's transcripts are auto-mode classifier denials that the agent worked
/// around and kept going; treating them as session errors would light the surface
/// up for something already handled.
fn error_in_line(line: &str) -> Option<Option<ApiError>> {
    let v: Value = serde_json::from_str(line.trim()).ok()?;
    match v.get("type").and_then(Value::as_str)? {
        "assistant" | "user" => {}
        _ => return None,
    }
    if v.get("isApiErrorMessage").and_then(Value::as_bool) != Some(true) {
        return Some(None);
    }
    Some(Some(ApiError {
        status: v.get("apiErrorStatus").and_then(Value::as_u64),
        at: v
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(crate::session::parse_iso8601_ms),
    }))
}

/// Pull the last tool call out of one transcript line, if it holds one.
fn activity_in_line(line: &str) -> Option<String> {
    let v: Value = serde_json::from_str(line.trim()).ok()?;
    let content = v.get("message")?.get("content")?.as_array()?;
    let mut found = None;
    for block in content {
        if block.get("type").and_then(Value::as_str) != Some("tool_use") {
            continue;
        }
        let name = block.get("name").and_then(Value::as_str).unwrap_or("");
        let input = block.get("input").unwrap_or(&Value::Null);
        found = Some(derive_activity(name, input));
    }
    found
}

/// Turn one tool call into the very short line shown beside the pet.
///
/// The agent's own wording is used wherever it supplied any. It often does not:
/// across 733 tool calls in 25 transcripts only `Bash` and `Agent` carry a
/// `description`, so 41% of rows reach the fallback below. For those, the most
/// useful token available is whatever the tool is acting on — a filename, a query,
/// a host — which is already sitting in the tool input.
pub fn derive_activity(tool: &str, input: &Value) -> String {
    if let Some(desc) = input.get("description").and_then(Value::as_str) {
        if !desc.trim().is_empty() {
            return truncate(desc.trim());
        }
    }
    match salient_argument(input) {
        Some(arg) => truncate(&match tool {
            "Read" => format!("Reading {arg}"),
            "Edit" | "NotebookEdit" => format!("Editing {arg}"),
            "Write" => format!("Writing {arg}"),
            "Grep" | "WebSearch" => format!("Searching {arg}"),
            "Glob" => format!("Finding {arg}"),
            "WebFetch" => format!("Fetching {arg}"),
            "Skill" => format!("Running {arg}"),
            other => format!("{other} {arg}"),
        }),
        None => truncate(tool),
    }
}

/// The one part of a tool's input worth showing.
fn salient_argument(input: &Value) -> Option<String> {
    for key in [
        "file_path",
        "notebook_path",
        "path",
        "pattern",
        "query",
        "url",
        "skill",
    ] {
        let Some(raw) = input.get(key).and_then(Value::as_str) else {
            continue;
        };
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        return Some(match key {
            // A full path is mostly noise on a small surface; the filename is the signal.
            "file_path" | "notebook_path" | "path" => raw
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or(raw)
                .to_string(),
            "url" => raw
                .split("://")
                .nth(1)
                .and_then(|r| r.split('/').next())
                .unwrap_or(raw)
                .to_string(),
            _ => raw.to_string(),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn agents_own_wording_is_used_verbatim() {
        let input = json!({"command": "git log", "description": "Show working tree status"});
        assert_eq!(derive_activity("Bash", &input), "Show working tree status");
    }

    #[test]
    fn missing_description_falls_back_to_tool_and_filename() {
        let input = json!({"file_path": "/Users/a/Projects/pet/docs/backlog.md"});
        assert_eq!(derive_activity("Read", &input), "Reading backlog.md");
        assert_eq!(derive_activity("Edit", &input), "Editing backlog.md");
        assert_eq!(derive_activity("Write", &input), "Writing backlog.md");
    }

    #[test]
    fn blank_description_is_treated_as_absent() {
        let input = json!({"file_path": "/tmp/x.rs", "description": "   "});
        assert_eq!(derive_activity("Read", &input), "Reading x.rs");
    }

    #[test]
    fn url_falls_back_to_host_and_unknown_tools_still_render() {
        assert_eq!(
            derive_activity("WebFetch", &json!({"url": "https://docs.rs/serde/latest"})),
            "Fetching docs.rs"
        );
        assert_eq!(derive_activity("ListAgents", &json!({})), "ListAgents");
    }

    #[test]
    fn long_descriptions_are_truncated_on_a_word_boundary() {
        let input = json!({"description": "Find and delete every temporary file recursively across the tree"});
        let out = derive_activity("Bash", &input);
        assert!(out.chars().count() <= 46, "got {out:?}");
        assert!(out.ends_with('…'));
        assert!(!out.contains("  "));
    }

    fn write_transcript(name: &str, lines: &[String]) -> PathBuf {
        let dir = std::env::temp_dir().join("agentpet-transcript-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, lines.join("\n") + "\n").unwrap();
        p
    }

    fn tool_line(tool: &str, input: Value) -> String {
        json!({"message": {"content": [{"type": "tool_use", "name": tool, "input": input}]}})
            .to_string()
    }

    #[test]
    fn newest_tool_call_wins() {
        let p = write_transcript(
            "newest.jsonl",
            &[
                tool_line("Bash", json!({"description": "First thing"})),
                tool_line("Bash", json!({"description": "Second thing"})),
            ],
        );
        let mut t = Tailer::new();
        assert_eq!(t.activity(&p).as_deref(), Some("Second thing"));
    }

    #[test]
    fn transcript_with_no_tool_activity_yet_yields_nothing() {
        let p = write_transcript(
            "empty.jsonl",
            &[json!({"type": "user", "message": {"content": "hello"}}).to_string()],
        );
        let mut t = Tailer::new();
        assert_eq!(t.activity(&p), None);
    }

    #[test]
    fn reads_only_what_is_new_and_keeps_the_previous_answer() {
        let p = write_transcript("append.jsonl", &[tool_line("Bash", json!({"description": "One"}))]);
        let mut t = Tailer::new();
        assert_eq!(t.activity(&p).as_deref(), Some("One"));
        let after_first = t.offsets.get(&p).copied().unwrap();

        // Nothing new: the remembered answer stands and the offset does not move.
        assert_eq!(t.activity(&p).as_deref(), Some("One"));
        assert_eq!(t.offsets.get(&p).copied().unwrap(), after_first);

        let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
        use std::io::Write;
        writeln!(f, "{}", tool_line("Bash", json!({"description": "Two"}))).unwrap();
        assert_eq!(t.activity(&p).as_deref(), Some("Two"));
        assert!(t.offsets.get(&p).copied().unwrap() > after_first);
    }

    #[test]
    fn an_error_carries_the_time_of_the_entry_it_stopped_on() {
        let line = json!({
            "type": "assistant",
            "isApiErrorMessage": true,
            "apiErrorStatus": 529,
            "timestamp": "2026-08-25T00:41:35.372Z",
        })
        .to_string();
        let p = write_transcript("errored-at.jsonl", &[line]);
        let mut t = Tailer::new();
        let e = t.error(&p).expect("the error was not seen");
        assert_eq!(e.status, Some(529));
        assert_eq!(e.at, Some(1_787_618_495_372));
        assert_eq!(e.line(), "Error: 529");
    }

    #[test]
    fn an_error_with_no_readable_timestamp_still_reports_the_error() {
        // The age falls back to first-seen; the state must not.
        let missing = json!({"type": "assistant", "isApiErrorMessage": true}).to_string();
        let unreadable = json!({
            "type": "assistant", "isApiErrorMessage": true, "timestamp": "yesterday",
        })
        .to_string();
        for (name, line) in [("no-ts.jsonl", missing), ("bad-ts.jsonl", unreadable)] {
            let p = write_transcript(name, &[line]);
            let e = Tailer::new().error(&p).expect("the error was lost");
            assert_eq!(e.at, None);
            assert_eq!(e.line(), "Errored");
        }
    }

    #[test]
    fn a_half_written_line_is_not_consumed_until_complete() {
        let dir = std::env::temp_dir().join("agentpet-transcript-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("partial.jsonl");
        let full = tool_line("Bash", json!({"description": "Complete line"}));
        // Write a whole line plus the front half of the next one.
        std::fs::write(&p, format!("{full}\n{{\"message\": {{\"cont")).unwrap();
        let mut t = Tailer::new();
        assert_eq!(t.activity(&p).as_deref(), Some("Complete line"));

        // Finish the partial line; it is picked up now, not before.
        let rest = tool_line("Bash", json!({"description": "Finished line"}));
        std::fs::write(&p, format!("{full}\n{rest}\n")).unwrap();
        assert_eq!(t.activity(&p).as_deref(), Some("Finished line"));
    }
}
