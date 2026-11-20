use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_specman"))
}

/// Creates a minimal git repository marker (a `.git` directory) so workspace discovery succeeds
/// without shelling out to `git`.
fn init_git(root: &std::path::Path) {
    fs::create_dir_all(root.join(".git")).unwrap();
}

#[test]
fn init_creates_workspace_scaffold() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("workspace");
    fs::create_dir_all(&target).unwrap();
    init_git(&target);

    cli()
        .args(["init", target.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Initialized workspace at"));

    assert!(target.join(".specman").is_dir());
    assert!(target.join(".specman/cache").is_dir());
    assert!(target.join("spec").is_dir());
}

#[test]
fn init_dry_run_does_not_write() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("workspace-dry-run");
    fs::create_dir_all(&target).unwrap();
    init_git(&target);

    cli()
        .args(["init", target.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .stdout(contains("Dry run: would initialize workspace at"));

    assert!(!target.join(".specman").exists());
}

#[test]
fn init_defaults_to_current_directory() {
    let temp = TempDir::new().unwrap();
    init_git(temp.path());

    cli()
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success()
        .stdout(contains("Initialized workspace at"));

    assert!(temp.path().join(".specman").is_dir());
}

#[test]
fn init_rejects_relative_path() {
    cli()
        .args(["init", "relative/path"])
        .assert()
        .failure()
        .code(64)
        .stderr(contains("must be an absolute path"));
}

#[test]
fn init_requires_git_repository() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("no-git");
    fs::create_dir_all(&target).unwrap();

    cli()
        .args(["init", target.to_str().unwrap()])
        .assert()
        .failure()
        .code(64)
        .stderr(contains("git init"));
}
