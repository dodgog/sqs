use std::path::PathBuf;

use clap::Parser;

use crate::adapter::Adapter;
use crate::adapters::markdown_todolists::MarkdownTodolistsAdapter;
use crate::app::app_error::AppError;
use crate::cli::commands::helpers;

#[derive(Debug, Parser)]
#[command(about = "Show item details")]
pub struct Show {
    pub task: Option<String>,
}

pub fn handle_show(Show { task }: Show, root: Option<PathBuf>) -> Result<(), AppError> {
    let resolved = helpers::resolve_config(root)?;
    let adapter = MarkdownTodolistsAdapter::new(resolved.tasks_root.clone());

    let query = task.ok_or_else(|| AppError::usage("item ID required"))?;
    let item = helpers::resolve_item(&adapter, &query)?;

    println!("ID:    {}", item.ext_id);
    println!("List:  {}", item.list);
    println!("Title: {}", item.title);
    if let Ok(path) = adapter.editor_path(&item.ext_id) {
        println!("Path:  {}", path.display());
    }
    println!();
    println!("{}", item.body);
    Ok(())
}
