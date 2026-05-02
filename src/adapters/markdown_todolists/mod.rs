pub mod frontmatter;
pub mod identity;
pub mod io;

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use chrono::Utc;

use crate::adapter::{Adapter, EditOutcome, Item, ListDef};
use crate::app::app_error::AppError;

use frontmatter::{ItemFrontmatter, item_from_frontmatter, parse_item_file};

pub struct MarkdownTodolistsAdapter {
    root: PathBuf,
}

impl MarkdownTodolistsAdapter {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn find_path(&self, ext_id: &str) -> Result<PathBuf, AppError> {
        io::find_item_path(&self.root, ext_id).ok_or_else(|| AppError::not_found(ext_id))
    }

    fn read_item(&self, ext_id: &str) -> Result<(ItemFrontmatter, String), AppError> {
        let path = self.find_path(ext_id)?;
        let content = fs::read_to_string(&path).map_err(|_| AppError::not_found(ext_id))?;
        parse_item_file(&content)
    }

    fn write_item_fm(
        &self,
        ext_id: &str,
        fm: &ItemFrontmatter,
        body: &str,
    ) -> Result<PathBuf, AppError> {
        io::write_item_file(&self.root, ext_id, fm, body)
    }

    fn validate_and_write(
        &self,
        ext_id: &str,
        path: &std::path::Path,
        content: &str,
        original: &str,
    ) -> Result<(), AppError> {
        match parse_item_file(content) {
            Ok((mut fm, body)) => {
                fm.updated_at = Utc::now();
                self.write_item_fm(ext_id, &fm, &body)?;
                Ok(())
            }
            Err(e) => {
                fs::write(path, original)?;
                Err(AppError::message(format!("invalid file: {e}")))
            }
        }
    }

    fn existing_ids(&self) -> HashSet<String> {
        self.scan()
            .unwrap_or_default()
            .into_iter()
            .map(|i| i.ext_id)
            .collect()
    }
}

impl Adapter for MarkdownTodolistsAdapter {
    fn name(&self) -> &str {
        "markdown-todolists"
    }

    fn lists(&self) -> Vec<ListDef> {
        io::read_lists_yaml(&self.root).unwrap_or_else(|_| io::default_lists())
    }

    fn set_lists(&mut self, lists: &[ListDef]) -> Result<(), AppError> {
        io::write_lists_yaml(&self.root, lists)?;
        io::ensure_list_dirs(&self.root, lists)?;
        Ok(())
    }

    fn scan(&self) -> Result<Vec<Item>, AppError> {
        io::scan_dir(&self.root)
    }

    fn find_item(&self, ext_id: &str) -> Result<Item, AppError> {
        let (fm, body) = self.read_item(ext_id)?;
        Ok(item_from_frontmatter(ext_id, &fm, &body))
    }

    fn create_item(
        &mut self,
        id: Option<&str>,
        list: &str,
        title: &str,
        body: &str,
        order: f64,
    ) -> Result<(Item, PathBuf), AppError> {
        let ext_id = match id {
            Some(id) => {
                if io::find_item_path(&self.root, id).is_some() {
                    return Err(AppError::usage(format!("id '{id}' already exists")));
                }
                id.to_string()
            }
            None => identity::generate_id(&self.existing_ids()),
        };

        let now = Utc::now();
        let fm = ItemFrontmatter {
            title: title.to_string(),
            list: list.to_string(),
            order,
            created_at: now,
            updated_at: now,
        };

        let path = self.write_item_fm(&ext_id, &fm, body)?;
        let item = item_from_frontmatter(&ext_id, &fm, body);
        Ok((item, path))
    }

    fn move_item(&mut self, ext_id: &str, target_list: &str) -> Result<Item, AppError> {
        let (mut fm, body) = self.read_item(ext_id)?;
        fm.list = target_list.to_string();
        fm.updated_at = Utc::now();
        // Move file to target list directory
        io::move_item_file(&self.root, ext_id, target_list)?;
        // Rewrite frontmatter in new location
        self.write_item_fm(ext_id, &fm, &body)?;
        Ok(item_from_frontmatter(ext_id, &fm, &body))
    }

    fn reorder_items(&mut self, _list: &str, ordered_ids: &[String]) -> Result<(), AppError> {
        for (i, id) in ordered_ids.iter().enumerate() {
            let new_order = i as f64;
            let (mut fm, body) = self.read_item(id)?;
            if (fm.order - new_order).abs() > f64::EPSILON {
                fm.order = new_order;
                self.write_item_fm(id, &fm, &body)?;
            }
        }
        Ok(())
    }

    fn delete_item(&mut self, ext_id: &str) -> Result<(), AppError> {
        let path = self.find_path(ext_id)?;
        fs::remove_file(&path)?;
        Ok(())
    }

    fn editor_path(&self, ext_id: &str) -> Result<PathBuf, AppError> {
        self.find_path(ext_id)
    }

    fn apply_edit(
        &mut self,
        ext_id: &str,
        path: &std::path::Path,
        original_content: &str,
    ) -> Result<EditOutcome, AppError> {
        let edited = fs::read_to_string(path)?;
        if edited.trim().is_empty() {
            fs::write(path, original_content)?;
            return Err(AppError::message("file cannot be empty"));
        }
        if edited == original_content {
            return Ok(EditOutcome::Unchanged);
        }
        self.validate_and_write(ext_id, path, &edited, original_content)?;
        Ok(EditOutcome::Applied)
    }

    fn finalize_add_edit(
        &mut self,
        ext_id: &str,
        path: &std::path::Path,
        original_content: &str,
    ) -> Result<(), AppError> {
        let edited = fs::read_to_string(path)?;
        if edited.trim().is_empty() {
            fs::write(path, original_content)?;
            return Err(AppError::message("file cannot be empty"));
        }
        if edited != original_content {
            self.validate_and_write(ext_id, path, &edited, original_content)?;
        }
        Ok(())
    }
}
