use std::path::PathBuf;

use clap::Parser;

use crate::adapter::Adapter;
use crate::app::app_error::AppError;
use crate::cli::commands::helpers;
use crate::io::output;

#[derive(Debug, Parser)]
#[command(about = "List items")]
pub struct List {
    pub list: Option<String>,
}

pub fn handle_list(List { list }: List, root: Option<PathBuf>) -> Result<(), AppError> {
    let adapter = helpers::build_adapter(root)?;
    let items = adapter.scan()?;

    match list {
        Some(name) => {
            let filtered: Vec<_> = items.iter().filter(|i| i.list == name).collect();
            println!("{name} ({})", filtered.len());
            for item in &filtered {
                println!("  {}  {}", item.ext_id, item.title);
            }
        }
        None => {
            let lists = adapter.lists();
            for list_def in &lists {
                let list_items: Vec<_> = items.iter().filter(|i| i.list == list_def.name).collect();
                if !list_items.is_empty() {
                    println!("{} ({})", list_def.name, list_items.len());
                    for item in &list_items {
                        println!("  {}  {}", item.ext_id, item.title);
                    }
                    println!();
                }
            }
            if items.is_empty() {
                output::print_getting_started(None);
            }
        }
    }

    Ok(())
}
