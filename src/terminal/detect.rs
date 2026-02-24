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
            TerminalKind::Zellij => "Zellij",
            TerminalKind::WezTerm => "WezTerm",
            TerminalKind::Kitty => "Kitty",
            TerminalKind::Tmux => "tmux",
            TerminalKind::Unknown => "Unknown",
        }
    }
}

/// Detect terminal from env vars. Priority: Zellij > WezTerm > Kitty > tmux > Unknown
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

#[derive(Debug, Clone)]
pub struct SplitCommand {
    pub program: String,
    pub args: Vec<String>,
}

/// Build split command for a given terminal kind.
pub fn build_split_command(kind: TerminalKind, binary_path: &str) -> Option<SplitCommand> {
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
                binary_path.to_string(),
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
                binary_path.to_string(),
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
                format!("{} --attach", binary_path),
            ],
        }),
        TerminalKind::Kitty => Some(SplitCommand {
            program: "kitty".to_string(),
            args: vec![
                "@".to_string(),
                "launch".to_string(),
                "--location=vsplit".to_string(),
                binary_path.to_string(),
                "--attach".to_string(),
            ],
        }),
        TerminalKind::Unknown => None,
    }
}

/// Build fallback command (new terminal tab).
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
        SplitCommand {
            program: "xterm".to_string(),
            args: vec!["-e".to_string(), format!("{} --attach", binary_path)],
        }
    }
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
        assert_eq!(cmd.unwrap().program, "zellij");
    }

    #[test]
    fn build_split_tmux() {
        let cmd = build_split_command(TerminalKind::Tmux, "/usr/bin/pixel-agents-tui");
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap().program, "tmux");
    }

    #[test]
    fn build_split_kitty() {
        let cmd = build_split_command(TerminalKind::Kitty, "/usr/bin/pixel-agents-tui");
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap().program, "kitty");
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
