use std::path::PathBuf;

use clap::Parser;

use crate::adapter::Adapter;
use crate::app::app_error::AppError;
use crate::cli::commands::helpers;

#[derive(Debug, Parser)]
#[command(about = "Find items by text")]
pub struct Find {
    pub query: String,
}

pub fn handle_find(Find { query }: Find, root: Option<PathBuf>) -> Result<(), AppError> {
    let adapter = helpers::build_adapter(root)?;
    let items = adapter.scan()?;
    let matches: Vec<_> = items.iter().filter(|i| i.matches_query(&query)).collect();

    if matches.is_empty() {
        println!("No items found");
    } else {
        for item in &matches {
            println!("[{}] {}  {}", item.list, item.ext_id, item.title);
        }
    }
    Ok(())
}
