use predicates::str::contains;
use std::fs;

mod common;
use common::{cli, scaffold};

// A full 40-character commit hash avoids `git ls-remote` (the library short-circuits tag
// resolution for commit hashes), keeping tests hermetic with no network or git interaction.
fn hash(n: u8) -> String {
    format!("{n:040x}")
}

#[test]
fn metadata_set_updates_name() {
    let temp = scaffold();

    cli()
        .current_dir(temp.path())
        .args(["spec", "new", "before", "--index", "spec.md"])
        .assert()
        .success();

    cli()
        .current_dir(temp.path())
        .args(["metadata", "set", "--name", "after"])
        .assert()
        .success()
        .stdout(contains("Updated specman.json manifest for 'after'"));

    let manifest = fs::read_to_string(temp.path().join("specman.json")).unwrap();
    assert!(manifest.contains("\"name\": \"after\""));
    assert!(!manifest.contains("\"name\": \"before\""));
}

#[test]
fn metadata_references_requires_spec_project() {
    let temp = scaffold();

    // An implementation project cannot use `metadata references`.
    cli()
        .current_dir(temp.path())
        .args([
            "impl",
            "new",
            "impl-proj",
            "--implements",
            &format!("core=https://github.com/owner/core.git@{}", hash(1)),
        ])
        .assert()
        .success();

    cli()
        .current_dir(temp.path())
        .args([
            "metadata",
            "references",
            "add",
            &format!("other=https://github.com/owner/other.git@{}", hash(2)),
        ])
        .assert()
        .failure()
        .code(64)
        .stderr(contains("specification projects only"));
}

#[test]
fn metadata_references_add_removes() {
    let temp = scaffold();

    cli()
        .current_dir(temp.path())
        .args(["spec", "new", "spec-proj", "--index", "spec.md"])
        .assert()
        .success();

    cli()
        .current_dir(temp.path())
        .args([
            "metadata",
            "references",
            "add",
            &format!("dep=https://github.com/owner/dep.git@{}", hash(3)),
        ])
        .assert()
        .success();

    let manifest = fs::read_to_string(temp.path().join("specman.json")).unwrap();
    assert!(manifest.contains("dep.git"));

    cli()
        .current_dir(temp.path())
        .args(["metadata", "references", "remove", "--alias", "dep"])
        .assert()
        .success();

    let manifest = fs::read_to_string(temp.path().join("specman.json")).unwrap();
    assert!(!manifest.contains("dep.git"));
}
