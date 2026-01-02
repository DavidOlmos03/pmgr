use std::time::{Duration, Instant};

/// Spinner animation for loading states
#[derive(Debug, Clone)]
pub struct Spinner {
    frames: Vec<&'static str>,
    current_frame: usize,
    last_update: Instant,
    interval: Duration,
}

impl Spinner {
    /// Create a new spinner with braille dot animation
    pub fn new() -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            current_frame: 0,
            last_update: Instant::now(),
            interval: Duration::from_millis(80),
        }
    }

    /// Create a dots spinner
    pub fn dots() -> Self {
        Self {
            frames: vec!["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"],
            current_frame: 0,
            last_update: Instant::now(),
            interval: Duration::from_millis(80),
        }
    }

    /// Create a simple dots spinner
    pub fn simple_dots() -> Self {
        Self {
            frames: vec!["   ", ".  ", ".. ", "..."],
            current_frame: 0,
            last_update: Instant::now(),
            interval: Duration::from_millis(200),
        }
    }

    /// Create a line spinner
    pub fn line() -> Self {
        Self {
            frames: vec!["-", "\\", "|", "/"],
            current_frame: 0,
            last_update: Instant::now(),
            interval: Duration::from_millis(100),
        }
    }

    /// Create a box spinner
    pub fn boxed() -> Self {
        Self {
            frames: vec!["◰", "◳", "◲", "◱"],
            current_frame: 0,
            last_update: Instant::now(),
            interval: Duration::from_millis(100),
        }
    }

    /// Update the spinner animation
    pub fn tick(&mut self) {
        if self.last_update.elapsed() >= self.interval {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            self.last_update = Instant::now();
        }
    }

    /// Get the current frame
    pub fn current(&self) -> &str {
        self.frames[self.current_frame]
    }

    /// Reset the spinner to the first frame
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.last_update = Instant::now();
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

/// Loading state with spinner
#[derive(Debug, Clone)]
pub struct LoadingState {
    pub active: bool,
    pub message: String,
    pub spinner: Spinner,
}

impl LoadingState {
    pub fn new() -> Self {
        Self {
            active: false,
            message: String::new(),
            spinner: Spinner::new(),
        }
    }

    pub fn start(&mut self, message: String) {
        self.active = true;
        self.message = message;
        self.spinner.reset();
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.message.clear();
    }

    pub fn tick(&mut self) {
        if self.active {
            self.spinner.tick();
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for LoadingState {
    fn default() -> Self {
        Self::new()
    }
}
