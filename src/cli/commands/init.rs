use std::fs;
use std::path::PathBuf;

use clap::Parser;

use crate::adapters::markdown_todolists::io::{default_lists, write_lists_yaml};
use crate::app::app_error::AppError;
use crate::io::output;

#[derive(Debug, Parser)]
#[command(about = "Initialize a new sqs project in the current directory")]
pub struct Init;

pub fn handle_init(_: Init, root: Option<PathBuf>) -> Result<(), AppError> {
    let base =
        root.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let tasks_dir = base.join("tasks");
    let cache_dir = base.join("cache");
    let config_path = base.join("sqs.toml");

    if config_path.exists() {
        return Err(AppError::message(format!(
            "sqs.toml already exists at {}",
            config_path.display()
        )));
    }

    // Create directories
    fs::create_dir_all(&tasks_dir)?;
    fs::create_dir_all(&cache_dir)?;

    // Create list subdirectories
    let lists = default_lists();
    for list in &lists {
        fs::create_dir_all(tasks_dir.join(&list.name))?;
    }

    // Write sqs.toml
    fs::write(
        &config_path,
        "# sqs project configuration\ndefault_adapter = \"markdown-todolists\"\n\n[adapters.markdown-todolists]\nroot = \"./tasks\"\n",
    )?;

    // Write lists.yaml
    let lists = default_lists();
    write_lists_yaml(&tasks_dir, &lists)?;

    // Add cache/ to .gitignore
    let gitignore_path = base.join(".gitignore");
    let gitignore_entry = "cache/\n";
    if gitignore_path.exists() {
        let content = fs::read_to_string(&gitignore_path)?;
        if !content.contains("cache/") {
            fs::write(&gitignore_path, format!("{content}{gitignore_entry}"))?;
        }
    } else {
        fs::write(&gitignore_path, gitignore_entry)?;
    }

    output::print_info(&format!("Initialized sqs project at {}", base.display()));
    output::print_info(&format!("  Config: {}", config_path.display()));
    output::print_info(&format!("  Tasks:  {}", tasks_dir.display()));
    output::print_info(&format!("  Cache:  {}", cache_dir.display()));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_creates_project_structure() {
        let temp = TempDir::new().unwrap();
        handle_init(Init, Some(temp.path().to_path_buf())).unwrap();

        assert!(temp.path().join("sqs.toml").exists());
        assert!(temp.path().join("tasks").exists());
        assert!(temp.path().join("tasks/lists.yaml").exists());
        assert!(temp.path().join("cache").exists());
        assert!(temp.path().join(".gitignore").exists());

        let gitignore = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
        assert!(gitignore.contains("cache/"));
    }

    #[test]
    fn init_fails_if_config_exists() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("sqs.toml"), "existing").unwrap();

        let result = handle_init(Init, Some(temp.path().to_path_buf()));
        assert!(result.is_err());
    }

    #[test]
    fn init_appends_to_existing_gitignore() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join(".gitignore"), "*.log\n").unwrap();
        handle_init(Init, Some(temp.path().to_path_buf())).unwrap();

        let gitignore = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
        assert!(gitignore.contains("*.log"));
        assert!(gitignore.contains("cache/"));
    }
}
