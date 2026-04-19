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
    fn lists(&self) -> Vec<ListDef>;
    fn scan(&self) -> Result<Vec<Item>, AppError>;
    fn find_item(&self, ext_id: &str) -> Result<Item, AppError>;
    fn create_item(
        &mut self,
        id: Option<&str>,
        list: &str,
        title: &str,
        body: &str,
    ) -> Result<(Item, PathBuf), AppError>;
    fn move_item(&mut self, ext_id: &str, target_list: &str) -> Result<Item, AppError>;
    fn delete_item(&mut self, ext_id: &str) -> Result<(), AppError>;
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
