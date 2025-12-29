use super::app::App;
use super::render::ui;
use super::theme::Theme;
use super::types::ActionType;
use anyhow::Result;
use crossterm::{
    event::{self, poll, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::process::Command;
use std::time::Duration;

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
            app.update_window.close(false); // Not cancelled by user
        }

        // Clear terminal if window was just closed to force full redraw
        if app.update_window.just_closed {
            terminal.clear()?;
            app.update_window.clear_just_closed_flag();
        }

        // Use Default theme for standalone selector
        let palette = Theme::Default.palette();
        terminal.draw(|f| ui(f, &mut app, prompt, &palette))?;

        // Use poll with timeout to allow periodic UI updates
        if poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // If update window is active, only allow Alt+X to close it (if has error)
                if app.update_window.active {
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('x'), KeyModifiers::ALT) => {
                            if app.update_window.has_error || app.update_window.completed {
                                app.update_window.close(true); // Cancelled by user
                            }
                        }
                        _ => {} // Ignore other keys while update window is active
                    }
                    continue;
                }

                // If confirmation dialog is active, handle separately
                if app.confirm_dialog.active {
                    match (key.code, key.modifiers) {
                        // Confirm with Y or Enter
                        (KeyCode::Char('y'), KeyModifiers::NONE | KeyModifiers::SHIFT)
                        | (KeyCode::Enter, _) => {
                            app.confirm_dialog.confirm();
                            return Ok(app.confirm_dialog.packages.clone());
                        }
                        // Cancel with N or ESC
                        (KeyCode::Char('n'), KeyModifiers::NONE | KeyModifiers::SHIFT)
                        | (KeyCode::Esc, _) => {
                            app.confirm_dialog.cancel();
                        }
                        // Scroll down
                        (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                            app.confirm_dialog.scroll_down();
                        }
                        // Scroll up
                        (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                            app.confirm_dialog.scroll_up();
                        }
                        _ => {} // Ignore other keys while dialog is active
                    }
                    continue;
                }

                // If help screen is visible, handle separately
                if app.help_visible {
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('?'), KeyModifiers::NONE | KeyModifiers::SHIFT)
                        | (KeyCode::Esc, _) => {
                            app.help_visible = false;
                            app.help_scroll = 0; // Reset scroll when closing
                        }
                        // Scroll down
                        (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                            app.help_scroll = app.help_scroll.saturating_add(1);
                        }
                        // Scroll up
                        (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                            app.help_scroll = app.help_scroll.saturating_sub(1);
                        }
                        _ => {} // Ignore other keys while help is visible
                    }
                    continue;
                }

                match (key.code, key.modifiers) {
                    // Show help on '?'
                    (KeyCode::Char('?'), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                        app.help_visible = true;
                        app.help_scroll = 0; // Reset scroll when opening
                    }
                    // Exit on ESC
                    (KeyCode::Esc, _) => {
                        return Ok(Vec::new());
                    }
                    // Confirm on Enter - show confirmation dialog
                    (KeyCode::Enter, _) => {
                        let selected = app.get_selected_items();
                        if !selected.is_empty() {
                            app.confirm_dialog.show(app.action_type, selected);
                        }
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
        action_type: ActionType,
    ) -> Result<Vec<String>> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Create app and run
        let app = App::new(items, multi, preview_cmd, action_type);
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
            ActionType::Remove,
        )
    }

    /// Select from available packages
    pub fn select_available(packages: Vec<String>) -> Result<Vec<String>> {
        Self::select_packages(
            packages,
            "Select packages to install (TAB: multi-select, ENTER: confirm): ",
            true,
            Some("echo {} | xargs yay -Si".to_string()),
            ActionType::Install,
        )
    }

    /// Browse installed packages (view only)
    pub fn browse_installed(packages: Vec<String>) -> Result<Option<String>> {
        let result = Self::select_packages(
            packages,
            "Browse installed packages (ESC to exit): ",
            false,
            Some("echo {} | xargs yay -Qi".to_string()),
            ActionType::Install, // Default to Install for browse mode
        )?;

        Ok(result.first().cloned())
    }
}
