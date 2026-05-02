use std::path::PathBuf;

use clap::Parser;

use crate::adapter::Adapter;
use crate::app::app_error::AppError;
use crate::cli::commands::helpers;
use crate::storage::editor::ResolvedEditor;

#[derive(Debug, Parser)]
#[command(about = "Check configuration and storage health")]
pub struct Doctor;

pub fn handle_doctor(_: Doctor, root: Option<PathBuf>) -> Result<(), AppError> {
    let adapter = helpers::build_adapter(root)?;
    let mut errors = 0;
    let mut ok = 0;

    println!("[ok] config resolved");
    ok += 1;

    match adapter.scan() {
        Ok(items) => {
            println!("[ok] scan: {} items", items.len());
            ok += 1;
        }
        Err(e) => {
            println!("[error] scan: {e}");
            errors += 1;
        }
    }

    println!("[ok] lists: {} defined", adapter.lists().len());
    ok += 1;

    match ResolvedEditor::resolve() {
        Ok(editor) => {
            println!("[ok] editor: '{}'", editor.program);
            ok += 1;
        }
        Err(e) => {
            println!("[error] editor: {e}");
            errors += 1;
        }
    }

    println!("summary: {ok} ok, {errors} error(s)");
    if errors > 0 {
        return Err(AppError::message(format!("doctor found {errors} error(s)")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Doctor;
    use clap::Parser;

    #[test]
    fn parses_doctor_command() {
        Doctor::parse_from(["doctor"]);
    }
}
