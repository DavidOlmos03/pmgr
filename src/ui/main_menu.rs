use super::app::App;
use super::home_state::{HomeState, SystemStats};
use super::render::{render_home_view, render_loading_spinner, render_tab_bar, render_theme_selector, ui_in_area};
use super::spinner::LoadingState;
use super::theme::Theme;
use super::types::{ActionType, ViewType};
use crate::config;
use crate::package::PackageManager;
use anyhow::Result;
use crossterm::{
    event::{self, poll, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::{Constraint, Direction, Layout}, Terminal};
use std::io;
use std::time::Duration;

/// Actions that can be requested during event handling
enum Action {
    None,
    Exit,
    SwitchView(ViewType),
    RefreshView,
    RefreshHomeStats,
}

/// Pending data load state
enum PendingLoad {
    None,
    Home,
    Install,
    Remove,
    List,
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
    // Theme system
    theme: Theme,
    theme_selector_active: bool,
    theme_selector_selected: usize,
    // Loading state
    loading_state: LoadingState,
    pending_load: PendingLoad,
}

impl MainMenu {
    pub fn new() -> Result<Self> {
        let package_manager = PackageManager::new();
        let home_state = HomeState::new();
        let settings = config::load_settings();

        Ok(Self {
            current_view: ViewState::Home(home_state),
            selected_tab: ViewType::Home as usize,
            package_manager,
            cached_installed: None,
            theme: settings.theme,
            theme_selector_active: false,
            theme_selector_selected: settings.theme as usize,
            loading_state: LoadingState::new(),
            pending_load: PendingLoad::Home, // Load home stats on start
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
            // Update spinner animation
            self.loading_state.tick();

            // Render current view FIRST (so spinner is visible)
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Tab bar
                        Constraint::Min(0),    // Content
                    ])
                    .split(f.area());

                // Get theme palette
                let palette = self.theme.palette();

                // Render tab bar
                render_tab_bar(f, chunks[0], self.selected_tab, &palette);

                // Render current view content
                match &mut self.current_view {
                    ViewState::Home(home_state) => {
                        render_home_view(f, chunks[1], home_state, &palette);
                    }
                    ViewState::Install(app) => {
                        ui_in_area(f, app, "Select packages to install (TAB: multi-select, ENTER: confirm): ", chunks[1], &palette);
                    }
                    ViewState::Remove(app) => {
                        ui_in_area(f, app, "Select packages to remove (TAB: multi-select, ENTER: confirm): ", chunks[1], &palette);
                    }
                    ViewState::List(app) => {
                        ui_in_area(f, app, "Browse installed packages (ESC to go back): ", chunks[1], &palette);
                    }
                }

                // Render theme selector on top if active
                if self.theme_selector_active {
                    render_theme_selector(f, &palette, self.theme_selector_selected);
                }

                // Render loading spinner overlay if active
                if self.loading_state.is_active() {
                    render_loading_spinner(f, &self.loading_state, &palette);
                }
            })?;

            // Handle pending loads AFTER rendering (so spinner is visible during load)
            if !matches!(self.pending_load, PendingLoad::None) {
                let load_type = std::mem::replace(&mut self.pending_load, PendingLoad::None);

                match load_type {
                    PendingLoad::Home => {
                        self.perform_home_load()?;
                    }
                    PendingLoad::Install => {
                        self.perform_install_load()?;
                    }
                    PendingLoad::Remove => {
                        self.perform_remove_load()?;
                    }
                    PendingLoad::List => {
                        self.perform_list_load()?;
                    }
                    PendingLoad::None => {}
                }
                // After load completes, continue to next iteration to render the data
                continue;
            }

            // Handle events with polling
            if poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    // Handle global shortcuts first (work in any view)
                    let handled_globally = match (key.code, key.modifiers) {
                        // Show theme selector with Ctrl+T
                        (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
                            self.theme_selector_active = !self.theme_selector_active;
                            if self.theme_selector_active {
                                // Reset selection to current theme when opening
                                self.theme_selector_selected = self.theme as usize;
                            }
                            true
                        }
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
                            // Start system update with pkexec (polkit will handle authentication)
                            if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                                app.update_window.start_update();
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
                                app.update_window.close(false); // Not cancelled by user
                            }

                            // Clear terminal if window was just closed
                            if app.update_window.just_closed {
                                terminal.clear()?;

                                // Show appropriate alert based on operation result
                                if app.update_window.cancelled_by_user {
                                    app.alert.show(super::types::AlertType::Info, "⚠ Operation cancelled by user".to_string());
                                } else if app.update_window.was_successful {
                                    // Show success message based on operation type
                                    let message = if let Some(ref op_type) = app.update_window.operation_type {
                                        if op_type == "system_update" {
                                            "✓ System updated successfully".to_string()
                                        } else {
                                            "✓ Operation completed successfully".to_string()
                                        }
                                    } else {
                                        "✓ Operation completed successfully".to_string()
                                    };
                                    app.alert.show(super::types::AlertType::Success, message);
                                } else if let Some(_) = app.update_window.operation_type {
                                    app.alert.show(super::types::AlertType::Error, "✗ Operation failed".to_string());
                                }

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
                                        app.update_window.close(true); // Cancelled by user
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

                    // Theme selector is active
                    if self.theme_selector_active {
                        match (key.code, key.modifiers) {
                            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                                let num_themes = Theme::all().len();
                                self.theme_selector_selected = (self.theme_selector_selected + 1) % num_themes;
                            }
                            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                                let num_themes = Theme::all().len();
                                self.theme_selector_selected = if self.theme_selector_selected == 0 {
                                    num_themes - 1
                                } else {
                                    self.theme_selector_selected - 1
                                };
                            }
                            (KeyCode::Enter, _) => {
                                // Apply theme
                                self.theme = Theme::all()[self.theme_selector_selected];

                                // Save to config
                                let settings = config::Settings {
                                    theme: self.theme,
                                };
                                if let Err(e) = config::save_settings(&settings) {
                                    // Could show error alert, but for now just ignore
                                    eprintln!("Failed to save theme: {}", e);
                                }

                                self.theme_selector_active = false;
                            }
                            (KeyCode::Esc, _) => {
                                self.theme_selector_active = false;
                            }
                            _ => {}
                        }
                        continue; // Don't process other keys when modal is active
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

            // Check if confirmation dialog was confirmed and start operation
            if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                if app.confirm_dialog.is_confirmed() {
                    let packages = app.confirm_dialog.packages.clone();
                    let action_type = app.action_type;

                    // Reset confirmation dialog first
                    app.confirm_dialog.cancel();

                    match action_type {
                        ActionType::Install => {
                            // Separate AUR vs official packages
                            let (aur_packages, official_packages) = self.package_manager.separate_packages(&packages);

                            // Handle official packages first (if any) using pkexec within TUI
                            if !official_packages.is_empty() {
                                app.update_window.start_install_official(&official_packages);
                            }

                            // Handle AUR packages using handoff (exit TUI, run yay, return)
                            if !aur_packages.is_empty() {
                                // Exit TUI for handoff
                                disable_raw_mode()?;
                                execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

                                println!("\n📦 Installing {} AUR package(s) with yay...\n", aur_packages.len());

                                // Extract package names
                                let pkg_names: Vec<String> = aur_packages
                                    .iter()
                                    .map(|p| {
                                        if let Some(idx) = p.rfind('/') {
                                            p[idx + 1..].to_string()
                                        } else {
                                            p.clone()
                                        }
                                    })
                                    .collect();

                                // Run yay with full control (handoff)
                                // Ignore SIGINT (Ctrl+C) temporarily so yay can handle it
                                use signal_hook::consts::SIGINT;
                                use signal_hook::flag;
                                use std::sync::Arc;
                                use std::sync::atomic::AtomicBool;

                                let term = Arc::new(AtomicBool::new(false));
                                let _guard = flag::register(SIGINT, Arc::clone(&term));

                                let result = std::process::Command::new("yay")
                                    .arg("-S")
                                    .args(&pkg_names)
                                    .stdin(std::process::Stdio::inherit())
                                    .stdout(std::process::Stdio::inherit())
                                    .stderr(std::process::Stdio::inherit())
                                    .status();

                                // Guard drops here, restoring normal SIGINT handling

                                // Flush and add spacing
                                use std::io::Write;
                                let _ = io::stdout().flush();
                                let _ = io::stderr().flush();

                                // Determine if operation was successful or cancelled
                                let (was_successful, was_cancelled) = match &result {
                                    Ok(status) => {
                                        let success = status.success();
                                        // Exit code 130 = SIGINT (Ctrl+C)
                                        // Also check for other interrupt codes
                                        let code = status.code().unwrap_or(1);
                                        let cancelled = code == 130 || code == 2;
                                        (success, cancelled)
                                    }
                                    Err(e) => {
                                        // Check if error is due to interrupt
                                        let cancelled = e.kind() == std::io::ErrorKind::Interrupted;
                                        (false, cancelled)
                                    }
                                };

                                println!("\n{}", "=".repeat(60));

                                if was_successful {
                                    // Success - wait for user to see the result
                                    println!("✓ Installation completed successfully!");
                                    println!("{}", "=".repeat(60));
                                    println!("\nPress Enter to return to pmgr...");
                                    let _ = io::stdout().flush();
                                    let mut input = String::new();
                                    let _ = io::stdin().read_line(&mut input);
                                } else if was_cancelled {
                                    // Cancelled - return automatically after short delay
                                    println!("⚠ Installation cancelled by user");
                                    println!("{}", "=".repeat(60));
                                    println!("\nReturning to pmgr in 3 seconds...");
                                    let _ = io::stdout().flush();
                                    std::thread::sleep(Duration::from_secs(3));
                                } else {
                                    // Failed - give user a moment to see error
                                    println!("✗ Installation failed");
                                    println!("{}", "=".repeat(60));
                                    println!("\nPress Enter to return to pmgr...");
                                    let _ = io::stdout().flush();
                                    let mut input = String::new();
                                    let _ = io::stdin().read_line(&mut input);
                                }

                                // Re-enter TUI
                                enable_raw_mode()?;
                                execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
                                terminal.clear()?;

                                // Clear cache and refresh FIRST
                                self.cached_installed = None;
                                self.refresh_current_view()?;

                                // Show result alert AFTER refresh (so it persists in the new App)
                                if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                                    if was_successful {
                                        app.alert.show(super::types::AlertType::Success,
                                            format!("✓ Successfully installed {} AUR package(s)", aur_packages.len()));
                                    } else if was_cancelled {
                                        app.alert.show(super::types::AlertType::Info,
                                            "⚠ AUR installation cancelled by user".to_string());
                                    } else {
                                        app.alert.show(super::types::AlertType::Error,
                                            "✗ AUR installation failed".to_string());
                                    }
                                }
                            }
                        }
                        ActionType::Remove => {
                            // For remove, use pkexec pacman directly (works for both AUR and official)
                            app.update_window.start_remove(&packages);
                        }
                    }
                }
            }

            // Always check for updates (even without key events)
            let mut need_view_refresh = false;
            let mut pending_alert: Option<(super::types::AlertType, String)> = None;

            if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                // Check for preview updates (so previews load automatically)
                app.check_preview_updates();

                // Check for update window updates
                app.update_window.check_updates();

                // Auto-close update window if completed successfully
                if app.update_window.should_auto_close() {
                    app.update_window.close(false); // Not cancelled by user
                    need_view_refresh = true; // Refresh view after successful operation
                }

                // Clear terminal if window was just closed to force full redraw
                if app.update_window.just_closed {
                    terminal.clear()?;

                    // Prepare alert based on operation result (will show after refresh)
                    if app.update_window.cancelled_by_user {
                        pending_alert = Some((super::types::AlertType::Info, "⚠ Operation cancelled by user".to_string()));
                    } else if app.update_window.was_successful {
                        // Show success message based on operation type
                        let message = if let Some(ref op_type) = app.update_window.operation_type {
                            if op_type.starts_with("remove_") {
                                let count = op_type.strip_prefix("remove_").unwrap_or("0");
                                format!("✓ Successfully removed {} package(s)", count)
                            } else if op_type.starts_with("install_official_") {
                                let count = op_type.strip_prefix("install_official_").unwrap_or("0");
                                format!("✓ Successfully installed {} official package(s)", count)
                            } else if op_type == "system_update" {
                                "✓ System updated successfully".to_string()
                            } else {
                                "✓ Operation completed successfully".to_string()
                            }
                        } else {
                            "✓ Operation completed successfully".to_string()
                        };
                        pending_alert = Some((super::types::AlertType::Success, message));
                    } else if let Some(_) = app.update_window.operation_type {
                        // Operation failed (not cancelled, not successful)
                        pending_alert = Some((super::types::AlertType::Error, "✗ Operation failed".to_string()));
                    }

                    app.update_window.clear_just_closed_flag();
                }
            }

            // Refresh view if needed (after window closes)
            if need_view_refresh {
                self.cached_installed = None;
                self.refresh_current_view()?;
            }

            // Show pending alert AFTER refresh (so it persists in the new App)
            if let Some((alert_type, message)) = pending_alert {
                if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                    app.alert.show(alert_type, message);
                }
            }
        }
    }

    /// Switch to a different view
    fn switch_to_view(&mut self, view_type: ViewType) -> Result<()> {
        self.selected_tab = view_type as usize;

        // Set loading state and pending load
        match view_type {
            ViewType::Home => {
                self.loading_state.start("Loading system information".to_string());
                self.current_view = ViewState::Home(HomeState::new());
                self.pending_load = PendingLoad::Home;
            }
            ViewType::Install => {
                self.loading_state.start("Loading available packages".to_string());
                // Create empty app temporarily
                self.current_view = ViewState::Install(App::new(
                    vec![],
                    true,
                    Some("echo {} | xargs yay -Si".to_string()),
                    ActionType::Install,
                ));
                self.pending_load = PendingLoad::Install;
            }
            ViewType::Remove => {
                self.loading_state.start("Loading installed packages".to_string());
                self.current_view = ViewState::Remove(App::new(
                    vec![],
                    true,
                    Some("echo {} | xargs yay -Qi".to_string()),
                    ActionType::Remove,
                ));
                self.pending_load = PendingLoad::Remove;
            }
            ViewType::List => {
                self.loading_state.start("Loading installed packages".to_string());
                self.current_view = ViewState::List(App::new(
                    vec![],
                    false,
                    Some("echo {} | xargs yay -Qi".to_string()),
                    ActionType::Install,
                ));
                self.pending_load = PendingLoad::List;
            }
        }

        Ok(())
    }

    /// Load home view statistics
    fn load_home_stats(&mut self) -> Result<()> {
        self.loading_state.start("Refreshing system information".to_string());

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

        self.loading_state.stop();
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

    /// Perform the actual home data load
    fn perform_home_load(&mut self) -> Result<()> {
        if let ViewState::Home(home_state) = &mut self.current_view {
            let installed = self.package_manager.list_installed()?;
            let available = self.package_manager.list_available()?;
            let updates_available = 0; // TODO: Implement

            home_state.set_stats(SystemStats {
                installed_count: installed.len(),
                available_count: available.len(),
                updates_available,
            });
        }
        self.loading_state.stop();
        Ok(())
    }

    /// Perform the actual install view data load
    fn perform_install_load(&mut self) -> Result<()> {
        let packages = self.get_or_load_available()?;
        let package_names: Vec<String> = packages
            .iter()
            .map(|p| format!("{}/{}", p.repository, p.name))
            .collect();

        let app = App::new(
            package_names,
            true,
            Some("echo {} | xargs yay -Si".to_string()),
            ActionType::Install,
        );

        self.current_view = ViewState::Install(app);
        self.loading_state.stop();
        Ok(())
    }

    /// Perform the actual remove view data load
    fn perform_remove_load(&mut self) -> Result<()> {
        let packages = self.get_or_load_installed()?;
        let app = App::new(
            packages,
            true,
            Some("echo {} | xargs yay -Qi".to_string()),
            ActionType::Remove,
        );

        self.current_view = ViewState::Remove(app);
        self.loading_state.stop();
        Ok(())
    }

    /// Perform the actual list view data load
    fn perform_list_load(&mut self) -> Result<()> {
        let packages = self.get_or_load_installed()?;
        let app = App::new(
            packages,
            false,
            Some("echo {} | xargs yay -Qi".to_string()),
            ActionType::Install,
        );

        self.current_view = ViewState::List(app);
        self.loading_state.stop();
        Ok(())
    }
}
