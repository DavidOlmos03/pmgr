use crate::package::PackageManager;
use anyhow::Result;
use colored::Colorize;

pub struct SearchCommand;

impl SearchCommand {
    pub fn execute(query: String) -> Result<()> {
        let pm = PackageManager::new();

        println!("{} '{}'...", "Searching for".cyan(), query);

        let results = pm.search(&query)?;

        if results.is_empty() {
            println!("{}", "No packages found.".yellow());
            return Ok(());
        }

        println!("\n{} packages found:\n", results.len().to_string().green());

        for pkg in results {
            println!(
                "{} {} {}",
                format!("{}/{}", pkg.repository, pkg.name).blue().bold(),
                pkg.version.green(),
                format!("[installed]").yellow()
            );
            println!("    {}", pkg.description.dimmed());
        }

        Ok(())
    }
}
