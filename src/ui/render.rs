use super::app::App;
use super::types::{ActionType, AlertType, PreviewLayout};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn ui(f: &mut Frame, app: &mut App, prompt: &str) {
    ui_in_area(f, app, prompt, f.area());
}

pub fn ui_in_area(f: &mut Frame, app: &mut App, prompt: &str, area: Rect) {
    let chunks = match app.layout {
        PreviewLayout::Vertical => Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area),
        PreviewLayout::Horizontal => Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area),
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
                //.bg(Color::DarkGray)
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

    // Confirmation dialog overlay
    if app.confirm_dialog.active {
        render_confirm_dialog(f, app);
    }

    // Alert overlay (rendered last so it appears on top)
    if app.alert.active {
        render_alert(f, app);
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
    let base_title = if app.update_window.title.is_empty() {
        "Operation"
    } else {
        &app.update_window.title
    };

    let title = if app.update_window.completed {
        if app.update_window.has_error {
            format!(" {} - FAILED (Alt+X to close) ", base_title)
        } else {
            format!(" {} - COMPLETED (closing...) ", base_title)
        }
    } else {
        format!(" {} - Running... ", base_title)
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
        .style(Style::default().fg(border_color));

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
        .style(Style::default().fg(Color::White));

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
        .style(Style::default().fg(Color::Cyan));

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
        .style(Style::default().fg(Color::White));

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
            .style(Style::default().fg(Color::White));

        let right_para = Paragraph::new(right_content)
            .scroll((app.help_scroll, 0))
            .style(Style::default().fg(Color::White));

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
            .style(Style::default().fg(Color::White));

        f.render_widget(para, main_chunks[1]);
    }
}

fn render_confirm_dialog(f: &mut Frame, app: &App) {
    // Create a responsive centered dialog
    let area = f.area();

    // Calculate width based on longest package name
    let min_width = 40u16;
    let max_width = 55u16;

    // Find longest package name
    let max_pkg_len = app.confirm_dialog.packages
        .iter()
        .map(|p| p.len())
        .max()
        .unwrap_or(20) as u16;

    // Calculate needed width based on:
    // - Longest package + "  • " + padding: max_pkg_len + 8
    // - Buttons line width: ~30 chars ("  ┌─────────┐  ┌──────────┐")
    // - Message width: ~45 chars
    let message_width = 45u16;
    let buttons_width = 30u16;
    let pkg_width = max_pkg_len + 8;

    let content_width = message_width.max(buttons_width).max(pkg_width);
    let dialog_width = content_width.min(max_width).max(min_width).min(area.width.saturating_sub(4));

    // Calculate height based on content
    let max_visible_packages = 6u16;
    let package_count = (app.confirm_dialog.packages.len() as u16).min(max_visible_packages);

    // Height breakdown:
    // - Title border: 2 lines
    // - message + empty line: 2 lines
    // - Packages: package_count lines
    // - Empty line: 1 line
    // - Separator + empty line: 2 lines
    // - Question + empty line: 2 lines
    // - Buttons: 3 lines
    // - ESC text: 1 line
    // - Bottom border included in calculation
    let content_height = 2 + 2 + package_count + 1 + 2 + 2 + 3 + 1;
    let max_height = (area.height as f32 * 0.7) as u16;
    let dialog_height = content_height.min(max_height).max(16).min(area.height.saturating_sub(4));

    let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect {
        x: dialog_x,
        y: dialog_y,
        width: dialog_width,
        height: dialog_height,
    };

    // Clear the background
    f.render_widget(Clear, dialog_area);

    // Determine colors and title based on action type
    let (title_text, border_color) = match app.confirm_dialog.action_type {
        ActionType::Install => (
            " Confirm Installation ",
            Color::Green,
        ),
        ActionType::Remove => (
            " Confirm Removal ",
            Color::Red,
        ),
    };

    // Add scroll hint to title if there are many packages
    let title = if app.confirm_dialog.packages.len() > max_visible_packages as usize {
        format!("{} - ↑/↓ to scroll ", title_text)
    } else {
        title_text.to_string()
    };

    let dialog_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(border_color));

    let inner_area = dialog_block.inner(dialog_area);

    // Render block first
    f.render_widget(dialog_block, dialog_area);

    // Split inner area: package list area + buttons area
    // Package area height: 2 (header) + package_count + 1 (bottom padding)
    let package_area_height = 2 + package_count + 1;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(package_area_height), // Package list (scrollable)
            Constraint::Min(9),                      // Buttons (fixed)
        ])
        .split(inner_area);

    // Create package list content
    let mut package_lines = vec![];

    // Action message
    let action_msg = match app.confirm_dialog.action_type {
        ActionType::Install => "The following packages will be installed:",
        ActionType::Remove => "The following packages will be removed:",
    };
    package_lines.push(Line::from(vec![
        Span::styled(action_msg, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    ]));
    package_lines.push(Line::from(""));

    // All packages (no limit, scroll handles overflow)
    for pkg in &app.confirm_dialog.packages {
        // Truncate package name if too long
        let max_pkg_width = (dialog_width.saturating_sub(8)) as usize;
        let pkg_display = if pkg.len() > max_pkg_width {
            format!("{}...", &pkg[..max_pkg_width.saturating_sub(3)])
        } else {
            pkg.clone()
        };

        package_lines.push(Line::from(vec![
            Span::raw("  • "),
            Span::styled(pkg_display, Style::default().fg(Color::Cyan))
        ]));
    }

    package_lines.push(Line::from(""));

    // Package list with scroll
    let package_list = Paragraph::new(package_lines)
        .scroll((app.confirm_dialog.scroll, 0))
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));

    f.render_widget(package_list, chunks[0]);

    // Create buttons content (fixed, no scroll)
    let mut button_lines = vec![];

    // Calculate separator width
    let separator_width = dialog_width.saturating_sub(4) as usize;
    let separator = "━".repeat(separator_width);

    button_lines.push(Line::from(separator));
    button_lines.push(Line::from(""));

    // Confirmation prompt with icon
    button_lines.push(Line::from(vec![
        Span::styled("", Style::default().fg(Color::Yellow)), // Question icon
        Span::raw(" "),
        Span::styled("Do you want to continue?", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
    ]));
    button_lines.push(Line::from(""));

    // Buttons with box drawing and icons
    button_lines.push(Line::from(vec![
        Span::styled("┌───────────┐", Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled("┌────────────┐", Style::default().fg(Color::Red)),
    ]));
    button_lines.push(Line::from(vec![
        Span::styled("│ ", Style::default().fg(Color::Green)),
        Span::styled("✓ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)), // Checkmark icon
        Span::styled("Y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled(" - Yes │", Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled("│ ", Style::default().fg(Color::Red)),
        Span::styled("✗ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)), // X icon
        Span::styled("N", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled(" - No   │", Style::default().fg(Color::Red)),
    ]));
    button_lines.push(Line::from(vec![
        Span::styled("└───────────┘", Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled("└────────────┘", Style::default().fg(Color::Red)),
    ]));
    button_lines.push(Line::from(vec![
        Span::styled(" ", Style::default().fg(Color::Gray)), // Keyboard icon
        Span::raw(" Press "),
        Span::styled("ESC", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::raw(" to cancel"),
    ]));

    let buttons = Paragraph::new(button_lines)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    f.render_widget(buttons, chunks[1]);
}

/// Render tab bar at the top of the screen
pub fn render_tab_bar(f: &mut Frame, area: Rect, selected_tab: usize) {
    use super::types::ViewType;

    let tabs = vec![
        ("[1] Home", ViewType::Home as usize),
        ("[2] Install", ViewType::Install as usize),
        ("[3] Remove", ViewType::Remove as usize),
        ("[4] List", ViewType::List as usize),
    ];

    let mut tab_spans = vec![];

    for (i, (label, tab_idx)) in tabs.iter().enumerate() {
        if i > 0 {
            tab_spans.push(Span::raw(" │ "));
        }

        let style = if *tab_idx == selected_tab {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
                //.bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        tab_spans.push(Span::styled(*label, style));
    }

    let tabs_line = Line::from(tab_spans);
    let tabs_paragraph = Paragraph::new(tabs_line)
        .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(tabs_paragraph, area);
}

/// Render the home view
pub fn render_home_view(f: &mut Frame, area: Rect, home_state: &super::home_state::HomeState) {
    // Create centered content area
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" PMGR - Package Manager ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Split into header and sections
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Title + separator
            Constraint::Min(0),     // Sections
        ])
        .split(inner_area);

    // Render title section (ASCII art + subtitle)
    let mut title_lines = vec![];
    title_lines.push(Line::from(""));
    title_lines.push(Line::from(Span::styled(
        "  ______   _____    ___________",
        Style::default().fg(Color::Rgb(210, 215, 255)),
    )));
    title_lines.push(Line::from(Span::styled(
        "   \\____ \\ /     \\  / ___\\_  __ \\",
        Style::default().fg(Color::Rgb(200, 205, 245)),
    )));
    title_lines.push(Line::from(Span::styled(
        "   |  |_> >  Y Y  \\/ /_/  >  | \\/",
        Style::default().fg(Color::Rgb(190, 195, 235)),
    )));
    title_lines.push(Line::from(Span::styled(
        " |   __/|__|_|  /\\___  /|__| ",
        Style::default().fg(Color::Rgb(175, 180, 220)),
    )));
    title_lines.push(Line::from(Span::styled(
        "|__|         \\//_____/    ",
        Style::default().fg(Color::Rgb(165, 170, 210)),
    )));
    title_lines.push(Line::from(""));
    title_lines.push(Line::from(vec![
        "Modern package manager ".fg(Color::Cyan),
        "for Arch Linux".yellow().italic(),
    ]));
    title_lines.push(Line::from(
        ratatui::symbols::line::HORIZONTAL
            .repeat(50)
            .fg(Color::Rgb(100, 100, 100)),
    ));
    title_lines.push(Line::from(
    env!("CARGO_PKG_REPOSITORY")
        .italic()
        .fg(Color::Gray),
    ));
    title_lines.push(Line::from(vec![
        "[".fg(Color::Rgb(100, 100, 100)),
        "with ".into(),
        "♥".cyan(),
        " by ".into(),
        "@DavidOlmos03".cyan(),
        "]".fg(Color::Rgb(100, 100, 100)),
    ]));

    let title_widget = Paragraph::new(title_lines)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    f.render_widget(title_widget, main_chunks[0]);

    // Determine number of columns based on width
    let width = inner_area.width;
    let num_columns = if width >= 120 {
        3 // 3 columns for wide screens
    } else if width >= 80 {
        2 // 2 columns for medium screens
    } else {
        1 // 1 column for narrow screens
    };

    // Create System Information section
    let mut sys_info_lines = vec![];
    sys_info_lines.push(Line::from(vec![
        Span::styled("System Information", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    ]));
    sys_info_lines.push(Line::from(
        ratatui::symbols::line::HORIZONTAL
            .repeat(18)
            .fg(Color::Rgb(100, 100, 100)),
    ));
    sys_info_lines.push(Line::from(""));

    if let Some(ref stats) = home_state.stats {
        sys_info_lines.push(Line::from(vec![
            "Installed".cyan(),
            Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
            Span::styled(
                stats.installed_count.to_string(),
                Style::default().fg(Color::Rgb(150, 255, 150))
            )
        ]));
        sys_info_lines.push(Line::from(vec![
            "Available".cyan(),
            Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
            Span::styled(
                stats.available_count.to_string(),
                Style::default().fg(Color::Rgb(150, 200, 255))
            )
        ]));
        sys_info_lines.push(Line::from(vec![
            "Updates".cyan(),
            Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
            Span::styled(
                format!("{}", stats.updates_available),
                Style::default().fg(if stats.updates_available > 0 { Color::Rgb(255, 150, 150) } else { Color::Rgb(150, 255, 150) })
            )
        ]));
    } else {
        sys_info_lines.push(Line::from("Loading...".italic()));
    }

    // Create Quick Actions section
    let mut quick_actions_lines = vec![];
    quick_actions_lines.push(Line::from(vec![
        Span::styled("Quick Actions", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    ]));
    quick_actions_lines.push(Line::from(
        ratatui::symbols::line::HORIZONTAL
            .repeat(13)
            .fg(Color::Rgb(100, 100, 100)),
    ));
    quick_actions_lines.push(Line::from(""));
    quick_actions_lines.push(Line::from(vec![
        "[1]".cyan(),
        Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
        "Install packages".into(),
    ]));
    quick_actions_lines.push(Line::from(vec![
        "[2]".cyan(),
        Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
        "Remove packages".into(),
    ]));
    quick_actions_lines.push(Line::from(vec![
        "[3]".cyan(),
        Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
        "List packages".into(),
    ]));
    quick_actions_lines.push(Line::from(vec![
        "[Ctrl+U]".fg(Color::Magenta),
        Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
        "System update".into(),
    ]));

    // Create Keyboard Shortcuts section
    let mut shortcuts_lines = vec![];
    shortcuts_lines.push(Line::from(vec![
        Span::styled("Keyboard Shortcuts", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    ]));
    shortcuts_lines.push(Line::from(
        ratatui::symbols::line::HORIZONTAL
            .repeat(18)
            .fg(Color::Rgb(100, 100, 100)),
    ));
    shortcuts_lines.push(Line::from(""));
    shortcuts_lines.push(Line::from(vec![
        "1-4".cyan(),
        Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
        "Switch tabs".into(),
    ]));
    shortcuts_lines.push(Line::from(vec![
        "?".cyan(),
        Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
        "Show help".into(),
    ]));
    shortcuts_lines.push(Line::from(vec![
        "Ctrl+R".cyan(),
        Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
        "Refresh data".into(),
    ]));
    shortcuts_lines.push(Line::from(vec![
        "ESC".fg(Color::Red),
        Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
        "Exit".into(),
    ]));

    // Render sections based on number of columns
    if num_columns == 3 {
        // 3 columns layout
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(main_chunks[1]);

        let sys_info = Paragraph::new(sys_info_lines)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));

        let quick_actions = Paragraph::new(quick_actions_lines)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));

        let shortcuts = Paragraph::new(shortcuts_lines)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));

        f.render_widget(sys_info, columns[0]);
        f.render_widget(quick_actions, columns[1]);
        f.render_widget(shortcuts, columns[2]);
    } else if num_columns == 2 {
        // 2 columns layout
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(main_chunks[1]);

        // Left column: System Info
        let sys_info = Paragraph::new(sys_info_lines)
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::White));

        // Right column: Quick Actions + Shortcuts
        let mut right_column_lines = vec![];
        right_column_lines.extend(quick_actions_lines);
        right_column_lines.push(Line::from(""));
        right_column_lines.extend(shortcuts_lines);

        let right_column = Paragraph::new(right_column_lines)
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::White));

        f.render_widget(sys_info, columns[0]);
        f.render_widget(right_column, columns[1]);
    } else {
        // 1 column layout
        let mut all_lines = vec![];
        all_lines.extend(sys_info_lines);
        all_lines.push(Line::from(""));
        all_lines.extend(quick_actions_lines);
        all_lines.push(Line::from(""));
        all_lines.extend(shortcuts_lines);

        let single_column = Paragraph::new(all_lines)
            .alignment(Alignment::Center)
            .scroll((home_state.scroll_position, 0))
            .style(Style::default().fg(Color::White));

        f.render_widget(single_column, main_chunks[1]);
    }
}

fn render_alert(f: &mut Frame, app: &mut App) {
    // Create a centered overlay area for alert (60% width, auto height)
    let area = f.area();
    let overlay_width = (area.width as f32 * 0.6).min(80.0) as u16;
    let overlay_height = 7; // Fixed height for alert

    let overlay_x = (area.width.saturating_sub(overlay_width)) / 2;
    let overlay_y = (area.height.saturating_sub(overlay_height)) / 2;

    let overlay_area = Rect {
        x: overlay_x,
        y: overlay_y,
        width: overlay_width,
        height: overlay_height,
    };

    // Clear the area
    f.render_widget(Clear, overlay_area);

    // Determine color based on alert type
    let (border_color, title_style) = match app.alert.alert_type {
        AlertType::Success => (Color::Green, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        AlertType::Error => (Color::Red, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        AlertType::Info => (Color::Cyan, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    };

    let title = match app.alert.alert_type {
        AlertType::Success => "Success",
        AlertType::Error => "Error",
        AlertType::Info => "Info",
    };

    // Create the alert block
    let block = Block::default()
        .title(Span::styled(
            format!(" {} ", title),
            title_style,
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default());

    // Create message paragraph
    let message_lines = vec![
        Line::from(""),
        Line::from(Span::styled(&app.alert.message, Style::default().fg(Color::White))),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )),
    ];

    let paragraph = Paragraph::new(message_lines)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, overlay_area);
}
