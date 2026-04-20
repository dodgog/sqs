use crate::app::app_error::AppError;
use crate::io::output;
use crate::storage::config;

use super::args::{Cli, Command};
use super::commands::{
    add, config as config_cmd, delete, doctor, edit, find, init, list, move_cmd, show,
};

pub fn handle(cli: Cli) -> Result<(), AppError> {
    match cli.command {
        Some(Command::Add(command)) => add::handle_add(command, cli.root),
        Some(Command::Init(command)) => init::handle_init(command, cli.root),
        Some(Command::List(command)) => list::handle_list(command, cli.root),
        Some(Command::Move(command)) => move_cmd::handle_move(command, cli.root),
        Some(Command::Delete(command)) => delete::handle_delete(command, cli.root),
        Some(Command::Edit(command)) => edit::handle_edit(command, cli.root),
        Some(Command::Show(command)) => show::handle_show(command, cli.root),
        Some(Command::Find(command)) => find::handle_find(command, cli.root),
        Some(Command::Config(command)) => config_cmd::handle_config(command, cli.root),
        Some(Command::Doctor(command)) => doctor::handle_doctor(command, cli.root),
        Some(Command::Tui) => handle_tui(cli.root),
        None => handle_default(cli.root),
    }
}

fn handle_tui(root: Option<std::path::PathBuf>) -> Result<(), AppError> {
    let resolved = config::resolve(root)?;
    let adapter = Box::new(
        crate::adapters::markdown_todolists::MarkdownTodolistsAdapter::new(
            resolved.tasks_root.clone(),
        ),
    );
    crate::tui::run(adapter)
}

fn handle_default(root: Option<std::path::PathBuf>) -> Result<(), AppError> {
    let resolved = match config::resolve(root) {
        Ok(resolved) => resolved,
        Err(_) => {
            let inspection = config::inspect(None)?;
            output::print_getting_started(inspection.config_path.as_deref());
            return Ok(());
        }
    };

    let adapter = crate::adapters::markdown_todolists::MarkdownTodolistsAdapter::new(
        resolved.tasks_root.clone(),
    );
    let items = crate::adapter::Adapter::scan(&adapter)?;
    if items.is_empty() {
        let inspection = config::inspect(None)?;
        output::print_getting_started(inspection.config_path.as_deref());
    } else {
        let lists = crate::adapter::Adapter::lists(&adapter);
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
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::handle;
    use crate::cli::args::Cli;
    use crate::test_support::LockedEnv;
    use tempfile::TempDir;

    #[test]
    fn handle_shows_getting_started_when_no_command_and_no_config() {
        let _env = LockedEnv::new(&["XDG_CONFIG_HOME", "SQS_ROOT"]);
        handle(Cli {
            root: None,
            command: None,
        })
        .expect("bare invocation should succeed with getting-started guide");
    }

    #[test]
    fn handle_shows_getting_started_when_no_command_and_no_tasks() {
        let mut env = LockedEnv::new(&["XDG_CONFIG_HOME", "SQS_ROOT"]);
        let temp = TempDir::new().expect("temp dir should exist");
        env.set("SQS_ROOT", temp.path().as_os_str());

        handle(Cli {
            root: None,
            command: None,
        })
        .expect("bare invocation with empty repo should show dashboard");
    }
}
