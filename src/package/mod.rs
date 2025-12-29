use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub repository: String,
}

pub struct PackageManager {
    use_yay: bool,
}

impl PackageManager {
    pub fn new() -> Self {
        let use_yay = Command::new("which")
            .arg("yay")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        Self { use_yay }
    }

    fn get_cmd(&self) -> &str {
        if self.use_yay {
            "yay"
        } else {
            "pacman"
        }
    }

    /// List all available packages
    pub fn list_available(&self) -> Result<Vec<Package>> {
        let output = Command::new(self.get_cmd())
            .args(["-Sl"])
            .output()
            .context("Failed to list available packages")?;

        if !output.status.success() {
            anyhow::bail!("Package manager command failed");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    Some(Package {
                        repository: parts[0].to_string(),
                        name: parts[1].to_string(),
                        version: parts[2].to_string(),
                        description: parts.get(3..).map(|s| s.join(" ")).unwrap_or_default(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(packages)
    }

    /// List installed packages
    pub fn list_installed(&self) -> Result<Vec<String>> {
        let output = Command::new(self.get_cmd())
            .args(["-Qq"])
            .output()
            .context("Failed to list installed packages")?;

        if !output.status.success() {
            anyhow::bail!("Package manager command failed");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout.lines().map(|s| s.to_string()).collect();

        Ok(packages)
    }

    /// Get package info
    pub fn get_info(&self, package: &str, installed: bool) -> Result<String> {
        let flag = if installed { "-Qi" } else { "-Si" };

        let output = Command::new(self.get_cmd())
            .args([flag, package])
            .output()
            .context("Failed to get package info")?;

        if !output.status.success() {
            anyhow::bail!("Package not found: {}", package);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Install packages
    pub fn install(&self, packages: &[String]) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        let mut cmd = Command::new(self.get_cmd());
        cmd.arg("-S");

        for pkg in packages {
            cmd.arg(pkg);
        }

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd.status().context("Failed to install packages")?;

        if !status.success() {
            anyhow::bail!("Installation failed");
        }

        Ok(())
    }

    /// Check if a package is from AUR (not in official repos)
    pub fn is_aur_package(&self, package: &str) -> bool {
        // Extract package name from "repository/package" format
        let pkg_name = if let Some(idx) = package.rfind('/') {
            &package[idx + 1..]
        } else {
            package
        };

        // Try to get info from official repos using pacman
        // If it succeeds, it's an official package. If it fails, it's AUR.
        Command::new("pacman")
            .args(["-Si", pkg_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| !status.success()) // If pacman -Si fails, it's AUR
            .unwrap_or(true) // On error, assume AUR
    }

    /// Separate packages into AUR and official repos
    pub fn separate_packages(&self, packages: &[String]) -> (Vec<String>, Vec<String>) {
        let mut aur_packages = Vec::new();
        let mut official_packages = Vec::new();

        for pkg in packages {
            if self.is_aur_package(pkg) {
                aur_packages.push(pkg.clone());
            } else {
                official_packages.push(pkg.clone());
            }
        }

        (aur_packages, official_packages)
    }

    /// Remove packages
    pub fn remove(&self, packages: &[String]) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        let mut cmd = Command::new(self.get_cmd());
        cmd.arg("-Rns");

        for pkg in packages {
            cmd.arg(pkg);
        }

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd.status().context("Failed to remove packages")?;

        if !status.success() {
            anyhow::bail!("Removal failed");
        }

        Ok(())
    }

    /// Search packages
    pub fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new(self.get_cmd())
            .args(["-Ss", query])
            .output()
            .context("Failed to search packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        let mut current_pkg: Option<Package> = None;

        for line in stdout.lines() {
            if line.starts_with(' ') {
                // Description line
                if let Some(ref mut pkg) = current_pkg {
                    pkg.description = line.trim().to_string();
                    packages.push(pkg.clone());
                    current_pkg = None;
                }
            } else {
                // Package name line
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    let name_parts: Vec<&str> = parts[0].split('/').collect();
                    if name_parts.len() >= 2 {
                        current_pkg = Some(Package {
                            repository: name_parts[0].to_string(),
                            name: name_parts[1].to_string(),
                            version: parts.get(1).unwrap_or(&"").to_string(),
                            description: String::new(),
                        });
                    }
                }
            }
        }

        Ok(packages)
    }
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new()
    }
}
