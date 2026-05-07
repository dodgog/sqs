use std::path::PathBuf;

use clap::Parser;

use crate::adapter::Adapter;
use crate::app::app_error::AppError;
use crate::cli::commands::helpers;
use crate::io::output;

#[derive(Debug, Parser)]
#[command(about = "Rebuild spaced order keys for every list and item")]
pub struct Renormalize;

pub fn handle_renormalize(_cmd: Renormalize, root: Option<PathBuf>) -> Result<(), AppError> {
    let mut adapter = helpers::build_adapter(root)?;
    let (lists, items) = adapter.renormalize()?;
    output::print_info(&format!("Renormalized {lists} list(s), {items} item(s)"));
    Ok(())
}
