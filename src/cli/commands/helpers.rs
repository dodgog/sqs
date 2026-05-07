use std::path::PathBuf;

use crate::adapter::{Adapter, Item};
use crate::app::app_error::AppError;
use crate::ordering;
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

/// Compute the order_key for appending to the bottom of `list`.
/// If the list is full (would collide with the next list marker) and
/// `auto_renorm` is true, renormalizes the adapter and retries.
pub fn compute_bottom_key(
    adapter: &mut dyn Adapter,
    list: &str,
    auto_renorm: bool,
) -> Result<f64, AppError> {
    let candidate = bottom_key_candidate(adapter, list)?;
    if let Some(key) = candidate {
        return Ok(key);
    }
    if !auto_renorm {
        return Err(AppError::message(format!(
            "list '{list}' has no room — run `sqs renormalize`"
        )));
    }
    adapter.renormalize()?;
    bottom_key_candidate(adapter, list)?
        .ok_or_else(|| AppError::message(format!("list '{list}' has no room after renormalize")))
}

fn bottom_key_candidate(adapter: &dyn Adapter, list: &str) -> Result<Option<f64>, AppError> {
    let lists = adapter.lists();
    let mut sorted = lists.clone();
    sorted.sort_by(|a, b| {
        a.order
            .partial_cmp(&b.order)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let pos = sorted
        .iter()
        .position(|l| l.name == list)
        .ok_or_else(|| AppError::usage(format!("unknown list: {list}")))?;
    let list_key = sorted[pos].order;
    let next_marker = sorted
        .get(pos + 1)
        .map(|l| l.order)
        .unwrap_or(f64::INFINITY);

    let items = adapter.scan()?;
    let max_in_list = items
        .iter()
        .filter(|i| i.list == list)
        .map(|i| i.order)
        .fold(list_key, f64::max);
    let candidate = max_in_list + 1.0;
    if candidate >= next_marker - ordering::EPSILON {
        Ok(None)
    } else {
        Ok(Some(candidate))
    }
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
