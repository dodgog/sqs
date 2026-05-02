use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::app::app_error::AppError;

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigInspection {
    pub config_path: Option<PathBuf>,
    pub file_exists: bool,
    pub explicit_root: Option<PathBuf>,
    pub env_root: Option<PathBuf>,
    pub resolved: Option<ResolvedConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub tasks_root: PathBuf,
    pub state_dir: PathBuf,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    tasks_root: Option<PathBuf>,
}

pub fn resolve(explicit_root: Option<PathBuf>) -> Result<ResolvedConfig, AppError> {
    let file_config = load_file_config()?;
    let tasks_root = explicit_root
        .or_else(|| env_path("SQS_ROOT"))
        .or_else(|| {
            file_config
                .as_ref()
                .and_then(|config| config.tasks_root.clone())
        })
        .or_else(find_sqs_toml_root)
        .ok_or_else(missing_root_error)?;

    let state_dir = tasks_root.join(".sqs");

    Ok(ResolvedConfig {
        tasks_root,
        state_dir,
    })
}

pub fn inspect(explicit_root: Option<PathBuf>) -> Result<ConfigInspection, AppError> {
    let config_path = config_path();
    let file_exists = config_path.as_ref().is_some_and(|path| path.exists());
    let env_root = env_path("SQS_ROOT");
    let resolved = match resolve(explicit_root.clone()) {
        Ok(resolved) => Some(resolved),
        Err(AppError::NoConfig(_)) => None,
        Err(error) => return Err(error),
    };

    Ok(ConfigInspection {
        config_path,
        file_exists,
        explicit_root,
        env_root,
        resolved,
    })
}

pub fn starter_config(config_path: Option<&Path>) -> String {
    let config_display = config_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "~/.config/sqs/config.toml".to_string());
    let mut message = String::new();
    message.push_str("To get started:\n");
    message.push_str("  1. Run: sqs init\n");
    message.push_str("  2. Or try: sqs --root ~/tasks add \"My first task\"\n");
    message.push_str(&format!("  3. Or create {config_display} with:\n"));
    message.push_str("     tasks_root = \"~/tasks\"\n");
    message
}

fn missing_root_error() -> AppError {
    let config_path = config_path();
    let location = config_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "~/.config/sqs/config.toml".to_string());

    AppError::NoConfig(format!(
        "no sqs.toml found; run `sqs init` or pass --root, set SQS_ROOT, or configure {location}\n\n{}",
        starter_config(config_path.as_deref())
    ))
}

fn load_file_config() -> Result<Option<FileConfig>, AppError> {
    let Some(path) = config_path() else {
        return Ok(None);
    };

    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&path)?;
    let mut parsed: FileConfig = toml::from_str(&contents).map_err(|error| {
        AppError::message(format!("invalid config file {}: {error}", path.display()))
    })?;

    let base_dir = path.parent().unwrap_or(Path::new("."));
    parsed.tasks_root = parsed
        .tasks_root
        .map(|value| absolutize_from(base_dir, value));

    Ok(Some(parsed))
}

/// Walk up from CWD looking for sqs.toml; extract tasks_root from adapter config.
fn find_sqs_toml_root() -> Option<PathBuf> {
    let mut dir = env::current_dir().ok()?;
    loop {
        let candidate = dir.join("sqs.toml");
        if candidate.is_file() {
            let content = fs::read_to_string(&candidate).ok()?;
            let parsed: toml::Value = toml::from_str(&content).ok()?;
            let root_str = parsed
                .get("adapters")
                .and_then(|a| a.get("markdown-todolists"))
                .and_then(|a| a.get("root"))
                .and_then(|v| v.as_str())?;
            let base = candidate.parent().unwrap_or(Path::new("."));
            return Some(absolutize_from(base, PathBuf::from(root_str)));
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn config_path() -> Option<PathBuf> {
    if let Some(xdg_config) = env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        return Some(xdg_config.join("sqs").join(CONFIG_FILE_NAME));
    }

    env_path("HOME").map(|home| home.join(".config").join("sqs").join(CONFIG_FILE_NAME))
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn absolutize_from(base_dir: &Path, value: PathBuf) -> PathBuf {
    if let Ok(stripped) = value.strip_prefix("~")
        && let Some(home) = env_path("HOME")
    {
        return home.join(stripped);
    }
    if value.is_absolute() {
        value
    } else {
        base_dir.join(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LockedEnv;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn resolve_uses_cli_root() {
        let mut env = LockedEnv::new(&["XDG_CONFIG_HOME", "SQS_ROOT"]);
        env.remove("SQS_ROOT");
        let temp = TempDir::new().expect("temp dir");
        let resolved = resolve(Some(temp.path().to_path_buf())).expect("config should resolve");
        assert_eq!(resolved.tasks_root, temp.path());
    }

    #[test]
    fn resolve_errors_without_config() {
        let mut env = LockedEnv::new(&["XDG_CONFIG_HOME", "SQS_ROOT"]);
        let temp = TempDir::new().expect("temp dir");
        env.remove("SQS_ROOT");
        env.set("XDG_CONFIG_HOME", temp.path().as_os_str());

        let error = resolve(None).expect_err("should fail");
        assert!(error.to_string().contains("no sqs.toml found"));
    }
}
