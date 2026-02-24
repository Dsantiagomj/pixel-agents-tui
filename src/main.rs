use std::fs;
use std::io;
use std::process::Command;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use pixel_agents_tui::app::App;
use pixel_agents_tui::terminal::{build_fallback_command, build_split_command, detect_terminal};
use pixel_agents_tui::ui::layout;

const PID_FILE: &str = "/tmp/pixel-agents-tui.pid";
const TICK_RATE: Duration = Duration::from_millis(100);

#[derive(Parser, Debug)]
#[command(
    name = "pixel-agents-tui",
    about = "TUI dashboard for Claude Code agents"
)]
struct Cli {
    /// Run in attach mode (renders the TUI)
    #[arg(long)]
    attach: bool,

    /// Launched from a Claude Code session hook
    #[arg(long)]
    session_hook: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    if cli.attach {
        run_tui()
    } else {
        launch_split()
    }
}

/// Check if a process with the given PID is still alive.
fn is_process_alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Launcher mode: detect terminal, create a split pane, and launch the TUI in --attach mode.
fn launch_split() -> io::Result<()> {
    // Check PID file - if process is still alive, don't launch another instance
    if let Ok(pid_str) = fs::read_to_string(PID_FILE) {
        let pid = pid_str.trim();
        if !pid.is_empty() && is_process_alive(pid) {
            return Ok(());
        }
    }

    // Get the current binary path
    let binary_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "pixel-agents-tui".to_string());

    // Detect terminal and build the appropriate split command
    let kind = detect_terminal();
    let split_cmd = build_split_command(kind, &binary_path)
        .unwrap_or_else(|| build_fallback_command(&binary_path));

    // Spawn the split command
    Command::new(&split_cmd.program)
        .args(&split_cmd.args)
        .spawn()
        .map_err(|e| {
            io::Error::other(format!(
                "Failed to spawn {} {}: {}",
                split_cmd.program,
                split_cmd.args.join(" "),
                e
            ))
        })?;

    Ok(())
}

/// Attach mode: run the TUI with the event loop.
fn run_tui() -> io::Result<()> {
    // Write PID file
    let pid = std::process::id();
    fs::write(PID_FILE, pid.to_string())?;

    // Determine the Claude directory
    let claude_dir = directories::BaseDirs::new()
        .map(|d| d.home_dir().join(".claude"))
        .unwrap_or_else(|| std::path::PathBuf::from(".claude"));

    // Initialize the terminal
    let mut terminal = ratatui::init();

    // Create the application state
    let mut app = App::new(claude_dir);

    // Main event loop
    let result = loop {
        // Draw the UI
        if let Err(e) = terminal.draw(|frame| layout::render(frame, &app)) {
            break Err(e);
        }

        // Poll for events at the tick rate (10 FPS)
        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                // Only handle key press events (not release/repeat)
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Tab => {
                            app.toggle_focus();
                        }
                        KeyCode::Up => {
                            app.scroll_up();
                        }
                        KeyCode::Down => {
                            app.scroll_down();
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                            app.select_agent(c.to_digit(10).unwrap());
                        }
                        KeyCode::Char('r') => {
                            // Reset tick count to force an immediate refresh
                            app.tick_count = 0;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Tick the app state forward
        app.tick();

        if app.should_quit {
            break Ok(());
        }
    };

    // Restore the terminal
    ratatui::restore();

    // Cleanup PID file
    let _ = fs::remove_file(PID_FILE);

    result
}
