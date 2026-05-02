use std::{env, path::Path};

use crate::app::app_error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEditor {
    pub command: String,
    pub program: String,
    pub args: Vec<String>,
}

impl ResolvedEditor {
    pub fn resolve() -> Result<Self, AppError> {
        let command = env::var("VISUAL")
            .or_else(|_| env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".to_string());

        let mut parts = shell_words::split(&command).map_err(|error| {
            AppError::message(format!("invalid editor command '{}': {}", command, error))
        })?;

        if parts.is_empty() {
            return Err(AppError::message("editor command is empty"));
        }

        let program = parts.remove(0);
        Ok(Self {
            command,
            program,
            args: parts,
        })
    }

    pub fn open_file(&self, path: &Path) -> Result<(), AppError> {
        let status = std::process::Command::new(&self.program)
            .args(&self.args)
            .arg(path)
            .status()?;
        if !status.success() {
            return Err(AppError::message("editor command failed"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ResolvedEditor;
    use crate::test_support::LockedEnv;

    #[test]
    fn visual_overrides_editor() {
        let mut env = LockedEnv::new(&["VISUAL", "EDITOR"]);
        env.set("VISUAL", "nvim --clean");
        env.set("EDITOR", "vim");

        let editor = ResolvedEditor::resolve().expect("editor should resolve");
        assert_eq!(editor.command, "nvim --clean");
        assert_eq!(editor.program, "nvim");
        assert_eq!(editor.args, vec!["--clean"]);
    }

    #[test]
    fn editor_is_used_when_visual_is_unset() {
        let mut env = LockedEnv::new(&["VISUAL", "EDITOR"]);
        env.remove("VISUAL");
        env.set("EDITOR", "hx");

        let editor = ResolvedEditor::resolve().expect("editor should resolve");
        assert_eq!(editor.command, "hx");
        assert_eq!(editor.program, "hx");
        assert!(editor.args.is_empty());
    }

    #[test]
    fn falls_back_to_vi() {
        let mut env = LockedEnv::new(&["VISUAL", "EDITOR"]);
        env.remove("VISUAL");
        env.remove("EDITOR");

        let editor = ResolvedEditor::resolve().expect("editor should resolve");
        assert_eq!(editor.command, "vi");
        assert_eq!(editor.program, "vi");
        assert!(editor.args.is_empty());
    }

    #[test]
    fn parses_shell_quoted_command() {
        let mut env = LockedEnv::new(&["VISUAL", "EDITOR"]);
        env.set("VISUAL", "code --wait \"notes.md\"");
        env.remove("EDITOR");

        let editor = ResolvedEditor::resolve().expect("editor should resolve");
        assert_eq!(editor.program, "code");
        assert_eq!(editor.args, vec!["--wait", "notes.md"]);
    }

    #[test]
    fn rejects_invalid_shell_syntax() {
        let mut env = LockedEnv::new(&["VISUAL", "EDITOR"]);
        env.set("VISUAL", "\"unterminated");
        env.remove("EDITOR");

        let error = ResolvedEditor::resolve().expect_err("resolution should fail");
        assert!(
            error
                .to_string()
                .contains("invalid editor command '\"unterminated'")
        );
    }
}
