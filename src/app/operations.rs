use std::fs;

use chrono::Utc;

use crate::app::app_error::AppError;
use crate::domain::task::Queue;
use crate::storage::repo::TaskRepo;

/// Move a task to the done queue. Returns the updated task and its path.
pub fn mark_done(
    repo: &TaskRepo,
    task_id: &str,
) -> Result<(crate::domain::task::Task, std::path::PathBuf), AppError> {
    let (task, path, _) = repo.move_to_queue(task_id, Queue::Done, Utc::now())?;
    Ok((task, path))
}

/// Result of applying an edit: either the task was unchanged, or it was updated.
pub enum EditOutcome {
    Unchanged,
    Applied,
}

/// Validate and apply an edited task file.
pub fn apply_edit(
    repo: &TaskRepo,
    task_id: &str,
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
    match repo.replace_edited(task_id, &edited_content, Utc::now()) {
        Ok(_) => Ok(EditOutcome::Applied),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::Task;
    use crate::storage::config::QueueDirs;
    use tempfile::TempDir;

    fn make_repo_with_task(temp: &TempDir) -> (TaskRepo, std::path::PathBuf, String) {
        let root = temp.path().to_path_buf();
        let repo = TaskRepo::new(root, QueueDirs::default());
        let task = Task::new("abc".to_string(), "Test task", Utc::now());
        let path = repo.create(&task).unwrap();
        let original = fs::read_to_string(&path).unwrap();
        (repo, path, original)
    }

    #[test]
    fn apply_edit_unchanged_returns_unchanged() {
        let temp = TempDir::new().unwrap();
        let (repo, path, original) = make_repo_with_task(&temp);

        let result = apply_edit(&repo, "abc", &path, &original).unwrap();
        assert!(matches!(result, EditOutcome::Unchanged));
    }

    #[test]
    fn apply_edit_valid_change_returns_applied() {
        let temp = TempDir::new().unwrap();
        let (repo, path, original) = make_repo_with_task(&temp);

        let edited = original.replace("# Test task", "# Test task\n\nNew body content");
        fs::write(&path, &edited).unwrap();

        let result = apply_edit(&repo, "abc", &path, &original).unwrap();
        assert!(matches!(result, EditOutcome::Applied));
    }

    #[test]
    fn apply_edit_empty_file_restores_original() {
        let temp = TempDir::new().unwrap();
        let (repo, path, original) = make_repo_with_task(&temp);

        fs::write(&path, "   \n").unwrap();

        let result = apply_edit(&repo, "abc", &path, &original);
        assert!(result.is_err());

        let on_disk = fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk, original);
    }

    #[test]
    fn apply_edit_malformed_yaml_restores_original_and_task_survives() {
        let temp = TempDir::new().unwrap();
        let (repo, path, original) = make_repo_with_task(&temp);

        fs::write(&path, "---\nthis is not: [valid: yaml\n---\n").unwrap();

        let result = apply_edit(&repo, "abc", &path, &original);
        assert!(result.is_err());

        let on_disk = fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk, original);

        let task = repo.read("abc").unwrap();
        assert_eq!(task.title, "Test task");
    }
}
