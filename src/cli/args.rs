use std::path::PathBuf;

use clap::{Parser, Subcommand};

use super::commands::{
    Add, Config, Delete, Doctor, Edit, Find, Init, List, Move, Renormalize, Show,
};

const TOP_LEVEL_HELP: &str = "\
Task Commands:
  add          Add a task
  list         List tasks
  find         Find tasks by text
  show         Show task details

Workflow Commands:
  move         Move a task to a different list
  delete       Delete a task permanently
  edit         Edit a task
  renormalize  Rebuild spaced order keys

Setup Commands:
  init         Initialize a new sqs project
  config       Show effective configuration and setup help
  doctor       Check configuration and task storage health
  tui          Launch interactive TUI dashboard

Help:
  help         Print this message or the help of the given subcommand(s)
";

#[derive(Debug, Parser)]
#[command(
    name = "sqs",
    version,
    about = "Reorder lists from the terminal",
    help_template = "{about-with-newline}\n{usage-heading} {usage}\n\nOptions:\n{options}{after-help}",
    after_help = TOP_LEVEL_HELP
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub root: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Add(Add),
    Init(Init),
    List(List),
    Move(Move),
    Delete(Delete),
    Edit(Edit),
    Show(Show),
    Find(Find),
    Config(Config),
    Doctor(Doctor),
    Renormalize(Renormalize),
    /// Launch interactive TUI dashboard
    Tui,
}
