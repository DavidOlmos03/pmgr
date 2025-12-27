use super::app::App;
use super::help_window;
use super::types::PreviewLayout;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn ui(f: &mut Frame, app: &mut App, prompt: &str) {
    let chunks = match app.layout {
        PreviewLayout::Vertical => Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area()),
        PreviewLayout::Horizontal => Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area()),
    };

    // Left/Top panel (list)
    let list_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Min(0),    // List
            Constraint::Length(3), // Footer
        ])
        .split(chunks[0]);

    // Search bar
    let search_block = Block::default()
        .borders(Borders::ALL)
        .title(prompt)
        .style(Style::default().fg(Color::Cyan));

    let search_text = Paragraph::new(app.search_query.as_str())
        .block(search_block)
        .style(Style::default().fg(Color::Yellow));

    f.render_widget(search_text, list_chunks[0]);

    // List of items
    let items: Vec<ListItem> = app
        .filtered_items
        .iter()
        .enumerate()
        .map(|(idx, (item, _))| {
            let style = Style::default();

            // Mark selected items with checkmark
            let prefix = if app.selected_indices.contains(&idx) {
                "✓ "
            } else {
                "  "
            };

            let content = format!("{}{}", prefix, item);

            ListItem::new(content).style(style)
        })
        .collect();

    let items_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} items ", app.filtered_items.len()))
                .style(Style::default().fg(Color::White)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(items_list, list_chunks[1], &mut app.list_state);

    // Footer with help hint
    let footer_text = "Press '?' for help";

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(footer, list_chunks[2]);

    // Right/Bottom panel (preview)
    if app.preview_cmd.is_some() {
        let preview_block = Block::default()
            .borders(Borders::ALL)
            .title(" Preview ")
            .style(Style::default().fg(Color::Green));

        let preview = Paragraph::new(app.preview_content.clone())
            .block(preview_block)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::White));

        f.render_widget(preview, chunks[1]);
    }

    // System update overlay window
    if app.update_window.active {
        render_update_window(f, app);
    }

    // Help screen overlay
    if app.help_visible {
        render_help_window(f, app);
    }
}

fn render_update_window(f: &mut Frame, app: &mut App) {
    // Create a centered overlay area (80% width, 80% height)
    let area = f.area();
    let overlay_width = (area.width as f32 * 0.8) as u16;
    let overlay_height = (area.height as f32 * 0.8) as u16;
    let overlay_x = (area.width - overlay_width) / 2;
    let overlay_y = (area.height - overlay_height) / 2;

    let overlay_area = Rect {
        x: overlay_x,
        y: overlay_y,
        width: overlay_width,
        height: overlay_height,
    };

    // Clear the background to create a dimmed effect
    f.render_widget(Clear, overlay_area);

    // Title based on status
    let title = if app.update_window.completed {
        if app.update_window.has_error {
            " System Update - FAILED (Alt+X to close) "
        } else {
            " System Update - COMPLETED (closing...) "
        }
    } else {
        " System Update - Running... "
    };

    let border_color = if app.update_window.completed {
        if app.update_window.has_error {
            Color::Red
        } else {
            Color::Green
        }
    } else {
        Color::Yellow
    };

    let update_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(border_color).bg(Color::Black));

    // Calculate how many lines we can show (subtract 2 for borders)
    let content_height = overlay_height.saturating_sub(2) as usize;
    let content_width = overlay_width.saturating_sub(4) as usize; // Subtract borders and padding

    // Helper function to remove ANSI escape codes
    fn strip_ansi_codes(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Skip escape sequence
                if chars.next() == Some('[') {
                    // Skip until we find a letter (end of escape sequence)
                    while let Some(next_c) = chars.next() {
                        if next_c.is_alphabetic() {
                            break;
                        }
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    // Process output: strip ANSI codes and truncate long lines
    let processed_output: Vec<String> = app.update_window.output
        .iter()
        .map(|line| {
            let stripped = strip_ansi_codes(line);
            if stripped.len() > content_width {
                // Truncate and add ellipsis
                format!("{}...", &stripped[..content_width.saturating_sub(3)])
            } else {
                stripped
            }
        })
        .collect();

    // Get the last N lines that fit in the window
    let start_idx = if processed_output.len() > content_height {
        processed_output.len() - content_height
    } else {
        0
    };

    let visible_output: Vec<String> = processed_output
        .iter()
        .skip(start_idx)
        .cloned()
        .collect();

    let output_text = visible_output.join("\n");

    let update_content = Paragraph::new(output_text)
        .block(update_block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White).bg(Color::Black));

    f.render_widget(update_content, overlay_area);
}

fn render_help_window(f: &mut Frame, app: &mut App) {
    // Create a centered overlay area - responsive sizing
    let area = f.area();

    // Calculate responsive dimensions (min 80 cols for two columns)
    let min_width = 80u16;
    let max_width_percent = 0.90;
    let overlay_width = ((area.width as f32 * max_width_percent) as u16).max(min_width).min(area.width - 4);

    // Height: 90% of screen or max available
    let overlay_height = ((area.height as f32 * 0.90) as u16).min(area.height - 4);

    let overlay_x = (area.width.saturating_sub(overlay_width)) / 2;
    let overlay_y = (area.height.saturating_sub(overlay_height)) / 2;

    let overlay_area = Rect {
        x: overlay_x,
        y: overlay_y,
        width: overlay_width,
        height: overlay_height,
    };

    // Clear the background
    f.render_widget(Clear, overlay_area);

    let help_block = Block::default()
        .borders(Borders::ALL)
        .title(" Help - Press '?' or ESC to close | ↑/↓ to scroll ")
        .style(Style::default().fg(Color::Cyan).bg(Color::Black));

    // Split into title area and content area
    let inner_area = help_block.inner(overlay_area);

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Title
            Constraint::Min(0),    // Content
        ])
        .split(inner_area);

    // Render block first
    f.render_widget(help_block, overlay_area);

    // Title - centered
    let title_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("PMGR - Package Manager", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        ]),
        Line::from(vec![
            Span::styled("Keyboard Shortcuts", Style::default().fg(Color::Cyan))
        ]),
        Line::from(""),
    ];

    let title_widget = Paragraph::new(title_lines)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White).bg(Color::Black));

    f.render_widget(title_widget, main_chunks[0]);

    // Determine number of columns based on width
    let use_two_columns = overlay_width >= 80;

    if use_two_columns {
        // Two column layout
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(main_chunks[1]);

        // Left column content
        let left_content = vec![
            Line::from(vec![
                Span::styled("NAVIGATION", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  ↑ / k        Move up in list"),
            Line::from("  ↓ / j        Move down in list"),
            Line::from(""),
            Line::from(vec![
                Span::styled("SELECTION & ACTIONS", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  TAB          Toggle selection"),
            Line::from("  ENTER        Confirm selection"),
            Line::from("  ESC          Cancel and exit"),
            Line::from(""),
            Line::from(vec![
                Span::styled("SEARCH", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  Type         Filter packages"),
            Line::from("  Backspace    Delete character"),
            Line::from(""),
        ];

        // Right column content
        let right_content = vec![
            Line::from(vec![
                Span::styled("LAYOUT", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  Alt+O        Horizontal layout"),
            Line::from("  Alt+V        Vertical layout"),
            Line::from(""),
            Line::from(vec![
                Span::styled("SYSTEM", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  Ctrl+U       Update system"),
            Line::from(""),
            Line::from(vec![
                Span::styled("HELP", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  ?            Show/hide help"),
            Line::from(""),
            Line::from(vec![
                Span::styled("TIPS", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            ]),
            Line::from("• Fuzzy search available"),
            Line::from("• Multi-select with TAB"),
            Line::from("• Updates auto-close"),
            Line::from("• Alt+X closes errors"),
        ];

        let left_para = Paragraph::new(left_content)
            .scroll((app.help_scroll, 0))
            .style(Style::default().fg(Color::White).bg(Color::Black));

        let right_para = Paragraph::new(right_content)
            .scroll((app.help_scroll, 0))
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(left_para, columns[0]);
        f.render_widget(right_para, columns[1]);
    } else {
        // Single column layout for narrow screens
        let content = vec![
            Line::from(vec![
                Span::styled("NAVIGATION", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  ↑ / k        Move up in list"),
            Line::from("  ↓ / j        Move down in list"),
            Line::from(""),
            Line::from(vec![
                Span::styled("SELECTION", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  TAB          Toggle selection"),
            Line::from("  ENTER        Confirm"),
            Line::from("  ESC          Cancel"),
            Line::from(""),
            Line::from(vec![
                Span::styled("SEARCH", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  Type         Filter"),
            Line::from("  Backspace    Delete"),
            Line::from(""),
            Line::from(vec![
                Span::styled("LAYOUT", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  Alt+O        Horizontal"),
            Line::from("  Alt+V        Vertical"),
            Line::from(""),
            Line::from(vec![
                Span::styled("SYSTEM", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  Ctrl+U       Update"),
            Line::from(""),
            Line::from(vec![
                Span::styled("HELP", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  ?            Toggle help"),
            Line::from(""),
        ];

        let para = Paragraph::new(content)
            .scroll((app.help_scroll, 0))
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(para, main_chunks[1]);
    }
}
