mod commands;
mod config;
mod package;
mod ui;

use clap::{Parser, Subcommand};
use colored::Colorize;

/// pmgr - Modern TUI package manager for Arch Linux
#[derive(Parser)]
#[command(name = "pmgr")]
#[command(author = "David")]
#[command(version = "0.1.0")]
#[command(about = "Modern TUI package manager for Arch Linux", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install packages (interactive by default)
    #[command(alias = "i")]
    Install {
        /// Package names to install
        packages: Vec<String>,

        /// Skip interactive mode
        #[arg(short = 'y', long)]
        no_interactive: bool,
    },

    /// Remove packages (interactive by default)
    #[command(alias = "r")]
    Remove {
        /// Package names to remove
        packages: Vec<String>,

        /// Skip interactive mode
        #[arg(short = 'y', long)]
        no_interactive: bool,
    },

    /// Search for packages
    #[command(alias = "s")]
    Search {
        /// Search query
        query: String,
    },

    /// List installed packages
    #[command(alias = "l")]
    List {
        /// Interactive browsing mode
        #[arg(short, long)]
        interactive: bool,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => match cmd {
            Commands::Install {
                packages,
                no_interactive,
            } => {
                commands::InstallCommand::execute(packages, !no_interactive)?;
            }
            Commands::Remove {
                packages,
                no_interactive,
            } => {
                commands::RemoveCommand::execute(packages, !no_interactive)?;
            }
            Commands::Search { query } => {
                commands::SearchCommand::execute(query)?;
            }
            Commands::List { interactive } => {
                commands::ListCommand::execute(interactive)?;
            }
        },
        None => {
            // No command provided - start interactive menu mode
            ui::MainMenu::run()?;
        }
    }

    Ok(())
}
