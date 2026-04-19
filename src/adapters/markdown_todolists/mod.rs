pub mod frontmatter;
pub mod identity;
pub mod io;

use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::Utc;

use crate::adapter::{Adapter, EditOutcome, Item, ListDef};
use crate::app::app_error::AppError;
use crate::domain::id::validate_user_id;
use crate::domain::task::{Queue, Task};
use crate::storage::config::ResolvedConfig;
use crate::storage::id_state::SharedIdAllocator;
use crate::storage::repo::TaskRepo;

pub struct MarkdownTodolistsAdapter {
    repo: TaskRepo,
    config: ResolvedConfig,
}

impl MarkdownTodolistsAdapter {
    pub fn new(config: ResolvedConfig) -> Self {
        let repo = TaskRepo::new(config.tasks_root.clone(), config.queue_dirs.clone());
        Self { repo, config }
    }

    pub fn config(&self) -> &ResolvedConfig {
        &self.config
    }

    fn list_name_to_queue(name: &str) -> Result<Queue, AppError> {
        Queue::from_str(name).map_err(|_| {
            AppError::usage(format!(
                "invalid list '{}'; expected one of: inbox, now, next, later, done",
                name
            ))
        })
    }

    fn task_to_item(task: &Task) -> Item {
        Item {
            ext_id: task.id.clone(),
            title: task.title.clone(),
            body: task.body.clone(),
            list: task.queue.to_string(),
            order: task.updated_at.timestamp() as f64,
            content_hash: 0,
        }
    }
}

impl Adapter for MarkdownTodolistsAdapter {
    fn name(&self) -> &str {
        "markdown-todolists"
    }

    fn lists(&self) -> Vec<ListDef> {
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

    fn scan(&self) -> Result<Vec<Item>, AppError> {
        let stored = self.repo.scan_all()?;
        Ok(stored.iter().map(|s| Self::task_to_item(&s.task)).collect())
    }

    fn find_item(&self, ext_id: &str) -> Result<Item, AppError> {
        let stored = self.repo.find_by_id(ext_id)?;
        Ok(Self::task_to_item(&stored.task))
    }

    fn create_item(
        &mut self,
        id: Option<&str>,
        list: &str,
        title: &str,
        body: &str,
    ) -> Result<(Item, PathBuf), AppError> {
        let queue = Self::list_name_to_queue(list)?;
        let now = Utc::now();

        let task_id = match id {
            Some(id) => {
                validate_user_id(id)?;
                if self.repo.id_exists(id) {
                    return Err(AppError::usage(format!("id '{}' already exists", id)));
                }
                id.to_string()
            }
            None => SharedIdAllocator::new(&self.config).generate(&self.repo)?,
        };

        let mut task = Task::new(task_id, title, now);
        if !body.is_empty() {
            task.body = format!("# {}\n\n{}\n", title, body);
        }
        task.move_to(queue, now);

        let path = self.repo.create(&task)?;
        Ok((Self::task_to_item(&task), path))
    }

    fn move_item(&mut self, ext_id: &str, target_list: &str) -> Result<Item, AppError> {
        let queue = Self::list_name_to_queue(target_list)?;
        let (task, _, _) = self.repo.move_to_queue(ext_id, queue, Utc::now())?;
        Ok(Self::task_to_item(&task))
    }

    fn delete_item(&mut self, ext_id: &str) -> Result<(), AppError> {
        self.repo.delete(ext_id)?;
        Ok(())
    }

    fn editor_path(&self, ext_id: &str) -> Result<PathBuf, AppError> {
        let stored = self.repo.find_by_id(ext_id)?;
        Ok(stored.path)
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
            return Err(AppError::message("task file cannot be empty"));
        }

        if edited_content == original_content {
            return Ok(EditOutcome::Unchanged);
        }

        fs::write(path, original_content)?;
        self.repo
            .replace_edited(ext_id, &edited_content, Utc::now())?;
        Ok(EditOutcome::Applied)
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
            return Err(AppError::message("task file cannot be empty"));
        }

        if edited_content != original_content
            && let Err(error) =
                self.repo
                    .finalize_added_edit(ext_id, path, &edited_content, Utc::now())
        {
            fs::write(path, original_content)?;
            return Err(error);
        }

        Ok(())
    }
}
