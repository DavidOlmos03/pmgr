use anyhow::Result;
use crossterm::{
    event::{self, poll, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

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

#[derive(Debug)]
enum UpdateMessage {
    Output(String),
    Completed(bool), // true if successful, false if error
}

struct SystemUpdateWindow {
    active: bool,
    output: Vec<String>,
    completed: bool,
    has_error: bool,
    rx: Option<Receiver<UpdateMessage>>,
    just_closed: bool, // Flag to indicate we need to redraw
}

impl SystemUpdateWindow {
    fn new() -> Self {
        Self {
            active: false,
            output: Vec::new(),
            completed: false,
            has_error: false,
            rx: None,
            just_closed: false,
        }
    }

    fn start_update(&mut self) {
        self.active = true;
        self.output.clear();
        self.output.push("Starting system update...".to_string());
        self.completed = false;
        self.has_error = false;

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            // First, validate sudo access (password should already be cached)
            let validate_status = Command::new("sudo")
                .arg("-n")
                .arg("true")
                .status();

            if let Err(_) = validate_status {
                let _ = tx.send(UpdateMessage::Output("Error: sudo password not cached. This shouldn't happen.".to_string()));
                let _ = tx.send(UpdateMessage::Completed(false));
                return;
            }

            let mut child = match Command::new("sudo")
                .arg("pacman")
                .arg("-Syu")
                .arg("--noconfirm")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => {
                    let _ = tx.send(UpdateMessage::Output(format!("Error: Failed to start command: {}", e)));
                    let _ = tx.send(UpdateMessage::Completed(false));
                    return;
                }
            };

            // Read stdout in separate thread
            let stdout = child.stdout.take();
            let tx_stdout = tx.clone();
            let stdout_handle = thread::spawn(move || {
                if let Some(stdout) = stdout {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            let _ = tx_stdout.send(UpdateMessage::Output(line));
                        }
                    }
                }
            });

            // Read stderr in separate thread
            let stderr = child.stderr.take();
            let tx_stderr = tx.clone();
            let stderr_handle = thread::spawn(move || {
                if let Some(stderr) = stderr {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            let _ = tx_stderr.send(UpdateMessage::Output(line));
                        }
                    }
                }
            });

            // Wait for both reading threads to complete
            let _ = stdout_handle.join();
            let _ = stderr_handle.join();

            // Wait for process to complete
            match child.wait() {
                Ok(status) => {
                    let success = status.success();
                    if success {
                        let _ = tx.send(UpdateMessage::Output("\n✓ System update completed successfully!".to_string()));
                    } else {
                        let _ = tx.send(UpdateMessage::Output(format!("\n✗ System update failed with code: {:?}", status.code())));
                    }
                    let _ = tx.send(UpdateMessage::Completed(success));
                }
                Err(e) => {
                    let _ = tx.send(UpdateMessage::Output(format!("\nError waiting for process: {}", e)));
                    let _ = tx.send(UpdateMessage::Completed(false));
                }
            }
        });
    }

    fn check_updates(&mut self) {
        if let Some(ref rx) = self.rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    UpdateMessage::Output(line) => {
                        self.output.push(line);
                    }
                    UpdateMessage::Completed(success) => {
                        self.completed = true;
                        self.has_error = !success;
                    }
                }
            }
        }
    }

    fn should_auto_close(&self) -> bool {
        self.completed && !self.has_error
    }

    fn close(&mut self) {
        self.active = false;
        self.output.clear();
        self.completed = false;
        self.has_error = false;
        self.rx = None;
        self.just_closed = true;
    }

    fn clear_just_closed_flag(&mut self) {
        self.just_closed = false;
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
    preview_cache: HashMap<String, String>, // Cache for loaded previews
    preview_tx: Option<Sender<(String, String)>>, // Send preview requests
    preview_rx: Option<Receiver<(String, String)>>, // Receive preview results
    layout: PreviewLayout,
    matcher: SkimMatcherV2,
    current_preview_item: Option<String>, // Track current item being previewed
    update_window: SystemUpdateWindow,
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

        // Create channels for async preview loading
        let (preview_tx, preview_rx) = if preview_cmd.is_some() {
            let (tx, rx) = mpsc::channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        let mut app = Self {
            items,
            filtered_items,
            list_state,
            search_query: String::new(),
            selected_indices: Vec::new(),
            multi,
            preview_cmd,
            preview_content: String::new(),
            preview_cache: HashMap::new(),
            preview_tx,
            preview_rx,
            layout: PreviewLayout::Vertical,
            matcher: SkimMatcherV2::default(),
            current_preview_item: None,
            update_window: SystemUpdateWindow::new(),
        };

        app.request_preview();
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

        self.request_preview();
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
        self.request_preview();
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
        self.request_preview();
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

    fn request_preview(&mut self) {
        if let Some(ref cmd) = self.preview_cmd {
            if let Some(selected) = self.list_state.selected() {
                if let Some((item, _)) = self.filtered_items.get(selected) {
                    // Check if already in cache
                    if let Some(cached) = self.preview_cache.get(item) {
                        self.preview_content = cached.clone();
                        self.current_preview_item = Some(item.clone());
                        return;
                    }

                    // Check if already loading this item
                    if self.current_preview_item.as_ref() == Some(item) {
                        return;
                    }

                    self.current_preview_item = Some(item.clone());
                    self.preview_content = "Loading preview...".to_string();

                    // Spawn thread to load preview
                    if let Some(ref tx) = self.preview_tx {
                        let item_clone = item.clone();
                        let cmd_clone = cmd.clone();
                        let tx_clone = tx.clone();

                        thread::spawn(move || {
                            let preview_cmd = cmd_clone.replace("{}", &item_clone);

                            let content = if let Ok(output) = Command::new("sh")
                                .arg("-c")
                                .arg(&preview_cmd)
                                .output()
                            {
                                String::from_utf8_lossy(&output.stdout).to_string()
                            } else {
                                "Failed to load preview".to_string()
                            };

                            let _ = tx_clone.send((item_clone, content));
                        });
                    }
                }
            }
        }
    }

    fn check_preview_updates(&mut self) {
        if let Some(ref rx) = self.preview_rx {
            // Try to receive without blocking
            while let Ok((item, content)) = rx.try_recv() {
                // Cache the result
                self.preview_cache.insert(item.clone(), content.clone());

                // Update display if this is still the current item
                if self.current_preview_item.as_ref() == Some(&item) {
                    self.preview_content = content;
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

    // Footer with keybindings
    let layout_text = match app.layout {
        PreviewLayout::Vertical => "[alt-o:horizontal ✗] [alt-v:vertical ✓]",
        PreviewLayout::Horizontal => "[alt-o:horizontal ✓] [alt-v:vertical ✗]",
    };

    let footer_text = if app.multi {
        format!("TAB:select | ENTER:confirm | ESC:cancel | Ctrl+U:update system | {} | Selected: {}",
            layout_text, app.selected_indices.len())
    } else {
        format!("ENTER:confirm | ESC:cancel | Ctrl+U:update system | {}", layout_text)
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

    // System update overlay window
    if app.update_window.active {
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
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    prompt: &str,
) -> Result<Vec<String>> {
    loop {
        // Check for preview updates from background threads
        app.check_preview_updates();

        // Check for system update progress
        app.update_window.check_updates();

        // Auto-close update window if completed successfully
        if app.update_window.should_auto_close() {
            app.update_window.close();
        }

        // Clear terminal if window was just closed to force full redraw
        if app.update_window.just_closed {
            terminal.clear()?;
            app.update_window.clear_just_closed_flag();
        }

        terminal.draw(|f| ui(f, &mut app, prompt))?;

        // Use poll with timeout to allow periodic UI updates
        if poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // If update window is active, only allow Alt+X to close it (if has error)
                if app.update_window.active {
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('x'), KeyModifiers::ALT) => {
                            if app.update_window.has_error || app.update_window.completed {
                                app.update_window.close();
                            }
                        }
                        _ => {} // Ignore other keys while update window is active
                    }
                    continue;
                }

                match (key.code, key.modifiers) {
                    // Exit on ESC
                    (KeyCode::Esc, _) => {
                        return Ok(Vec::new());
                    }
                    // Confirm on Enter
                    (KeyCode::Enter, _) => {
                        return Ok(app.get_selected_items());
                    }
                    // Start system update with Ctrl+U
                    (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                        // Exit raw mode temporarily to ask for sudo password
                        disable_raw_mode()?;
                        execute!(
                            io::stdout(),
                            LeaveAlternateScreen,
                            DisableMouseCapture
                        )?;

                        // Ask for sudo password
                        println!("System update requires sudo access. Please enter your password:");
                        let sudo_result = Command::new("sudo")
                            .arg("-v")
                            .status();

                        // Re-enter raw mode
                        enable_raw_mode()?;
                        execute!(
                            io::stdout(),
                            EnterAlternateScreen,
                            EnableMouseCapture
                        )?;

                        // Start update if sudo was successful
                        match sudo_result {
                            Ok(status) if status.success() => {
                                app.update_window.start_update();
                            }
                            _ => {
                                // Could show error message, but for now just ignore
                            }
                        }
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
