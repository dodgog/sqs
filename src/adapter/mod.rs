use std::path::{Path, PathBuf};

pub const DEFAULT_LIST: &str = "inbox";

use crate::app::app_error::AppError;

/// A task item. `list` is a denormalized cache rederived from `order` against
/// the current list markers; `order` is the global source of truth.
#[derive(Debug, Clone)]
pub struct Item {
    pub ext_id: String,
    pub title: String,
    pub body: String,
    pub list: String,
    pub order: f64,
    pub tags: Vec<String>,
    pub content_hash: u64,
}

/// A list marker in the global ordering. `order` is in the same numeric space
/// as item keys; an item belongs to the list with the largest `order` ≤ its own.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListDef {
    pub name: String,
    pub display: String,
    pub order: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl Item {
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.title.to_lowercase().contains(&q)
            || self.ext_id.to_lowercase().contains(&q)
            || self.body.to_lowercase().contains(&q)
            || self.tags.iter().any(|t| t.to_lowercase().contains(&q))
    }
}

pub enum EditOutcome {
    Unchanged,
    Applied,
}

pub trait Adapter: Send {
    fn name(&self) -> &str;

    fn lists(&self) -> Vec<ListDef>;

    /// Persist list definitions. Caller is responsible for assigning order
    /// keys consistent with the unified ordering invariants.
    fn set_lists(&mut self, lists: &[ListDef]) -> Result<(), AppError>;

    /// Scan all items. Each returned `Item.list` is rederived from the global
    /// ordering, even if the on-disk frontmatter says otherwise.
    fn scan(&self) -> Result<Vec<Item>, AppError>;

    fn find_item(&self, ext_id: &str) -> Result<Item, AppError>;

    /// Create a new item at the given global `order_key`. The list is
    /// derived from `order_key` against the current list markers.
    fn create_item(
        &mut self,
        id: Option<&str>,
        title: &str,
        body: &str,
        order_key: f64,
    ) -> Result<(Item, PathBuf), AppError>;

    /// Set an item's global `order_key`. The list field (and the file's
    /// directory) are rederived from the new key.
    fn update_item_order(&mut self, ext_id: &str, order_key: f64) -> Result<(), AppError>;

    /// Bulk update. Used for block moves (reordering a list carries items)
    /// and renormalization.
    fn batch_update_orders(&mut self, updates: &[(String, f64)]) -> Result<(), AppError>;

    /// Rebuild spaced order_keys for every list and item from their current
    /// visible order. Idempotent. Used by `sqs renormalize` and as a
    /// recovery step when an insert midpoint underflows EPSILON.
    fn renormalize(&mut self) -> Result<(usize, usize), AppError>;

    fn delete_item(&mut self, ext_id: &str) -> Result<(), AppError>;

    /// Remove a list. The list must be empty — callers that want to drop a
    /// non-empty list should first move its items elsewhere.
    fn delete_list(&mut self, name: &str) -> Result<(), AppError>;

    /// Replace an item's tag set with `tags`. Empty `tags` clears them.
    fn set_item_tags(&mut self, ext_id: &str, tags: &[String]) -> Result<(), AppError>;

    /// Replace a list's tag set with `tags`. Empty `tags` clears them.
    fn set_list_tags(&mut self, name: &str, tags: &[String]) -> Result<(), AppError>;

    /// Distinct tag names across all items and lists, sorted alphabetically.
    fn all_tags(&self) -> Result<Vec<String>, AppError>;

    fn editor_path(&self, ext_id: &str) -> Result<PathBuf, AppError>;

    fn apply_edit(
        &mut self,
        ext_id: &str,
        path: &Path,
        original_content: &str,
    ) -> Result<EditOutcome, AppError>;

    fn finalize_add_edit(
        &mut self,
        ext_id: &str,
        path: &Path,
        original_content: &str,
    ) -> Result<(), AppError>;
}
