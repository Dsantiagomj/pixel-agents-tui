# Pixel Agents TUI

<p align="center">
  <strong>Your Claude Code agents, alive in the terminal</strong><br>
  <em>Animated ASCII characters. Real-time activity. Zero dependencies.</em>
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> &bull;
  <a href="#how-it-works">How It Works</a> &bull;
  <a href="#installation">Installation</a> &bull;
  <a href="#the-pixel-agents-family">Family</a> &bull;
  <a href="#keybindings">Keybindings</a> &bull;
  <a href="#configuration">Configuration</a>
</p>

---

A **Rust binary** that opens a split pane in your terminal and visualizes your Claude Code agents as animated ASCII art characters in a virtual office. Watch them type, read, spawn sub-agents, and track SDD workflow phases — all in real-time.

```
┌─────── Office ──────┬──── Agent Details ────┐
│                      │ ▸ Agent #1 [● active] │
│   ╔═══╗  ╔═══╗      │   Tool: Write main.ts │
│   ╚═══╝  ╚═══╝      │   Prompt: "Fix auth.."│
│    ◉₁     ◉₂        │   SDD: Apply (6/8)    │
│                      │   Sub-agents:         │
│   ╔═══╗             │   └─ Explore: search  │
│   ╚═══╝             ├───────────────────────┤
│    ◉₃               │   Agent #2 [○ waiting]│
│                      │   Last: Read config.ts│
├──────────────────────┴───────────────────────┤
│ [q]uit  [1-9]select  [Tab]focus      10 FPS │
└──────────────────────────────────────────────┘
```

## The Pixel Agents Family

This is the **terminal edition** of the Pixel Agents ecosystem:

| Version | Runtime | Link |
|---------|---------|------|
| [Pixel Agents](https://github.com/pablodelucca/pixel-agents) | VS Code Extension | Full pixel art office inside VS Code, by [@pablodelucca](https://github.com/pablodelucca) |
| [Pixel Agents Desktop](https://github.com/Dsantiagomj/pixel-agents-desktop) | Electron App | Standalone desktop app, works with any editor |
| **Pixel Agents TUI** | Terminal (Rust) | This project — lightweight ASCII art in your terminal |

All three watch Claude Code's JSONL session files at `~/.claude/projects/` to visualize agent activity. The TUI version trades pixel-perfect sprites for universal terminal compatibility and near-zero resource usage.

## Quick Start

### Install via Homebrew (recommended)

```bash
brew tap Dsantiagomj/tap
brew install pixel-agents-tui
```

Upgrade to latest:

```bash
brew update && brew upgrade pixel-agents-tui
```

### Install via Cargo

```bash
cargo install pixel-agents-tui
```

### Install from source

```bash
git clone https://github.com/Dsantiagomj/pixel-agents-tui.git
cd pixel-agents-tui
cargo install --path .
```

### Set up the Claude Code plugin

The plugin auto-launches the TUI when you start a Claude Code session:

```bash
# Via Claude Code marketplace
claude plugin marketplace add Dsantiagomj/pixel-agents-tui
claude plugin install pixel-agents-tui

# Or manually — copy the plugin to your Claude plugins dir
cp -r plugin/.claude-plugin ~/.claude/plugins/pixel-agents-tui/.claude-plugin
cp -r plugin/hooks ~/.claude/plugins/pixel-agents-tui/hooks
```

That's it. Start Claude Code and the TUI opens automatically in a split pane.

## How It Works

```
~/.claude/projects/**/*.jsonl     (Claude Code session logs)
        │
        ▼
   pixel-agents-tui               (this binary)
   ├── Watches JSONL files         (notify + polling)
   ├── Parses tool_use events      (incremental, offset-based)
   ├── Tracks agent state          (active/waiting/dormant)
   ├── Detects SDD phases          (explore → apply → verify)
   └── Renders ASCII office        (ratatui @ 10 FPS)
        │
        ▼
   Terminal split pane             (auto-detected)
```

### What it shows

- **Animated ASCII characters** — each agent gets a character that types, reads, or idles based on what tool it's using
- **Real-time tool activity** — see exactly what each agent is doing (Reading main.rs, Running: cargo test, Searching code...)
- **Sub-agent trees** — when an agent spawns sub-agents via the Task tool, they appear in the office and sidebar
- **SDD workflow tracking** — detects Spec-Driven Development phases (Explore, Propose, Spec, Design, Tasks, Apply, Verify, Archive)
- **Prompt summary** — shows a summary of what each agent is working on

### Terminal auto-detection

The TUI automatically detects your terminal and creates a split pane:

| Terminal | Detection | Split method |
|----------|-----------|-------------|
| Zellij | `$ZELLIJ` | `zellij action new-pane --direction right` |
| WezTerm | `$WEZTERM_PANE` | `wezterm cli split-pane --right` |
| Kitty | `$KITTY_PID` | `kitty @ launch --location=vsplit` |
| tmux | `$TMUX` | `tmux split-window -h` |
| Other | fallback | Opens new terminal tab |

## Usage

### Automatic (via plugin)

Once the Claude Code plugin is installed, the TUI launches automatically when you start a session. It detects your terminal, creates a split pane, and starts watching for agent activity.

### Manual

```bash
# Launch the TUI (detects terminal, creates split)
pixel-agents-tui

# Or run directly in the current terminal
pixel-agents-tui --attach
```

### CLI flags

| Flag | Description |
|------|-------------|
| `--attach` | Run the TUI directly (skip terminal detection and split creation) |
| `--session-hook` | Used by the Claude Code plugin hook (same as no flags) |

## Keybindings

| Key | Action |
|-----|--------|
| `q` | Quit |
| `1-9` | Select agent by number |
| `Tab` | Toggle focus between Office and Sidebar |
| `↑` / `↓` | Scroll sidebar |
| `r` | Force refresh |

## Layout

```
┌──────────────────── pixel-agents-tui ────────────────────┐
│ ◉ Pixel Agents TUI    N agents │ SDD: Phase (x/y)       │
├──────────── Office ─────────────┼──── Agent Details ─────┤
│                                 │                        │
│   Animated ASCII characters     │ ▸ Selected agent info  │
│   at desks, typing/reading      │   Tool, prompt, SDD    │
│   based on current activity     │   Sub-agent tree       │
│                                 ├────────────────────────┤
│   Sub-agents spawn nearby       │   Other agents listed  │
│                                 │   with status icons    │
├─────────────────────────────────┴────────────────────────┤
│ [q]uit  [1-9]select  [Tab]focus  [↑↓]scroll      10 FPS │
└──────────────────────────────────────────────────────────┘
```

### Status indicators

| Symbol | Meaning |
|--------|---------|
| `●` | Agent is active (using tools) |
| `○` | Agent is waiting (turn ended, needs input) |
| `◌` | Agent is dormant (no activity for 5+ minutes) |

### Character animations

| State | Appearance | When |
|-------|-----------|------|
| Typing | Arms moving | Agent is writing, editing, running commands |
| Reading | Holding document | Agent is reading files, searching code |
| Idle | Standing still | Agent has no active tools |
| Walking | Legs moving | Agent moving between positions |

## Configuration

The TUI stores its PID file at `/tmp/pixel-agents-tui.pid` to prevent duplicate panels. It watches `~/.claude/projects/` for JSONL session files.

No additional configuration is needed — it works out of the box.

## Architecture

Single Rust binary, zero runtime dependencies.

```
src/
├── main.rs              # CLI entry point (launcher/attach modes)
├── app.rs               # Central state + tick loop
├── terminal/
│   └── detect.rs        # Terminal detection + split commands
├── watcher/
│   ├── types.rs         # JSONL record types (serde)
│   ├── parser.rs        # Line parser + tool formatting
│   ├── discovery.rs     # Session file discovery
│   └── file_watcher.rs  # Incremental offset-based reader
├── state/
│   ├── agent.rs         # Agent state machine
│   └── sdd.rs           # SDD phase detection
└── ui/
    ├── sprites.rs       # ASCII character definitions
    └── layout.rs        # Ratatui rendering
```

### Tech stack

- **Rust** — single binary, instant startup, low memory
- **[Ratatui](https://ratatui.rs)** — terminal UI framework
- **[Crossterm](https://github.com/crossterm-rs/crossterm)** — cross-platform terminal manipulation
- **[notify](https://github.com/notify-rs/notify)** — filesystem watching (kqueue on macOS, inotify on Linux)

## Development

```bash
# Run in dev mode
cargo run -- --attach

# Run tests
cargo test

# Lint
cargo clippy -- -W clippy::all

# Format
cargo fmt
```

## Credits

- Original [Pixel Agents](https://github.com/pablodelucca/pixel-agents) VS Code extension by [@pablodelucca](https://github.com/pablodelucca)
- [Pixel Agents Desktop](https://github.com/Dsantiagomj/pixel-agents-desktop) by [@Dsantiagomj](https://github.com/Dsantiagomj)
- Built with [Ratatui](https://ratatui.rs)

## License

MIT
