use std::{fs, path::PathBuf, process::Command};

use clap::Parser;

use crate::adapter::{Adapter, EditOutcome};
use crate::adapters::markdown_todolists::MarkdownTodolistsAdapter;
use crate::app::app_error::AppError;
use crate::cli::commands::helpers;
use crate::io::output;

#[derive(Debug, Parser)]
#[command(about = "Edit an item")]
pub struct Edit {
    pub task: Option<String>,
}

pub fn handle_edit(Edit { task }: Edit, root: Option<PathBuf>) -> Result<(), AppError> {
    let resolved = helpers::resolve_config(root)?;
    let mut adapter = MarkdownTodolistsAdapter::new(resolved.tasks_root.clone());

    let query = task.ok_or_else(|| AppError::usage("item ID required"))?;
    let item = helpers::resolve_item(&adapter, &query)?;
    let path = adapter.editor_path(&item.ext_id)?;

    let original_content = fs::read_to_string(&path)?;
    let editor = helpers::resolve_editor()?;
    let status = Command::new(&editor.program)
        .args(&editor.args)
        .arg(&path)
        .status()?;
    if !status.success() {
        return Err(AppError::message("editor command failed"));
    }

    match adapter.apply_edit(&item.ext_id, &path, &original_content)? {
        EditOutcome::Unchanged => {
            output::print_info(&format!("No changes: {}", item.ext_id));
        }
        EditOutcome::Applied => {
            output::print_info(&format!("Edited: {}", item.ext_id));
        }
    }
    Ok(())
}
