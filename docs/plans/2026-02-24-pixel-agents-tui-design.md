# Pixel Agents TUI — Design Document

**Date:** 2026-02-24
**Status:** Approved

## Overview

A Rust-based TUI (Terminal UI) application that visualizes Claude Code agents as animated ASCII art characters in a virtual office environment, with a detailed information sidebar showing agent activity, sub-agent trees, SDD workflow status, and prompt summaries.

Distributed as a standalone binary (via `cargo install` / `brew`) with a minimal Claude Code plugin hook that auto-launches the TUI in a split pane when a session starts.

## Requirements

### Functional
- Watch `~/.claude/projects/` for JSONL session files in real-time
- Display agents as animated ASCII characters in a virtual office
- Show detailed agent info: active tools, sub-agents, SDD phase, prompt summary
- Auto-detect terminal/multiplexer and create split panes (WezTerm, Zellij, tmux, Kitty)
- Fallback to opening a new terminal tab when no multiplexer is detected
- Claude Code plugin hook for auto-launch on session start

### Non-Functional
- Single binary, zero runtime dependencies
- Low CPU usage (~10 FPS render loop)
- Instant startup
- Cross-platform (macOS, Linux)

## Architecture

### Approach: Monolith

Single Rust binary handling all responsibilities: file watching, JSONL parsing, state management, and TUI rendering.

```
pixel-agents-tui (single binary)
├── Terminal Adapter (detect + split)
├── JSONL Watcher (notify crate)
├── Agent State Machine (tools, sub-agents, SDD, prompts)
└── Ratatui Renderer (office view + sidebar)
```

### Data Flow

```
~/.claude/projects/**/*.jsonl
        │ (notify/kqueue)
        ▼
   JSONL Parser (incremental, offset-based)
        │
        ▼
   Agent State Machine
   ├── tool_use → update active tools
   ├── tool_result → clear tool, update animation
   ├── Task tool_use → spawn sub-agent
   ├── Skill invocation → detect SDD phase
   └── assistant text → extract prompt summary
        │
        ▼
   Ratatui render loop (~10 FPS)
   ├── Office canvas (ASCII art + animations)
   └── Info sidebar (agent details)
```

## Module Design

### 1. Terminal Adapter (`terminal/`)

Detects the running terminal/multiplexer via environment variables and creates split panes using native CLIs.

**Detection priority:**
1. `ZELLIJ` env var → Zellij
2. `WEZTERM_PANE` env var → WezTerm
3. `KITTY_PID` env var → Kitty
4. `TMUX` env var → tmux
5. Fallback → open new terminal tab/window

**Split commands:**

| Terminal | Command |
|----------|---------|
| WezTerm | `wezterm cli split-pane --right --percent 35 -- pixel-agents-tui --attach` |
| Zellij | `zellij action new-pane --direction right -- pixel-agents-tui --attach` |
| tmux | `tmux split-window -h -l 35% pixel-agents-tui --attach` |
| Kitty | `kitty @ launch --location=vsplit pixel-agents-tui --attach` |
| Fallback | Platform-specific new terminal tab |

**Two modes:**
- `pixel-agents-tui` (no flags): Launcher mode — detects terminal, creates split, launches `--attach` instance
- `pixel-agents-tui --attach`: Render mode — the actual TUI that displays the office and info

### 2. JSONL Watcher (`watcher/`)

Uses `notify` crate for filesystem watching with polling fallback.

**Session discovery:**
- Watches `~/.claude/projects/` recursively
- Detects new/modified `.jsonl` files
- Ignores files with no modification in >5 minutes (dormant)
- Tracks file offset for incremental reads (only new lines)

**JSONL record types:**

| Record | Meaning |
|--------|---------|
| `assistant` + `tool_use` | Agent starts using a tool |
| `user` + `tool_result` | Tool completed |
| `system` + `turn_duration` | Turn ended (agent waiting) |
| `progress` + `agent_progress` | Sub-agent activity |
| `bash_progress` / `mcp_progress` | Long-running tool output |
| `assistant` + `text` | Agent text (for prompt summary) |

**SDD phase detection:**
Matches Skill tool invocations by name:
- `sdd-explore` → Explore
- `sdd-propose` → Propose
- `sdd-spec` → Spec
- `sdd-design` → Design
- `sdd-tasks` → Tasks
- `sdd-apply` → Apply
- `sdd-verify` → Verify
- `sdd-archive` → Archive

### 3. Agent State Machine (`state/`)

```rust
struct AgentState {
    id: u32,
    session_file: PathBuf,
    status: AgentStatus,           // Active, Waiting, Dormant
    active_tools: Vec<ToolInfo>,
    sub_agents: Vec<SubAgent>,
    sdd_phase: Option<SddPhase>,
    prompt_summary: String,
    last_activity: Instant,
    animation_state: AnimState,
}
```

**Status transitions:**
```
[Spawn] → Active (tool_use detected)
Active → Waiting (turn_duration received, or 5s text idle)
Waiting → Active (new tool_use)
Active/Waiting → Dormant (5 min no activity)
```

**Tool display formatting:**
- `Read` → "Reading <filename>"
- `Write` → "Writing <filename>"
- `Edit` → "Editing <filename>"
- `Bash` → "Running: <command>"
- `Grep` → "Searching code"
- `Glob` → "Searching files"
- `Task` → "Subtask: <description>"
- `AskUserQuestion` → "Waiting for answer"

### 4. ASCII Art Engine (`ui/sprites.rs`)

Characters are 3x3 Unicode block characters with 4 animation states:

**States:**
- Idle (standing)
- Typing (active tool — write-type)
- Reading (active tool — read-type)
- Walking (moving to/from seat)

**Colors:**
- Each agent gets a unique ANSI color (Cyan, Magenta, Yellow, Green, Blue, Red)
- Sub-agents use dim variant of parent's color
- Status: green=active, yellow=waiting, gray=dormant

**Office layout:**
- ASCII desks (double-line box characters)
- Agents sit at desks when active
- Wander when idle
- Sub-agents spawn near parent

**Animation:**
- ~10 FPS (configurable)
- Typing: 2-3 frame alternation every 300ms
- Walking: 1 cell per 200ms
- Spawn: top-to-bottom character reveal (simplified matrix effect)

### 5. TUI Layout (`ui/layout.rs`)

```
┌──────────────────── pixel-agents-tui ────────────────────┐
│ ◉ Pixel Agents TUI    N agents │ SDD: Phase (x/y)       │
├──────────── Office ─────────────┼──── Agent Details ─────┤
│                                 │                        │
│   [ASCII office with            │ ▸ Agent #1 [● status]  │
│    animated characters          │   Tool: <current>      │
│    at desks, walking,           │   Duration: Xs         │
│    sub-agents nearby]           │   Prompt: "..."        │
│                                 │   SDD: <phase>         │
│                                 │   Sub-agents:          │
│                                 │   └─ Type: activity    │
│                                 ├────────────────────────┤
│                                 │   Agent #2 [○ status]  │
│                                 │   ...                  │
├─────────────────────────────────┴────────────────────────┤
│ [q]uit  [1-9]select  [Tab]focus  [r]efresh      10 FPS  │
└──────────────────────────────────────────────────────────┘
```

**Components:**
1. **Header**: Project name, agent count, global SDD phase
2. **Office panel** (~60% width): ASCII office with animated characters
3. **Agent sidebar** (~40% width): Scrollable agent list with expanded details
4. **Status bar**: Keyboard shortcuts, FPS

**Keybindings:**
- `q` — quit
- `1-9` — select agent
- `Tab` — toggle focus (office/sidebar)
- `↑↓` — scroll sidebar
- `r` — manual refresh
- `Enter` — expand/collapse agent details

## Claude Code Plugin

Minimal plugin — just a SessionStart hook.

```
plugin/
├── .claude-plugin/
│   └── plugin.json
└── hooks/
    ├── hooks.json
    └── session-start.sh
```

**Hook behavior:**
1. Check if TUI already running (PID file at `/tmp/pixel-agents-tui.pid`)
2. If not running → detect terminal, create split, launch TUI
3. If already running → no-op
4. Returns immediately (async hook)

## Project Structure

```
pixel-agents-tui/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── terminal/
│   │   ├── mod.rs, detect.rs
│   │   ├── wezterm.rs, zellij.rs, tmux.rs, kitty.rs
│   │   └── fallback.rs
│   ├── watcher/
│   │   ├── mod.rs, discovery.rs
│   │   ├── file_watcher.rs
│   │   └── parser.rs
│   ├── state/
│   │   ├── mod.rs, agent.rs
│   │   ├── sdd.rs, tools.rs
│   ├── ui/
│   │   ├── mod.rs, layout.rs
│   │   ├── office.rs, sidebar.rs
│   │   ├── header.rs, statusbar.rs
│   │   └── sprites.rs
│   └── config.rs
├── plugin/
│   ├── .claude-plugin/plugin.json
│   └── hooks/hooks.json
└── tests/
```

## Dependencies

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
notify = "7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
directories = "5"
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

## Key Decisions

1. **Monolith over client-server**: Simpler distribution, instant startup, sufficient for current use case
2. **Rust + Ratatui**: Zero runtime deps, fast startup, efficient file watching
3. **ASCII art over graphics protocols**: Universal terminal compatibility
4. **~10 FPS**: Low CPU while maintaining smooth-enough animation for ASCII
5. **Launcher/attach two-mode binary**: Clean separation of split creation vs rendering
6. **PID file for singleton**: Prevents duplicate TUI panels
7. **Incremental JSONL parsing**: Only reads new lines via offset tracking
