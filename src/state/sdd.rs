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
            SddPhase::Explore => "Explore",
            SddPhase::Propose => "Propose",
            SddPhase::Spec => "Spec",
            SddPhase::Design => "Design",
            SddPhase::Tasks => "Tasks",
            SddPhase::Apply => "Apply",
            SddPhase::Verify => "Verify",
            SddPhase::Archive => "Archive",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            SddPhase::Explore => 0,
            SddPhase::Propose => 1,
            SddPhase::Spec => 2,
            SddPhase::Design => 3,
            SddPhase::Tasks => 4,
            SddPhase::Apply => 5,
            SddPhase::Verify => 6,
            SddPhase::Archive => 7,
        }
    }

    pub fn total() -> usize {
        8
    }
}

/// Detect SDD phase from a Skill tool invocation.
pub fn detect_sdd_phase(tool: &ToolUseEvent) -> Option<SddPhase> {
    if tool.tool_name != "Skill" {
        return None;
    }

    // display_status is formatted as "Skill: sdd-{phase}"
    let suffix = tool.display_status.strip_prefix("Skill: sdd-")?;

    match suffix {
        "explore" => Some(SddPhase::Explore),
        "propose" => Some(SddPhase::Propose),
        "spec" => Some(SddPhase::Spec),
        "design" => Some(SddPhase::Design),
        "tasks" => Some(SddPhase::Tasks),
        "apply" => Some(SddPhase::Apply),
        "verify" => Some(SddPhase::Verify),
        "archive" => Some(SddPhase::Archive),
        _ => None,
    }
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
        assert_eq!(
            detect_sdd_phase(&skill_event("sdd-explore")),
            Some(SddPhase::Explore)
        );
        assert_eq!(
            detect_sdd_phase(&skill_event("sdd-propose")),
            Some(SddPhase::Propose)
        );
        assert_eq!(
            detect_sdd_phase(&skill_event("sdd-spec")),
            Some(SddPhase::Spec)
        );
        assert_eq!(
            detect_sdd_phase(&skill_event("sdd-design")),
            Some(SddPhase::Design)
        );
        assert_eq!(
            detect_sdd_phase(&skill_event("sdd-tasks")),
            Some(SddPhase::Tasks)
        );
        assert_eq!(
            detect_sdd_phase(&skill_event("sdd-apply")),
            Some(SddPhase::Apply)
        );
        assert_eq!(
            detect_sdd_phase(&skill_event("sdd-verify")),
            Some(SddPhase::Verify)
        );
        assert_eq!(
            detect_sdd_phase(&skill_event("sdd-archive")),
            Some(SddPhase::Archive)
        );
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
