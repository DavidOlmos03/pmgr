use super::types::{SystemUpdateWindow, UpdateMessage};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

impl SystemUpdateWindow {
    pub fn new() -> Self {
        Self {
            active: false,
            output: Vec::new(),
            completed: false,
            has_error: false,
            rx: None,
            just_closed: false,
        }
    }

    pub fn start_update(&mut self) {
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

    pub fn check_updates(&mut self) {
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

    pub fn should_auto_close(&self) -> bool {
        self.completed && !self.has_error
    }

    pub fn close(&mut self) {
        self.active = false;
        self.output.clear();
        self.completed = false;
        self.has_error = false;
        self.rx = None;
        self.just_closed = true;
    }

    pub fn clear_just_closed_flag(&mut self) {
        self.just_closed = false;
    }
}
