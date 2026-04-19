use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::adapter::Item;
use crate::app::app_error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemFrontmatter {
    pub title: String,
    pub list: String,
    pub order: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub fn parse_item_file(input: &str) -> Result<(ItemFrontmatter, String), AppError> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with("---") {
        return Err(AppError::message("missing frontmatter start delimiter"));
    }

    let after_start = &trimmed[3..];
    let Some(end_pos) = after_start.find("\n---") else {
        return Err(AppError::message("missing frontmatter end delimiter"));
    };

    let yaml_str = &after_start[..end_pos];
    let body_start = end_pos + 4; // skip "\n---"
    let body = if body_start < after_start.len() {
        after_start[body_start..]
            .trim_start_matches('\n')
            .to_string()
    } else {
        String::new()
    };

    let fm: ItemFrontmatter = serde_yaml::from_str(yaml_str)
        .map_err(|e| AppError::message(format!("invalid frontmatter: {e}")))?;

    Ok((fm, body))
}

pub fn render_item_file(fm: &ItemFrontmatter, body: &str) -> String {
    let yaml = serde_yaml::to_string(fm).expect("frontmatter should serialize");
    format!("---\n{yaml}---\n{body}")
}

pub fn item_from_frontmatter(ext_id: &str, fm: &ItemFrontmatter, body: &str) -> Item {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    fm.title.hash(&mut hasher);
    fm.list.hash(&mut hasher);
    fm.order.to_bits().hash(&mut hasher);
    body.hash(&mut hasher);
    let content_hash = hasher.finish();

    Item {
        ext_id: ext_id.to_string(),
        title: fm.title.clone(),
        body: body.to_string(),
        list: fm.list.clone(),
        order: fm.order,
        content_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_fm() -> ItemFrontmatter {
        ItemFrontmatter {
            title: "Test task".into(),
            list: "now".into(),
            order: 1024.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn roundtrip_preserves_fields() {
        let fm = sample_fm();
        let body = "# Test task\n\nSome content\n";
        let rendered = render_item_file(&fm, body);
        let (parsed_fm, parsed_body) = parse_item_file(&rendered).unwrap();
        assert_eq!(parsed_fm.title, fm.title);
        assert_eq!(parsed_fm.list, fm.list);
        assert_eq!(parsed_fm.order, fm.order);
        assert_eq!(parsed_body, body);
    }

    #[test]
    fn parse_rejects_missing_frontmatter() {
        let result = parse_item_file("no frontmatter here");
        assert!(result.is_err());
    }

    #[test]
    fn parse_rejects_missing_end_delimiter() {
        let result = parse_item_file("---\ntitle: test\n");
        assert!(result.is_err());
    }

    #[test]
    fn content_hash_changes_with_content() {
        let fm = sample_fm();
        let item1 = item_from_frontmatter("a1", &fm, "body1");
        let item2 = item_from_frontmatter("a1", &fm, "body2");
        assert_ne!(item1.content_hash, item2.content_hash);
    }
}
