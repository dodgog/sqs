use std::path::Path;

use rusqlite::{Connection, params};

use crate::adapter::{Item, ListDef};
use crate::app::app_error::AppError;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS lists (
    name TEXT PRIMARY KEY,
    display TEXT NOT NULL,
    order_key REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS items (
    ext_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    list TEXT NOT NULL,
    order_key REAL NOT NULL,
    content_hash INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_items_list_order ON items(list, order_key);
";

pub struct SqliteCache {
    conn: Connection,
}

impl SqliteCache {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)
            .map_err(|e| AppError::message(format!("failed to open cache: {e}")))?;
        let cache = Self { conn };
        cache.migrate()?;
        Ok(cache)
    }

    pub fn open_in_memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| AppError::message(format!("failed to open in-memory cache: {e}")))?;
        let cache = Self { conn };
        cache.migrate()?;
        Ok(cache)
    }

    fn migrate(&self) -> Result<(), AppError> {
        self.conn
            .execute_batch(SCHEMA)
            .map_err(|e| AppError::message(format!("cache migration failed: {e}")))?;
        Ok(())
    }

    pub fn upsert_lists(&self, lists: &[ListDef]) -> Result<(), AppError> {
        let mut stmt = self
            .conn
            .prepare("INSERT OR REPLACE INTO lists (name, display, order_key) VALUES (?1, ?2, ?3)")
            .map_err(|e| AppError::message(format!("cache error: {e}")))?;
        for list in lists {
            stmt.execute(params![list.name, list.display, list.order])
                .map_err(|e| AppError::message(format!("cache error: {e}")))?;
        }
        Ok(())
    }

    pub fn upsert_items(&self, items: &[Item]) -> Result<(), AppError> {
        let mut stmt = self
            .conn
            .prepare(
                "INSERT OR REPLACE INTO items (ext_id, title, list, order_key, content_hash) VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .map_err(|e| AppError::message(format!("cache error: {e}")))?;
        for item in items {
            stmt.execute(params![
                item.ext_id,
                item.title,
                item.list,
                item.order,
                item.content_hash as i64,
            ])
            .map_err(|e| AppError::message(format!("cache error: {e}")))?;
        }
        Ok(())
    }

    pub fn query_lists(&self) -> Result<Vec<ListDef>, AppError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, display, order_key FROM lists ORDER BY order_key")
            .map_err(|e| AppError::message(format!("cache error: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ListDef {
                    name: row.get(0)?,
                    display: row.get(1)?,
                    order: row.get(2)?,
                })
            })
            .map_err(|e| AppError::message(format!("cache error: {e}")))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| AppError::message(format!("cache error: {e}")))?);
        }
        Ok(result)
    }

    pub fn query_items(&self, list: Option<&str>) -> Result<Vec<Item>, AppError> {
        let (sql, list_param) = match list {
            Some(l) => (
                "SELECT ext_id, title, list, order_key, content_hash FROM items WHERE list = ?1 ORDER BY order_key",
                Some(l.to_string()),
            ),
            None => (
                "SELECT ext_id, title, list, order_key, content_hash FROM items ORDER BY list, order_key",
                None,
            ),
        };
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| AppError::message(format!("cache error: {e}")))?;
        let rows = if let Some(ref l) = list_param {
            stmt.query_map(params![l], map_item_row)
        } else {
            stmt.query_map([], map_item_row)
        }
        .map_err(|e| AppError::message(format!("cache error: {e}")))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| AppError::message(format!("cache error: {e}")))?);
        }
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
                .execute("DELETE FROM items WHERE ext_id = ?1", params![item.ext_id])
                .map_err(|e| AppError::message(format!("cache error: {e}")))?;
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
        body: String::new(), // body is not cached
        list: row.get(2)?,
        order: row.get(3)?,
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
                order: 0.0,
            },
            ListDef {
                name: "now".into(),
                display: "Now".into(),
                order: 1.0,
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
                order: 1.0,
                content_hash: 100,
            },
            Item {
                ext_id: "b2".into(),
                title: "Task B".into(),
                body: "body".into(),
                list: "now".into(),
                order: 2.0,
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
        assert_eq!(lists[0].name, "inbox");
        assert_eq!(lists[1].name, "now");
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
        assert!(inbox[0].body.is_empty()); // body not cached
    }

    #[test]
    fn remove_stale_deletes_missing_ids() {
        let cache = SqliteCache::open_in_memory().unwrap();
        cache.upsert_items(&sample_items()).unwrap();

        let mut current = std::collections::HashSet::new();
        current.insert("a1".to_string());
        let removed = cache.remove_stale(&current).unwrap();
        assert_eq!(removed, 1);

        let remaining = cache.query_items(None).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].ext_id, "a1");
    }

    #[test]
    fn reconcile_inserts_updates_and_removes() {
        let cache = SqliteCache::open_in_memory().unwrap();
        cache.upsert_items(&sample_items()).unwrap();

        let updated = vec![
            Item {
                ext_id: "a1".into(),
                title: "Updated A".into(),
                body: String::new(),
                list: "now".into(),
                order: 1.0,
                content_hash: 101,
            },
            Item {
                ext_id: "c3".into(),
                title: "Task C".into(),
                body: String::new(),
                list: "inbox".into(),
                order: 3.0,
                content_hash: 300,
            },
        ];
        cache.reconcile(&updated, &sample_lists()).unwrap();

        let items = cache.query_items(None).unwrap();
        assert_eq!(items.len(), 2);
        let ids: Vec<_> = items.iter().map(|i| i.ext_id.as_str()).collect();
        assert!(ids.contains(&"a1"));
        assert!(ids.contains(&"c3"));
        assert!(!ids.contains(&"b2")); // removed
    }

    #[test]
    fn upsert_updates_existing_item() {
        let cache = SqliteCache::open_in_memory().unwrap();
        cache.upsert_items(&sample_items()).unwrap();

        let updated = vec![Item {
            ext_id: "a1".into(),
            title: "Updated".into(),
            body: String::new(),
            list: "now".into(),
            order: 5.0,
            content_hash: 999,
        }];
        cache.upsert_items(&updated).unwrap();

        let items = cache.query_items(Some("now")).unwrap();
        assert_eq!(items.len(), 2); // b2 still in now + updated a1
        let a1 = items.iter().find(|i| i.ext_id == "a1").unwrap();
        assert_eq!(a1.title, "Updated");
        assert_eq!(a1.content_hash, 999);
    }

    #[test]
    fn open_file_based_cache() {
        let temp = tempfile::TempDir::new().unwrap();
        let db_path = temp.path().join("cache").join("test.db");
        let cache = SqliteCache::open(&db_path).unwrap();
        cache.upsert_lists(&sample_lists()).unwrap();

        // Reopen and verify persistence
        drop(cache);
        let cache2 = SqliteCache::open(&db_path).unwrap();
        let lists = cache2.query_lists().unwrap();
        assert_eq!(lists.len(), 2);
    }
}
