use std::path::Path;

use rusqlite::{Connection, params};

use crate::adapter::{Item, ListDef};
use crate::app::app_error::AppError;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS lists (
    name TEXT PRIMARY KEY,
    display TEXT NOT NULL,
    order_key REAL NOT NULL UNIQUE
);
CREATE TABLE IF NOT EXISTS items (
    ext_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    list TEXT NOT NULL,
    order_key REAL NOT NULL UNIQUE,
    content_hash INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS item_tags (
    ext_id TEXT NOT NULL,
    tag    TEXT NOT NULL,
    PRIMARY KEY (ext_id, tag)
);
CREATE INDEX IF NOT EXISTS idx_items_order ON items(order_key);
CREATE INDEX IF NOT EXISTS idx_lists_order ON lists(order_key);
CREATE INDEX IF NOT EXISTS idx_item_tags_tag ON item_tags(tag);
CREATE VIEW IF NOT EXISTS entries AS
    SELECT 'list' AS kind, name AS id, display AS title, NULL AS list, order_key
        FROM lists
    UNION ALL
    SELECT 'item' AS kind, ext_id AS id, title, list, order_key
        FROM items;
";

pub struct SqliteCache {
    conn: Connection,
}

impl SqliteCache {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let cache = Self { conn };
        cache.conn.execute_batch(SCHEMA)?;
        Ok(cache)
    }

    pub fn open_in_memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory()?;
        let cache = Self { conn };
        cache.conn.execute_batch(SCHEMA)?;
        Ok(cache)
    }

    pub fn upsert_lists(&self, lists: &[ListDef]) -> Result<(), AppError> {
        self.conn.execute("DELETE FROM lists", [])?;
        let mut stmt = self
            .conn
            .prepare("INSERT INTO lists (name, display, order_key) VALUES (?1, ?2, ?3)")?;
        for list in lists {
            stmt.execute(params![list.name, list.display, list.order])?;
        }
        Ok(())
    }

    pub fn upsert_items(&self, items: &[Item]) -> Result<(), AppError> {
        let mut stmt = self.conn.prepare(
            "INSERT OR REPLACE INTO items (ext_id, title, list, order_key, content_hash) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for item in items {
            stmt.execute(params![
                item.ext_id,
                item.title,
                item.list,
                item.order,
                item.content_hash as i64,
            ])?;
        }
        for item in items {
            self.conn.execute(
                "DELETE FROM item_tags WHERE ext_id = ?1",
                params![item.ext_id],
            )?;
            for tag in &item.tags {
                self.conn.execute(
                    "INSERT OR IGNORE INTO item_tags (ext_id, tag) VALUES (?1, ?2)",
                    params![item.ext_id, tag],
                )?;
            }
        }
        Ok(())
    }

    /// Distinct tag names across all cached items, sorted.
    pub fn query_tags(&self) -> Result<Vec<String>, AppError> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT tag FROM item_tags ORDER BY tag")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Items carrying *all* of `tags`. Empty `tags` returns every item.
    pub fn query_items_with_all_tags(&self, tags: &[String]) -> Result<Vec<Item>, AppError> {
        if tags.is_empty() {
            return self.query_items(None);
        }
        let placeholders: Vec<String> = (1..=tags.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "SELECT i.ext_id, i.title, i.list, i.order_key, i.content_hash
             FROM items i
             JOIN item_tags t ON t.ext_id = i.ext_id
             WHERE t.tag IN ({})
             GROUP BY i.ext_id
             HAVING COUNT(DISTINCT t.tag) = ?{}
             ORDER BY i.order_key",
            placeholders.join(","),
            tags.len() + 1
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut binds: Vec<rusqlite::types::Value> = tags
            .iter()
            .map(|t| rusqlite::types::Value::from(t.clone()))
            .collect();
        binds.push(rusqlite::types::Value::from(tags.len() as i64));
        let rows = stmt.query_map(rusqlite::params_from_iter(binds), map_item_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn query_lists(&self) -> Result<Vec<ListDef>, AppError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, display, order_key FROM lists ORDER BY order_key")?;
        let rows = stmt.query_map([], |row| {
            Ok(ListDef {
                name: row.get(0)?,
                display: row.get(1)?,
                order: row.get(2)?,
                tags: Vec::new(),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn query_items(&self, list: Option<&str>) -> Result<Vec<Item>, AppError> {
        match list {
            Some(l) => {
                let mut stmt = self.conn.prepare(
                    "SELECT ext_id, title, list, order_key, content_hash FROM items WHERE list = ?1 ORDER BY order_key",
                )?;
                let rows = stmt.query_map(params![l], map_item_row)?;
                rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT ext_id, title, list, order_key, content_hash FROM items ORDER BY order_key",
                )?;
                let rows = stmt.query_map([], map_item_row)?;
                rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
            }
        }
    }

    /// Derive the list name an item with `order_key` would belong to.
    /// Returns the name of the list with the largest `order_key <= input`.
    pub fn derive_list_for_order(&self, order_key: f64) -> Result<Option<String>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM lists WHERE order_key <= ?1 ORDER BY order_key DESC LIMIT 1",
        )?;
        let result = stmt
            .query_row(params![order_key], |row| row.get::<_, String>(0))
            .ok();
        Ok(result)
    }

    pub fn remove_stale(
        &self,
        current_ids: &std::collections::HashSet<String>,
    ) -> Result<usize, AppError> {
        let existing = self.query_items(None)?;
        let stale: Vec<_> = existing
            .iter()
            .filter(|item| !current_ids.contains(&item.ext_id))
            .collect();
        let count = stale.len();
        for item in &stale {
            self.conn
                .execute("DELETE FROM items WHERE ext_id = ?1", params![item.ext_id])?;
            self.conn.execute(
                "DELETE FROM item_tags WHERE ext_id = ?1",
                params![item.ext_id],
            )?;
        }
        Ok(count)
    }

    pub fn reconcile(&self, items: &[Item], lists: &[ListDef]) -> Result<(), AppError> {
        self.upsert_lists(lists)?;
        self.upsert_items(items)?;
        let current_ids: std::collections::HashSet<String> =
            items.iter().map(|i| i.ext_id.clone()).collect();
        self.remove_stale(&current_ids)?;
        Ok(())
    }
}

fn map_item_row(row: &rusqlite::Row) -> rusqlite::Result<Item> {
    Ok(Item {
        ext_id: row.get(0)?,
        title: row.get(1)?,
        body: String::new(),
        list: row.get(2)?,
        order: row.get(3)?,
        tags: Vec::new(),
        content_hash: row.get::<_, i64>(4)? as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_lists() -> Vec<ListDef> {
        vec![
            ListDef {
                name: "inbox".into(),
                display: "Inbox".into(),
                order: 1000.0,
                tags: Vec::new(),
            },
            ListDef {
                name: "now".into(),
                display: "Now".into(),
                order: 2000.0,
                tags: Vec::new(),
            },
        ]
    }

    fn sample_items() -> Vec<Item> {
        vec![
            Item {
                ext_id: "a1".into(),
                title: "Task A".into(),
                body: "body".into(),
                list: "inbox".into(),
                order: 1001.0,
                tags: Vec::new(),
                content_hash: 100,
            },
            Item {
                ext_id: "b2".into(),
                title: "Task B".into(),
                body: "body".into(),
                list: "now".into(),
                order: 2001.0,
                tags: Vec::new(),
                content_hash: 200,
            },
        ]
    }

    #[test]
    fn open_and_migrate() {
        SqliteCache::open_in_memory().unwrap();
    }

    #[test]
    fn upsert_and_query_lists() {
        let cache = SqliteCache::open_in_memory().unwrap();
        cache.upsert_lists(&sample_lists()).unwrap();
        let lists = cache.query_lists().unwrap();
        assert_eq!(lists.len(), 2);
    }

    #[test]
    fn upsert_and_query_items() {
        let cache = SqliteCache::open_in_memory().unwrap();
        cache.upsert_items(&sample_items()).unwrap();
        let all = cache.query_items(None).unwrap();
        assert_eq!(all.len(), 2);
        let inbox = cache.query_items(Some("inbox")).unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].ext_id, "a1");
    }

    #[test]
    fn remove_stale() {
        let cache = SqliteCache::open_in_memory().unwrap();
        cache.upsert_items(&sample_items()).unwrap();
        let mut current = std::collections::HashSet::new();
        current.insert("a1".to_string());
        assert_eq!(cache.remove_stale(&current).unwrap(), 1);
        assert_eq!(cache.query_items(None).unwrap().len(), 1);
    }

    #[test]
    fn reconcile_inserts_updates_and_removes() {
        let cache = SqliteCache::open_in_memory().unwrap();
        cache.upsert_items(&sample_items()).unwrap();
        let updated = vec![
            Item {
                ext_id: "a1".into(),
                title: "Updated".into(),
                body: String::new(),
                list: "now".into(),
                order: 2002.0,
                tags: Vec::new(),
                content_hash: 101,
            },
            Item {
                ext_id: "c3".into(),
                title: "Task C".into(),
                body: String::new(),
                list: "inbox".into(),
                order: 1002.0,
                tags: Vec::new(),
                content_hash: 300,
            },
        ];
        cache.reconcile(&updated, &sample_lists()).unwrap();
        let items = cache.query_items(None).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|i| i.ext_id == "a1"));
        assert!(items.iter().any(|i| i.ext_id == "c3"));
    }

    #[test]
    fn open_file_based() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("cache/test.db");
        let cache = SqliteCache::open(&path).unwrap();
        cache.upsert_lists(&sample_lists()).unwrap();
        drop(cache);
        let cache2 = SqliteCache::open(&path).unwrap();
        assert_eq!(cache2.query_lists().unwrap().len(), 2);
    }

    #[test]
    fn tags_upsert_and_query() {
        let cache = SqliteCache::open_in_memory().unwrap();
        let items = vec![
            Item {
                ext_id: "a1".into(),
                title: "A".into(),
                body: String::new(),
                list: "now".into(),
                order: 1001.0,
                tags: vec!["alpha".into(), "beta".into()],
                content_hash: 1,
            },
            Item {
                ext_id: "b2".into(),
                title: "B".into(),
                body: String::new(),
                list: "now".into(),
                order: 1002.0,
                tags: vec!["beta".into()],
                content_hash: 2,
            },
        ];
        cache.upsert_items(&items).unwrap();
        assert_eq!(cache.query_tags().unwrap(), vec!["alpha", "beta"]);
        let only_beta = cache.query_items_with_all_tags(&["beta".into()]).unwrap();
        assert_eq!(only_beta.len(), 2);
        let alpha_and_beta = cache
            .query_items_with_all_tags(&["alpha".into(), "beta".into()])
            .unwrap();
        assert_eq!(alpha_and_beta.len(), 1);
        assert_eq!(alpha_and_beta[0].ext_id, "a1");
    }

    #[test]
    fn derive_list_from_order_key() {
        let cache = SqliteCache::open_in_memory().unwrap();
        cache.upsert_lists(&sample_lists()).unwrap();
        assert_eq!(
            cache.derive_list_for_order(1001.0).unwrap().as_deref(),
            Some("inbox")
        );
        assert_eq!(
            cache.derive_list_for_order(1500.0).unwrap().as_deref(),
            Some("inbox")
        );
        assert_eq!(
            cache.derive_list_for_order(2000.0).unwrap().as_deref(),
            Some("now")
        );
        assert_eq!(cache.derive_list_for_order(999.0).unwrap(), None);
    }
}
