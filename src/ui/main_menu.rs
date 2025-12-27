use super::app::App;
use super::home_state::{HomeState, SystemStats};
use super::render::{render_home_view, render_tab_bar, ui_in_area};
use super::types::{ActionType, ViewType};
use crate::package::PackageManager;
use anyhow::Result;
use crossterm::{
    event::{self, poll, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::{Constraint, Direction, Layout}, Terminal};
use std::io;
use std::process::Command;
use std::time::Duration;

/// Actions that can be requested during event handling
enum Action {
    None,
    Exit,
    SwitchView(ViewType),
    RefreshView,
    RefreshHomeStats,
}

/// Enum to represent different view states in the main menu
pub enum ViewState {
    Home(HomeState),
    Install(App),
    Remove(App),
    List(App),
}

/// Main menu coordinator that manages navigation between views
pub struct MainMenu {
    current_view: ViewState,
    selected_tab: usize,
    package_manager: PackageManager,
    // Cache to avoid reloading
    cached_installed: Option<Vec<String>>,
}

impl MainMenu {
    pub fn new() -> Result<Self> {
        let package_manager = PackageManager::new();
        let home_state = HomeState::new();

        Ok(Self {
            current_view: ViewState::Home(home_state),
            selected_tab: ViewType::Home as usize,
            package_manager,
            cached_installed: None,
        })
    }

    /// Main entry point - runs the interactive menu
    pub fn run() -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Create main menu and run
        let mut menu = MainMenu::new()?;
        let result = menu.run_loop(&mut terminal);

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

    /// Main event loop
    fn run_loop<B: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Render current view
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Tab bar
                        Constraint::Min(0),    // Content
                    ])
                    .split(f.area());

                // Render tab bar
                render_tab_bar(f, chunks[0], self.selected_tab);

                // Render current view content
                match &mut self.current_view {
                    ViewState::Home(home_state) => {
                        render_home_view(f, chunks[1], home_state);
                    }
                    ViewState::Install(app) => {
                        ui_in_area(f, app, "Select packages to install (TAB: multi-select, ENTER: confirm): ", chunks[1]);
                    }
                    ViewState::Remove(app) => {
                        ui_in_area(f, app, "Select packages to remove (TAB: multi-select, ENTER: confirm): ", chunks[1]);
                    }
                    ViewState::List(app) => {
                        ui_in_area(f, app, "Browse installed packages (ESC to go back): ", chunks[1]);
                    }
                }
            })?;

            // Handle events with polling
            if poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    // Handle global shortcuts first (work in any view)
                    let handled_globally = match (key.code, key.modifiers) {
                        // Show help with '?'
                        (KeyCode::Char('?'), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                            if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                                app.help_visible = !app.help_visible;
                                if !app.help_visible {
                                    app.help_scroll = 0;
                                }
                            }
                            true
                        }
                        // System update with Ctrl+U
                        (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                            // Check if we're in a package view
                            let should_update = matches!(self.current_view, ViewState::Install(_) | ViewState::Remove(_) | ViewState::List(_));

                            if should_update {
                                // Exit TUI and run update interactively
                                disable_raw_mode()?;
                                execute!(
                                    io::stdout(),
                                    LeaveAlternateScreen,
                                    DisableMouseCapture
                                )?;

                                // Run system update interactively
                                let result = self.run_update_interactive();

                                // Wait for user to press Enter
                                println!("\nPress Enter to continue...");
                                let mut input = String::new();
                                let _ = io::stdin().read_line(&mut input);

                                // Re-enter TUI
                                enable_raw_mode()?;
                                execute!(
                                    io::stdout(),
                                    EnterAlternateScreen,
                                    EnableMouseCapture
                                )?;
                                terminal.clear()?;

                                // Show alert based on result
                                if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                                    match result {
                                        Ok(success) => {
                                            if success {
                                                app.alert.show(super::types::AlertType::Success, "✓ System updated successfully".to_string());
                                            } else {
                                                app.alert.show(super::types::AlertType::Error, "✗ System update failed".to_string());
                                            }
                                        }
                                        Err(e) => {
                                            app.alert.show(super::types::AlertType::Error, format!("✗ Error: {}", e));
                                        }
                                    }
                                }
                            }
                            true
                        }
                        _ => false,
                    };

                    // If handled globally, skip view-specific handling
                    if handled_globally {
                        // Check for preview updates in package views
                        if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                            app.check_preview_updates();
                            app.update_window.check_updates();

                            // Auto-close update window if completed successfully
                            if app.update_window.should_auto_close() {
                                app.update_window.close();
                            }

                            // Clear terminal if window was just closed
                            if app.update_window.just_closed {
                                terminal.clear()?;
                                app.update_window.clear_just_closed_flag();
                            }
                        }
                        continue;
                    }

                    // Handle modal windows (update, help, confirm) in package views
                    if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                        // Update window is active
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

                        // Confirmation dialog is active
                        if app.confirm_dialog.active {
                            match (key.code, key.modifiers) {
                                // Confirm with Y or Enter
                                (KeyCode::Char('y'), KeyModifiers::NONE | KeyModifiers::SHIFT)
                                | (KeyCode::Enter, _) => {
                                    app.confirm_dialog.confirm();
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

                        // Help screen is active
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

                        // Alert is active
                        if app.alert.active {
                            // Any key closes the alert
                            app.alert.close();
                            continue;
                        }
                    }

                    // Handle view-specific events
                    let mut action = Action::None;
                    match &mut self.current_view {
                        ViewState::Home(_) => {
                            // Home view key handling
                            action = match (key.code, key.modifiers) {
                                // Switch tabs
                                (KeyCode::Char('1'), _) => Action::SwitchView(ViewType::Home),
                                (KeyCode::Char('2'), _) => Action::SwitchView(ViewType::Install),
                                (KeyCode::Char('3'), _) => Action::SwitchView(ViewType::Remove),
                                (KeyCode::Char('4'), _) => Action::SwitchView(ViewType::List),
                                // Exit on ESC
                                (KeyCode::Esc, _) => Action::Exit,
                                // Refresh stats
                                (KeyCode::Char('r'), KeyModifiers::CONTROL) => Action::RefreshHomeStats,
                                _ => Action::None,
                            };
                        }
                        ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) => {
                            // Package view key handling
                            action = match (key.code, key.modifiers) {
                                // Switch tabs
                                (KeyCode::Char('1'), _) => Action::SwitchView(ViewType::Home),
                                (KeyCode::Char('2'), _) => Action::SwitchView(ViewType::Install),
                                (KeyCode::Char('3'), _) => Action::SwitchView(ViewType::Remove),
                                (KeyCode::Char('4'), _) => Action::SwitchView(ViewType::List),
                                // Go back to home on ESC (if not in search mode)
                                (KeyCode::Esc, _) => {
                                    if app.search_query.is_empty() {
                                        Action::SwitchView(ViewType::Home)
                                    } else {
                                        app.search_query.clear();
                                        app.filter_items();
                                        Action::None
                                    }
                                }
                                // Refresh current view data
                                (KeyCode::Char('r'), KeyModifiers::CONTROL) => Action::RefreshView,
                                // Enter to confirm selection
                                (KeyCode::Enter, _) => {
                                    let selected = app.get_selected_items();
                                    if !selected.is_empty() {
                                        app.confirm_dialog.show(app.action_type, selected);
                                    }
                                    Action::None
                                }
                                // Handle other navigation keys
                                (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                                    app.next();
                                    Action::None
                                }
                                (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                                    app.previous();
                                    Action::None
                                }
                                (KeyCode::Tab, _) => {
                                    app.toggle_select();
                                    Action::None
                                }
                                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                                    // Don't add if it's a tab switch key
                                    if !matches!(c, '1' | '2' | '3' | '4') {
                                        app.search_query.push(c);
                                        app.filter_items();
                                    }
                                    Action::None
                                }
                                (KeyCode::Backspace, _) => {
                                    app.search_query.pop();
                                    app.filter_items();
                                    Action::None
                                }
                                // Layout switching
                                (KeyCode::Char('o'), KeyModifiers::ALT) => {
                                    app.layout.toggle_to_horizontal();
                                    Action::None
                                }
                                (KeyCode::Char('v'), KeyModifiers::ALT) => {
                                    app.layout.toggle_to_vertical();
                                    Action::None
                                }
                                _ => Action::None,
                            };
                        }
                    }

                    // Execute the action after match ends
                    match action {
                        Action::Exit => return Ok(()),
                        Action::SwitchView(view_type) => self.switch_to_view(view_type)?,
                        Action::RefreshView => self.refresh_current_view()?,
                        Action::RefreshHomeStats => self.load_home_stats()?,
                        Action::None => {}
                    }
                }
            }

            // Check if confirmation dialog was confirmed (outside event loop so it's immediate)
            let mut pending_operation: Option<(ActionType, Vec<String>)> = None;
            if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                if app.confirm_dialog.is_confirmed() {
                    pending_operation = Some((app.action_type, app.confirm_dialog.packages.clone()));
                    app.confirm_dialog.cancel(); // Reset dialog
                }
            }

            // Execute pending operation if any
            if let Some((action_type, packages)) = pending_operation {
                // Exit TUI and run command interactively
                disable_raw_mode()?;
                execute!(io::stdout(), LeaveAlternateScreen)?;

                let result = match action_type {
                    ActionType::Install => self.run_install_interactive(&packages),
                    ActionType::Remove => self.run_remove_interactive(&packages),
                };

                // Wait for user to press Enter
                println!("\nPress Enter to continue...");
                let mut input = String::new();
                let _ = io::stdin().read_line(&mut input);

                // Re-enter TUI
                enable_raw_mode()?;
                execute!(io::stdout(), EnterAlternateScreen)?;
                terminal.clear()?;

                // Prepare alert message based on result
                let alert_to_show = match result {
                    Ok(success) => {
                        if success {
                            let message = match action_type {
                                ActionType::Install => format!("✓ Successfully installed {} package(s)", packages.len()),
                                ActionType::Remove => format!("✓ Successfully removed {} package(s)", packages.len()),
                            };
                            Some((super::types::AlertType::Success, message))
                        } else {
                            let message = match action_type {
                                ActionType::Install => "✗ Installation failed".to_string(),
                                ActionType::Remove => "✗ Removal failed".to_string(),
                            };
                            Some((super::types::AlertType::Error, message))
                        }
                    }
                    Err(e) => {
                        Some((super::types::AlertType::Error, format!("✗ Error: {}", e)))
                    }
                };

                // Clear cache for refresh after operation completes
                self.cached_installed = None;
                // Refresh the current view (this creates a new App)
                self.refresh_current_view()?;

                // Show alert after refresh (so it persists in the new App)
                if let Some((alert_type, message)) = alert_to_show {
                    if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                        app.alert.show(alert_type, message);
                    }
                }
            }

            // Always check for updates (even without key events)
            let mut need_view_refresh = false;
            if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                // Check for preview updates (so previews load automatically)
                app.check_preview_updates();

                // Check for update window updates
                app.update_window.check_updates();

                // Auto-close update window if completed successfully
                if app.update_window.should_auto_close() {
                    app.update_window.close();
                    need_view_refresh = true; // Refresh view after successful operation
                }

                // Clear terminal if window was just closed to force full redraw
                if app.update_window.just_closed {
                    terminal.clear()?;
                    app.update_window.clear_just_closed_flag();
                }
            }

            // Refresh view if needed (after window closes)
            if need_view_refresh {
                self.refresh_current_view()?;
            }
        }
    }

    /// Switch to a different view
    fn switch_to_view(&mut self, view_type: ViewType) -> Result<()> {
        self.selected_tab = view_type as usize;

        self.current_view = match view_type {
            ViewType::Home => {
                let mut home_state = HomeState::new();
                self.load_home_stats_into(&mut home_state)?;
                ViewState::Home(home_state)
            }
            ViewType::Install => {
                let packages = self.get_or_load_available()?;
                let package_names: Vec<String> = packages
                    .iter()
                    .map(|p| format!("{}/{}", p.repository, p.name))
                    .collect();

                let app = App::new(
                    package_names,
                    true, // multi-select
                    Some("echo {} | xargs yay -Si".to_string()),
                    ActionType::Install,
                );
                ViewState::Install(app)
            }
            ViewType::Remove => {
                let packages = self.get_or_load_installed()?;
                let app = App::new(
                    packages,
                    true, // multi-select
                    Some("echo {} | xargs yay -Qi".to_string()),
                    ActionType::Remove,
                );
                ViewState::Remove(app)
            }
            ViewType::List => {
                let packages = self.get_or_load_installed()?;
                let app = App::new(
                    packages,
                    false, // single-select (browse mode)
                    Some("echo {} | xargs yay -Qi".to_string()),
                    ActionType::Install, // Default action type
                );
                ViewState::List(app)
            }
        };

        Ok(())
    }

    /// Load home view statistics
    fn load_home_stats(&mut self) -> Result<()> {
        // Load stats data
        let installed = self.package_manager.list_installed()?;
        let available = self.package_manager.list_available()?;

        // TODO: Implement system update check
        let updates_available = 0;

        let stats = SystemStats {
            installed_count: installed.len(),
            available_count: available.len(),
            updates_available,
        };

        // Apply to home state if currently in home view
        if let ViewState::Home(home_state) = &mut self.current_view {
            home_state.set_stats(stats);
        }

        Ok(())
    }

    fn load_home_stats_into(&mut self, home_state: &mut HomeState) -> Result<()> {
        let installed = self.package_manager.list_installed()?;
        let available = self.package_manager.list_available()?;

        // TODO: Implement system update check
        let updates_available = 0;

        home_state.set_stats(SystemStats {
            installed_count: installed.len(),
            available_count: available.len(),
            updates_available,
        });

        Ok(())
    }

    /// Get or load installed packages (with caching)
    fn get_or_load_installed(&mut self) -> Result<Vec<String>> {
        if let Some(ref cached) = self.cached_installed {
            return Ok(cached.clone());
        }

        let packages = self.package_manager.list_installed()?;
        self.cached_installed = Some(packages.clone());
        Ok(packages)
    }

    /// Get or load available packages (no caching for now as it's large)
    fn get_or_load_available(&self) -> Result<Vec<crate::package::Package>> {
        self.package_manager.list_available()
    }

    /// Refresh the current view's data
    fn refresh_current_view(&mut self) -> Result<()> {
        match self.selected_tab {
            0 => self.load_home_stats()?,
            1 => {
                let view_type = ViewType::Install;
                self.switch_to_view(view_type)?;
            }
            2 | 3 => {
                self.cached_installed = None;
                let view_type = if self.selected_tab == 2 {
                    ViewType::Remove
                } else {
                    ViewType::List
                };
                self.switch_to_view(view_type)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Run install command interactively (outside TUI)
    fn run_install_interactive(&self, packages: &[String]) -> Result<bool> {
        // Extract package names from "repository/package" format
        let package_names: Vec<String> = packages
            .iter()
            .map(|p| {
                if let Some(idx) = p.rfind('/') {
                    p[idx + 1..].to_string()
                } else {
                    p.clone()
                }
            })
            .collect();

        println!("Installing {} package(s)...", packages.len());
        println!("Packages: {}\n", package_names.join(", "));

        let status = Command::new("yay")
            .arg("-S")
            .args(&package_names)
            .status()?;

        Ok(status.success())
    }

    /// Run remove command interactively (outside TUI)
    fn run_remove_interactive(&self, packages: &[String]) -> Result<bool> {
        println!("Removing {} package(s)...", packages.len());
        println!("Packages: {}\n", packages.join(", "));

        let status = Command::new("yay")
            .arg("-Rns")
            .args(packages)
            .status()?;

        Ok(status.success())
    }

    /// Run system update interactively (outside TUI)
    fn run_update_interactive(&self) -> Result<bool> {
        println!("Running system update...\n");

        let status = Command::new("sudo")
            .arg("pacman")
            .arg("-Syu")
            .status()?;

        Ok(status.success())
    }
}
