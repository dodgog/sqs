use std::fs;
use std::path::{Path, PathBuf};

use crate::adapter::{Item, ListDef};
use crate::app::app_error::AppError;

use super::frontmatter::{
    ItemFrontmatter, item_from_frontmatter, parse_item_file, render_item_file,
};

/// Scan all list subdirectories for .md files and parse them into Items.
pub fn scan_dir(root: &Path) -> Result<Vec<Item>, AppError> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        // Scan subdirectories (each is a list)
        if path.is_dir() {
            scan_subdir(&path, &mut items);
        }

        // Also scan root-level .md files (flat-folder compat)
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            scan_file(&path, &mut items);
        }
    }

    items.sort_by(|a, b| {
        a.order
            .partial_cmp(&b.order)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(items)
}

fn scan_subdir(dir: &Path, items: &mut Vec<Item>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            scan_file(&path, items);
        }
    }
}

fn scan_file(path: &Path, items: &mut Vec<Item>) {
    let ext_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    match fs::read_to_string(path) {
        Ok(content) => match parse_item_file(&content) {
            Ok((fm, body)) => {
                items.push(item_from_frontmatter(&ext_id, &fm, &body));
            }
            Err(e) => {
                eprintln!("Warning: skipping malformed file {}: {e}", path.display());
            }
        },
        Err(e) => {
            eprintln!("Warning: could not read {}: {e}", path.display());
        }
    }
}

/// Write an item into its list subdirectory.
pub fn write_item_file(
    root: &Path,
    ext_id: &str,
    fm: &ItemFrontmatter,
    body: &str,
) -> Result<PathBuf, AppError> {
    let dir = root.join(&fm.list);
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{ext_id}.md"));
    let content = render_item_file(fm, body);
    fs::write(&path, content)?;
    Ok(path)
}

/// Find an item's file path by scanning (it could be in any subdirectory or root).
pub fn find_item_path(root: &Path, ext_id: &str) -> Option<PathBuf> {
    let filename = format!("{ext_id}.md");

    // Check root level
    let root_path = root.join(&filename);
    if root_path.exists() {
        return Some(root_path);
    }

    // Check subdirectories
    let Ok(entries) = fs::read_dir(root) else {
        return None;
    };
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.is_dir() {
            let sub_path = path.join(&filename);
            if sub_path.exists() {
                return Some(sub_path);
            }
        }
    }

    None
}

/// Move an item file from its current location to the target list subdirectory.
pub fn move_item_file(root: &Path, ext_id: &str, target_list: &str) -> Result<PathBuf, AppError> {
    let Some(old_path) = find_item_path(root, ext_id) else {
        return Err(AppError::not_found(ext_id));
    };

    let target_dir = root.join(target_list);
    fs::create_dir_all(&target_dir)?;
    let new_path = target_dir.join(format!("{ext_id}.md"));

    if old_path != new_path {
        fs::rename(&old_path, &new_path)?;
    }

    Ok(new_path)
}

/// Read lists.yaml from the adapter root.
pub fn read_lists_yaml(root: &Path) -> Result<Vec<ListDef>, AppError> {
    let path = root.join("lists.yaml");
    if !path.exists() {
        return Ok(default_lists());
    }
    let content = fs::read_to_string(&path)?;
    let lists: Vec<ListDef> = serde_yaml::from_str(&content)
        .map_err(|e| AppError::message(format!("invalid lists.yaml: {e}")))?;
    Ok(lists)
}

pub fn write_lists_yaml(root: &Path, lists: &[ListDef]) -> Result<(), AppError> {
    let path = root.join("lists.yaml");
    let content = serde_yaml::to_string(&lists)
        .map_err(|e| AppError::message(format!("failed to serialize lists.yaml: {e}")))?;
    fs::write(path, content)?;
    Ok(())
}

/// Ensure all list directories exist.
pub fn ensure_list_dirs(root: &Path, lists: &[ListDef]) -> Result<(), AppError> {
    for list in lists {
        fs::create_dir_all(root.join(&list.name))?;
    }
    Ok(())
}

pub fn default_lists() -> Vec<ListDef> {
    use crate::ordering::list_key_for_index;
    [
        ("now", "Now"),
        ("next", "Next"),
        ("later", "Later"),
        ("inbox", "Inbox"),
        ("done", "Done"),
    ]
    .iter()
    .enumerate()
    .map(|(i, (name, display))| ListDef {
        name: (*name).into(),
        display: (*display).into(),
        order: list_key_for_index(i),
        tags: Vec::new(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn scan_empty_dir_returns_empty() {
        let temp = TempDir::new().unwrap();
        let items = scan_dir(temp.path()).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn scan_nonexistent_dir_returns_empty() {
        let items = scan_dir(Path::new("/nonexistent/path")).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn write_and_scan_roundtrip() {
        let temp = TempDir::new().unwrap();
        let fm = super::super::frontmatter::ItemFrontmatter {
            title: "Test".into(),
            list: "now".into(),
            order: 1.0,
            tags: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        write_item_file(temp.path(), "abc1234", &fm, "# Test\n").unwrap();

        // File should be in now/ subdirectory
        assert!(temp.path().join("now").join("abc1234.md").exists());

        let items = scan_dir(temp.path()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].ext_id, "abc1234");
        assert_eq!(items[0].title, "Test");
        assert_eq!(items[0].list, "now");
    }

    #[test]
    fn move_item_between_lists() {
        let temp = TempDir::new().unwrap();
        let fm = super::super::frontmatter::ItemFrontmatter {
            title: "Test".into(),
            list: "now".into(),
            order: 1.0,
            tags: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        write_item_file(temp.path(), "abc1", &fm, "body\n").unwrap();
        assert!(temp.path().join("now").join("abc1.md").exists());

        move_item_file(temp.path(), "abc1", "done").unwrap();
        assert!(!temp.path().join("now").join("abc1.md").exists());
        assert!(temp.path().join("done").join("abc1.md").exists());
    }

    #[test]
    fn lists_yaml_roundtrip() {
        let temp = TempDir::new().unwrap();
        let lists = default_lists();
        write_lists_yaml(temp.path(), &lists).unwrap();
        let loaded = read_lists_yaml(temp.path()).unwrap();
        assert_eq!(loaded.len(), 5);
        assert_eq!(loaded[0].name, "now");
        assert_eq!(loaded[4].name, "done");
    }

    #[test]
    fn missing_lists_yaml_returns_defaults() {
        let temp = TempDir::new().unwrap();
        let lists = read_lists_yaml(temp.path()).unwrap();
        assert_eq!(lists.len(), 5);
    }
}
