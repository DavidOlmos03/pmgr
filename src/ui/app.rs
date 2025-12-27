use super::types::{ActionType, ConfirmDialog, PreviewLayout, SystemUpdateWindow};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::widgets::ListState;
use std::collections::HashMap;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub struct App {
    pub items: Vec<String>,
    pub filtered_items: Vec<(String, i64)>, // (item, score)
    pub list_state: ListState,
    pub search_query: String,
    pub selected_indices: Vec<usize>, // For multi-select
    pub multi: bool,
    pub preview_cmd: Option<String>,
    pub preview_content: String,
    pub preview_cache: HashMap<String, String>, // Cache for loaded previews
    pub preview_tx: Option<Sender<(String, String)>>, // Send preview requests
    pub preview_rx: Option<Receiver<(String, String)>>, // Receive preview results
    pub layout: PreviewLayout,
    pub matcher: SkimMatcherV2,
    pub current_preview_item: Option<String>, // Track current item being previewed
    pub update_window: SystemUpdateWindow,
    pub help_visible: bool, // Flag to show help screen
    pub help_scroll: u16, // Vertical scroll position for help window
    pub confirm_dialog: ConfirmDialog, // Confirmation dialog for install/remove
    pub action_type: ActionType, // Type of action (install/remove)
}

impl App {
    pub fn new(items: Vec<String>, multi: bool, preview_cmd: Option<String>, action_type: ActionType) -> Self {
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
            help_visible: false,
            help_scroll: 0,
            confirm_dialog: ConfirmDialog::new(),
            action_type,
        };

        app.request_preview();
        app
    }

    pub fn filter_items(&mut self) {
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

    pub fn next(&mut self) {
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

    pub fn previous(&mut self) {
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

    pub fn toggle_select(&mut self) {
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

    pub fn get_selected_items(&self) -> Vec<String> {
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

    pub fn request_preview(&mut self) {
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

    pub fn check_preview_updates(&mut self) {
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
