use std::path::PathBuf;

use clap::Parser;

use crate::adapter::Adapter;
use crate::adapters::markdown_todolists::MarkdownTodolistsAdapter;
use crate::app::app_error::AppError;
use crate::cli::commands::helpers;

#[derive(Debug, Parser)]
#[command(about = "Find items by text")]
pub struct Find {
    pub query: String,
}

pub fn handle_find(Find { query }: Find, root: Option<PathBuf>) -> Result<(), AppError> {
    let resolved = helpers::resolve_config(root)?;
    let adapter = MarkdownTodolistsAdapter::new(resolved.tasks_root.clone());
    let items = adapter.scan()?;
    let q = query.to_lowercase();
    let matches: Vec<_> = items
        .iter()
        .filter(|i| {
            i.title.to_lowercase().contains(&q)
                || i.ext_id.to_lowercase().contains(&q)
                || i.body.to_lowercase().contains(&q)
        })
        .collect();

    if matches.is_empty() {
        println!("No items found");
    } else {
        for item in &matches {
            println!("[{}] {}  {}", item.list, item.ext_id, item.title);
        }
    }
    Ok(())
}
