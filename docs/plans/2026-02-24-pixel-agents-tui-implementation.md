# Pixel Agents TUI — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust TUI binary that visualizes Claude Code agents as animated ASCII characters with real-time tool activity, sub-agent trees, SDD workflow tracking, and prompt summaries.

**Architecture:** Single monolith binary using Ratatui for rendering, notify for filesystem watching, and crossterm for terminal events. Two-mode execution: launcher (detects terminal, creates split) and attach (renders TUI). Modules: terminal/, watcher/, state/, ui/.

**Tech Stack:** Rust 2021 edition, Ratatui 0.30, Crossterm 0.28, notify 7, serde_json 1, clap 4, tokio 1

**Design doc:** `docs/plans/2026-02-24-pixel-agents-tui-design.md`

---

## Phase 1: Foundation

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `.gitignore`
- Create: `rust-toolchain.toml`

**Step 1: Initialize Cargo project**

```toml
# Cargo.toml
[package]
name = "pixel-agents-tui"
version = "0.1.0"
edition = "2021"
description = "TUI dashboard for visualizing Claude Code agents as pixel art characters"
license = "MIT"

[dependencies]
ratatui = "0.30"
crossterm = "0.28"
notify = "7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
directories = "5"
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }

[dev-dependencies]
tempfile = "3"
```

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
```

```gitignore
# .gitignore
/target
```

```rust
// src/lib.rs
pub mod state;
pub mod watcher;
```

```rust
// src/main.rs
fn main() {
    println!("pixel-agents-tui");
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs src/lib.rs .gitignore rust-toolchain.toml
git commit -m "feat: scaffold Rust project with dependencies"
```

---

### Task 2: JSONL Types

**Files:**
- Create: `src/watcher/mod.rs`
- Create: `src/watcher/types.rs`

**Step 1: Write the test for JSONL record type deserialization**

```rust
// src/watcher/types.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_tool_use_record() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tool_1","name":"Read","input":{"file_path":"/tmp/test.rs"}}]}}"#;
        let record: JsonlRecord = serde_json::from_str(json).unwrap();
        match record {
            JsonlRecord::Assistant { message } => {
                assert_eq!(message.content.len(), 1);
                match &message.content[0] {
                    ContentBlock::ToolUse { id, name, input } => {
                        assert_eq!(name, "Read");
                        assert_eq!(id, "tool_1");
                    }
                    _ => panic!("Expected ToolUse"),
                }
            }
            _ => panic!("Expected Assistant record"),
        }
    }

    #[test]
    fn deserialize_tool_result_record() {
        let json = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tool_1"}]}}"#;
        let record: JsonlRecord = serde_json::from_str(json).unwrap();
        match record {
            JsonlRecord::User { message } => {
                match &message.content[0] {
                    ContentBlock::ToolResult { tool_use_id } => {
                        assert_eq!(tool_use_id, "tool_1");
                    }
                    _ => panic!("Expected ToolResult"),
                }
            }
            _ => panic!("Expected User record"),
        }
    }

    #[test]
    fn deserialize_turn_duration_record() {
        let json = r#"{"type":"system","subtype":"turn_duration","duration_ms":1500}"#;
        let record: JsonlRecord = serde_json::from_str(json).unwrap();
        match record {
            JsonlRecord::System { subtype, .. } => {
                assert_eq!(subtype.as_deref(), Some("turn_duration"));
            }
            _ => panic!("Expected System record"),
        }
    }

    #[test]
    fn deserialize_text_content() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}]}}"#;
        let record: JsonlRecord = serde_json::from_str(json).unwrap();
        match record {
            JsonlRecord::Assistant { message } => {
                match &message.content[0] {
                    ContentBlock::Text { text } => {
                        assert_eq!(text, "Hello world");
                    }
                    _ => panic!("Expected Text"),
                }
            }
            _ => panic!("Expected Assistant"),
        }
    }

    #[test]
    fn unknown_record_types_dont_crash() {
        let json = r#"{"type":"unknown_future_type","data":123}"#;
        let record: JsonlRecord = serde_json::from_str(json).unwrap();
        assert!(matches!(record, JsonlRecord::Unknown));
    }
}
```

**Step 2: Run the test to verify it fails**

Run: `cargo test --lib watcher::types`
Expected: FAIL — types not defined yet

**Step 3: Implement the JSONL types**

```rust
// src/watcher/types.rs
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum JsonlRecord {
    #[serde(rename = "assistant")]
    Assistant { message: AssistantMessage },
    #[serde(rename = "user")]
    User { message: UserMessage },
    #[serde(rename = "system")]
    System {
        subtype: Option<String>,
        #[serde(default)]
        duration_ms: Option<u64>,
    },
    #[serde(rename = "progress")]
    Progress {
        subtype: Option<String>,
        #[serde(flatten)]
        data: Value,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
pub struct UserMessage {
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
    },
    #[serde(rename = "text")]
    Text {
        text: String,
    },
    #[serde(other)]
    Other,
}

// src/watcher/mod.rs
pub mod types;
```

**Step 4: Run test to verify it passes**

Run: `cargo test --lib watcher::types`
Expected: All 5 tests PASS

**Step 5: Commit**

```bash
git add src/watcher/
git commit -m "feat: add JSONL record types with serde deserialization"
```

---

### Task 3: JSONL Line Parser

**Files:**
- Create: `src/watcher/parser.rs`
- Modify: `src/watcher/mod.rs`

**Step 1: Write the failing test for line-by-line parsing**

```rust
// src/watcher/parser.rs
use super::types::{JsonlRecord, ContentBlock};

/// Parse a single JSONL line into a record.
/// Returns None for empty lines or unparseable records.
pub fn parse_line(line: &str) -> Option<JsonlRecord> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

/// Extract tool use events from an assistant record.
pub fn extract_tool_uses(record: &JsonlRecord) -> Vec<ToolUseEvent> {
    match record {
        JsonlRecord::Assistant { message } => {
            message.content.iter().filter_map(|block| {
                match block {
                    ContentBlock::ToolUse { id, name, input } => {
                        Some(ToolUseEvent {
                            tool_id: id.clone(),
                            tool_name: name.clone(),
                            display_status: format_tool_status(name, input),
                            is_reading: is_reading_tool(name),
                        })
                    }
                    _ => None,
                }
            }).collect()
        }
        _ => vec![],
    }
}

/// Extract tool result IDs from a user record.
pub fn extract_tool_results(record: &JsonlRecord) -> Vec<String> {
    match record {
        JsonlRecord::User { message } => {
            message.content.iter().filter_map(|block| {
                match block {
                    ContentBlock::ToolResult { tool_use_id } => Some(tool_use_id.clone()),
                    _ => None,
                }
            }).collect()
        }
        _ => vec![],
    }
}

/// Extract assistant text content for prompt summary.
pub fn extract_text(record: &JsonlRecord) -> Option<String> {
    match record {
        JsonlRecord::Assistant { message } => {
            let texts: Vec<&str> = message.content.iter().filter_map(|block| {
                match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                }
            }).collect();
            if texts.is_empty() { None } else { Some(texts.join(" ")) }
        }
        _ => None,
    }
}

/// Check if a turn ended (system record with turn_duration subtype).
pub fn is_turn_end(record: &JsonlRecord) -> bool {
    matches!(record, JsonlRecord::System { subtype: Some(s), .. } if s == "turn_duration")
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolUseEvent {
    pub tool_id: String,
    pub tool_name: String,
    pub display_status: String,
    pub is_reading: bool,
}

fn is_reading_tool(name: &str) -> bool {
    matches!(name, "Read" | "Grep" | "Glob" | "WebFetch" | "WebSearch")
}

fn format_tool_status(name: &str, input: &serde_json::Value) -> String {
    match name {
        "Read" => {
            let file = input.get("file_path")
                .and_then(|v| v.as_str())
                .map(|p| {
                    p.rsplit('/').next().unwrap_or(p)
                })
                .unwrap_or("...");
            format!("Reading {file}")
        }
        "Write" => {
            let file = input.get("file_path")
                .and_then(|v| v.as_str())
                .map(|p| p.rsplit('/').next().unwrap_or(p))
                .unwrap_or("...");
            format!("Writing {file}")
        }
        "Edit" => {
            let file = input.get("file_path")
                .and_then(|v| v.as_str())
                .map(|p| p.rsplit('/').next().unwrap_or(p))
                .unwrap_or("...");
            format!("Editing {file}")
        }
        "Bash" => {
            let cmd = input.get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("...");
            let truncated = if cmd.len() > 30 { &cmd[..30] } else { cmd };
            format!("Running: {truncated}")
        }
        "Grep" => "Searching code".to_string(),
        "Glob" => "Searching files".to_string(),
        "WebFetch" => "Fetching web content".to_string(),
        "WebSearch" => "Searching the web".to_string(),
        "Task" => {
            let desc = input.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("subtask");
            let truncated = if desc.len() > 30 { &desc[..30] } else { desc };
            format!("Subtask: {truncated}")
        }
        "Skill" => {
            let skill = input.get("skill")
                .and_then(|v| v.as_str())
                .unwrap_or("...");
            format!("Skill: {skill}")
        }
        "AskUserQuestion" => "Waiting for answer".to_string(),
        other => format!("Using {other}"),
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
        let json = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1"}]}}"#;
        let record = parse_line(json).unwrap();
        let results = extract_tool_results(&record);
        assert_eq!(results, vec!["t1"]);
    }

    #[test]
    fn extract_text_from_assistant() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}]}}"#;
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
        assert!(status.len() <= 40); // "Running: " + 30 chars max
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
```

**Step 2: Update mod.rs**

```rust
// src/watcher/mod.rs
pub mod types;
pub mod parser;
```

**Step 3: Run tests to verify they pass**

Run: `cargo test --lib watcher::parser`
Expected: All 10 tests PASS

**Step 4: Commit**

```bash
git add src/watcher/
git commit -m "feat: add JSONL line parser with tool extraction and formatting"
```

---

### Task 4: SDD Phase Detection

**Files:**
- Create: `src/state/mod.rs`
- Create: `src/state/sdd.rs`

**Step 1: Write tests for SDD phase detection**

```rust
// src/state/sdd.rs
use crate::watcher::parser::ToolUseEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SddPhase {
    Explore,
    Propose,
    Spec,
    Design,
    Tasks,
    Apply,
    Verify,
    Archive,
}

impl SddPhase {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Explore => "Explore",
            Self::Propose => "Propose",
            Self::Spec => "Spec",
            Self::Design => "Design",
            Self::Tasks => "Tasks",
            Self::Apply => "Apply",
            Self::Verify => "Verify",
            Self::Archive => "Archive",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Self::Explore => 0,
            Self::Propose => 1,
            Self::Spec => 2,
            Self::Design => 3,
            Self::Tasks => 4,
            Self::Apply => 5,
            Self::Verify => 6,
            Self::Archive => 7,
        }
    }

    pub fn total() -> usize {
        8
    }
}

/// Detect SDD phase from a Skill tool invocation.
/// Returns Some(phase) if the tool display_status matches an SDD skill pattern.
pub fn detect_sdd_phase(tool: &ToolUseEvent) -> Option<SddPhase> {
    if tool.tool_name != "Skill" {
        return None;
    }
    let status = &tool.display_status;
    if status.contains("sdd-explore") { return Some(SddPhase::Explore); }
    if status.contains("sdd-propose") { return Some(SddPhase::Propose); }
    if status.contains("sdd-spec") { return Some(SddPhase::Spec); }
    if status.contains("sdd-design") { return Some(SddPhase::Design); }
    if status.contains("sdd-tasks") { return Some(SddPhase::Tasks); }
    if status.contains("sdd-apply") { return Some(SddPhase::Apply); }
    if status.contains("sdd-verify") { return Some(SddPhase::Verify); }
    if status.contains("sdd-archive") { return Some(SddPhase::Archive); }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill_event(skill_name: &str) -> ToolUseEvent {
        ToolUseEvent {
            tool_id: "t1".to_string(),
            tool_name: "Skill".to_string(),
            display_status: format!("Skill: {skill_name}"),
            is_reading: false,
        }
    }

    #[test]
    fn detects_all_sdd_phases() {
        assert_eq!(detect_sdd_phase(&skill_event("sdd-explore")), Some(SddPhase::Explore));
        assert_eq!(detect_sdd_phase(&skill_event("sdd-propose")), Some(SddPhase::Propose));
        assert_eq!(detect_sdd_phase(&skill_event("sdd-spec")), Some(SddPhase::Spec));
        assert_eq!(detect_sdd_phase(&skill_event("sdd-design")), Some(SddPhase::Design));
        assert_eq!(detect_sdd_phase(&skill_event("sdd-tasks")), Some(SddPhase::Tasks));
        assert_eq!(detect_sdd_phase(&skill_event("sdd-apply")), Some(SddPhase::Apply));
        assert_eq!(detect_sdd_phase(&skill_event("sdd-verify")), Some(SddPhase::Verify));
        assert_eq!(detect_sdd_phase(&skill_event("sdd-archive")), Some(SddPhase::Archive));
    }

    #[test]
    fn non_skill_tool_returns_none() {
        let tool = ToolUseEvent {
            tool_id: "t1".to_string(),
            tool_name: "Read".to_string(),
            display_status: "Reading file.rs".to_string(),
            is_reading: true,
        };
        assert_eq!(detect_sdd_phase(&tool), None);
    }

    #[test]
    fn non_sdd_skill_returns_none() {
        assert_eq!(detect_sdd_phase(&skill_event("brainstorming")), None);
    }

    #[test]
    fn phase_labels_are_correct() {
        assert_eq!(SddPhase::Apply.label(), "Apply");
        assert_eq!(SddPhase::Apply.index(), 5);
        assert_eq!(SddPhase::total(), 8);
    }
}
```

```rust
// src/state/mod.rs
pub mod sdd;
```

**Step 2: Run tests**

Run: `cargo test --lib state::sdd`
Expected: All 4 tests PASS

**Step 3: Commit**

```bash
git add src/state/
git commit -m "feat: add SDD phase detection from Skill tool invocations"
```

---

### Task 5: Agent State Machine

**Files:**
- Create: `src/state/agent.rs`
- Modify: `src/state/mod.rs`

**Step 1: Write the agent state struct and transition tests**

```rust
// src/state/agent.rs
use std::path::PathBuf;
use std::time::Instant;

use crate::state::sdd::SddPhase;
use crate::watcher::parser::ToolUseEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Active,
    Waiting,
    Dormant,
}

impl AgentStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Waiting => "waiting",
            Self::Dormant => "dormant",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Active => "●",
            Self::Waiting => "○",
            Self::Dormant => "◌",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubAgent {
    pub id: i32,
    pub parent_tool_id: String,
    pub agent_type: String,
    pub active_tools: Vec<ToolUseEvent>,
}

#[derive(Debug)]
pub struct AgentState {
    pub id: u32,
    pub session_file: PathBuf,
    pub status: AgentStatus,
    pub active_tools: Vec<ToolUseEvent>,
    pub sub_agents: Vec<SubAgent>,
    pub sdd_phase: Option<SddPhase>,
    pub prompt_summary: String,
    pub last_activity: Instant,
}

impl AgentState {
    pub fn new(id: u32, session_file: PathBuf) -> Self {
        Self {
            id,
            session_file,
            status: AgentStatus::Waiting,
            active_tools: Vec::new(),
            sub_agents: Vec::new(),
            sdd_phase: None,
            prompt_summary: String::new(),
            last_activity: Instant::now(),
        }
    }

    pub fn add_tool(&mut self, tool: ToolUseEvent) {
        // Check for SDD phase
        if let Some(phase) = crate::state::sdd::detect_sdd_phase(&tool) {
            self.sdd_phase = Some(phase);
        }

        // Check for Task tool → spawn sub-agent
        if tool.tool_name == "Task" {
            let sub_id = -(self.sub_agents.len() as i32 + 1);
            self.sub_agents.push(SubAgent {
                id: sub_id,
                parent_tool_id: tool.tool_id.clone(),
                agent_type: tool.display_status.clone(),
                active_tools: Vec::new(),
            });
        }

        self.active_tools.push(tool);
        self.status = AgentStatus::Active;
        self.last_activity = Instant::now();
    }

    pub fn remove_tool(&mut self, tool_id: &str) {
        // Remove sub-agent if this was a Task tool
        self.sub_agents.retain(|s| s.parent_tool_id != tool_id);
        self.active_tools.retain(|t| t.tool_id != tool_id);
        self.last_activity = Instant::now();
    }

    pub fn mark_waiting(&mut self) {
        self.active_tools.clear();
        self.sub_agents.clear();
        self.status = AgentStatus::Waiting;
        self.last_activity = Instant::now();
    }

    pub fn set_prompt_summary(&mut self, text: &str) {
        if self.prompt_summary.is_empty() {
            // Take first 150 chars as summary
            let summary: String = text.chars().take(150).collect();
            self.prompt_summary = summary;
        }
    }

    pub fn is_dormant(&self, timeout_secs: u64) -> bool {
        self.last_activity.elapsed().as_secs() > timeout_secs
    }

    pub fn current_tool_display(&self) -> Option<&str> {
        self.active_tools.last().map(|t| t.display_status.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_agent() -> AgentState {
        AgentState::new(1, PathBuf::from("/tmp/test.jsonl"))
    }

    fn read_tool() -> ToolUseEvent {
        ToolUseEvent {
            tool_id: "t1".to_string(),
            tool_name: "Read".to_string(),
            display_status: "Reading main.rs".to_string(),
            is_reading: true,
        }
    }

    fn task_tool() -> ToolUseEvent {
        ToolUseEvent {
            tool_id: "t2".to_string(),
            tool_name: "Task".to_string(),
            display_status: "Subtask: explore code".to_string(),
            is_reading: false,
        }
    }

    fn sdd_skill_tool() -> ToolUseEvent {
        ToolUseEvent {
            tool_id: "t3".to_string(),
            tool_name: "Skill".to_string(),
            display_status: "Skill: sdd-apply".to_string(),
            is_reading: false,
        }
    }

    #[test]
    fn new_agent_starts_waiting() {
        let agent = make_agent();
        assert_eq!(agent.status, AgentStatus::Waiting);
        assert!(agent.active_tools.is_empty());
    }

    #[test]
    fn adding_tool_sets_active() {
        let mut agent = make_agent();
        agent.add_tool(read_tool());
        assert_eq!(agent.status, AgentStatus::Active);
        assert_eq!(agent.active_tools.len(), 1);
    }

    #[test]
    fn removing_tool_keeps_remaining() {
        let mut agent = make_agent();
        agent.add_tool(read_tool());
        agent.add_tool(ToolUseEvent {
            tool_id: "t99".to_string(),
            tool_name: "Write".to_string(),
            display_status: "Writing foo.rs".to_string(),
            is_reading: false,
        });
        agent.remove_tool("t1");
        assert_eq!(agent.active_tools.len(), 1);
        assert_eq!(agent.active_tools[0].tool_id, "t99");
    }

    #[test]
    fn mark_waiting_clears_tools() {
        let mut agent = make_agent();
        agent.add_tool(read_tool());
        agent.mark_waiting();
        assert_eq!(agent.status, AgentStatus::Waiting);
        assert!(agent.active_tools.is_empty());
    }

    #[test]
    fn task_tool_spawns_sub_agent() {
        let mut agent = make_agent();
        agent.add_tool(task_tool());
        assert_eq!(agent.sub_agents.len(), 1);
        assert_eq!(agent.sub_agents[0].id, -1);
        assert_eq!(agent.sub_agents[0].parent_tool_id, "t2");
    }

    #[test]
    fn removing_task_tool_removes_sub_agent() {
        let mut agent = make_agent();
        agent.add_tool(task_tool());
        agent.remove_tool("t2");
        assert!(agent.sub_agents.is_empty());
    }

    #[test]
    fn sdd_skill_sets_phase() {
        let mut agent = make_agent();
        agent.add_tool(sdd_skill_tool());
        assert_eq!(agent.sdd_phase, Some(SddPhase::Apply));
    }

    #[test]
    fn prompt_summary_set_once() {
        let mut agent = make_agent();
        agent.set_prompt_summary("First message");
        agent.set_prompt_summary("Second message should be ignored");
        assert_eq!(agent.prompt_summary, "First message");
    }

    #[test]
    fn current_tool_display_returns_last() {
        let mut agent = make_agent();
        assert!(agent.current_tool_display().is_none());
        agent.add_tool(read_tool());
        assert_eq!(agent.current_tool_display(), Some("Reading main.rs"));
    }

    #[test]
    fn status_labels_and_symbols() {
        assert_eq!(AgentStatus::Active.label(), "active");
        assert_eq!(AgentStatus::Active.symbol(), "●");
        assert_eq!(AgentStatus::Waiting.symbol(), "○");
        assert_eq!(AgentStatus::Dormant.symbol(), "◌");
    }
}
```

**Step 2: Update state/mod.rs**

```rust
// src/state/mod.rs
pub mod sdd;
pub mod agent;
```

**Step 3: Run tests**

Run: `cargo test --lib state::agent`
Expected: All 10 tests PASS

**Step 4: Commit**

```bash
git add src/state/
git commit -m "feat: add agent state machine with tool tracking and sub-agents"
```

---

## Phase 2: File System Integration

### Task 6: Session Discovery

**Files:**
- Create: `src/watcher/discovery.rs`
- Modify: `src/watcher/mod.rs`

**Step 1: Write session discovery logic with tests**

```rust
// src/watcher/discovery.rs
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const DORMANCY_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

/// Scan the claude projects directory for active JSONL session files.
pub fn scan_sessions(claude_dir: &Path) -> Vec<PathBuf> {
    let projects_dir = claude_dir.join("projects");
    if !projects_dir.exists() {
        return Vec::new();
    }

    let mut sessions = Vec::new();
    let now = SystemTime::now();

    if let Ok(entries) = fs::read_dir(&projects_dir) {
        for entry in entries.flatten() {
            let project_path = entry.path();
            if !project_path.is_dir() {
                continue;
            }
            if let Ok(files) = fs::read_dir(&project_path) {
                for file in files.flatten() {
                    let path = file.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                        if let Ok(metadata) = fs::metadata(&path) {
                            if let Ok(modified) = metadata.modified() {
                                if let Ok(age) = now.duration_since(modified) {
                                    if age < DORMANCY_TIMEOUT {
                                        sessions.push(path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    sessions
}

/// Track known sessions and detect new/removed ones.
pub struct SessionTracker {
    known: HashMap<PathBuf, u32>,
    next_id: u32,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self {
            known: HashMap::new(),
            next_id: 1,
        }
    }

    /// Returns (new_sessions, removed_sessions)
    pub fn update(&mut self, current: &[PathBuf]) -> (Vec<(u32, PathBuf)>, Vec<u32>) {
        let current_set: std::collections::HashSet<&PathBuf> = current.iter().collect();

        // Find new sessions
        let mut new_sessions = Vec::new();
        for path in current {
            if !self.known.contains_key(path) {
                let id = self.next_id;
                self.next_id += 1;
                self.known.insert(path.clone(), id);
                new_sessions.push((id, path.clone()));
            }
        }

        // Find removed sessions
        let mut removed = Vec::new();
        self.known.retain(|path, id| {
            if current_set.contains(path) {
                true
            } else {
                removed.push(*id);
                false
            }
        });

        (new_sessions, removed)
    }

    pub fn get_id(&self, path: &Path) -> Option<u32> {
        self.known.get(path).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_assigns_incremental_ids() {
        let mut tracker = SessionTracker::new();
        let paths = vec![
            PathBuf::from("/tmp/a.jsonl"),
            PathBuf::from("/tmp/b.jsonl"),
        ];
        let (new, removed) = tracker.update(&paths);
        assert_eq!(new.len(), 2);
        assert_eq!(new[0].0, 1);
        assert_eq!(new[1].0, 2);
        assert!(removed.is_empty());
    }

    #[test]
    fn tracker_detects_removed_sessions() {
        let mut tracker = SessionTracker::new();
        let paths = vec![
            PathBuf::from("/tmp/a.jsonl"),
            PathBuf::from("/tmp/b.jsonl"),
        ];
        tracker.update(&paths);

        let (new, removed) = tracker.update(&[PathBuf::from("/tmp/a.jsonl")]);
        assert!(new.is_empty());
        assert_eq!(removed, vec![2]);
    }

    #[test]
    fn tracker_detects_new_sessions() {
        let mut tracker = SessionTracker::new();
        tracker.update(&[PathBuf::from("/tmp/a.jsonl")]);

        let (new, _) = tracker.update(&[
            PathBuf::from("/tmp/a.jsonl"),
            PathBuf::from("/tmp/c.jsonl"),
        ]);
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].0, 2); // Next ID after 1
    }

    #[test]
    fn tracker_get_id() {
        let mut tracker = SessionTracker::new();
        let paths = vec![PathBuf::from("/tmp/a.jsonl")];
        tracker.update(&paths);
        assert_eq!(tracker.get_id(&PathBuf::from("/tmp/a.jsonl")), Some(1));
        assert_eq!(tracker.get_id(&PathBuf::from("/tmp/nope.jsonl")), None);
    }

    #[test]
    fn scan_sessions_handles_missing_dir() {
        let sessions = scan_sessions(Path::new("/nonexistent/path"));
        assert!(sessions.is_empty());
    }
}
```

**Step 2: Update mod.rs and run tests**

```rust
// src/watcher/mod.rs
pub mod types;
pub mod parser;
pub mod discovery;
```

Run: `cargo test --lib watcher::discovery`
Expected: All 5 tests PASS

**Step 3: Commit**

```bash
git add src/watcher/
git commit -m "feat: add session discovery with incremental tracking"
```

---

### Task 7: File Watcher

**Files:**
- Create: `src/watcher/file_watcher.rs`
- Modify: `src/watcher/mod.rs`

**Step 1: Write file reader with offset tracking and tests**

```rust
// src/watcher/file_watcher.rs
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::watcher::parser;
use crate::watcher::types::JsonlRecord;

/// Tracks read offsets per file and returns only new lines.
pub struct IncrementalReader {
    offsets: HashMap<PathBuf, u64>,
}

impl IncrementalReader {
    pub fn new() -> Self {
        Self {
            offsets: HashMap::new(),
        }
    }

    /// Read new lines from a JSONL file since last read.
    /// Returns parsed records.
    pub fn read_new_lines(&mut self, path: &Path) -> Vec<JsonlRecord> {
        let offset = self.offsets.get(path).copied().unwrap_or(0);

        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let metadata = match file.metadata() {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        // If file is smaller than offset, it was truncated/rotated
        let current_offset = if metadata.len() < offset { 0 } else { offset };

        let mut reader = BufReader::new(file);
        if reader.seek(SeekFrom::Start(current_offset)).is_err() {
            return Vec::new();
        }

        let mut records = Vec::new();
        let mut new_offset = current_offset;

        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    new_offset += n as u64;
                    if let Some(record) = parser::parse_line(&line) {
                        records.push(record);
                    }
                }
                Err(_) => break,
            }
        }

        self.offsets.insert(path.to_path_buf(), new_offset);
        records
    }

    /// Remove tracking for a file.
    pub fn remove(&mut self, path: &Path) {
        self.offsets.remove(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn reads_new_lines_incrementally() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.jsonl");

        // Write initial content
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"hello"}}]}}}}"#).unwrap();
        }

        let mut reader = IncrementalReader::new();

        // First read gets 1 record
        let records = reader.read_new_lines(&file_path);
        assert_eq!(records.len(), 1);

        // Second read without changes gets 0
        let records = reader.read_new_lines(&file_path);
        assert_eq!(records.len(), 0);

        // Append new content
        {
            let mut f = std::fs::OpenOptions::new().append(true).open(&file_path).unwrap();
            writeln!(f, r#"{{"type":"system","subtype":"turn_duration","duration_ms":100}}"#).unwrap();
        }

        // Third read gets only new record
        let records = reader.read_new_lines(&file_path);
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0], JsonlRecord::System { .. }));
    }

    #[test]
    fn handles_missing_file() {
        let mut reader = IncrementalReader::new();
        let records = reader.read_new_lines(Path::new("/nonexistent/file.jsonl"));
        assert!(records.is_empty());
    }

    #[test]
    fn handles_file_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.jsonl");

        // Write and read
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"first"}}]}}}}"#).unwrap();
            writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"second"}}]}}}}"#).unwrap();
        }
        let mut reader = IncrementalReader::new();
        reader.read_new_lines(&file_path);

        // Truncate file (simulates /clear creating new file)
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"new"}}]}}}}"#).unwrap();
        }

        // Should re-read from beginning
        let records = reader.read_new_lines(&file_path);
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn remove_clears_tracking() {
        let mut reader = IncrementalReader::new();
        let path = PathBuf::from("/tmp/test.jsonl");
        reader.offsets.insert(path.clone(), 100);
        reader.remove(&path);
        assert!(!reader.offsets.contains_key(&path));
    }
}
```

**Step 2: Update mod.rs and run tests**

```rust
// src/watcher/mod.rs
pub mod types;
pub mod parser;
pub mod discovery;
pub mod file_watcher;
```

Run: `cargo test --lib watcher::file_watcher`
Expected: All 4 tests PASS

**Step 3: Commit**

```bash
git add src/watcher/
git commit -m "feat: add incremental JSONL file reader with offset tracking"
```

---

## Phase 3: Terminal Integration

### Task 8: Terminal Detection

**Files:**
- Create: `src/terminal/mod.rs`
- Create: `src/terminal/detect.rs`
- Modify: `src/lib.rs`

**Step 1: Write terminal detection with tests**

```rust
// src/terminal/detect.rs
use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKind {
    Zellij,
    WezTerm,
    Kitty,
    Tmux,
    Unknown,
}

impl TerminalKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Zellij => "Zellij",
            Self::WezTerm => "WezTerm",
            Self::Kitty => "Kitty",
            Self::Tmux => "tmux",
            Self::Unknown => "Unknown",
        }
    }
}

/// Detect the current terminal/multiplexer from environment variables.
/// Priority: Zellij > WezTerm > Kitty > tmux > Unknown
pub fn detect_terminal() -> TerminalKind {
    if env::var("ZELLIJ").is_ok() || env::var("ZELLIJ_SESSION_NAME").is_ok() {
        return TerminalKind::Zellij;
    }
    if env::var("WEZTERM_PANE").is_ok() || env::var("WEZTERM_EXECUTABLE").is_ok() {
        return TerminalKind::WezTerm;
    }
    if env::var("KITTY_PID").is_ok() || env::var("KITTY_WINDOW_ID").is_ok() {
        return TerminalKind::Kitty;
    }
    if env::var("TMUX").is_ok() {
        return TerminalKind::Tmux;
    }
    TerminalKind::Unknown
}

/// Build the split command for the detected terminal.
/// Returns the command and args to execute pixel-agents-tui --attach in a split pane.
pub fn build_split_command(kind: TerminalKind, binary_path: &str) -> Option<SplitCommand> {
    let attach_cmd = format!("{binary_path}");
    match kind {
        TerminalKind::WezTerm => Some(SplitCommand {
            program: "wezterm".to_string(),
            args: vec![
                "cli".to_string(),
                "split-pane".to_string(),
                "--right".to_string(),
                "--percent".to_string(),
                "35".to_string(),
                "--".to_string(),
                attach_cmd,
                "--attach".to_string(),
            ],
        }),
        TerminalKind::Zellij => Some(SplitCommand {
            program: "zellij".to_string(),
            args: vec![
                "action".to_string(),
                "new-pane".to_string(),
                "--direction".to_string(),
                "right".to_string(),
                "--".to_string(),
                attach_cmd,
                "--attach".to_string(),
            ],
        }),
        TerminalKind::Tmux => Some(SplitCommand {
            program: "tmux".to_string(),
            args: vec![
                "split-window".to_string(),
                "-h".to_string(),
                "-l".to_string(),
                "35%".to_string(),
                format!("{attach_cmd} --attach"),
            ],
        }),
        TerminalKind::Kitty => Some(SplitCommand {
            program: "kitty".to_string(),
            args: vec![
                "@".to_string(),
                "launch".to_string(),
                "--location=vsplit".to_string(),
                attach_cmd,
                "--attach".to_string(),
            ],
        }),
        TerminalKind::Unknown => None,
    }
}

/// Build a fallback command to open a new terminal tab.
pub fn build_fallback_command(binary_path: &str) -> SplitCommand {
    if cfg!(target_os = "macos") {
        SplitCommand {
            program: "open".to_string(),
            args: vec![
                "-a".to_string(),
                "Terminal".to_string(),
                binary_path.to_string(),
                "--args".to_string(),
                "--attach".to_string(),
            ],
        }
    } else {
        // Linux fallback: try xterm
        SplitCommand {
            program: "xterm".to_string(),
            args: vec![
                "-e".to_string(),
                format!("{binary_path} --attach"),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct SplitCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_split_wezterm() {
        let cmd = build_split_command(TerminalKind::WezTerm, "/usr/bin/pixel-agents-tui");
        assert!(cmd.is_some());
        let cmd = cmd.unwrap();
        assert_eq!(cmd.program, "wezterm");
        assert!(cmd.args.contains(&"split-pane".to_string()));
        assert!(cmd.args.contains(&"--attach".to_string()));
    }

    #[test]
    fn build_split_zellij() {
        let cmd = build_split_command(TerminalKind::Zellij, "/usr/bin/pixel-agents-tui");
        assert!(cmd.is_some());
        let cmd = cmd.unwrap();
        assert_eq!(cmd.program, "zellij");
        assert!(cmd.args.contains(&"new-pane".to_string()));
    }

    #[test]
    fn build_split_tmux() {
        let cmd = build_split_command(TerminalKind::Tmux, "/usr/bin/pixel-agents-tui");
        assert!(cmd.is_some());
        let cmd = cmd.unwrap();
        assert_eq!(cmd.program, "tmux");
        assert!(cmd.args.contains(&"split-window".to_string()));
    }

    #[test]
    fn build_split_kitty() {
        let cmd = build_split_command(TerminalKind::Kitty, "/usr/bin/pixel-agents-tui");
        assert!(cmd.is_some());
        let cmd = cmd.unwrap();
        assert_eq!(cmd.program, "kitty");
    }

    #[test]
    fn unknown_terminal_returns_none() {
        let cmd = build_split_command(TerminalKind::Unknown, "/usr/bin/pixel-agents-tui");
        assert!(cmd.is_none());
    }

    #[test]
    fn fallback_provides_command() {
        let cmd = build_fallback_command("/usr/bin/pixel-agents-tui");
        assert!(!cmd.program.is_empty());
    }
}
```

```rust
// src/terminal/mod.rs
pub mod detect;
```

```rust
// src/lib.rs
pub mod state;
pub mod watcher;
pub mod terminal;
```

**Step 2: Run tests**

Run: `cargo test --lib terminal::detect`
Expected: All 6 tests PASS

**Step 3: Commit**

```bash
git add src/terminal/ src/lib.rs
git commit -m "feat: add terminal detection and split command builders"
```

---

## Phase 4: TUI Rendering

### Task 9: ASCII Sprites

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/sprites.rs`
- Modify: `src/lib.rs`

**Step 1: Define ASCII character sprites with tests**

```rust
// src/ui/sprites.rs
use ratatui::style::Color;

/// Animation state for a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimState {
    Idle,
    Typing,
    Reading,
    Walking,
}

/// Character sprite frames (3 lines each).
/// Each frame is a [&str; 3] representing top, middle, bottom rows.
pub fn sprite_frame(state: AnimState, frame: usize) -> [&'static str; 3] {
    match state {
        AnimState::Idle => IDLE_FRAMES[frame % IDLE_FRAMES.len()],
        AnimState::Typing => TYPING_FRAMES[frame % TYPING_FRAMES.len()],
        AnimState::Reading => READING_FRAMES[frame % READING_FRAMES.len()],
        AnimState::Walking => WALKING_FRAMES[frame % WALKING_FRAMES.len()],
    }
}

const IDLE_FRAMES: &[[&str; 3]] = &[
    [" ◉ ", "╔║╗", "╚╩╝"],
    [" ◉ ", "╔║╗", " ║ "],
];

const TYPING_FRAMES: &[[&str; 3]] = &[
    [" ◉ ", "╔║╗", "╚╩╝"],
    [" ◉ ", "╔║~", "╚╩╝"],
    [" ◉ ", "~║╗", "╚╩╝"],
];

const READING_FRAMES: &[[&str; 3]] = &[
    [" ◉ ", "╔║▐", "╚╩╝"],
    [" ◉ ", "╔║▐", "╚╩╝"],
];

const WALKING_FRAMES: &[[&str; 3]] = &[
    [" ◉ ", "╔║╗", "╝ ╚"],
    [" ◉ ", "╔║╗", "╚ ╝"],
];

/// Desk ASCII art (5 chars wide, 2 lines tall).
pub const DESK: [&str; 2] = ["╔═══╗", "╚═══╝"];

/// Agent colors by index (cycles for >6 agents).
pub const AGENT_COLORS: &[Color] = &[
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Green,
    Color::Blue,
    Color::Red,
];

pub fn agent_color(id: u32) -> Color {
    AGENT_COLORS[(id as usize - 1) % AGENT_COLORS.len()]
}

pub fn sub_agent_color(parent_id: u32) -> Color {
    // Dim version using indexed colors
    match agent_color(parent_id) {
        Color::Cyan => Color::DarkGray,
        Color::Magenta => Color::DarkGray,
        Color::Yellow => Color::DarkGray,
        Color::Green => Color::DarkGray,
        Color::Blue => Color::DarkGray,
        Color::Red => Color::DarkGray,
        c => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_frames_have_3_lines() {
        for state in [AnimState::Idle, AnimState::Typing, AnimState::Reading, AnimState::Walking] {
            let frame = sprite_frame(state, 0);
            assert_eq!(frame.len(), 3);
        }
    }

    #[test]
    fn sprite_frames_cycle() {
        let f0 = sprite_frame(AnimState::Typing, 0);
        let f3 = sprite_frame(AnimState::Typing, 3);
        assert_eq!(f0, f3); // 3 typing frames, so frame 3 == frame 0
    }

    #[test]
    fn agent_colors_cycle() {
        assert_eq!(agent_color(1), Color::Cyan);
        assert_eq!(agent_color(7), Color::Cyan); // Cycles back
    }

    #[test]
    fn desk_has_correct_dimensions() {
        assert_eq!(DESK.len(), 2);
        assert_eq!(DESK[0].chars().count(), 5);
    }
}
```

```rust
// src/ui/mod.rs
pub mod sprites;
```

Update `src/lib.rs` to add `pub mod ui;`

**Step 2: Run tests**

Run: `cargo test --lib ui::sprites`
Expected: All 4 tests PASS

**Step 3: Commit**

```bash
git add src/ui/ src/lib.rs
git commit -m "feat: add ASCII sprite definitions for agent characters"
```

---

### Task 10: App State

**Files:**
- Create: `src/app.rs`
- Modify: `src/lib.rs`

**Step 1: Write the central app state**

```rust
// src/app.rs
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crate::state::agent::AgentState;
use crate::ui::sprites::AnimState;
use crate::watcher::discovery::SessionTracker;
use crate::watcher::file_watcher::IncrementalReader;
use crate::watcher::parser;

pub struct App {
    pub agents: HashMap<u32, AgentState>,
    pub selected_agent: Option<u32>,
    pub session_tracker: SessionTracker,
    pub reader: IncrementalReader,
    pub claude_dir: PathBuf,
    pub should_quit: bool,
    pub tick_count: u64,
    pub focus: PanelFocus,
    pub sidebar_scroll: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    Office,
    Sidebar,
}

impl App {
    pub fn new(claude_dir: PathBuf) -> Self {
        Self {
            agents: HashMap::new(),
            selected_agent: None,
            session_tracker: SessionTracker::new(),
            reader: IncrementalReader::new(),
            claude_dir,
            should_quit: false,
            tick_count: 0,
            focus: PanelFocus::Sidebar,
            sidebar_scroll: 0,
        }
    }

    /// Process a tick: scan for sessions, read new JSONL lines, update agents.
    pub fn tick(&mut self) {
        self.tick_count += 1;

        // Scan for sessions every ~2 seconds (20 ticks at 10fps)
        if self.tick_count % 20 == 0 {
            let sessions = crate::watcher::discovery::scan_sessions(&self.claude_dir);
            let (new, removed) = self.session_tracker.update(&sessions);

            for (id, path) in new {
                self.agents.insert(id, AgentState::new(id, path));
                if self.selected_agent.is_none() {
                    self.selected_agent = Some(id);
                }
            }

            for id in removed {
                self.agents.remove(&id);
                if self.selected_agent == Some(id) {
                    self.selected_agent = self.agents.keys().next().copied();
                }
            }
        }

        // Read new lines from all active sessions
        let paths: Vec<(u32, PathBuf)> = self.agents.iter()
            .map(|(id, a)| (*id, a.session_file.clone()))
            .collect();

        for (id, path) in paths {
            let records = self.reader.read_new_lines(&path);
            for record in &records {
                if let Some(agent) = self.agents.get_mut(&id) {
                    // Extract tool uses
                    let tool_uses = parser::extract_tool_uses(record);
                    for tool in tool_uses {
                        agent.add_tool(tool);
                    }

                    // Extract tool results
                    let results = parser::extract_tool_results(record);
                    for tool_id in results {
                        agent.remove_tool(&tool_id);
                    }

                    // Extract text for prompt summary
                    if let Some(text) = parser::extract_text(record) {
                        agent.set_prompt_summary(&text);
                    }

                    // Check for turn end
                    if parser::is_turn_end(record) {
                        agent.mark_waiting();
                    }
                }
            }
        }

        // Check for dormant agents
        let dormant_ids: Vec<u32> = self.agents.iter()
            .filter(|(_, a)| a.is_dormant(300))
            .map(|(id, _)| *id)
            .collect();

        for id in dormant_ids {
            if let Some(agent) = self.agents.get_mut(&id) {
                agent.status = crate::state::agent::AgentStatus::Dormant;
            }
        }
    }

    pub fn select_agent(&mut self, num: u32) {
        if self.agents.contains_key(&num) {
            self.selected_agent = Some(num);
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            PanelFocus::Office => PanelFocus::Sidebar,
            PanelFocus::Sidebar => PanelFocus::Office,
        };
    }

    pub fn scroll_up(&mut self) {
        self.sidebar_scroll = self.sidebar_scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.sidebar_scroll = self.sidebar_scroll.saturating_add(1);
    }

    /// Get sorted agent IDs for consistent display order.
    pub fn sorted_agent_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.agents.keys().copied().collect();
        ids.sort();
        ids
    }

    /// Get the animation state for an agent based on its current tools.
    pub fn agent_anim_state(&self, id: u32) -> AnimState {
        if let Some(agent) = self.agents.get(&id) {
            if agent.active_tools.is_empty() {
                return AnimState::Idle;
            }
            if agent.active_tools.iter().any(|t| t.is_reading) {
                AnimState::Reading
            } else {
                AnimState::Typing
            }
        } else {
            AnimState::Idle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_app_has_empty_agents() {
        let app = App::new(PathBuf::from("/tmp"));
        assert!(app.agents.is_empty());
        assert!(app.selected_agent.is_none());
        assert!(!app.should_quit);
    }

    #[test]
    fn toggle_focus() {
        let mut app = App::new(PathBuf::from("/tmp"));
        assert_eq!(app.focus, PanelFocus::Sidebar);
        app.toggle_focus();
        assert_eq!(app.focus, PanelFocus::Office);
        app.toggle_focus();
        assert_eq!(app.focus, PanelFocus::Sidebar);
    }

    #[test]
    fn scroll_bounds() {
        let mut app = App::new(PathBuf::from("/tmp"));
        app.scroll_up(); // Should not go below 0
        assert_eq!(app.sidebar_scroll, 0);
        app.scroll_down();
        assert_eq!(app.sidebar_scroll, 1);
    }

    #[test]
    fn sorted_agent_ids() {
        let mut app = App::new(PathBuf::from("/tmp"));
        app.agents.insert(3, AgentState::new(3, PathBuf::from("/tmp/3.jsonl")));
        app.agents.insert(1, AgentState::new(1, PathBuf::from("/tmp/1.jsonl")));
        assert_eq!(app.sorted_agent_ids(), vec![1, 3]);
    }
}
```

Update `src/lib.rs` to add `pub mod app;`

**Step 2: Run tests**

Run: `cargo test --lib app`
Expected: All 4 tests PASS

**Step 3: Commit**

```bash
git add src/app.rs src/lib.rs
git commit -m "feat: add central app state with tick-based update loop"
```

---

### Task 11: TUI Layout

**Files:**
- Create: `src/ui/layout.rs`
- Modify: `src/ui/mod.rs`

**Step 1: Write the layout rendering function**

```rust
// src/ui/layout.rs
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, PanelFocus};
use crate::state::agent::AgentStatus;
use crate::ui::sprites;

/// Render the full TUI layout.
pub fn render(frame: &mut Frame, app: &App) {
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_header(frame, app, header_area);

    let horizontal = Layout::horizontal([
        Constraint::Percentage(60),
        Constraint::Percentage(40),
    ]);
    let [office_area, sidebar_area] = body_area.layout(&horizontal);

    render_office(frame, app, office_area);
    render_sidebar(frame, app, sidebar_area);
    render_footer(frame, app, footer_area);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let agent_count = app.agents.len();

    let sdd_info = app.selected_agent
        .and_then(|id| app.agents.get(&id))
        .and_then(|a| a.sdd_phase)
        .map(|p| format!("SDD: {} ({}/{})", p.label(), p.index() + 1, crate::state::sdd::SddPhase::total()))
        .unwrap_or_default();

    let title = Line::from(vec![
        Span::styled(" ◉ Pixel Agents TUI ", Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(format!("  {agent_count} agents")),
        Span::raw("  │  "),
        Span::styled(sdd_info, Style::new().fg(Color::Yellow)),
    ]);

    let header = Paragraph::new(title)
        .block(Block::default().borders(Borders::ALL).border_style(Style::new().fg(Color::DarkGray)));
    frame.render_widget(header, area);
}

fn render_office(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == PanelFocus::Office {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Office ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Render desks and agents in the office
    let agent_ids = app.sorted_agent_ids();
    let cols = 3;
    let desk_width = 7_u16; // 5 desk + 1 spacing each side
    let desk_height = 5_u16; // 2 desk + 3 character

    for (idx, &id) in agent_ids.iter().enumerate() {
        let col = (idx % cols) as u16;
        let row = (idx / cols) as u16;

        let x = inner.x + col * (desk_width + 2) + 1;
        let y = inner.y + row * (desk_height + 1);

        if x + desk_width > inner.x + inner.width || y + desk_height > inner.y + inner.height {
            break; // Out of bounds
        }

        let color = sprites::agent_color(id);
        let anim = app.agent_anim_state(id);
        let frame_idx = (app.tick_count / 3) as usize; // Change frame every ~300ms at 10fps
        let sprite = sprites::sprite_frame(anim, frame_idx);

        // Draw desk
        let desk_area = Rect::new(x, y, 5, 2);
        let desk = Paragraph::new(vec![
            Line::from(sprites::DESK[0]),
            Line::from(sprites::DESK[1]),
        ])
        .style(Style::new().fg(Color::DarkGray));
        frame.render_widget(desk, desk_area);

        // Draw character below desk
        let char_area = Rect::new(x + 1, y + 2, 4, 3);
        let character = Paragraph::new(vec![
            Line::from(Span::styled(sprite[0], Style::new().fg(color))),
            Line::from(Span::styled(sprite[1], Style::new().fg(color))),
            Line::from(Span::styled(sprite[2], Style::new().fg(color))),
        ]);
        frame.render_widget(character, char_area);

        // Draw agent label
        if let Some(agent) = app.agents.get(&id) {
            let status_style = match agent.status {
                AgentStatus::Active => Style::new().fg(Color::Green),
                AgentStatus::Waiting => Style::new().fg(Color::Yellow),
                AgentStatus::Dormant => Style::new().fg(Color::DarkGray),
            };
            let label = format!("#{id}");
            if y + 5 < inner.y + inner.height {
                let label_area = Rect::new(x + 1, y + 5, label.len() as u16, 1);
                frame.render_widget(
                    Paragraph::new(Span::styled(label, status_style)),
                    label_area,
                );
            }
        }

        // Draw sub-agents (small markers near parent)
        if let Some(agent) = app.agents.get(&id) {
            for (si, sub) in agent.sub_agents.iter().enumerate() {
                let sx = x + desk_width + 1;
                let sy = y + 2 + si as u16;
                if sx + 3 <= inner.x + inner.width && sy < inner.y + inner.height {
                    let sub_area = Rect::new(sx, sy, 3, 1);
                    frame.render_widget(
                        Paragraph::new(Span::styled("◇", Style::new().fg(sprites::sub_agent_color(id)))),
                        sub_area,
                    );
                }
            }
        }
    }
}

fn render_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == PanelFocus::Sidebar {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Agent Details ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    for &id in &app.sorted_agent_ids() {
        if let Some(agent) = app.agents.get(&id) {
            let is_selected = app.selected_agent == Some(id);
            let indicator = if is_selected { "▸" } else { " " };
            let color = sprites::agent_color(id);

            let status_color = match agent.status {
                AgentStatus::Active => Color::Green,
                AgentStatus::Waiting => Color::Yellow,
                AgentStatus::Dormant => Color::DarkGray,
            };

            // Agent header line
            lines.push(Line::from(vec![
                Span::styled(format!("{indicator} Agent #{id} "), Style::new().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("[{} {}]", agent.status.symbol(), agent.status.label()),
                    Style::new().fg(status_color),
                ),
            ]));

            if is_selected {
                // Tool info
                if let Some(tool) = agent.current_tool_display() {
                    lines.push(Line::from(vec![
                        Span::raw("  Tool: "),
                        Span::styled(tool, Style::new().fg(Color::White)),
                    ]));
                }

                // Prompt summary
                if !agent.prompt_summary.is_empty() {
                    let summary = if agent.prompt_summary.len() > 50 {
                        format!("{}...", &agent.prompt_summary[..50])
                    } else {
                        agent.prompt_summary.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::raw("  Prompt: "),
                        Span::styled(format!("\"{summary}\""), Style::new().fg(Color::DarkGray)),
                    ]));
                }

                // SDD phase
                if let Some(phase) = agent.sdd_phase {
                    lines.push(Line::from(vec![
                        Span::raw("  SDD: "),
                        Span::styled(
                            format!("{} ({}/{})", phase.label(), phase.index() + 1, crate::state::sdd::SddPhase::total()),
                            Style::new().fg(Color::Yellow),
                        ),
                    ]));
                }

                // Sub-agents
                if !agent.sub_agents.is_empty() {
                    lines.push(Line::from(Span::raw("  Sub-agents:")));
                    for sub in &agent.sub_agents {
                        let sub_tool = sub.active_tools.last()
                            .map(|t| t.display_status.as_str())
                            .unwrap_or("idle");
                        lines.push(Line::from(vec![
                            Span::raw("  └─ "),
                            Span::styled(&sub.agent_type, Style::new().fg(Color::DarkGray)),
                            Span::raw(format!(": {sub_tool}")),
                        ]));
                    }
                }
            }

            lines.push(Line::from("")); // Spacing between agents
        }
    }

    let paragraph = Paragraph::new(lines)
        .scroll((app.sidebar_scroll, 0));
    frame.render_widget(paragraph, inner);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let fps = 10;
    let footer = Line::from(vec![
        Span::styled(" [q]", Style::new().fg(Color::Cyan)),
        Span::raw("uit  "),
        Span::styled("[1-9]", Style::new().fg(Color::Cyan)),
        Span::raw("select  "),
        Span::styled("[Tab]", Style::new().fg(Color::Cyan)),
        Span::raw("focus  "),
        Span::styled("[↑↓]", Style::new().fg(Color::Cyan)),
        Span::raw("scroll  "),
        Span::styled("[r]", Style::new().fg(Color::Cyan)),
        Span::raw("efresh"),
        Span::raw(format!("  {:>30}{fps} FPS", "")),
    ]);

    frame.render_widget(Paragraph::new(footer), area);
}
```

```rust
// src/ui/mod.rs
pub mod sprites;
pub mod layout;
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/ui/
git commit -m "feat: add Ratatui layout with office view, sidebar, header, and footer"
```

---

### Task 12: Main Entry Point with CLI and Event Loop

**Files:**
- Modify: `src/main.rs`

**Step 1: Write the main entry point with clap args and event loop**

```rust
// src/main.rs
use std::io;
use std::process::Command;
use std::time::Duration;

use clap::Parser;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};

use pixel_agents_tui::app::App;
use pixel_agents_tui::terminal::detect;
use pixel_agents_tui::ui::layout;

#[derive(Parser, Debug)]
#[command(name = "pixel-agents-tui", about = "TUI dashboard for Claude Code agents")]
struct Cli {
    /// Run in attach mode (render the TUI). Used by the launcher.
    #[arg(long)]
    attach: bool,

    /// Run from Claude Code SessionStart hook.
    #[arg(long)]
    session_hook: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    if cli.session_hook || (!cli.attach && !cli.session_hook) {
        // Launcher mode: detect terminal and create split
        return launch_split();
    }

    // Attach mode: run the TUI
    run_tui()
}

fn launch_split() -> io::Result<()> {
    // Check if already running via PID file
    let pid_file = std::env::temp_dir().join("pixel-agents-tui.pid");
    if pid_file.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // Check if process is still running
                let status = Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .status();
                if status.map(|s| s.success()).unwrap_or(false) {
                    // Already running
                    return Ok(());
                }
            }
        }
    }

    let binary = std::env::current_exe()
        .unwrap_or_else(|_| "pixel-agents-tui".into());
    let binary_str = binary.to_string_lossy().to_string();

    let terminal = detect::detect_terminal();
    let cmd = detect::build_split_command(terminal, &binary_str)
        .unwrap_or_else(|| detect::build_fallback_command(&binary_str));

    Command::new(&cmd.program)
        .args(&cmd.args)
        .spawn()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to launch split: {e}")))?;

    Ok(())
}

fn run_tui() -> io::Result<()> {
    // Write PID file
    let pid_file = std::env::temp_dir().join("pixel-agents-tui.pid");
    std::fs::write(&pid_file, std::process::id().to_string())?;

    let claude_dir = directories::BaseDirs::new()
        .map(|d| d.home_dir().join(".claude"))
        .unwrap_or_else(|| std::path::PathBuf::from("~/.claude"));

    let mut terminal = ratatui::init();
    let mut app = App::new(claude_dir);

    let tick_rate = Duration::from_millis(100); // ~10 FPS

    loop {
        terminal.draw(|frame| layout::render(frame, &app))?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Tab => app.toggle_focus(),
                    KeyCode::Up => app.scroll_up(),
                    KeyCode::Down => app.scroll_down(),
                    KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                        app.select_agent(c.to_digit(10).unwrap());
                    }
                    KeyCode::Char('r') => {
                        // Force refresh on next tick
                        app.tick_count = 0;
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }

        app.tick();
    }

    ratatui::restore();

    // Cleanup PID file
    let _ = std::fs::remove_file(&pid_file);

    Ok(())
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: add CLI entry point with launcher/attach modes and event loop"
```

---

## Phase 5: Claude Code Plugin

### Task 13: Plugin Files

**Files:**
- Create: `plugin/.claude-plugin/plugin.json`
- Create: `plugin/hooks/hooks.json`

**Step 1: Create plugin manifest**

```json
// plugin/.claude-plugin/plugin.json
{
  "name": "pixel-agents-tui",
  "description": "TUI dashboard for visualizing Claude Code agents as pixel art characters with real-time tool activity, sub-agent trees, and SDD workflow tracking",
  "version": "0.1.0",
  "author": {
    "name": "Daniel Munoz"
  },
  "license": "MIT",
  "hooks": "./hooks/hooks.json"
}
```

**Step 2: Create hooks configuration**

```json
// plugin/hooks/hooks.json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup",
        "hooks": [
          {
            "type": "command",
            "command": "pixel-agents-tui --session-hook",
            "async": true
          }
        ]
      }
    ]
  }
}
```

**Step 3: Commit**

```bash
git add plugin/
git commit -m "feat: add Claude Code plugin with SessionStart hook"
```

---

## Phase 6: Integration & Polish

### Task 14: End-to-End Integration Test

**Files:**
- Create: `tests/integration.rs`

**Step 1: Write integration test that simulates JSONL activity**

```rust
// tests/integration.rs
use std::fs;
use std::io::Write;
use std::path::Path;

use pixel_agents_tui::app::App;
use pixel_agents_tui::state::agent::AgentStatus;
use pixel_agents_tui::state::sdd::SddPhase;

/// Helper to create a fake claude directory with a JSONL session.
fn setup_fake_claude_dir(dir: &Path) {
    let projects_dir = dir.join("projects").join("test-project");
    fs::create_dir_all(&projects_dir).unwrap();

    let jsonl_path = projects_dir.join("session-1.jsonl");
    let mut f = fs::File::create(&jsonl_path).unwrap();

    // Simulate a sequence of agent activity
    writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"Let me fix the auth bug"}}]}}}}"#).unwrap();
    writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t1","name":"Read","input":{{"file_path":"/src/auth.rs"}}}}]}}}}"#).unwrap();
    writeln!(f, r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","tool_use_id":"t1"}}]}}}}"#).unwrap();
    writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t2","name":"Skill","input":{{"skill":"sdd-apply"}}}}]}}}}"#).unwrap();
    writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t3","name":"Write","input":{{"file_path":"/src/auth.rs"}}}}]}}}}"#).unwrap();
    writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t4","name":"Task","input":{{"description":"Explore auth patterns","subagent_type":"Explore"}}}}]}}}}"#).unwrap();
    writeln!(f, r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","tool_use_id":"t3"}}]}}}}"#).unwrap();
    writeln!(f, r#"{{"type":"system","subtype":"turn_duration","duration_ms":5000}}"#).unwrap();
}

#[test]
fn full_lifecycle_simulation() {
    let dir = tempfile::tempdir().unwrap();
    setup_fake_claude_dir(dir.path());

    let mut app = App::new(dir.path().to_path_buf());

    // Force immediate session scan
    app.tick_count = 19; // Next tick will be 20, triggering scan
    app.tick();

    // Should have discovered 1 agent
    assert_eq!(app.agents.len(), 1);

    let agent_id = *app.agents.keys().next().unwrap();
    let agent = app.agents.get(&agent_id).unwrap();

    // Should have captured prompt summary
    assert!(agent.prompt_summary.contains("fix the auth bug"));

    // Should have detected SDD phase
    assert_eq!(agent.sdd_phase, Some(SddPhase::Apply));

    // Should have a sub-agent from Task tool
    assert_eq!(agent.sub_agents.len(), 1);
    assert!(agent.sub_agents[0].agent_type.contains("Explore"));

    // Turn ended → agent should be waiting
    assert_eq!(agent.status, AgentStatus::Waiting);

    // Active tools should be cleared after turn end
    assert!(agent.active_tools.is_empty());
}
```

**Step 2: Run integration test**

Run: `cargo test --test integration`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/
git commit -m "test: add end-to-end integration test with simulated JSONL activity"
```

---

### Task 15: Final Verification

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests PASS

**Step 2: Run clippy**

Run: `cargo clippy -- -W clippy::all`
Expected: No warnings (fix any that appear)

**Step 3: Run fmt check**

Run: `cargo fmt --check`
Expected: No formatting issues (run `cargo fmt` if needed)

**Step 4: Test the binary manually**

Run: `cargo run -- --attach`
Expected: TUI launches showing empty office (no active Claude sessions). Press `q` to quit.

**Step 5: Final commit if any fixes needed**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```

---

## Summary

| Phase | Tasks | What it builds |
|-------|-------|----------------|
| 1. Foundation | Tasks 1-5 | Scaffolding, JSONL types/parser, SDD detection, agent state machine |
| 2. File System | Tasks 6-7 | Session discovery, incremental file watcher |
| 3. Terminal | Task 8 | Terminal detection and split command builders |
| 4. TUI Rendering | Tasks 9-12 | ASCII sprites, app state, Ratatui layout, main event loop |
| 5. Plugin | Task 13 | Claude Code plugin with SessionStart hook |
| 6. Integration | Tasks 14-15 | End-to-end tests, clippy, fmt, manual verification |

**Total: 15 tasks across 6 phases.**

Each task follows TDD: write test → verify it fails → implement → verify it passes → commit.
