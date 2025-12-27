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
}
