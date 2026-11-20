use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

mod common;

use common::{init_git, init_workspace};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_specman"))
}

#[test]
fn spec_new_creates_manifest_and_index() {
    let temp = TempDir::new().unwrap();
    init_git(temp.path());
    init_workspace(temp.path());

    cli()
        .current_dir(temp.path())
        .args([
            "spec",
            "new",
            "my-spec",
            "--description",
            "A test spec",
            "--tags",
            "a,b",
            "--index",
            "docs/spec.md",
        ])
        .assert()
        .success()
        .stdout(contains("Created specification 'my-spec'"));

    let manifest = fs::read_to_string(temp.path().join("specman.json")).unwrap();
    assert!(manifest.contains("\"name\": \"my-spec\""));
    assert!(manifest.contains("\"index\": \"docs/spec.md\""));
    assert!(manifest.contains("\"description\": \"A test spec\""));
    assert!(temp.path().join("docs/spec.md").is_file());
}

#[test]
fn spec_new_without_index_does_not_assume_default() {
    let temp = TempDir::new().unwrap();
    init_git(temp.path());
    init_workspace(temp.path());

    cli()
        .current_dir(temp.path())
        .args(["spec", "new", "--name", "no-index-spec"])
        .assert()
        .success();

    let manifest = fs::read_to_string(temp.path().join("specman.json")).unwrap();
    // `spec` is present but no `index` field.
    assert!(manifest.contains("\"spec\""));
    assert!(!manifest.contains("spec.md"));
    // No default document was written.
    assert!(!temp.path().join("spec.md").exists());
}

#[test]
fn spec_new_refuses_to_overwrite_existing_project() {
    let temp = TempDir::new().unwrap();
    init_git(temp.path());
    init_workspace(temp.path());

    cli()
        .current_dir(temp.path())
        .args(["spec", "new", "first-spec"])
        .assert()
        .success();

    cli()
        .current_dir(temp.path())
        .args(["spec", "new", "second-spec"])
        .assert()
        .failure()
        .code(64)
        .stderr(contains("refusing to overwrite"));
}

#[test]
fn spec_new_validates_dependency_syntax() {
    let temp = TempDir::new().unwrap();
    init_git(temp.path());
    init_workspace(temp.path());

    // A malformed `--dependencies` entry must fail with EX_USAGE before writing anything.
    cli()
        .current_dir(temp.path())
        .args([
            "spec",
            "new",
            "bad-spec",
            "--dependencies",
            "not-a-valid-ref",
        ])
        .assert()
        .failure()
        .code(64);

    assert!(!temp.path().join("specman.json").exists());
}

#[test]
fn spec_dependencies_rejects_conflicting_flags() {
    let temp = TempDir::new().unwrap();
    init_git(temp.path());
    init_workspace(temp.path());

    cli()
        .current_dir(temp.path())
        .args(["spec", "new", "flag-spec"])
        .assert()
        .success();

    cli()
        .current_dir(temp.path())
        .args(["spec", "dependencies", "--upstream", "--downstream"])
        .assert()
        .failure()
        .code(64)
        .stderr(contains(
            "use only one of --downstream, --upstream, or --all",
        ));
}
