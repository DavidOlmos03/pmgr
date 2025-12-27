use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PreviewLayout {
    Vertical,   // Preview on the right
    Horizontal, // Preview below
}

impl PreviewLayout {
    pub fn toggle_to_horizontal(&mut self) {
        *self = PreviewLayout::Horizontal;
    }

    pub fn toggle_to_vertical(&mut self) {
        *self = PreviewLayout::Vertical;
    }
}

#[derive(Debug)]
pub enum UpdateMessage {
    Output(String),
    Completed(bool), // true if successful, false if error
}

pub struct SystemUpdateWindow {
    pub active: bool,
    pub output: Vec<String>,
    pub completed: bool,
    pub has_error: bool,
    pub rx: Option<Receiver<UpdateMessage>>,
    pub just_closed: bool, // Flag to indicate we need to redraw
    pub title: String, // Window title
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionType {
    Install,
    Remove,
}

#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    pub active: bool,
    pub action_type: ActionType,
    pub packages: Vec<String>,
    pub confirmed: bool,
    pub scroll: u16,
}

impl ConfirmDialog {
    pub fn new() -> Self {
        Self {
            active: false,
            action_type: ActionType::Install,
            packages: Vec::new(),
            confirmed: false,
            scroll: 0,
        }
    }

    pub fn show(&mut self, action_type: ActionType, packages: Vec<String>) {
        self.active = true;
        self.action_type = action_type;
        self.packages = packages;
        self.confirmed = false;
        self.scroll = 0;
    }

    pub fn confirm(&mut self) {
        self.confirmed = true;
        self.active = false;
        self.scroll = 0;
    }

    pub fn cancel(&mut self) {
        self.confirmed = false;
        self.active = false;
        self.scroll = 0;
    }

    pub fn is_confirmed(&self) -> bool {
        self.confirmed
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewType {
    Home = 0,
    Install = 1,
    Remove = 2,
    List = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlertType {
    Success,
    Error,
    Info,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub active: bool,
    pub alert_type: AlertType,
    pub message: String,
}

impl Alert {
    pub fn new() -> Self {
        Self {
            active: false,
            alert_type: AlertType::Info,
            message: String::new(),
        }
    }

    pub fn show(&mut self, alert_type: AlertType, message: String) {
        self.active = true;
        self.alert_type = alert_type;
        self.message = message;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.message.clear();
    }
}
