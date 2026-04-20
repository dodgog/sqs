use std::path::PathBuf;

use clap::Parser;

use crate::adapter::Adapter;
use crate::adapters::markdown_todolists::MarkdownTodolistsAdapter;
use crate::app::app_error::AppError;
use crate::cli::commands::helpers;
use crate::io::{input, output};

#[derive(Debug, Parser)]
#[command(about = "Delete an item permanently")]
pub struct Delete {
    pub task: Option<String>,

    #[arg(long, short = 'i')]
    pub interactive: bool,
}

pub fn handle_delete(
    Delete { task, interactive }: Delete,
    root: Option<PathBuf>,
) -> Result<(), AppError> {
    let resolved = helpers::resolve_config(root)?;
    let mut adapter = MarkdownTodolistsAdapter::new(resolved.tasks_root.clone());

    let query = task.ok_or_else(|| AppError::usage("item ID required"))?;
    let item = helpers::resolve_item(&adapter, &query)?;

    if interactive {
        let confirmed = input::prompt_confirm(&format!("Permanently delete '{}'?", item.title))?;
        if !confirmed {
            output::print_info("Delete cancelled");
            return Ok(());
        }
    }

    adapter.delete_item(&item.ext_id)?;
    output::print_info(&format!("Deleted: {}", item.ext_id));
    Ok(())
}
