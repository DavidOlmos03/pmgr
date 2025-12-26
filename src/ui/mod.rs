use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PreviewLayout {
    Vertical,   // Preview on the right
    Horizontal, // Preview below
}

impl PreviewLayout {
    fn toggle_to_horizontal(&mut self) {
        *self = PreviewLayout::Horizontal;
    }

    fn toggle_to_vertical(&mut self) {
        *self = PreviewLayout::Vertical;
    }
}

struct App {
    items: Vec<String>,
    filtered_items: Vec<(String, i64)>, // (item, score)
    list_state: ListState,
    search_query: String,
    selected_indices: Vec<usize>, // For multi-select
    multi: bool,
    preview_cmd: Option<String>,
    preview_content: String,
    layout: PreviewLayout,
    matcher: SkimMatcherV2,
}

impl App {
    fn new(items: Vec<String>, multi: bool, preview_cmd: Option<String>) -> Self {
        let filtered_items: Vec<(String, i64)> = items
            .iter()
            .map(|item| (item.clone(), 0))
            .collect();

        let mut list_state = ListState::default();
        if !filtered_items.is_empty() {
            list_state.select(Some(0));
        }

        let mut app = Self {
            items,
            filtered_items,
            list_state,
            search_query: String::new(),
            selected_indices: Vec::new(),
            multi,
            preview_cmd,
            preview_content: String::new(),
            layout: PreviewLayout::Vertical,
            matcher: SkimMatcherV2::default(),
        };

        app.update_preview();
        app
    }

    fn filter_items(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_items = self
                .items
                .iter()
                .map(|item| (item.clone(), 0))
                .collect();
        } else {
            let mut scored_items: Vec<(String, i64)> = self
                .items
                .iter()
                .filter_map(|item| {
                    self.matcher
                        .fuzzy_match(item, &self.search_query)
                        .map(|score| (item.clone(), score))
                })
                .collect();

            scored_items.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered_items = scored_items;
        }

        // Reset selection to first item
        if !self.filtered_items.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }

        self.update_preview();
    }

    fn next(&mut self) {
        if self.filtered_items.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.filtered_items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.update_preview();
    }

    fn previous(&mut self) {
        if self.filtered_items.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.update_preview();
    }

    fn toggle_select(&mut self) {
        if !self.multi {
            return;
        }

        if let Some(selected) = self.list_state.selected() {
            if self.selected_indices.contains(&selected) {
                self.selected_indices.retain(|&i| i != selected);
            } else {
                self.selected_indices.push(selected);
            }
            self.next(); // Move to next item after toggling
        }
    }

    fn get_selected_items(&self) -> Vec<String> {
        if self.multi {
            self.selected_indices
                .iter()
                .filter_map(|&i| self.filtered_items.get(i).map(|(item, _)| item.clone()))
                .collect()
        } else {
            self.list_state
                .selected()
                .and_then(|i| self.filtered_items.get(i).map(|(item, _)| vec![item.clone()]))
                .unwrap_or_default()
        }
    }

    fn update_preview(&mut self) {
        if let Some(ref cmd) = self.preview_cmd {
            if let Some(selected) = self.list_state.selected() {
                if let Some((item, _)) = self.filtered_items.get(selected) {
                    // Replace {} with the selected item
                    let preview_cmd = cmd.replace("{}", item);

                    // Execute the command
                    if let Ok(output) = Command::new("sh")
                        .arg("-c")
                        .arg(&preview_cmd)
                        .output()
                    {
                        self.preview_content = String::from_utf8_lossy(&output.stdout).to_string();
                    } else {
                        self.preview_content = "Failed to load preview".to_string();
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App, prompt: &str) {
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
            let mut style = Style::default();

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

    // Footer with keybindings
    let layout_text = match app.layout {
        PreviewLayout::Vertical => "[alt-o:horizontal ✗] [alt-v:vertical ✓]",
        PreviewLayout::Horizontal => "[alt-o:horizontal ✓] [alt-v:vertical ✗]",
    };

    let footer_text = if app.multi {
        format!("TAB:select | ENTER:confirm | ESC:cancel | {} | Selected: {}",
            layout_text, app.selected_indices.len())
    } else {
        format!("ENTER:confirm | ESC:cancel | {}", layout_text)
    };

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

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
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    prompt: &str,
) -> Result<Vec<String>> {
    loop {
        terminal.draw(|f| ui(f, &mut app, prompt))?;

        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                // Exit on ESC
                (KeyCode::Esc, _) => {
                    return Ok(Vec::new());
                }
                // Confirm on Enter
                (KeyCode::Enter, _) => {
                    return Ok(app.get_selected_items());
                }
                // Navigation
                (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                    app.next();
                }
                (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                    app.previous();
                }
                // Multi-select with Tab
                (KeyCode::Tab, _) => {
                    app.toggle_select();
                }
                // Layout switching
                (KeyCode::Char('o'), KeyModifiers::ALT) => {
                    app.layout.toggle_to_horizontal();
                }
                (KeyCode::Char('v'), KeyModifiers::ALT) => {
                    app.layout.toggle_to_vertical();
                }
                // Search input
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    app.search_query.push(c);
                    app.filter_items();
                }
                (KeyCode::Backspace, _) => {
                    app.search_query.pop();
                    app.filter_items();
                }
                _ => {}
            }
        }
    }
}

pub struct Selector;

impl Selector {
    /// Show interactive selector for packages
    pub fn select_packages(
        items: Vec<String>,
        prompt: &str,
        multi: bool,
        preview_cmd: Option<String>,
    ) -> Result<Vec<String>> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Create app and run
        let app = App::new(items, multi, preview_cmd);
        let result = run_app(&mut terminal, app, prompt);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    /// Select from installed packages
    pub fn select_installed(packages: Vec<String>) -> Result<Vec<String>> {
        Self::select_packages(
            packages,
            "Select packages to remove (TAB: multi-select, ENTER: confirm): ",
            true,
            Some("echo {} | xargs yay -Qi".to_string()),
        )
    }

    /// Select from available packages
    pub fn select_available(packages: Vec<String>) -> Result<Vec<String>> {
        Self::select_packages(
            packages,
            "Select packages to install (TAB: multi-select, ENTER: confirm): ",
            true,
            Some("echo {} | xargs yay -Si".to_string()),
        )
    }

    /// Browse installed packages (view only)
    pub fn browse_installed(packages: Vec<String>) -> Result<Option<String>> {
        let result = Self::select_packages(
            packages,
            "Browse installed packages (ESC to exit): ",
            false,
            Some("echo {} | xargs yay -Qi".to_string()),
        )?;

        Ok(result.first().cloned())
    }
}
