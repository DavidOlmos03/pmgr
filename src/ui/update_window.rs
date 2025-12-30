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
            cancelled_by_user: false,
            operation_type: None,
            was_successful: false,
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
            // Log the command being executed for debugging
            let _ = tx.send(UpdateMessage::Output(format!("Executing: {} {}", command, args.join(" "))));
            let _ = tx.send(UpdateMessage::Output(String::new())); // Empty line for readability

            let mut child = match Command::new(&command)
                .args(&args)
                .stdin(Stdio::null()) // Polkit will handle authentication via GUI
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
        self.operation_type = Some("system_update".to_string());
        self.start_command(
            "pkexec".to_string(),
            vec!["pacman".to_string(), "-Syu".to_string(), "--noconfirm".to_string()],
            "Starting system update...",
            "✓ System update completed successfully!",
            "System Update"
        );
    }

    pub fn start_install_official(&mut self, packages: &[String]) {
        self.operation_type = Some(format!("install_official_{}", packages.len()));

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

        let mut args = vec!["pacman".to_string(), "-S".to_string(), "--noconfirm".to_string()];
        args.extend(package_names);

        self.start_command(
            "pkexec".to_string(),
            args,
            &format!("Installing {} official package(s)...", packages.len()),
            "✓ Installation completed successfully!",
            "Installing Official Packages"
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

        let mut args = vec![
            "-S".to_string(),
            "--noconfirm".to_string(),
            "--answerdiff".to_string(), "None".to_string(),
            "--answerclean".to_string(), "None".to_string(),
            "--answeredit".to_string(), "None".to_string(),
            "--answerupgrade".to_string(), "None".to_string(),
            "--removemake".to_string(),
        ];
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
        self.operation_type = Some(format!("remove_{}", packages.len()));

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

        let mut args = vec!["pacman".to_string(), "-Rns".to_string(), "--noconfirm".to_string()];
        args.extend(package_names);

        self.start_command(
            "pkexec".to_string(),
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

    pub fn close(&mut self, cancelled_by_user: bool) {
        // Capture success state before clearing
        self.was_successful = self.completed && !self.has_error;

        self.active = false;
        self.output.clear();
        self.completed = false;
        self.has_error = false;
        self.rx = None;
        self.just_closed = true;
        self.cancelled_by_user = cancelled_by_user;
        // Keep operation_type and was_successful for showing alert
    }

    pub fn clear_just_closed_flag(&mut self) {
        self.just_closed = false;
        self.cancelled_by_user = false;
        self.operation_type = None;
        self.was_successful = false;
    }
}
