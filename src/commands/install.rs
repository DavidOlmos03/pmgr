use crate::package::PackageManager;
use crate::ui::Selector;
use anyhow::Result;
use colored::Colorize;

pub struct InstallCommand;

impl InstallCommand {
    pub fn execute(packages: Vec<String>, interactive: bool) -> Result<()> {
        let pm = PackageManager::new();

        if interactive || packages.is_empty() {
            // Interactive mode: show all available packages
            println!("{}", "Loading available packages...".cyan());
            let available = pm.list_available()?;

            let package_names: Vec<String> = available
                .iter()
                .map(|p| format!("{}/{}", p.repository, p.name))
                .collect();

            let selected = Selector::select_available(package_names)?;

            if selected.is_empty() {
                println!("{}", "No packages selected.".yellow());
                return Ok(());
            }

            // Extract package names (remove repository prefix)
            let to_install: Vec<String> = selected
                .iter()
                .map(|s| s.split('/').last().unwrap_or(s).to_string())
                .collect();

            println!(
                "\n{} {}",
                "Installing:".green().bold(),
                to_install.join(", ")
            );

            pm.install(&to_install)?;
            println!("{}", "Installation complete!".green());
        } else {
            // Direct mode: install specified packages
            println!(
                "{} {}",
                "Installing:".green().bold(),
                packages.join(", ")
            );
            pm.install(&packages)?;
            println!("{}", "Installation complete!".green());
        }

        Ok(())
    }
}
