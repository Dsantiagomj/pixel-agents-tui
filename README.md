# Pixel Agents TUI

<p align="center">
  <strong>Your Claude Code agents, alive in the terminal</strong><br>
  <em>Animated ASCII characters. Real-time activity. Single binary. Zero dependencies.</em>
</p>

<p align="center">
  <a href="#installation">Installation</a> &bull;
  <a href="#claude-code-plugin-setup">Plugin Setup</a> &bull;
  <a href="#usage">Usage</a> &bull;
  <a href="#configuration">Configuration</a> &bull;
  <a href="#about">About</a>
</p>

---

```
┌──────────────────── pixel-agents-tui ────────────────────┐
│ ◉ Pixel Agents TUI    3 agents │ SDD: Apply (6/8)       │
├──────────── Office ─────────────┼──── Agent Details ─────┤
│                                 │                        │
│   ╔═══╗  ╔═══╗  ╔═══╗         │ ▸ Agent #1 [● active]  │
│   ╚═══╝  ╚═══╝  ╚═══╝         │   Tool: Write main.ts  │
│    ◉₁     ◉₂                   │   Prompt: "Fix auth.." │
│                                 │   SDD: Apply (6/8)     │
│   ╔═══╗  ╔═══╗                 │   Sub-agents:          │
│   ╚═══╝  ╚═══╝                 │   └─ Explore: search   │
│           ◉₃    ◇              ├────────────────────────┤
│                                 │   Agent #2 [○ waiting] │
│                                 │   Last: Read config.ts │
├─────────────────────────────────┴────────────────────────┤
│ [q]uit  [1-9]select  [Tab]focus  [↑↓]scroll      10 FPS │
└──────────────────────────────────────────────────────────┘
```

## About

This project is inspired by [Pixel Agents](https://github.com/pablodelucca/pixel-agents), the VS Code extension created by [@pablodelucca](https://github.com/pablodelucca) that turns your Claude Code agents into animated pixel art characters inside a virtual office. If you use VS Code, check out the original — it's beautiful.

**Pixel Agents TUI** brings that same idea to the terminal. Instead of pixel-perfect sprites, it uses animated ASCII characters rendered with [Ratatui](https://ratatui.rs). The result is a lightweight dashboard that works in any terminal, on any machine, with zero runtime dependencies.

There's also [Pixel Agents Desktop](https://github.com/Dsantiagomj/pixel-agents-desktop), a standalone Electron app that runs independently of any editor. All three versions share the same core concept: watch Claude Code's JSONL session files and visualize what your agents are doing in real-time.

| Version | Runtime | Best for |
|---------|---------|----------|
| [Pixel Agents](https://github.com/pablodelucca/pixel-agents) | VS Code Extension | VS Code users who want pixel art sprites |
| [Pixel Agents Desktop](https://github.com/Dsantiagomj/pixel-agents-desktop) | Electron App | Editor-agnostic pixel art visualization |
| **Pixel Agents TUI** | Rust binary | Terminal-native, minimal resource usage |

### What it shows

- **Animated ASCII characters** at desks in a virtual office — each agent types, reads, or idles based on the tool it's currently using
- **Real-time tool activity** — `Reading main.rs`, `Running: cargo test`, `Searching code`, `Writing auth.rs`...
- **Sub-agent trees** — when an agent spawns sub-agents via the Task tool, they appear as smaller characters near the parent
- **SDD workflow tracking** — detects [Spec-Driven Development](https://github.com/Dsantiagomj/pixel-agents-tui) phases: Explore, Propose, Spec, Design, Tasks, Apply, Verify, Archive
- **Prompt summary** — the first meaningful text from each agent, so you know what it's working on

---

## Installation

You need to install two things: the **binary** (the TUI itself) and the **Claude Code plugin** (so it launches automatically).

### Step 1: Install the binary

Choose one of these methods:

#### Homebrew (macOS / Linux)

```bash
brew tap Dsantiagomj/tap
brew install pixel-agents-tui
```

To upgrade later:

```bash
brew update && brew upgrade pixel-agents-tui
```

#### Cargo (any platform with Rust)

```bash
cargo install pixel-agents-tui
```

#### From source

```bash
git clone https://github.com/Dsantiagomj/pixel-agents-tui.git
cd pixel-agents-tui
cargo install --path .
```

Verify it's installed:

```bash
pixel-agents-tui --help
```

### Step 2: Install the Claude Code plugin

The plugin adds a `SessionStart` hook that auto-launches the TUI in a split pane every time you start a Claude Code session.

#### Via Claude Code marketplace (recommended)

```bash
claude plugin marketplace add Dsantiagomj/pixel-agents-tui
claude plugin install pixel-agents-tui
```

#### Manual install

If you prefer to install the plugin manually, copy it to your Claude plugins directory:

```bash
mkdir -p ~/.claude/plugins/pixel-agents-tui
cp -r plugin/.claude-plugin ~/.claude/plugins/pixel-agents-tui/.claude-plugin
cp -r plugin/hooks ~/.claude/plugins/pixel-agents-tui/hooks
```

That's it. The next time you run `claude`, the TUI will open automatically in a split pane next to your session.

---

## Usage

### Automatic (with the plugin installed)

Just start Claude Code normally. The plugin detects your terminal, creates a split pane to the right, and launches the TUI. If a TUI panel is already running, it won't create a duplicate.

### Manual

You can also run it manually without the plugin:

```bash
# Detect terminal, create split pane, launch TUI in it
pixel-agents-tui

# Run the TUI directly in the current terminal (no split)
pixel-agents-tui --attach
```

### CLI flags

| Flag | Description |
|------|-------------|
| `--attach` | Run the TUI directly in the current terminal window. Skips terminal detection and split pane creation. Use this when you want to open the TUI in a terminal you already have open. |
| `--session-hook` | Used internally by the Claude Code plugin. Behaves the same as running without flags. |
| *(no flags)* | Launcher mode. Detects your terminal, creates a split pane, and starts a `--attach` instance inside it. |

### Keybindings

| Key | Action |
|-----|--------|
| `q` | Quit the TUI |
| `1`-`9` | Select agent by number |
| `Tab` | Toggle focus between Office panel and Sidebar |
| `↑` / `↓` | Scroll the sidebar when focused |
| `r` | Force an immediate refresh (resets the scan timer) |

---

## Configuration

### Terminal auto-detection

The TUI detects which terminal multiplexer or emulator you're running and uses its native API to create a split pane:

| Terminal | How it's detected | Split command |
|----------|------------------|---------------|
| **Zellij** | `$ZELLIJ` or `$ZELLIJ_SESSION_NAME` | `zellij action new-pane --direction right` |
| **WezTerm** | `$WEZTERM_PANE` or `$WEZTERM_EXECUTABLE` | `wezterm cli split-pane --right --percent 35` |
| **Kitty** | `$KITTY_PID` or `$KITTY_WINDOW_ID` | `kitty @ launch --location=vsplit` |
| **tmux** | `$TMUX` | `tmux split-window -h -l 35%` |
| **Other** | fallback | Opens a new terminal tab |

The split pane takes ~35% of the terminal width. The detection order is: Zellij > WezTerm > Kitty > tmux > fallback.

### Singleton behavior

The TUI writes a PID file to `/tmp/pixel-agents-tui.pid` when it starts. If the plugin hook fires and detects the TUI is already running, it does nothing. This prevents multiple panels from opening when you start new Claude Code sessions.

### Session discovery

The TUI watches `~/.claude/projects/` recursively for `.jsonl` session files. It uses the OS-native filesystem watcher (`kqueue` on macOS, `inotify` on Linux) with a 2-second polling fallback for reliability.

- **Active sessions**: `.jsonl` files modified within the last 5 minutes
- **Dormant sessions**: files with no changes for 5+ minutes are marked dormant and the agent character turns gray
- **Scan interval**: new sessions are checked every ~2 seconds

### SDD phase detection

If you use Spec-Driven Development, the TUI detects which phase the agent is in by watching for `Skill` tool invocations that match `sdd-*` patterns:

| Detected skill | Phase shown |
|---------------|-------------|
| `sdd-explore` | Explore (1/8) |
| `sdd-propose` | Propose (2/8) |
| `sdd-spec` | Spec (3/8) |
| `sdd-design` | Design (4/8) |
| `sdd-tasks` | Tasks (5/8) |
| `sdd-apply` | Apply (6/8) |
| `sdd-verify` | Verify (7/8) |
| `sdd-archive` | Archive (8/8) |

The current SDD phase appears in the header bar and in the selected agent's detail panel.

### Status indicators

| Symbol | Color | Meaning |
|--------|-------|---------|
| `●` | Green | Agent is actively using tools |
| `○` | Yellow | Agent finished its turn and is waiting for input |
| `◌` | Gray | Agent has been inactive for 5+ minutes |

### Character animations

Each agent is a 3x3 ASCII character that animates based on the tool it's currently using:

| Animation | Triggers | Tools |
|-----------|----------|-------|
| **Typing** | Agent is writing or executing | Write, Edit, Bash, Task, Skill |
| **Reading** | Agent is consuming information | Read, Grep, Glob, WebFetch, WebSearch |
| **Idle** | No tools active | *(between turns)* |

Agent colors cycle through Cyan, Magenta, Yellow, Green, Blue, Red. Sub-agents spawned via the Task tool appear as dim `◇` markers near their parent.

---

## Architecture

Single Rust binary. No Node.js, no Python, no Docker, no runtime dependencies.

```
pixel-agents-tui
├── Terminal Adapter        Detect terminal, create split pane
├── JSONL Watcher           Watch ~/.claude/projects/ for session files
│   ├── Session Discovery   Find new/removed .jsonl files
│   ├── File Reader         Incremental offset-based reading
│   └── Line Parser         Parse tool_use, tool_result, text, turn_duration
├── Agent State Machine     Track status, tools, sub-agents, SDD phase
└── Ratatui Renderer        Office view + agent sidebar @ 10 FPS
```

```
src/
├── main.rs              # CLI (launcher / attach modes)
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

### Built with

- **[Rust](https://www.rust-lang.org)** — single binary, instant startup, low memory
- **[Ratatui](https://ratatui.rs)** — terminal UI framework
- **[Crossterm](https://github.com/crossterm-rs/crossterm)** — cross-platform terminal backend
- **[notify](https://github.com/notify-rs/notify)** — filesystem watching

---

## Development

```bash
# Clone
git clone https://github.com/Dsantiagomj/pixel-agents-tui.git
cd pixel-agents-tui

# Run in dev mode
cargo run -- --attach

# Run tests (61 tests)
cargo test

# Lint
cargo clippy -- -W clippy::all

# Format
cargo fmt
```

---

## Credits

- Inspired by the original [Pixel Agents](https://github.com/pablodelucca/pixel-agents) VS Code extension by [@pablodelucca](https://github.com/pablodelucca)
- [Pixel Agents Desktop](https://github.com/Dsantiagomj/pixel-agents-desktop) by [@Dsantiagomj](https://github.com/Dsantiagomj)
- Built with [Ratatui](https://ratatui.rs)

## License

MIT
