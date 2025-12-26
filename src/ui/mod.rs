use anyhow::Result;
use skim::prelude::*;
use std::io::Cursor;

pub struct Selector;

impl Selector {
    /// Show interactive selector for packages
    pub fn select_packages(
        items: Vec<String>,
        prompt: &str,
        multi: bool,
        preview_cmd: Option<String>,
    ) -> Result<Vec<String>> {
        let items_str = items.join("\n");
        let item_reader = SkimItemReader::default();
        let items = item_reader.of_bufread(Cursor::new(items_str));

        let mut builder = SkimOptionsBuilder::default();
        builder
            .height(Some("100%"))
            .prompt(Some(prompt))
            .multi(multi)
            .reverse(true);

        if let Some(ref cmd) = preview_cmd {
            builder.preview(Some(cmd)).preview_window(Some("right:50%:wrap"));
        }

        let options = builder.build().unwrap();

        let output = Skim::run_with(&options, Some(items))
            .ok_or_else(|| anyhow::anyhow!("Selection cancelled"))?;

        if output.is_abort {
            return Ok(Vec::new());
        }

        let selected = output
            .selected_items
            .iter()
            .map(|item| item.output().to_string())
            .collect();

        Ok(selected)
    }

    /// Select from installed packages
    pub fn select_installed(packages: Vec<String>) -> Result<Vec<String>> {
        Self::select_packages(
            packages,
            "Select packages to remove (TAB: multi-select, ENTER: confirm): ",
            true,
            Some("echo {} | xargs yay -Qi".to_string()),
        )
    }

    /// Select from available packages
    pub fn select_available(packages: Vec<String>) -> Result<Vec<String>> {
        Self::select_packages(
            packages,
            "Select packages to install (TAB: multi-select, ENTER: confirm): ",
            true,
            Some("echo {} | xargs yay -Si".to_string()),
        )
    }

    /// Browse installed packages (view only)
    pub fn browse_installed(packages: Vec<String>) -> Result<Option<String>> {
        let result = Self::select_packages(
            packages,
            "Browse installed packages (ESC to exit): ",
            false,
            Some("echo {} | xargs yay -Qi".to_string()),
        )?;

        Ok(result.first().cloned())
    }
}
