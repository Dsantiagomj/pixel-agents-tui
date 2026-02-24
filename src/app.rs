use std::collections::HashMap;
use std::path::PathBuf;

use crate::state::agent::{AgentState, AgentStatus};
use crate::ui::sprites::AnimState;
use crate::watcher::discovery::{scan_sessions, SessionTracker};
use crate::watcher::file_watcher::IncrementalReader;
use crate::watcher::parser;

const DORMANCY_TIMEOUT_SECS: u64 = 300;
const SESSION_SCAN_INTERVAL: u64 = 20;

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

    pub fn tick(&mut self) {
        self.tick_count += 1;

        // Every 20 ticks (~2s at 10fps): scan sessions, create/remove agents
        if self.tick_count % SESSION_SCAN_INTERVAL == 0 {
            let sessions = scan_sessions(&self.claude_dir);
            let (new_sessions, removed_ids) = self.session_tracker.update(&sessions);

            // Create agents for new sessions
            for (id, path) in new_sessions {
                self.agents.insert(id, AgentState::new(id, path));
            }

            // Remove agents for gone sessions
            for id in &removed_ids {
                self.agents.remove(id);
                self.reader.remove(
                    &self
                        .agents
                        .get(id)
                        .map(|a| a.session_file.clone())
                        .unwrap_or_default(),
                );
                // Deselect if the selected agent was removed
                if self.selected_agent == Some(*id) {
                    self.selected_agent = None;
                }
            }
        }

        // Every tick: read new JSONL lines for each agent and process them
        let agent_files: Vec<(u32, PathBuf)> = self
            .agents
            .iter()
            .map(|(&id, agent)| (id, agent.session_file.clone()))
            .collect();

        for (id, path) in agent_files {
            let records = self.reader.read_new_lines(&path);
            for record in &records {
                // Extract tool uses and add them to the agent
                let tool_uses = parser::extract_tool_uses(record);
                for tool in tool_uses {
                    if let Some(agent) = self.agents.get_mut(&id) {
                        agent.add_tool(tool);
                    }
                }

                // Extract tool results and remove completed tools
                let tool_results = parser::extract_tool_results(record);
                for tool_id in tool_results {
                    if let Some(agent) = self.agents.get_mut(&id) {
                        agent.remove_tool(&tool_id);
                    }
                }

                // Extract text for prompt summary
                if let Some(text) = parser::extract_text(record) {
                    if let Some(agent) = self.agents.get_mut(&id) {
                        agent.set_prompt_summary(&text);
                    }
                }

                // Check for turn end
                if parser::is_turn_end(record) {
                    if let Some(agent) = self.agents.get_mut(&id) {
                        agent.mark_waiting();
                    }
                }
            }
        }

        // Check for dormant agents (300s timeout)
        for agent in self.agents.values_mut() {
            if agent.status != AgentStatus::Dormant && agent.is_dormant(DORMANCY_TIMEOUT_SECS) {
                agent.status = AgentStatus::Dormant;
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

    pub fn sorted_agent_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.agents.keys().copied().collect();
        ids.sort();
        ids
    }

    pub fn agent_anim_state(&self, id: u32) -> AnimState {
        match self.agents.get(&id) {
            Some(agent) => {
                if agent.active_tools.is_empty() {
                    AnimState::Idle
                } else if agent.active_tools.iter().any(|t| t.is_reading) {
                    AnimState::Reading
                } else {
                    AnimState::Typing
                }
            }
            None => AnimState::Idle,
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
        app.scroll_up();
        assert_eq!(app.sidebar_scroll, 0);
        app.scroll_down();
        assert_eq!(app.sidebar_scroll, 1);
    }

    #[test]
    fn sorted_agent_ids() {
        let mut app = App::new(PathBuf::from("/tmp"));
        app.agents
            .insert(3, AgentState::new(3, PathBuf::from("/tmp/3.jsonl")));
        app.agents
            .insert(1, AgentState::new(1, PathBuf::from("/tmp/1.jsonl")));
        assert_eq!(app.sorted_agent_ids(), vec![1, 3]);
    }

    #[test]
    fn select_agent_existing() {
        let mut app = App::new(PathBuf::from("/tmp"));
        app.agents
            .insert(1, AgentState::new(1, PathBuf::from("/tmp/1.jsonl")));
        app.select_agent(1);
        assert_eq!(app.selected_agent, Some(1));
    }

    #[test]
    fn select_agent_nonexistent() {
        let mut app = App::new(PathBuf::from("/tmp"));
        app.select_agent(99);
        assert!(app.selected_agent.is_none());
    }

    #[test]
    fn agent_anim_state_idle_when_no_tools() {
        let mut app = App::new(PathBuf::from("/tmp"));
        app.agents
            .insert(1, AgentState::new(1, PathBuf::from("/tmp/1.jsonl")));
        assert_eq!(app.agent_anim_state(1), AnimState::Idle);
    }

    #[test]
    fn agent_anim_state_reading() {
        let mut app = App::new(PathBuf::from("/tmp"));
        let mut agent = AgentState::new(1, PathBuf::from("/tmp/1.jsonl"));
        agent.add_tool(parser::ToolUseEvent {
            tool_id: "t1".to_string(),
            tool_name: "Read".to_string(),
            display_status: "Reading foo.rs".to_string(),
            is_reading: true,
        });
        app.agents.insert(1, agent);
        assert_eq!(app.agent_anim_state(1), AnimState::Reading);
    }

    #[test]
    fn agent_anim_state_typing() {
        let mut app = App::new(PathBuf::from("/tmp"));
        let mut agent = AgentState::new(1, PathBuf::from("/tmp/1.jsonl"));
        agent.add_tool(parser::ToolUseEvent {
            tool_id: "t1".to_string(),
            tool_name: "Write".to_string(),
            display_status: "Writing foo.rs".to_string(),
            is_reading: false,
        });
        app.agents.insert(1, agent);
        assert_eq!(app.agent_anim_state(1), AnimState::Typing);
    }

    #[test]
    fn agent_anim_state_missing_agent() {
        let app = App::new(PathBuf::from("/tmp"));
        assert_eq!(app.agent_anim_state(999), AnimState::Idle);
    }
}
