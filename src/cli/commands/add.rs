use std::{fs, path::PathBuf};

use clap::Parser;

use crate::adapter::Adapter;
use crate::app::app_error::AppError;
use crate::cli::commands::helpers;
use crate::io::{input, output};

#[derive(Debug, Parser)]
#[command(about = "Add an item")]
pub struct Add {
    pub title: Option<String>,

    #[arg(long)]
    pub list: Option<String>,

    #[arg(long)]
    pub no_edit: bool,

    #[arg(long)]
    pub content: Option<String>,

    #[arg(long, hide = true)]
    pub id: Option<String>,
}

pub fn handle_add(
    Add {
        title,
        list,
        no_edit,
        content,
        id,
    }: Add,
    root: Option<PathBuf>,
) -> Result<(), AppError> {
    let mut adapter = helpers::build_adapter(root)?;

    let title = match title {
        Some(title) => title,
        None => input::prompt_input("Title:")?,
    };

    let list = list.unwrap_or_else(|| crate::adapter::DEFAULT_LIST.to_string());
    let has_content = content.is_some();
    let body = content.unwrap_or_default();

    let (item, path) = adapter.create_item(id.as_deref(), &list, &title, &body, 0.0)?;

    if !no_edit && !has_content {
        let original_content = fs::read_to_string(&path)?;
        helpers::resolve_editor()?.open_file(&path)?;
        adapter.finalize_add_edit(&item.ext_id, &path, &original_content)?;
    }

    output::print_info(&format!("Created: {} ({})", item.ext_id, path.display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Add;
    use clap::Parser;

    #[test]
    fn parses_add_command() {
        let add = Add::parse_from(["add", "Ship v2"]);
        assert_eq!(add.title.as_deref(), Some("Ship v2"));
    }
}
