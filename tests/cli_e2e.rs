use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn prints_help() {
    let mut command = Command::cargo_bin("dsync").unwrap();

    command
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: dsync"));
}

#[test]
fn fails_without_arguments() {
    let mut command = Command::cargo_bin("dsync").unwrap();

    command.assert().failure().stderr(predicate::str::contains(
        "Provide a .md file or use --gdoc <id> / --linear <id>",
    ));
}

#[test]
fn syncs_a_plain_markdown_file_without_providers() {
    let temp = TempDir::new().unwrap();
    let note = temp.child("note.md");
    note.write_str("# Title\nBody\n").unwrap();

    let mut command = Command::cargo_bin("dsync").unwrap();

    command
        .current_dir(temp.path())
        .arg(note.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Tri-sync completed"));

    let written = fs::read_to_string(note.path()).unwrap();
    assert_eq!(written, "# Title\nBody\n");
}

#[test]
fn sync_in_a_git_repo_adds_git_links_to_frontmatter_and_content() {
    let temp = TempDir::new().unwrap();
    init_git_repo(temp.path());

    let note = temp.child("note.md");
    note.write_str("# Title\nBody\n").unwrap();

    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-q", "-m", "Initial commit"]);
    git(
        temp.path(),
        &["remote", "add", "origin", "git@github.com:acme/project.git"],
    );

    let mut command = Command::cargo_bin("dsync").unwrap();

    command
        .current_dir(temp.path())
        .arg(note.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Tri-sync completed"));

    let written = fs::read_to_string(note.path()).unwrap();

    assert!(written.contains("gitUrl: https://github.com/acme/project/blob/main/note.md"));
    assert!(written.contains("## Document Links"));
    assert!(written.contains("- Git: https://github.com/acme/project/blob/main/note.md"));
}

fn init_git_repo(path: &Path) {
    git(path, &["-c", "init.defaultBranch=main", "init", "-q"]);
    git(path, &["config", "user.name", "Codex Test"]);
    git(path, &["config", "user.email", "codex@example.com"]);
}

fn git(path: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(path)
        .args(args)
        .status()
        .unwrap();

    assert!(
        status.success(),
        "git command failed: git {}",
        args.join(" ")
    );
}
