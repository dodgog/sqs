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

    fn item_path(&self, ext_id: &str) -> PathBuf {
        self.root.join(format!("{ext_id}.md"))
    }

    fn read_item(&self, ext_id: &str) -> Result<(ItemFrontmatter, String), AppError> {
        let path = self.item_path(ext_id);
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
        io::write_lists_yaml(&self.root, lists)
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
                if self.item_path(id).exists() {
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
        self.write_item_fm(ext_id, &fm, &body)?;
        Ok(item_from_frontmatter(ext_id, &fm, &body))
    }

    fn reorder_items(&mut self, _list: &str, ordered_ids: &[String]) -> Result<(), AppError> {
        for (i, id) in ordered_ids.iter().enumerate() {
            let (mut fm, body) = self.read_item(id)?;
            fm.order = i as f64;
            self.write_item_fm(id, &fm, &body)?;
        }
        Ok(())
    }

    fn delete_item(&mut self, ext_id: &str) -> Result<(), AppError> {
        let path = self.item_path(ext_id);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    fn editor_path(&self, ext_id: &str) -> Result<PathBuf, AppError> {
        let path = self.item_path(ext_id);
        if !path.exists() {
            return Err(AppError::not_found(ext_id));
        }
        Ok(path)
    }

    fn apply_edit(
        &mut self,
        ext_id: &str,
        path: &std::path::Path,
        original_content: &str,
    ) -> Result<EditOutcome, AppError> {
        let edited_content = fs::read_to_string(path)?;

        if edited_content.trim().is_empty() {
            fs::write(path, original_content)?;
            return Err(AppError::message("file cannot be empty"));
        }

        if edited_content == original_content {
            return Ok(EditOutcome::Unchanged);
        }

        // Validate the edited frontmatter
        match parse_item_file(&edited_content) {
            Ok((mut fm, body)) => {
                fm.updated_at = Utc::now();
                self.write_item_fm(ext_id, &fm, &body)?;
                Ok(EditOutcome::Applied)
            }
            Err(e) => {
                fs::write(path, original_content)?;
                Err(AppError::message(format!("invalid file: {e}")))
            }
        }
    }

    fn finalize_add_edit(
        &mut self,
        ext_id: &str,
        path: &std::path::Path,
        original_content: &str,
    ) -> Result<(), AppError> {
        let edited_content = fs::read_to_string(path)?;

        if edited_content.trim().is_empty() {
            fs::write(path, original_content)?;
            return Err(AppError::message("file cannot be empty"));
        }

        if edited_content != original_content {
            match parse_item_file(&edited_content) {
                Ok((mut fm, body)) => {
                    fm.updated_at = Utc::now();
                    self.write_item_fm(ext_id, &fm, &body)?;
                }
                Err(e) => {
                    fs::write(path, original_content)?;
                    return Err(AppError::message(format!("invalid file: {e}")));
                }
            }
        }

        Ok(())
    }
}
