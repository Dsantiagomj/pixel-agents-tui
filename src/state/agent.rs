use std::path::PathBuf;
use std::time::Instant;

use crate::state::sdd::{detect_sdd_phase, SddPhase};
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
            AgentStatus::Active => "active",
            AgentStatus::Waiting => "waiting",
            AgentStatus::Dormant => "dormant",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            AgentStatus::Active => "●",
            AgentStatus::Waiting => "○",
            AgentStatus::Dormant => "◌",
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
        self.status = AgentStatus::Active;
        self.last_activity = Instant::now();

        // Check for SDD phase from Skill tools
        if let Some(phase) = detect_sdd_phase(&tool) {
            self.sdd_phase = Some(phase);
        }

        // Spawn a sub-agent for Task tools
        if tool.tool_name == "Task" {
            let sub_agent = SubAgent {
                id: -1,
                parent_tool_id: tool.tool_id.clone(),
                agent_type: "task".to_string(),
                active_tools: Vec::new(),
            };
            self.sub_agents.push(sub_agent);
        }

        self.active_tools.push(tool);
    }

    pub fn remove_tool(&mut self, tool_id: &str) {
        self.active_tools.retain(|t| t.tool_id != tool_id);
        self.sub_agents.retain(|s| s.parent_tool_id != tool_id);
        self.last_activity = Instant::now();
    }

    pub fn mark_waiting(&mut self) {
        self.status = AgentStatus::Waiting;
        self.active_tools.clear();
        self.sub_agents.clear();
        self.last_activity = Instant::now();
    }

    pub fn set_prompt_summary(&mut self, text: &str) {
        if !self.prompt_summary.is_empty() {
            return;
        }
        let chars: String = text.chars().take(150).collect();
        self.prompt_summary = chars;
    }

    pub fn is_dormant(&self, timeout_secs: u64) -> bool {
        self.last_activity.elapsed().as_secs() >= timeout_secs
    }

    pub fn current_tool_display(&self) -> Option<&str> {
        self.active_tools.last().map(|t| t.display_status.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
