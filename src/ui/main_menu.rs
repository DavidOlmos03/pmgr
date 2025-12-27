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
                    // Determine what action to take based on current view and key
                    let mut action = Action::None;

                    // Handle view-specific events
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

                            // Check if confirmation dialog was confirmed
                            if app.confirm_dialog.is_confirmed() {
                                let _packages = app.confirm_dialog.packages.clone();
                                app.confirm_dialog.cancel(); // Reset dialog

                                // Execute the action
                                match app.action_type {
                                    ActionType::Install => {
                                        // TODO: Execute install
                                        // For now, just refresh the view
                                        self.cached_installed = None;
                                    }
                                    ActionType::Remove => {
                                        // TODO: Execute remove
                                        // For now, just refresh the view
                                        self.cached_installed = None;
                                    }
                                }
                                action = Action::RefreshView;
                            }
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

                    // Check for preview updates in package views
                    if let ViewState::Install(app) | ViewState::Remove(app) | ViewState::List(app) = &mut self.current_view {
                        app.check_preview_updates();
                    }
                }
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
}
