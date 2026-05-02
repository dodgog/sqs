use crate::storage::config::ConfigInspection;
use dialoguer::console::style;
use std::path::Path;

pub fn print_info(message: &str) {
    println!("{}", style(message).cyan());
}

pub fn print_config_inspection(inspection: &ConfigInspection) {
    match &inspection.config_path {
        Some(path) => println!("config_path = {}", path.display()),
        None => println!("config_path = <unavailable>"),
    }

    println!(
        "config_file = {}",
        if inspection.file_exists {
            "present"
        } else {
            "missing"
        }
    );

    match &inspection.explicit_root {
        Some(path) => println!("root_cli = {}", path.display()),
        None => println!("root_cli = <unset>"),
    }

    match &inspection.env_root {
        Some(path) => println!("root_env = {}", path.display()),
        None => println!("root_env = <unset>"),
    }

    match &inspection.resolved {
        Some(config) => {
            println!();
            println!("tasks_root = {}", config.tasks_root.display());
        }
        None => {
            println!("tasks_root = <unset>");
            println!();
            println!(
                "{}",
                crate::storage::config::starter_config(inspection.config_path.as_deref())
            );
        }
    }
}

pub fn print_getting_started(config_path: Option<&Path>) {
    println!("{}", style("Welcome to sqs!").bold());
    println!();
    println!("{}", crate::storage::config::starter_config(config_path));
    println!(
        "Run {} for detailed configuration info.",
        style("sqs config").cyan()
    );
}
