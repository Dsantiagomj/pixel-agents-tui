use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, PanelFocus};
use crate::state::agent::AgentStatus;
use crate::state::sdd::SddPhase;
use crate::ui::sprites;

/// Main render entry point. Splits the frame into header, body (office + sidebar), and footer.
pub fn render(frame: &mut Frame, app: &App) {
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(3),
    ])
    .areas(frame.area());

    let [office_area, sidebar_area] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
            .areas(body_area);

    render_header(frame, app, header_area);
    render_office(frame, app, office_area);
    render_sidebar(frame, app, sidebar_area);
    render_footer(frame, app, footer_area);
}

/// Render the header bar with title, agent count, and global SDD phase.
fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let agent_count = app.agents.len();

    // Find the most advanced SDD phase across all agents
    let sdd_display = global_sdd_display(app);

    let title_span = Span::styled(
        " \u{25c9} Pixel Agents TUI ",
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    );
    let count_span = Span::styled(
        format!("   {agent_count} agents"),
        Style::new().fg(Color::White),
    );
    let sdd_span = Span::styled(format!("   {sdd_display}"), Style::new().fg(Color::Yellow));

    let header_line = Line::from(vec![title_span, count_span, sdd_span]);
    let header = Paragraph::new(header_line).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" pixel-agents-tui ")
            .title_style(Style::new().fg(Color::Cyan)),
    );

    frame.render_widget(header, area);
}

/// Render the office view with desks and animated agent characters.
fn render_office(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == PanelFocus::Office;
    let border_style = if focused {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Office ")
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let ids = app.sorted_agent_ids();
    let frame_idx = (app.tick_count / 5) as usize; // animate every 5 ticks

    // Layout: 3 desks per row. Each desk cell is ~10 chars wide, ~6 lines tall.
    let desks_per_row: usize = 3;
    let cell_width: u16 = 10;
    let cell_height: u16 = 6;

    for (i, &id) in ids.iter().enumerate() {
        let col = i % desks_per_row;
        let row = i / desks_per_row;

        let x = inner.x + (col as u16) * cell_width + 1;
        let y = inner.y + (row as u16) * cell_height;

        // Check if this desk fits within the inner area
        if x + cell_width > inner.x + inner.width || y + cell_height > inner.y + inner.height {
            continue;
        }

        let color = sprites::agent_color(id);
        let anim = app.agent_anim_state(id);
        let sprite = sprites::sprite_frame(anim, frame_idx);

        // Render desk (2 lines)
        let desk = sprites::DESK;
        for (dy, desk_line) in desk.iter().enumerate() {
            let desk_span = Span::styled(*desk_line, Style::new().fg(Color::White));
            let desk_paragraph = Paragraph::new(Line::from(desk_span));
            let desk_rect = Rect::new(x + 1, y + dy as u16, desk_line.chars().count() as u16, 1);
            if desk_rect.y < inner.y + inner.height {
                frame.render_widget(desk_paragraph, desk_rect);
            }
        }

        // Render character sprite (3 lines) below desk
        for (dy, sprite_line) in sprite.iter().enumerate() {
            let sprite_span = Span::styled(*sprite_line, Style::new().fg(color));
            let sprite_paragraph = Paragraph::new(Line::from(sprite_span));
            let sprite_rect = Rect::new(
                x + 2,
                y + 2 + dy as u16,
                sprite_line.chars().count() as u16,
                1,
            );
            if sprite_rect.y < inner.y + inner.height {
                frame.render_widget(sprite_paragraph, sprite_rect);
            }
        }

        // Render agent label below sprite
        let label_text = format!("\u{25c9}{id}");
        let label_span = Span::styled(label_text, Style::new().fg(color));
        let label_paragraph = Paragraph::new(Line::from(label_span));
        let label_y = y + 5;
        if label_y < inner.y + inner.height {
            let label_rect = Rect::new(x + 2, label_y, 4, 1);
            frame.render_widget(label_paragraph, label_rect);
        }
    }
}

/// Render the sidebar with a scrollable agent detail list.
fn render_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == PanelFocus::Sidebar;
    let border_style = if focused {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Agent Details ")
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let ids = app.sorted_agent_ids();
    let mut lines: Vec<Line> = Vec::new();

    for &id in &ids {
        let agent = match app.agents.get(&id) {
            Some(a) => a,
            None => continue,
        };

        let is_selected = app.selected_agent == Some(id);
        let color = sprites::agent_color(id);
        let status_symbol = agent.status.symbol();
        let status_label = agent.status.label();

        // Agent header line
        let marker = if is_selected { "\u{25b8} " } else { "  " };
        let header_style = if is_selected {
            Style::new().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(color)
        };
        let status_color = match agent.status {
            AgentStatus::Active => Color::Green,
            AgentStatus::Waiting => Color::Yellow,
            AgentStatus::Dormant => Color::DarkGray,
        };

        lines.push(Line::from(vec![
            Span::styled(marker, header_style),
            Span::styled(format!("Agent #{id} "), header_style),
            Span::styled("[", Style::new().fg(Color::White)),
            Span::styled(
                format!("{status_symbol} {status_label}"),
                Style::new().fg(status_color),
            ),
            Span::styled("]", Style::new().fg(Color::White)),
        ]));

        // Expanded details for selected agent
        if is_selected {
            // Current tool
            if let Some(tool_display) = agent.current_tool_display() {
                let truncated: String = tool_display.chars().take(40).collect();
                lines.push(Line::from(vec![
                    Span::styled("   Tool: ", Style::new().fg(Color::DarkGray)),
                    Span::styled(truncated, Style::new().fg(Color::White)),
                ]));
            }

            // Prompt summary
            if !agent.prompt_summary.is_empty() {
                let prompt: String = agent.prompt_summary.chars().take(35).collect();
                lines.push(Line::from(vec![
                    Span::styled("   Prompt: ", Style::new().fg(Color::DarkGray)),
                    Span::styled(format!("\"{prompt}...\""), Style::new().fg(Color::White)),
                ]));
            }

            // SDD phase
            if let Some(ref phase) = agent.sdd_phase {
                let phase_display = format!(
                    "{} ({}/{})",
                    phase.label(),
                    phase.index() + 1,
                    SddPhase::total()
                );
                lines.push(Line::from(vec![
                    Span::styled("   SDD: ", Style::new().fg(Color::DarkGray)),
                    Span::styled(phase_display, Style::new().fg(Color::Yellow)),
                ]));
            }

            // Sub-agents
            if !agent.sub_agents.is_empty() {
                lines.push(Line::from(Span::styled(
                    "   Sub-agents:",
                    Style::new().fg(Color::DarkGray),
                )));
                for sub in &agent.sub_agents {
                    let sub_color = sprites::sub_agent_color(id);
                    let sub_tool = sub
                        .active_tools
                        .last()
                        .map(|t| t.display_status.as_str())
                        .unwrap_or(&sub.agent_type);
                    lines.push(Line::from(vec![
                        Span::styled("   \u{2514}\u{2500} ", Style::new().fg(Color::DarkGray)),
                        Span::styled(
                            format!("{}: {sub_tool}", sub.agent_type),
                            Style::new().fg(sub_color),
                        ),
                    ]));
                }
            }

            // Separator after expanded agent
            lines.push(Line::from(Span::styled(
                "\u{2500}".repeat(inner.width as usize),
                Style::new().fg(Color::DarkGray),
            )));
        }
    }

    // Apply scroll offset
    let scroll_offset = app.sidebar_scroll as usize;
    let visible_lines: Vec<Line> = lines.into_iter().skip(scroll_offset).collect();

    let paragraph = Paragraph::new(visible_lines);
    frame.render_widget(paragraph, inner);
}

/// Render the footer with keybindings and FPS counter.
fn render_footer(frame: &mut Frame, _app: &App, area: Rect) {
    let fps = 10; // Target FPS from the app design

    let keys = vec![
        Span::styled(" [q]", Style::new().fg(Color::Yellow)),
        Span::styled("uit  ", Style::new().fg(Color::DarkGray)),
        Span::styled("[1-9]", Style::new().fg(Color::Yellow)),
        Span::styled("select  ", Style::new().fg(Color::DarkGray)),
        Span::styled("[Tab]", Style::new().fg(Color::Yellow)),
        Span::styled("focus  ", Style::new().fg(Color::DarkGray)),
        Span::styled("[\u{2191}\u{2193}]", Style::new().fg(Color::Yellow)),
        Span::styled("scroll", Style::new().fg(Color::DarkGray)),
    ];

    // Calculate space needed for right-aligned FPS
    let fps_text = format!("{fps} FPS ");
    let key_line = Line::from(keys);

    // We'll put keys on the left and FPS on the right via two separate paragraphs
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Keys on the left
    let keys_paragraph = Paragraph::new(key_line);
    frame.render_widget(keys_paragraph, inner);

    // FPS on the right
    let fps_span = Span::styled(fps_text.clone(), Style::new().fg(Color::DarkGray));
    let fps_paragraph =
        Paragraph::new(Line::from(fps_span)).alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(fps_paragraph, inner);
}

/// Determine the global SDD display string from all agents.
fn global_sdd_display(app: &App) -> String {
    let mut best_phase: Option<&SddPhase> = None;
    for agent in app.agents.values() {
        if let Some(ref phase) = agent.sdd_phase {
            match best_phase {
                None => best_phase = Some(phase),
                Some(current) => {
                    if phase.index() > current.index() {
                        best_phase = Some(phase);
                    }
                }
            }
        }
    }

    match best_phase {
        Some(phase) => format!(
            "SDD: {} ({}/{})",
            phase.label(),
            phase.index() + 1,
            SddPhase::total()
        ),
        None => String::new(),
    }
}
