use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::TempDir;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

fn sqs_cmd() -> assert_cmd::Command {
    let mut cmd = cargo_bin_cmd!("sqs");
    cmd.env("XDG_CONFIG_HOME", "/dev/null/sqs-test-config");
    cmd
}

fn write_item(root: &std::path::Path, list: &str, id: &str, title: &str, body: &str) {
    let list_dir = root.join(list);
    std::fs::create_dir_all(&list_dir).expect("list dir should exist");
    std::fs::write(
        list_dir.join(format!("{id}.md")),
        format!(
            "---\ntitle: {title}\nlist: {list}\norder: 0.0\ncreated_at: 2026-03-09T10:34:12Z\nupdated_at: 2026-03-09T10:34:12Z\n---\n{body}"
        ),
    )
    .expect("item file should be written");
}

#[test]
fn malformed_files_are_skipped_during_list() {
    let temp = TempDir::new().expect("temp dir should exist");
    write_item(temp.path(), "inbox", "good", "Good task", "body");
    // Write a malformed file
    let inbox = temp.path().join("inbox");
    std::fs::write(inbox.join("bad.md"), "---\ntitle: Bad\n").expect("bad file should be written");

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("list")
        .arg("inbox")
        .assert()
        .success()
        .stdout(contains("good").and(contains("bad").not()))
        .stderr(contains("Warning: skipping malformed file"));
}

#[test]
fn add_omits_old_metadata_fields() {
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

    let content = std::fs::read_to_string(temp.path().join("inbox").join("task-1.md"))
        .expect("item file should exist");
    assert!(content.contains("title: Ship v2"));
    assert!(content.contains("list: inbox"));
    assert!(!content.contains("queue:"));
    assert!(!content.contains("id:"));
    assert!(!content.contains("completed_at:"));
}

#[test]
fn ambiguous_task_reference_is_reported_cleanly_without_tty() {
    let temp = TempDir::new().expect("temp dir should exist");
    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("add")
        .arg("--no-edit")
        .arg("--id")
        .arg("task-1")
        .arg("Ship release")
        .assert()
        .success();
    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("add")
        .arg("--no-edit")
        .arg("--id")
        .arg("task-2")
        .arg("Ship release prep")
        .assert()
        .success();

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("show")
        .arg("Ship release")
        .assert()
        .failure()
        .stderr(contains("task reference is ambiguous: Ship release"));
}

#[test]
fn add_with_content_sets_body() {
    let temp = TempDir::new().expect("temp dir should exist");

    sqs_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("add")
        .arg("--id")
        .arg("task-1")
        .arg("--content")
        .arg("Some details here")
        .arg("Ship v2")
        .assert()
        .success()
        .stdout(contains("Created: task-1"));

    let content = std::fs::read_to_string(temp.path().join("inbox").join("task-1.md"))
        .expect("item file should exist");
    assert!(content.contains("title: Ship v2"));
    assert!(content.contains("Some details here"));
}
