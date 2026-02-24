use serde_json::Value;

use super::types::{ContentBlock, JsonlRecord};

/// Represents a tool use event extracted from an assistant message.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolUseEvent {
    pub tool_id: String,
    pub tool_name: String,
    pub display_status: String,
    pub is_reading: bool,
}

/// Parse a single JSONL line into a JsonlRecord.
/// Returns None for empty, whitespace-only, or invalid JSON lines.
pub fn parse_line(line: &str) -> Option<JsonlRecord> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

/// Extract tool use events from an assistant record's content blocks.
pub fn extract_tool_uses(record: &JsonlRecord) -> Vec<ToolUseEvent> {
    let content = match record {
        JsonlRecord::Assistant { message } => &message.content,
        _ => return Vec::new(),
    };

    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolUse { id, name, input } => Some(ToolUseEvent {
                tool_id: id.clone(),
                tool_name: name.clone(),
                display_status: format_tool_status(name, input),
                is_reading: is_reading_tool(name),
            }),
            _ => None,
        })
        .collect()
}

/// Extract tool result IDs from a user record's content blocks.
pub fn extract_tool_results(record: &JsonlRecord) -> Vec<String> {
    let content = match record {
        JsonlRecord::User { message } => &message.content,
        _ => return Vec::new(),
    };

    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolResult { tool_use_id } => Some(tool_use_id.clone()),
            _ => None,
        })
        .collect()
}

/// Extract concatenated text content from an assistant record.
pub fn extract_text(record: &JsonlRecord) -> Option<String> {
    let content = match record {
        JsonlRecord::Assistant { message } => &message.content,
        _ => return None,
    };

    let texts: Vec<&str> = content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();

    if texts.is_empty() {
        None
    } else {
        Some(texts.join(""))
    }
}

/// Check if this record is a system turn_duration record (marks end of a turn).
pub fn is_turn_end(record: &JsonlRecord) -> bool {
    matches!(
        record,
        JsonlRecord::System {
            subtype: Some(subtype),
            ..
        } if subtype == "turn_duration"
    )
}

/// Check if a tool name corresponds to a read-type (non-mutating) tool.
pub fn is_reading_tool(name: &str) -> bool {
    matches!(name, "Read" | "Grep" | "Glob" | "WebFetch" | "WebSearch")
}

/// Format a human-readable status string for a tool invocation.
pub fn format_tool_status(name: &str, input: &Value) -> String {
    match name {
        "Read" | "Write" | "Edit" => {
            let verb = match name {
                "Read" => "Reading",
                "Write" => "Writing",
                "Edit" => "Editing",
                _ => unreachable!(),
            };
            let file_path = input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let basename = file_path.rsplit('/').next().unwrap_or(file_path);
            format!("{verb} {basename}")
        }
        "Bash" => {
            let cmd = input.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let truncated = truncate(cmd, 30);
            format!("Running: {truncated}")
        }
        "Grep" => "Searching code".to_string(),
        "Glob" => "Searching files".to_string(),
        "WebFetch" => "Fetching web content".to_string(),
        "WebSearch" => "Searching the web".to_string(),
        "Task" => {
            let desc = input
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let truncated = truncate(desc, 30);
            format!("Subtask: {truncated}")
        }
        "Skill" => {
            let skill_name = input
                .get("skill")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            format!("Skill: {skill_name}")
        }
        "AskUserQuestion" => "Waiting for answer".to_string(),
        other => format!("Using {other}"),
    }
}

/// Truncate a string to at most `max_len` characters (including "..." suffix if truncated).
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let take = max_len.saturating_sub(3);
        let truncated: String = s.chars().take(take).collect();
        format!("{truncated}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_line() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hi"}]}}"#;
        let record = parse_line(line);
        assert!(record.is_some());
    }

    #[test]
    fn parse_empty_line_returns_none() {
        assert!(parse_line("").is_none());
        assert!(parse_line("   ").is_none());
    }

    #[test]
    fn parse_invalid_json_returns_none() {
        assert!(parse_line("not json at all").is_none());
    }

    #[test]
    fn extract_tool_uses_from_assistant() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/tmp/foo/bar.rs"}}]}}"#;
        let record = parse_line(json).unwrap();
        let tools = extract_tool_uses(&record);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].tool_name, "Read");
        assert_eq!(tools[0].display_status, "Reading bar.rs");
        assert!(tools[0].is_reading);
    }

    #[test]
    fn extract_tool_results_from_user() {
        let json =
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1"}]}}"#;
        let record = parse_line(json).unwrap();
        let results = extract_tool_results(&record);
        assert_eq!(results, vec!["t1"]);
    }

    #[test]
    fn extract_text_from_assistant() {
        let json =
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}]}}"#;
        let record = parse_line(json).unwrap();
        let text = extract_text(&record);
        assert_eq!(text, Some("Hello world".to_string()));
    }

    #[test]
    fn is_turn_end_detects_turn_duration() {
        let json = r#"{"type":"system","subtype":"turn_duration","duration_ms":1500}"#;
        let record = parse_line(json).unwrap();
        assert!(is_turn_end(&record));
    }

    #[test]
    fn format_tool_status_bash_truncates() {
        let input: serde_json::Value = serde_json::json!({"command": "cargo test --lib watcher::parser -- --nocapture long_command"});
        let status = format_tool_status("Bash", &input);
        assert!(status.starts_with("Running: "));
        assert!(status.len() <= 40);
    }

    #[test]
    fn format_tool_status_task_shows_description() {
        let input: serde_json::Value = serde_json::json!({"description": "Explore codebase"});
        let status = format_tool_status("Task", &input);
        assert_eq!(status, "Subtask: Explore codebase");
    }

    #[test]
    fn format_tool_status_skill_shows_name() {
        let input: serde_json::Value = serde_json::json!({"skill": "sdd-apply"});
        let status = format_tool_status("Skill", &input);
        assert_eq!(status, "Skill: sdd-apply");
    }
}
