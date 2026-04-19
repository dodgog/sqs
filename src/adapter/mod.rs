use std::path::{Path, PathBuf};

use crate::app::app_error::AppError;

#[derive(Debug, Clone)]
pub struct Item {
    pub ext_id: String,
    pub title: String,
    pub body: String,
    pub list: String,
    pub order: f64,
    pub content_hash: u64,
}

#[derive(Debug, Clone)]
pub struct ListDef {
    pub name: String,
    pub display: String,
    pub order: f64,
}

/// Result of applying an edit: either the item was unchanged, or it was updated.
pub enum EditOutcome {
    Unchanged,
    Applied,
}

pub trait Adapter: Send {
    fn name(&self) -> &str;

    /// Get list definitions in display order.
    fn lists(&self) -> Vec<ListDef>;

    /// Persist list definitions (order, names).
    fn set_lists(&mut self, lists: &[ListDef]) -> Result<(), AppError>;

    /// Scan all items, returned in order (by list, then by order within list).
    fn scan(&self) -> Result<Vec<Item>, AppError>;

    /// Find a single item by ID.
    fn find_item(&self, ext_id: &str) -> Result<Item, AppError>;

    /// Create a new item. Returns the item and its file path.
    fn create_item(
        &mut self,
        id: Option<&str>,
        list: &str,
        title: &str,
        body: &str,
        order: f64,
    ) -> Result<(Item, PathBuf), AppError>;

    /// Move an item to a different list.
    fn move_item(&mut self, ext_id: &str, target_list: &str) -> Result<Item, AppError>;

    /// Set the display order of items within a list. `ordered_ids` is the desired order.
    fn reorder_items(&mut self, list: &str, ordered_ids: &[String]) -> Result<(), AppError>;

    /// Delete an item.
    fn delete_item(&mut self, ext_id: &str) -> Result<(), AppError>;

    /// Get the filesystem path for editing an item in $EDITOR.
    fn editor_path(&self, ext_id: &str) -> Result<PathBuf, AppError>;

    /// Apply an edit from $EDITOR. Validates and persists changes.
    fn apply_edit(
        &mut self,
        ext_id: &str,
        path: &Path,
        original_content: &str,
    ) -> Result<EditOutcome, AppError>;

    /// Finalize an edit made during item creation.
    fn finalize_add_edit(
        &mut self,
        ext_id: &str,
        path: &Path,
        original_content: &str,
    ) -> Result<(), AppError>;
}
