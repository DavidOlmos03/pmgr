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
            title: String::new(),
        }
    }

    /// Generic method to execute a command with arguments
    fn start_command(&mut self, command: String, args: Vec<String>, initial_message: &str, success_message: &str, title: &str) {
        self.active = true;
        self.output.clear();
        self.output.push(initial_message.to_string());
        self.completed = false;
        self.has_error = false;
        self.title = title.to_string();

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        let success_message = success_message.to_string();

        thread::spawn(move || {
            let mut child = match Command::new(&command)
                .args(&args)
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
                        let _ = tx.send(UpdateMessage::Output(format!("\n{}", success_message)));
                    } else {
                        let _ = tx.send(UpdateMessage::Output(format!("\n✗ Operation failed with code: {:?}", status.code())));
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

    pub fn start_update(&mut self) {
        self.start_command(
            "sudo".to_string(),
            vec!["pacman".to_string(), "-Syu".to_string(), "--noconfirm".to_string()],
            "Starting system update...",
            "✓ System update completed successfully!",
            "System Update"
        );
    }

    pub fn start_install(&mut self, packages: &[String]) {
        // Extract package names from "repository/package" format
        let package_names: Vec<String> = packages
            .iter()
            .map(|p| {
                // If package is in "repo/name" format, extract just the name
                if let Some(idx) = p.rfind('/') {
                    p[idx + 1..].to_string()
                } else {
                    p.clone()
                }
            })
            .collect();

        let mut args = vec!["-S".to_string(), "--noconfirm".to_string()];
        args.extend(package_names);

        self.start_command(
            "yay".to_string(),
            args,
            &format!("Installing {} package(s)...", packages.len()),
            "✓ Installation completed successfully!",
            "Installing Packages"
        );
    }

    pub fn start_remove(&mut self, packages: &[String]) {
        let mut args = vec!["-Rns".to_string(), "--noconfirm".to_string()];
        args.extend(packages.iter().map(|p| p.clone()));

        self.start_command(
            "yay".to_string(),
            args,
            &format!("Removing {} package(s)...", packages.len()),
            "✓ Removal completed successfully!",
            "Removing Packages"
        );
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
