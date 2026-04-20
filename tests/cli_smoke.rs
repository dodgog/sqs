use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::TempDir;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;

fn sqs_cmd() -> assert_cmd::Command {
    let mut cmd = cargo_bin_cmd!("sqs");
    cmd.env("XDG_CONFIG_HOME", "/dev/null/sqs-test-config");
    cmd
}

fn write_task(root: &std::path::Path, list: &str, id: &str, title: &str, body: &str) {
    let list_dir = root.join(list);
    fs::create_dir_all(&list_dir).expect("list dir should exist");
    fs::write(
        list_dir.join(format!("{id}.md")),
        format!(
            "---\ntitle: {title}\nlist: {list}\norder: 0.0\ncreated_at: 2026-03-09T10:34:12Z\nupdated_at: 2026-03-09T10:34:12Z\n---\n{body}"
        ),
    )
    .expect("item file should be written");
}

#[test]
fn help_command_works() {
    sqs_cmd().arg("--help").assert().success().stdout(
        contains("Reorder lists from the terminal")
            .and(contains("Task Commands:"))
            .and(contains("Workflow Commands:"))
            .and(contains("Setup Commands:"))
            .and(contains("Add a task"))
            .and(contains("List tasks"))
            .and(contains("Check configuration and task storage health"))
            .and(contains("Options:")),
    );
}

#[test]
fn bare_command_shows_getting_started_without_config() {
    sqs_cmd().assert().success().stdout(
        contains("Welcome to sqs!")
            .and(contains("To get started:"))
            .and(contains("sqs config")),
    );
}

#[test]
fn bare_command_shows_dashboard_when_tasks_exist() {
    let temp = TempDir::new().expect("temp dir should exist");
    write_task(temp.path(), "now", "task-1", "Ship v2", "# Ship v2");
    write_task(temp.path(), "inbox", "task-2", "Review PR", "# Review PR");

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .assert()
        .success()
        .stdout(contains("Ship v2").and(contains("Review PR")));
}

#[test]
fn bare_command_shows_getting_started_when_no_tasks() {
    let temp = TempDir::new().expect("temp dir should exist");

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .assert()
        .success()
        .stdout(contains("Welcome to sqs!").and(contains("To get started:")));
}

#[test]
fn global_flag_is_rejected() {
    sqs_cmd()
        .arg("--global")
        .arg("list")
        .assert()
        .failure()
        .stderr(contains("unexpected argument '--global'"));
}

#[test]
fn add_creates_task_in_inbox() {
    let temp = TempDir::new().expect("temp dir should exist");

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("add")
        .arg("--no-edit")
        .arg("--id")
        .arg("task-1")
        .arg("Ship v2")
        .assert()
        .success()
        .stdout(contains("Created: task-1"));

    let path = temp.path().join("inbox").join("task-1.md");
    assert!(path.exists());
    let content = fs::read_to_string(path).expect("task should exist");
    assert!(content.contains("title: Ship v2"));
    assert!(content.contains("list: inbox"));
}

#[test]
fn add_generates_random_alphanumeric_ids() {
    let temp = TempDir::new().expect("temp dir should exist");

    let assert = sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("add")
        .arg("--no-edit")
        .arg("Ship v2")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout is utf-8");
    let id = stdout
        .strip_prefix("Created: ")
        .and_then(|value| value.split_once(" ("))
        .map(|(id, _)| id)
        .expect("created-task output should include the generated ID");

    assert_eq!(id.len(), 4);
    assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
    assert!(temp.path().join("inbox").join(format!("{id}.md")).exists());
}

#[test]
fn list_without_queue_shows_dashboard() {
    let temp = TempDir::new().expect("temp dir should exist");
    write_task(temp.path(), "now", "task-1", "Do now", "# Do now");
    write_task(temp.path(), "inbox", "task-2", "Review PR", "# Review PR");

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(
            contains("now (1)")
                .and(contains("inbox (1)"))
                .and(contains("Review PR")),
        );
}

#[test]
fn list_queue_shows_only_requested_queue() {
    let temp = TempDir::new().expect("temp dir should exist");
    write_task(temp.path(), "now", "task-1", "Do now", "# Do now");
    write_task(temp.path(), "inbox", "task-2", "Review PR", "# Review PR");

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("list")
        .arg("now")
        .assert()
        .success()
        .stdout(
            contains("now (1)")
                .and(contains("Do now"))
                .and(contains("Review PR").not()),
        );
}

#[test]
fn move_relocates_file_to_target_queue() {
    let temp = TempDir::new().expect("temp dir should exist");

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("add")
        .arg("--no-edit")
        .arg("--id")
        .arg("task-1")
        .arg("Ship v2")
        .assert()
        .success();

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("move")
        .arg("task-1")
        .arg("now")
        .assert()
        .success()
        .stdout(contains("Moved task-1 to now"));

    assert!(!temp.path().join("inbox").join("task-1.md").exists());
    assert!(temp.path().join("now").join("task-1.md").exists());
}

#[test]
fn move_promotes_task_from_inbox_to_now() {
    let temp = TempDir::new().expect("temp dir should exist");

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("add")
        .arg("--no-edit")
        .arg("--id")
        .arg("task-1")
        .arg("Review outage notes")
        .assert()
        .success();

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("move")
        .arg("task-1")
        .arg("now")
        .assert()
        .success()
        .stdout(contains("Moved task-1 to now"));

    assert!(!temp.path().join("inbox").join("task-1.md").exists());
    assert!(temp.path().join("now").join("task-1.md").exists());
}

#[test]
fn move_is_noop_when_task_is_already_in_target_queue() {
    let temp = TempDir::new().expect("temp dir should exist");
    write_task(temp.path(), "now", "task-1", "Do now", "# Do now");

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("move")
        .arg("task-1")
        .arg("now")
        .assert()
        .success()
        .stdout(contains("task-1 is already in now"));

    assert!(temp.path().join("now").join("task-1.md").exists());
    assert!(!temp.path().join("inbox").join("task-1.md").exists());
}

#[test]
fn show_prints_metadata_path_and_body() {
    let temp = TempDir::new().expect("temp dir should exist");
    write_task(
        temp.path(),
        "inbox",
        "task-1",
        "Ship v2",
        "# Ship v2\n\n## Notes",
    );

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("show")
        .arg("task-1")
        .assert()
        .success()
        .stdout(contains("Path:").and(contains("# Ship v2")));
}

#[test]
fn find_matches_body_text() {
    let temp = TempDir::new().expect("temp dir should exist");
    write_task(
        temp.path(),
        "next",
        "task-1",
        "Investigate billing",
        "# Investigate billing\n\nLook at cost explorer.",
    );

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("find")
        .arg("cost explorer")
        .assert()
        .success()
        .stdout(contains("task-1").and(contains("Investigate billing")));
}

#[test]
fn old_command_names_are_rejected() {
    let temp = TempDir::new().expect("temp dir should exist");

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("create")
        .arg("Ship v2")
        .assert()
        .failure()
        .stderr(contains("unrecognized subcommand"));

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("complete")
        .arg("task-1")
        .assert()
        .failure()
        .stderr(contains("unrecognized subcommand"));
}

#[test]
fn show_resolves_unique_title_substring() {
    let temp = TempDir::new().expect("temp dir should exist");
    write_task(
        temp.path(),
        "inbox",
        "task-1",
        "Ship v2 release",
        "# Ship v2 release",
    );

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("show")
        .arg("v2 rel")
        .assert()
        .success()
        .stdout(contains("ID:").and(contains("task-1")));
}

#[test]
fn show_does_not_resolve_body_text_as_task_reference() {
    let temp = TempDir::new().expect("temp dir should exist");
    write_task(
        temp.path(),
        "inbox",
        "task-1",
        "Ship v2 release",
        "# Ship v2 release\n\nLook at cost explorer.",
    );

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("show")
        .arg("cost explorer")
        .assert()
        .failure()
        .stderr(contains("task not found: cost explorer"));
}

#[test]
fn add_reads_tasks_root_from_config_file() {
    let temp = TempDir::new().expect("temp dir should exist");
    let config_home = temp.path().join("config-home");
    let config_dir = config_home.join("sqs");
    let tasks_root = temp.path().join("configured-tasks");
    std::fs::create_dir_all(&config_dir).expect("config dir should exist");
    std::fs::write(
        config_dir.join("config.toml"),
        format!("tasks_root = '{}'\n", tasks_root.display()),
    )
    .expect("config file should be written");

    sqs_cmd()
        .env("XDG_CONFIG_HOME", &config_home)
        .arg("add")
        .arg("--no-edit")
        .arg("--id")
        .arg("task-1")
        .arg("Ship v2")
        .assert()
        .success()
        .stdout(contains("Created: task-1"));

    assert!(tasks_root.join("inbox").join("task-1.md").exists());
}

#[test]
fn command_fails_cleanly_when_tasks_root_is_not_configured() {
    let temp = TempDir::new().expect("temp dir should exist");

    sqs_cmd()
        .env("XDG_CONFIG_HOME", temp.path())
        .env_remove("SQS_ROOT")
        .arg("list")
        .assert()
        .failure()
        .stderr(
            contains("no sqs.toml found")
                .and(contains("To get started:"))
                .and(contains("sqs --root ~/tasks add \"My first task\"")),
        );
}

#[test]
fn config_command_prints_getting_started_guide_when_tasks_root_is_missing() {
    let temp = TempDir::new().expect("temp dir should exist");
    let config_home = temp.path().join("config-home");

    sqs_cmd()
        .env("XDG_CONFIG_HOME", &config_home)
        .env_remove("SQS_ROOT")
        .arg("config")
        .assert()
        .success()
        .stdout(
            contains(format!(
                "config_path = {}",
                config_home.join("sqs").join("config.toml").display()
            ))
            .and(contains("config_file = missing"))
            .and(contains("tasks_root = <unset>"))
            .and(contains("To get started:"))
            .and(contains("tasks_root = \"~/tasks\"")),
        );
}
