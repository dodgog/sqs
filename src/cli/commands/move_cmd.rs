use std::path::PathBuf;

use clap::Parser;

use crate::adapter::Adapter;
use crate::adapters::markdown_todolists::MarkdownTodolistsAdapter;
use crate::app::app_error::AppError;
use crate::cli::commands::helpers;
use crate::io::output;

#[derive(Debug, Parser)]
#[command(about = "Move an item to a different list")]
pub struct Move {
    pub task: Option<String>,

    pub list: Option<String>,
}

pub fn handle_move(Move { task, list }: Move, root: Option<PathBuf>) -> Result<(), AppError> {
    let resolved = helpers::resolve_config(root)?;
    let mut adapter = MarkdownTodolistsAdapter::new(resolved.tasks_root.clone());

    let query = task.ok_or_else(|| AppError::usage("item ID required"))?;
    let item = adapter.find_item(&query).or_else(|_| {
        // Try prefix match
        let items = adapter.scan()?;
        let matches: Vec<_> = items
            .iter()
            .filter(|i| i.ext_id.starts_with(&query))
            .collect();
        match matches.len() {
            1 => Ok(matches[0].clone()),
            0 => Err(AppError::not_found(&query)),
            _ => Err(AppError::ambiguous_task_ref(&query)),
        }
    })?;

    let target = list.ok_or_else(|| AppError::usage("target list required"))?;

    if item.list == target {
        output::print_info(&format!("{} is already in {target}", item.ext_id));
        return Ok(());
    }

    adapter.move_item(&item.ext_id, &target)?;
    output::print_info(&format!("Moved {} to {target}", item.ext_id));
    Ok(())
}
