pub mod frontmatter;
pub mod identity;
pub mod io;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use chrono::Utc;

use crate::adapter::{Adapter, EditOutcome, Item, ListDef};
use crate::app::app_error::AppError;
use crate::ordering;

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
        io::scan_dir(&self.root)
            .unwrap_or_default()
            .into_iter()
            .map(|i| i.ext_id)
            .collect()
    }

    fn sorted_lists(&self) -> Vec<ListDef> {
        let mut lists = io::read_lists_yaml(&self.root).unwrap_or_else(|_| io::default_lists());
        lists.sort_by(|a, b| a.order.partial_cmp(&b.order).unwrap_or(Ordering::Equal));
        lists
    }

    fn derive_list(&self, order_key: f64, lists_sorted: &[ListDef]) -> String {
        ordering::derive_list_for_order(order_key, lists_sorted)
            .or_else(|| lists_sorted.first().map(|l| l.name.as_str()))
            .unwrap_or(crate::adapter::DEFAULT_LIST)
            .to_string()
    }
}

impl Adapter for MarkdownTodolistsAdapter {
    fn name(&self) -> &str {
        "markdown-todolists"
    }

    fn lists(&self) -> Vec<ListDef> {
        self.sorted_lists()
    }

    fn set_lists(&mut self, lists: &[ListDef]) -> Result<(), AppError> {
        io::write_lists_yaml(&self.root, lists)?;
        io::ensure_list_dirs(&self.root, lists)?;
        Ok(())
    }

    fn scan(&self) -> Result<Vec<Item>, AppError> {
        let lists = self.sorted_lists();
        let mut items = io::scan_dir(&self.root)?;
        for item in &mut items {
            item.list = self.derive_list(item.order, &lists);
        }
        items.sort_by(|a, b| a.order.partial_cmp(&b.order).unwrap_or(Ordering::Equal));
        Ok(items)
    }

    fn find_item(&self, ext_id: &str) -> Result<Item, AppError> {
        let (fm, body) = self.read_item(ext_id)?;
        let lists = self.sorted_lists();
        let mut item = item_from_frontmatter(ext_id, &fm, &body);
        item.list = self.derive_list(fm.order, &lists);
        Ok(item)
    }

    fn create_item(
        &mut self,
        id: Option<&str>,
        title: &str,
        body: &str,
        order_key: f64,
    ) -> Result<(Item, PathBuf), AppError> {
        let lists = self.sorted_lists();
        let derived_list = self.derive_list(order_key, &lists);

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
            list: derived_list.clone(),
            order: order_key,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        let path = self.write_item_fm(&ext_id, &fm, body)?;
        let mut item = item_from_frontmatter(&ext_id, &fm, body);
        item.list = derived_list;
        Ok((item, path))
    }

    fn update_item_order(&mut self, ext_id: &str, order_key: f64) -> Result<(), AppError> {
        let lists = self.sorted_lists();
        let derived_list = self.derive_list(order_key, &lists);

        let (mut fm, body) = self.read_item(ext_id)?;
        let old_list = fm.list.clone();
        fm.order = order_key;
        fm.list = derived_list.clone();
        fm.updated_at = Utc::now();

        if old_list != derived_list {
            io::move_item_file(&self.root, ext_id, &derived_list)?;
        }
        self.write_item_fm(ext_id, &fm, &body)?;
        Ok(())
    }

    fn batch_update_orders(&mut self, updates: &[(String, f64)]) -> Result<(), AppError> {
        for (id, key) in updates {
            self.update_item_order(id, *key)?;
        }
        Ok(())
    }

    fn renormalize(&mut self) -> Result<(usize, usize), AppError> {
        let mut old_lists = self.sorted_lists();
        old_lists.sort_by(|a, b| a.order.partial_cmp(&b.order).unwrap_or(Ordering::Equal));

        let mut items = io::scan_dir(&self.root)?;
        items.sort_by(|a, b| a.order.partial_cmp(&b.order).unwrap_or(Ordering::Equal));

        let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
        for item in &items {
            let list_name = self.derive_list(item.order, &old_lists);
            grouped
                .entry(list_name)
                .or_default()
                .push(item.ext_id.clone());
        }

        let new_lists: Vec<ListDef> = old_lists
            .iter()
            .enumerate()
            .map(|(i, l)| ListDef {
                name: l.name.clone(),
                display: l.display.clone(),
                order: ordering::list_key_for_index(i),
                tags: l.tags.clone(),
            })
            .collect();

        let mut item_updates: Vec<(String, f64)> = Vec::new();
        for new_list in &new_lists {
            if let Some(ids) = grouped.get(&new_list.name) {
                for (j, id) in ids.iter().enumerate() {
                    item_updates.push((id.clone(), ordering::item_key_in_list(new_list.order, j)));
                }
            }
        }

        let list_count = new_lists.len();
        let item_count = item_updates.len();

        self.set_lists(&new_lists)?;
        self.batch_update_orders(&item_updates)?;
        Ok((list_count, item_count))
    }

    fn delete_item(&mut self, ext_id: &str) -> Result<(), AppError> {
        let path = self.find_path(ext_id)?;
        fs::remove_file(&path)?;
        Ok(())
    }

    fn set_item_tags(&mut self, ext_id: &str, tags: &[String]) -> Result<(), AppError> {
        let (mut fm, body) = self.read_item(ext_id)?;
        let mut deduped: Vec<String> = tags.iter().filter(|t| !t.is_empty()).cloned().collect();
        deduped.sort();
        deduped.dedup();
        fm.tags = deduped;
        fm.updated_at = Utc::now();
        self.write_item_fm(ext_id, &fm, &body)?;
        Ok(())
    }

    fn set_list_tags(&mut self, name: &str, tags: &[String]) -> Result<(), AppError> {
        let mut lists = self.sorted_lists();
        let Some(list) = lists.iter_mut().find(|l| l.name == name) else {
            return Err(AppError::usage(format!("unknown list: {name}")));
        };
        let mut deduped: Vec<String> = tags.iter().filter(|t| !t.is_empty()).cloned().collect();
        deduped.sort();
        deduped.dedup();
        list.tags = deduped;
        self.set_lists(&lists)
    }

    fn all_tags(&self) -> Result<Vec<String>, AppError> {
        let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for item in io::scan_dir(&self.root)? {
            for t in item.tags {
                set.insert(t);
            }
        }
        for list in self.sorted_lists() {
            for t in list.tags {
                set.insert(t);
            }
        }
        Ok(set.into_iter().collect())
    }

    fn delete_list(&mut self, name: &str) -> Result<(), AppError> {
        let lists = self.sorted_lists();
        if !lists.iter().any(|l| l.name == name) {
            return Err(AppError::usage(format!("unknown list: {name}")));
        }
        if lists.len() <= 1 {
            return Err(AppError::usage("cannot delete the last remaining list"));
        }
        let items = io::scan_dir(&self.root)?;
        let lists_for_derive = self.sorted_lists();
        let still_in_list = items
            .iter()
            .any(|i| self.derive_list(i.order, &lists_for_derive) == name);
        if still_in_list {
            return Err(AppError::usage(format!(
                "list '{name}' is not empty — move its items first"
            )));
        }
        let remaining: Vec<ListDef> = lists.into_iter().filter(|l| l.name != name).collect();
        self.set_lists(&remaining)?;
        let dir = self.root.join(name);
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh_adapter() -> (TempDir, MarkdownTodolistsAdapter) {
        let temp = TempDir::new().unwrap();
        let mut adapter = MarkdownTodolistsAdapter::new(temp.path().to_path_buf());
        adapter.set_lists(&io::default_lists()).unwrap();
        (temp, adapter)
    }

    #[test]
    fn create_item_derives_list_from_order_key() {
        let (_temp, mut adapter) = fresh_adapter();
        let (item, _) = adapter
            .create_item(Some("a001"), "in now", "", 1500.0)
            .unwrap();
        assert_eq!(item.list, "now");
        let (item2, _) = adapter
            .create_item(Some("a002"), "in inbox", "", 4500.0)
            .unwrap();
        assert_eq!(item2.list, "inbox");
    }

    #[test]
    fn update_item_order_rederives_list_and_moves_file() {
        let (temp, mut adapter) = fresh_adapter();
        let (item, _) = adapter
            .create_item(Some("a001"), "task", "", 1500.0)
            .unwrap();
        assert_eq!(item.list, "now");
        assert!(temp.path().join("now").join("a001.md").exists());

        adapter.update_item_order("a001", 4500.0).unwrap();
        let updated = adapter.find_item("a001").unwrap();
        assert_eq!(updated.list, "inbox");
        assert!(temp.path().join("inbox").join("a001.md").exists());
        assert!(!temp.path().join("now").join("a001.md").exists());
    }

    #[test]
    fn renormalize_is_idempotent() {
        let (_temp, mut adapter) = fresh_adapter();
        adapter.create_item(Some("a001"), "x", "", 1500.0).unwrap();
        adapter.create_item(Some("a002"), "y", "", 1700.0).unwrap();
        adapter.create_item(Some("b001"), "z", "", 4500.0).unwrap();

        adapter.renormalize().unwrap();
        let after_first: Vec<(String, f64)> = adapter
            .scan()
            .unwrap()
            .into_iter()
            .map(|i| (i.ext_id, i.order))
            .collect();
        adapter.renormalize().unwrap();
        let after_second: Vec<(String, f64)> = adapter
            .scan()
            .unwrap()
            .into_iter()
            .map(|i| (i.ext_id, i.order))
            .collect();
        assert_eq!(after_first, after_second);
    }

    #[test]
    fn renormalize_preserves_visible_order() {
        let (_temp, mut adapter) = fresh_adapter();
        adapter
            .create_item(Some("a001"), "x", "", 1500.123)
            .unwrap();
        adapter
            .create_item(Some("a002"), "y", "", 1700.456)
            .unwrap();
        adapter
            .create_item(Some("a003"), "z", "", 1900.789)
            .unwrap();

        let before: Vec<String> = adapter
            .scan()
            .unwrap()
            .into_iter()
            .map(|i| i.ext_id)
            .collect();
        adapter.renormalize().unwrap();
        let after: Vec<String> = adapter
            .scan()
            .unwrap()
            .into_iter()
            .map(|i| i.ext_id)
            .collect();
        assert_eq!(before, after);

        let lists = adapter.lists();
        let now = lists.iter().find(|l| l.name == "now").unwrap();
        assert!((now.order - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn set_item_tags_persists_through_scan() {
        let (_temp, mut adapter) = fresh_adapter();
        adapter.create_item(Some("a001"), "x", "", 1500.0).unwrap();
        adapter
            .set_item_tags("a001", &["MIL010".into(), "SCOPE-foo".into()])
            .unwrap();
        let items = adapter.scan().unwrap();
        let item = items.iter().find(|i| i.ext_id == "a001").unwrap();
        assert_eq!(item.tags, vec!["MIL010", "SCOPE-foo"]);
    }

    #[test]
    fn all_tags_returns_sorted_distinct() {
        let (_temp, mut adapter) = fresh_adapter();
        adapter.create_item(Some("a001"), "x", "", 1500.0).unwrap();
        adapter.create_item(Some("a002"), "y", "", 1700.0).unwrap();
        adapter
            .set_item_tags("a001", &["beta".into(), "alpha".into()])
            .unwrap();
        adapter
            .set_item_tags("a002", &["beta".into(), "gamma".into()])
            .unwrap();
        let tags = adapter.all_tags().unwrap();
        assert_eq!(tags, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn batch_update_orders_moves_items_atomically() {
        let (_temp, mut adapter) = fresh_adapter();
        adapter.create_item(Some("a001"), "x", "", 1500.0).unwrap();
        adapter.create_item(Some("a002"), "y", "", 1700.0).unwrap();
        adapter
            .batch_update_orders(&[("a001".to_string(), 4500.0), ("a002".to_string(), 4501.0)])
            .unwrap();
        let items = adapter.scan().unwrap();
        assert!(items.iter().all(|i| i.list == "inbox"));
    }
}
