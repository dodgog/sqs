use std::path::PathBuf;

use clap::Parser;

use crate::adapter::Adapter;
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
    let mut adapter = helpers::build_adapter(root)?;

    let query = task.ok_or_else(|| AppError::usage("item ID required"))?;
    let item = helpers::resolve_item(&adapter, &query)?;

    let target = list.ok_or_else(|| AppError::usage("target list required"))?;

    if item.list == target {
        output::print_info(&format!("{} is already in {target}", item.ext_id));
        return Ok(());
    }

    let new_order = helpers::compute_bottom_key(&mut adapter, &target, true)?;
    adapter.update_item_order(&item.ext_id, new_order)?;
    output::print_info(&format!("Moved {} to {target}", item.ext_id));
    Ok(())
}
