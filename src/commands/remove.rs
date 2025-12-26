use crate::package::PackageManager;
use crate::ui::Selector;
use anyhow::Result;
use colored::Colorize;

pub struct RemoveCommand;

impl RemoveCommand {
    pub fn execute(packages: Vec<String>, interactive: bool) -> Result<()> {
        let pm = PackageManager::new();

        if interactive || packages.is_empty() {
            // Interactive mode: show installed packages
            println!("{}", "Loading installed packages...".cyan());
            let installed = pm.list_installed()?;

            if installed.is_empty() {
                println!("{}", "No packages installed.".yellow());
                return Ok(());
            }

            let selected = Selector::select_installed(installed)?;

            if selected.is_empty() {
                println!("{}", "No packages selected.".yellow());
                return Ok(());
            }

            println!(
                "\n{} {}",
                "Removing:".red().bold(),
                selected.join(", ")
            );

            pm.remove(&selected)?;
            println!("{}", "Removal complete!".green());
        } else {
            // Direct mode: remove specified packages
            println!(
                "{} {}",
                "Removing:".red().bold(),
                packages.join(", ")
            );
            pm.remove(&packages)?;
            println!("{}", "Removal complete!".green());
        }

        Ok(())
    }
}
