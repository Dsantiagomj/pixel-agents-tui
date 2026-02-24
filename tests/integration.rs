use std::fs;
use std::io::Write;
use std::path::Path;

use pixel_agents_tui::app::App;
use pixel_agents_tui::state::agent::AgentStatus;
use pixel_agents_tui::state::sdd::SddPhase;

fn setup_fake_claude_dir(dir: &Path) {
    let projects_dir = dir.join("projects").join("test-project");
    fs::create_dir_all(&projects_dir).unwrap();
    let jsonl_path = projects_dir.join("session-1.jsonl");
    let mut f = fs::File::create(&jsonl_path).unwrap();

    // Simulate this sequence:
    // 1. Agent sends text: "Let me fix the auth bug"
    writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"Let me fix the auth bug"}}]}}}}"#).unwrap();
    // 2. Agent reads a file
    writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t1","name":"Read","input":{{"file_path":"/src/auth.rs"}}}}]}}}}"#).unwrap();
    // 3. Tool completes
    writeln!(f, r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","tool_use_id":"t1"}}]}}}}"#).unwrap();
    // 4. Agent invokes SDD skill
    writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t2","name":"Skill","input":{{"skill":"sdd-apply"}}}}]}}}}"#).unwrap();
    // 5. Agent writes a file
    writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t3","name":"Write","input":{{"file_path":"/src/auth.rs"}}}}]}}}}"#).unwrap();
    // 6. Agent spawns a sub-agent via Task
    writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t4","name":"Task","input":{{"description":"Explore auth patterns","subagent_type":"Explore"}}}}]}}}}"#).unwrap();
    // 7. Write tool completes
    writeln!(f, r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","tool_use_id":"t3"}}]}}}}"#).unwrap();
    // 8. Turn ends
    writeln!(f, r#"{{"type":"system","subtype":"turn_duration","duration_ms":5000}}"#).unwrap();
}

#[test]
fn full_lifecycle_simulation() {
    let dir = tempfile::tempdir().unwrap();
    setup_fake_claude_dir(dir.path());

    let mut app = App::new(dir.path().to_path_buf());

    // Force immediate session scan (tick 19 → next tick will be 20)
    app.tick_count = 19;
    app.tick();

    assert_eq!(app.agents.len(), 1);

    let agent_id = *app.agents.keys().next().unwrap();
    let agent = app.agents.get(&agent_id).unwrap();

    // Should have captured prompt summary from the first text block
    assert!(
        agent.prompt_summary.contains("fix the auth bug"),
        "Expected prompt_summary to contain 'fix the auth bug', got: '{}'",
        agent.prompt_summary
    );

    // Should have detected SDD phase from the Skill tool
    assert_eq!(agent.sdd_phase, Some(SddPhase::Apply));

    // Turn ended via turn_duration → mark_waiting() clears active_tools and sub_agents
    assert_eq!(agent.status, AgentStatus::Waiting);
    assert!(agent.active_tools.is_empty());
    assert!(
        agent.sub_agents.is_empty(),
        "sub_agents should be cleared after turn end"
    );
}

/// Verify that mid-turn state (before turn_duration) preserves active tools and sub-agents.
#[test]
fn mid_turn_has_active_tools_and_sub_agents() {
    let dir = tempfile::tempdir().unwrap();
    let projects_dir = dir.path().join("projects").join("mid-turn-project");
    fs::create_dir_all(&projects_dir).unwrap();
    let jsonl_path = projects_dir.join("session-mid.jsonl");
    {
        let mut f = fs::File::create(&jsonl_path).unwrap();
        // 1. Text message
        writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"Investigating the issue"}}]}}}}"#).unwrap();
        // 2. SDD skill
        writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t1","name":"Skill","input":{{"skill":"sdd-explore"}}}}]}}}}"#).unwrap();
        // 3. Task spawns sub-agent
        writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t2","name":"Task","input":{{"description":"Explore auth patterns"}}}}]}}}}"#).unwrap();
        // 4. Write tool (still pending, no result yet)
        writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t3","name":"Write","input":{{"file_path":"/src/lib.rs"}}}}]}}}}"#).unwrap();
        // NO turn_duration — turn is still in progress
    }

    let mut app = App::new(dir.path().to_path_buf());
    app.tick_count = 19;
    app.tick();

    assert_eq!(app.agents.len(), 1);
    let agent_id = *app.agents.keys().next().unwrap();
    let agent = app.agents.get(&agent_id).unwrap();

    // Mid-turn: agent should be active
    assert_eq!(agent.status, AgentStatus::Active);

    // SDD phase detected
    assert_eq!(agent.sdd_phase, Some(SddPhase::Explore));

    // Sub-agent from Task tool should still be present
    assert_eq!(agent.sub_agents.len(), 1);
    assert_eq!(agent.sub_agents[0].agent_type, "task");

    // Active tools: t1 (Skill), t2 (Task), t3 (Write) — all still pending
    assert_eq!(agent.active_tools.len(), 3);

    // Prompt summary captured
    assert!(agent.prompt_summary.contains("Investigating the issue"));
}

/// Verify incremental reads: new lines appended after initial tick are picked up.
#[test]
fn incremental_read_picks_up_new_lines() {
    let dir = tempfile::tempdir().unwrap();
    let projects_dir = dir.path().join("projects").join("incr-project");
    fs::create_dir_all(&projects_dir).unwrap();
    let jsonl_path = projects_dir.join("session-incr.jsonl");

    // Write initial content
    {
        let mut f = fs::File::create(&jsonl_path).unwrap();
        writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"Starting work"}}]}}}}"#).unwrap();
    }

    let mut app = App::new(dir.path().to_path_buf());
    app.tick_count = 19;
    app.tick(); // tick 20: discovers session + reads first line

    assert_eq!(app.agents.len(), 1);
    let agent_id = *app.agents.keys().next().unwrap();
    assert!(app.agents.get(&agent_id).unwrap().prompt_summary.contains("Starting work"));

    // Append a tool use line
    {
        let mut f = fs::OpenOptions::new()
            .append(true)
            .open(&jsonl_path)
            .unwrap();
        writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t1","name":"Read","input":{{"file_path":"/src/main.rs"}}}}]}}}}"#).unwrap();
    }

    // Next tick reads the new line (no session scan needed, just incremental read)
    app.tick();

    let agent = app.agents.get(&agent_id).unwrap();
    assert_eq!(agent.status, AgentStatus::Active);
    assert_eq!(agent.active_tools.len(), 1);
    assert_eq!(agent.active_tools[0].tool_name, "Read");
    assert!(agent.active_tools[0].is_reading);
}
