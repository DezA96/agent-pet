use crate::session::truncate_activity as truncate;
use serde_json::Value;

/// Turn one completed rollout item into the very short line shown beside the pet.
///
/// `None` means the item describes nothing worth showing — the user's own
/// message, a context compaction, a subagent bookkeeping event. The caller keeps
/// whatever line it already had rather than blanking the row.
///
/// The agent's own wording is preferred wherever it exists, as it is for Claude.
/// Here it usually does: `Reasoning` is the most frequent item type by a wide
/// margin (1016 of 2175 across every rollout on this disk), and its
/// `summary_text` is the agent describing its own next move. Codex therefore
/// reaches derived phrasing far less often than Claude's 41%.
pub fn activity_of(item: &Value) -> Option<String> {
    let line = match item.get("type").and_then(Value::as_str)? {
        "Reasoning" => reasoning(item)?,
        "AgentMessage" => agent_message(item)?,
        "CommandExecution" => command(item)?,
        "FileChange" => file_change(item)?,
        "McpToolCall" => qualified(item, "server", "tool")?,
        "DynamicToolCall" => qualified(item, "namespace", "tool")?,
        "Extension" => extension(item)?,
        "ImageView" => format!("Viewing {}", file_name(item.get("path")?.as_str()?)),
        // UserMessage, ContextCompaction, SubAgentActivity and anything a later
        // Codex adds: not a description of what the session is doing.
        _ => return None,
    };
    let line = truncate(&line);
    (!line.is_empty()).then_some(line)
}

/// The agent's own summary of what it is about to do.
///
/// `summary_text` is an array, and 844 of 1016 on this disk hold more than one
/// entry — successive thoughts within one reasoning block. The last is the
/// newest, which is the one the row should show. Each arrives wrapped in the
/// markdown bold the TUI renders (`**Planning codebase inspection**`); the pet
/// draws plain text, so the markers come off.
fn reasoning(item: &Value) -> Option<String> {
    let summary = item.get("summary_text")?.as_array()?;
    let last = summary
        .iter()
        .filter_map(Value::as_str)
        .map(|s| s.trim().trim_matches('*').trim())
        .rfind(|s| !s.is_empty())?;
    Some(last.to_string())
}

/// The first sentence of something the agent said.
///
/// Only 38 of 173 agent messages land immediately before `task_complete`; the
/// rest are mid-turn progress narration that already reads like a status line.
/// The turn-final ones never reach the pet anyway — `task_complete` follows
/// them, the state becomes idle, and an idle row shows no activity.
fn agent_message(item: &Value) -> Option<String> {
    let text = item
        .get("content")?
        .as_array()?
        .iter()
        .find(|b| b.get("type").and_then(Value::as_str) == Some("Text"))
        .and_then(|b| b.get("text"))
        .and_then(Value::as_str)?
        .trim();
    Some(first_sentence(text).to_string())
}

/// What a shell command is doing, from Codex's own parse of it.
///
/// `parsed_cmd` is the agent's structured reading — a `read` carries the file's
/// name, a `search` its query — which is the same signal Claude's tool input
/// gives. Where Codex could not classify the command it says `unknown`, and the
/// command text itself is then the most useful thing available.
fn command(item: &Value) -> Option<String> {
    let parsed = item
        .get("parsed_cmd")
        .and_then(Value::as_array)
        .and_then(|a| a.last());

    if let Some(p) = parsed {
        let text = |k: &str| p.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty());
        match p.get("type").and_then(Value::as_str) {
            Some("read") => {
                if let Some(name) = text("name") {
                    return Some(format!("Reading {name}"));
                }
            }
            Some("search") => {
                if let Some(query) = text("query") {
                    return Some(format!("Searching {query}"));
                }
            }
            Some("list_files") => {
                return Some(match text("path") {
                    Some(path) => format!("Listing {}", file_name(path)),
                    None => "Listing files".to_string(),
                });
            }
            _ => {}
        }
        if let Some(cmd) = text("cmd") {
            return Some(format!("Running {cmd}"));
        }
    }

    // No parse at all: the raw argv, whose last element is the script a shell was
    // asked to run (`["/bin/zsh", "-lc", "pwd && rg --files"]`).
    let argv = item.get("command")?.as_array()?;
    let last = argv.iter().filter_map(Value::as_str).next_back()?.trim();
    (!last.is_empty()).then(|| format!("Running {last}"))
}

fn file_change(item: &Value) -> Option<String> {
    let changes = item.get("changes")?.as_object()?;
    let mut entries = changes.iter();
    let (path, change) = entries.next()?;
    if entries.next().is_some() {
        return Some(format!("Editing {} files", changes.len()));
    }
    let verb = match change.get("type").and_then(Value::as_str) {
        Some("add") => "Writing",
        Some("delete") => "Deleting",
        _ => "Editing",
    };
    Some(format!("{verb} {}", file_name(path)))
}

/// A tool call named by the thing that provides it, e.g. `node_repl.js`.
fn qualified(item: &Value, owner_key: &str, tool_key: &str) -> Option<String> {
    let tool = item.get(tool_key)?.as_str()?.trim();
    if tool.is_empty() {
        return None;
    }
    Some(match item.get(owner_key).and_then(Value::as_str) {
        Some(owner) if !owner.trim().is_empty() => format!("Running {}.{tool}", owner.trim()),
        _ => format!("Running {tool}"),
    })
}

/// A built-in extension: web search, image generation, and whatever follows.
fn extension(item: &Value) -> Option<String> {
    let kind = item.get("kind").and_then(Value::as_str).unwrap_or("").trim();
    if let Some(query) = item.get("query").and_then(Value::as_str) {
        let query = query.trim();
        if !query.is_empty() {
            return Some(format!("Searching {query}"));
        }
    }
    (!kind.is_empty()).then(|| format!("Running {kind}"))
}

/// The last path segment, which is the signal; the rest is noise on a small row.
///
/// Codex writes some paths as `file://` URLs and others bare, so both are trimmed
/// the same way — the segment after the final separator.
fn file_name(raw: &str) -> &str {
    let raw = raw.trim().trim_end_matches('/');
    raw.rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(raw)
}

/// Where a piece of prose first stops.
///
/// A newline ends it as surely as a full stop does: agent messages routinely open
/// with a one-line lead ("Try this quick diagnosis:") and continue into a
/// markdown list, and only the lead belongs on a row.
fn first_sentence(text: &str) -> &str {
    let end = text
        .find('\n')
        .into_iter()
        .chain(text.find(". ").map(|i| i + 1))
        .min()
        .unwrap_or(text.len());
    text[..end].trim_end()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reasoning_shows_the_newest_thought_without_its_markdown() {
        let item = json!({
            "type": "Reasoning",
            "summary_text": ["**Planning codebase inspection**", "**Inspecting SSH configs**"],
        });
        assert_eq!(activity_of(&item).as_deref(), Some("Inspecting SSH configs"));
    }

    #[test]
    fn a_single_thought_reads_the_same_way() {
        let item = json!({"type": "Reasoning", "summary_text": ["**Escalating sandbox failure with details**"]});
        assert_eq!(
            activity_of(&item).as_deref(),
            Some("Escalating sandbox failure with details")
        );
    }

    #[test]
    fn an_empty_reasoning_block_says_nothing_rather_than_blank() {
        let item = json!({"type": "Reasoning", "summary_text": []});
        assert_eq!(activity_of(&item), None);
        let item = json!({"type": "Reasoning", "summary_text": ["", "  ", "**"]});
        assert_eq!(activity_of(&item), None);
    }

    #[test]
    fn an_agent_message_is_cut_at_its_first_sentence() {
        let item = json!({
            "type": "AgentMessage",
            "content": [{"type": "Text", "text": "Both SSH routes work. I\u{2019}m now checking the firewall."}],
        });
        assert_eq!(activity_of(&item).as_deref(), Some("Both SSH routes work."));
    }

    #[test]
    fn a_lead_line_before_a_list_stops_at_the_newline() {
        let item = json!({
            "type": "AgentMessage",
            "content": [{"type": "Text", "text": "Try this quick diagnosis:\n\n1. On the Windows PC:"}],
        });
        assert_eq!(activity_of(&item).as_deref(), Some("Try this quick diagnosis:"));
    }

    #[test]
    fn a_parsed_command_uses_codexs_own_reading_of_it() {
        let read = json!({
            "type": "CommandExecution",
            "command": ["/bin/zsh", "-lc", "sed -n '1,260p' /a/b/SKILL.md"],
            "parsed_cmd": [{"type": "read", "cmd": "sed -n '1,260p' /a/b/SKILL.md", "name": "SKILL.md"}],
        });
        assert_eq!(activity_of(&read).as_deref(), Some("Reading SKILL.md"));

        let search = json!({
            "type": "CommandExecution",
            "parsed_cmd": [{"type": "search", "cmd": "rg 'container_tools'", "query": "container_tools"}],
        });
        assert_eq!(activity_of(&search).as_deref(), Some("Searching container_tools"));

        let list = json!({
            "type": "CommandExecution",
            "parsed_cmd": [{"type": "list_files", "cmd": "rg --files x", "path": "26.819.11345"}],
        });
        assert_eq!(activity_of(&list).as_deref(), Some("Listing 26.819.11345"));
    }

    #[test]
    fn a_list_with_no_path_still_says_what_it_is_doing() {
        let item = json!({
            "type": "CommandExecution",
            "parsed_cmd": [{"type": "list_files", "cmd": "rg --files", "path": null}],
        });
        assert_eq!(activity_of(&item).as_deref(), Some("Listing files"));
    }

    #[test]
    fn an_unclassified_command_shows_the_command_itself() {
        // 432 of 761 parsed commands on this disk are `unknown`, so this is the
        // common path, not an edge case.
        let item = json!({
            "type": "CommandExecution",
            "command": ["/bin/zsh", "-lc", "pwd && rg --files"],
            "parsed_cmd": [{"type": "unknown", "cmd": "pwd && rg --files"}],
        });
        assert_eq!(activity_of(&item).as_deref(), Some("Running pwd && rg --files"));
    }

    #[test]
    fn a_command_with_no_parse_falls_back_to_its_argv() {
        let item = json!({
            "type": "CommandExecution",
            "command": ["/bin/zsh", "-lc", "git status"],
        });
        assert_eq!(activity_of(&item).as_deref(), Some("Running git status"));
    }

    #[test]
    fn a_single_file_change_names_the_file_and_what_happened_to_it() {
        let add = json!({"type": "FileChange", "changes": {"/tmp/x/build.mjs": {"type": "add"}}});
        assert_eq!(activity_of(&add).as_deref(), Some("Writing build.mjs"));
        let upd = json!({"type": "FileChange", "changes": {"/tmp/x/build.mjs": {"type": "update"}}});
        assert_eq!(activity_of(&upd).as_deref(), Some("Editing build.mjs"));
        let del = json!({"type": "FileChange", "changes": {"/tmp/x/build.mjs": {"type": "delete"}}});
        assert_eq!(activity_of(&del).as_deref(), Some("Deleting build.mjs"));
    }

    #[test]
    fn several_file_changes_are_counted_rather_than_listed() {
        let item = json!({
            "type": "FileChange",
            "changes": {"/a/one.rs": {"type": "add"}, "/a/two.rs": {"type": "update"}},
        });
        assert_eq!(activity_of(&item).as_deref(), Some("Editing 2 files"));
    }

    #[test]
    fn tool_calls_are_named_by_what_provides_them() {
        let mcp = json!({"type": "McpToolCall", "server": "node_repl", "tool": "js"});
        assert_eq!(activity_of(&mcp).as_deref(), Some("Running node_repl.js"));
        let dyn_call = json!({
            "type": "DynamicToolCall",
            "namespace": "codex_app",
            "tool": "load_workspace_dependencies",
        });
        assert_eq!(
            activity_of(&dyn_call).as_deref(),
            Some("Running codex_app.load_workspace_dependencies")
        );
    }

    #[test]
    fn a_web_search_shows_what_is_being_searched_for() {
        let item = json!({"type": "Extension", "kind": "web.search", "query": "rdp port connectivity"});
        assert_eq!(activity_of(&item).as_deref(), Some("Searching rdp port connectivity"));
    }

    #[test]
    fn an_extension_with_no_query_falls_back_to_its_kind() {
        let item = json!({"type": "Extension", "kind": "image_gen.generation"});
        assert_eq!(activity_of(&item).as_deref(), Some("Running image_gen.generation"));
    }

    #[test]
    fn an_image_view_names_the_image() {
        let item = json!({"type": "ImageView", "path": "file:///Users/a/out/preview_Summary.png"});
        assert_eq!(activity_of(&item).as_deref(), Some("Viewing preview_Summary.png"));
    }

    #[test]
    fn items_that_describe_nothing_the_session_is_doing_are_skipped() {
        // Skipped rather than rendered blank: the row keeps its previous line.
        for item in [
            json!({"type": "UserMessage", "content": [{"type": "text", "text": "hi"}]}),
            json!({"type": "ContextCompaction", "id": "x"}),
            json!({"type": "SubAgentActivity", "kind": "started"}),
            json!({"type": "SomethingCodexAddsLater"}),
            json!({"id": "no-type-at-all"}),
        ] {
            assert_eq!(activity_of(&item), None, "unexpectedly rendered {item}");
        }
    }

    #[test]
    fn every_line_is_cut_to_the_pets_width() {
        let item = json!({
            "type": "Reasoning",
            "summary_text": ["**Assessing codebase status and architecture risks in depth**"],
        });
        let out = activity_of(&item).unwrap();
        assert!(out.chars().count() <= 46, "got {out:?}");
        assert!(out.ends_with('…'));
    }
}
