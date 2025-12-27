#[derive(Debug, Clone)]
pub struct HomeState {
    pub scroll_position: u16,
    pub stats: Option<SystemStats>,
}

#[derive(Debug, Clone)]
pub struct SystemStats {
    pub installed_count: usize,
    pub available_count: usize,
    pub updates_available: usize,
}

impl HomeState {
    pub fn new() -> Self {
        Self {
            scroll_position: 0,
            stats: None,
        }
    }

    pub fn set_stats(&mut self, stats: SystemStats) {
        self.stats = Some(stats);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_position = self.scroll_position.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll_position = self.scroll_position.saturating_sub(1);
    }
}
