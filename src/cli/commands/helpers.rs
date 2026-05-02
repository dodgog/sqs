use std::path::PathBuf;

use crate::adapter::{Adapter, Item};
use crate::app::app_error::AppError;
use crate::storage::{config, config::ResolvedConfig, editor::ResolvedEditor};

pub fn build_adapter(
    root: Option<PathBuf>,
) -> Result<crate::adapters::markdown_todolists::MarkdownTodolistsAdapter, AppError> {
    let resolved = resolve_config(root)?;
    Ok(crate::adapters::markdown_todolists::MarkdownTodolistsAdapter::new(resolved.tasks_root))
}

pub fn resolve_editor() -> Result<ResolvedEditor, AppError> {
    ResolvedEditor::resolve()
}

pub fn resolve_config(root: Option<PathBuf>) -> Result<ResolvedConfig, AppError> {
    config::resolve(root)
}

/// Resolve an item by exact ID, prefix match, or title match.
pub fn resolve_item(adapter: &dyn Adapter, query: &str) -> Result<Item, AppError> {
    if let Ok(item) = adapter.find_item(query) {
        return Ok(item);
    }
    let items = adapter.scan()?;
    let prefix_matches: Vec<_> = items
        .iter()
        .filter(|i| i.ext_id.starts_with(query))
        .collect();
    if prefix_matches.len() == 1 {
        return Ok(prefix_matches[0].clone());
    }
    let q = query.to_lowercase();
    let title_matches: Vec<_> = items
        .iter()
        .filter(|i| i.title.to_lowercase().contains(&q))
        .collect();
    if title_matches.len() == 1 {
        return Ok(title_matches[0].clone());
    }
    if !prefix_matches.is_empty() || !title_matches.is_empty() {
        return Err(AppError::ambiguous_task_ref(query));
    }
    Err(AppError::not_found(query))
}
