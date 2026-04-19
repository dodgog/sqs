use std::fs;
use std::path::{Path, PathBuf};

use crate::adapter::{Item, ListDef};
use crate::app::app_error::AppError;

use super::frontmatter::{
    ItemFrontmatter, item_from_frontmatter, parse_item_file, render_item_file,
};

/// Scan a flat directory for .md files and parse them into Items.
pub fn scan_dir(root: &Path) -> Result<Vec<Item>, AppError> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    let entries = fs::read_dir(root)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let ext_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        match fs::read_to_string(&path) {
            Ok(content) => match parse_item_file(&content) {
                Ok((fm, body)) => {
                    items.push(item_from_frontmatter(&ext_id, &fm, &body));
                }
                Err(e) => {
                    eprintln!(
                        "Warning: skipping malformed task file {}: {e}",
                        path.display()
                    );
                }
            },
            Err(e) => {
                eprintln!("Warning: could not read {}: {e}", path.display());
            }
        }
    }

    items.sort_by(|a, b| {
        a.list.cmp(&b.list).then(
            a.order
                .partial_cmp(&b.order)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });
    Ok(items)
}

/// Write an item to disk as a .md file with YAML frontmatter.
pub fn write_item_file(
    root: &Path,
    ext_id: &str,
    fm: &ItemFrontmatter,
    body: &str,
) -> Result<PathBuf, AppError> {
    fs::create_dir_all(root)?;
    let path = root.join(format!("{ext_id}.md"));
    let content = render_item_file(fm, body);
    fs::write(&path, content)?;
    Ok(path)
}

/// Read lists.yaml from the adapter root.
pub fn read_lists_yaml(root: &Path) -> Result<Vec<ListDef>, AppError> {
    let path = root.join("lists.yaml");
    if !path.exists() {
        return Ok(default_lists());
    }
    let content = fs::read_to_string(&path)?;
    let lists: Vec<ListDefYaml> = serde_yaml::from_str(&content)
        .map_err(|e| AppError::message(format!("invalid lists.yaml: {e}")))?;
    Ok(lists.into_iter().map(|l| l.into()).collect())
}

/// Write lists.yaml to the adapter root.
pub fn write_lists_yaml(root: &Path, lists: &[ListDef]) -> Result<(), AppError> {
    let path = root.join("lists.yaml");
    let yaml_lists: Vec<ListDefYaml> = lists.iter().map(|l| l.into()).collect();
    let content = serde_yaml::to_string(&yaml_lists)
        .map_err(|e| AppError::message(format!("failed to serialize lists.yaml: {e}")))?;
    fs::write(path, content)?;
    Ok(())
}

pub fn default_lists() -> Vec<ListDef> {
    vec![
        ListDef {
            name: "inbox".into(),
            display: "Inbox".into(),
            order: 0.0,
        },
        ListDef {
            name: "now".into(),
            display: "Now".into(),
            order: 1.0,
        },
        ListDef {
            name: "next".into(),
            display: "Next".into(),
            order: 2.0,
        },
        ListDef {
            name: "later".into(),
            display: "Later".into(),
            order: 3.0,
        },
        ListDef {
            name: "done".into(),
            display: "Done".into(),
            order: 4.0,
        },
    ]
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ListDefYaml {
    name: String,
    display: String,
    order: f64,
}

impl From<ListDefYaml> for ListDef {
    fn from(l: ListDefYaml) -> Self {
        ListDef {
            name: l.name,
            display: l.display,
            order: l.order,
        }
    }
}

impl From<&ListDef> for ListDefYaml {
    fn from(l: &ListDef) -> Self {
        ListDefYaml {
            name: l.name.clone(),
            display: l.display.clone(),
            order: l.order,
        }
    }
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
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        write_item_file(temp.path(), "abc1234", &fm, "# Test\n").unwrap();

        let items = scan_dir(temp.path()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].ext_id, "abc1234");
        assert_eq!(items[0].title, "Test");
        assert_eq!(items[0].list, "now");
    }

    #[test]
    fn lists_yaml_roundtrip() {
        let temp = TempDir::new().unwrap();
        let lists = default_lists();
        write_lists_yaml(temp.path(), &lists).unwrap();
        let loaded = read_lists_yaml(temp.path()).unwrap();
        assert_eq!(loaded.len(), 5);
        assert_eq!(loaded[0].name, "inbox");
        assert_eq!(loaded[4].name, "done");
    }

    #[test]
    fn missing_lists_yaml_returns_defaults() {
        let temp = TempDir::new().unwrap();
        let lists = read_lists_yaml(temp.path()).unwrap();
        assert_eq!(lists.len(), 5);
    }
}
