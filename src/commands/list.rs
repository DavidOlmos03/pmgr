use crate::package::PackageManager;
use crate::ui::Selector;
use anyhow::Result;
use colored::Colorize;

pub struct ListCommand;

impl ListCommand {
    pub fn execute(interactive: bool) -> Result<()> {
        let pm = PackageManager::new();

        println!("{}", "Loading installed packages...".cyan());
        let installed = pm.list_installed()?;

        if installed.is_empty() {
            println!("{}", "No packages installed.".yellow());
            return Ok(());
        }

        if interactive {
            // Interactive browsing mode
            Selector::browse_installed(installed)?;
        } else {
            // Simple list mode
            println!(
                "\n{} packages installed:\n",
                installed.len().to_string().green().bold()
            );
            for pkg in installed {
                println!("  {}", pkg);
            }
        }

        Ok(())
    }
}
