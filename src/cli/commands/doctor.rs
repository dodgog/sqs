use std::path::PathBuf;

use clap::Parser;

use crate::adapter::Adapter;
use crate::adapters::markdown_todolists::MarkdownTodolistsAdapter;
use crate::app::app_error::AppError;
use crate::cli::commands::helpers;
use crate::storage::editor::ResolvedEditor;

#[derive(Debug, Parser)]
#[command(about = "Check configuration and storage health")]
pub struct Doctor;

pub fn handle_doctor(_: Doctor, root: Option<PathBuf>) -> Result<(), AppError> {
    let resolved = helpers::resolve_config(root)?;
    let mut errors = 0;
    let mut ok = 0;

    // Config
    println!(
        "[ok] config: resolved tasks_root = {}",
        resolved.tasks_root.display()
    );
    ok += 1;

    // Adapter scan
    let adapter = MarkdownTodolistsAdapter::new(resolved.tasks_root.clone());
    match adapter.scan() {
        Ok(items) => {
            println!("[ok] scan: found {} items", items.len());
            ok += 1;
        }
        Err(e) => {
            println!("[error] scan: {e}");
            errors += 1;
        }
    }

    // Lists
    let lists = adapter.lists();
    println!("[ok] lists: {} lists defined", lists.len());
    ok += 1;

    // Editor
    match ResolvedEditor::resolve() {
        Ok(editor) => {
            println!("[ok] editor: resolved to '{}'", editor.program);
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
