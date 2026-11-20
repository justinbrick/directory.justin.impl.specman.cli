use predicates::str::contains;
use std::fs;

mod common;
use common::{cli, scaffold};

#[test]
fn impl_new_requires_implements() {
    let temp = scaffold();

    cli()
        .current_dir(temp.path())
        .args(["impl", "new", "no-implements"])
        .assert()
        .failure()
        .code(64)
        .stderr(contains("at least one --implements entry is required"));

    assert!(!temp.path().join("specman.json").exists());
}

#[test]
fn impl_new_creates_manifest_and_document() {
    let temp = scaffold();

    cli()
        .current_dir(temp.path())
        .args([
            "impl",
            "new",
            "my-impl",
            "--implements",
            "core=https://github.com/owner/core.git@v1.0.0",
        ])
        .assert()
        .success()
        .stdout(contains("Created implementation 'my-impl'"));

    let manifest = fs::read_to_string(temp.path().join("specman.json")).unwrap();
    assert!(manifest.contains("\"name\": \"my-impl\""));
    assert!(
        manifest.contains(
            "core=https://github.com/owner/core.git"
                .split('=')
                .next()
                .unwrap()
        )
    );
    assert!(temp.path().join("impl.md").is_file());
}

#[test]
fn impl_new_rejects_both_positional_and_flag_name() {
    let temp = scaffold();

    cli()
        .current_dir(temp.path())
        .args([
            "impl",
            "new",
            "positional",
            "--name",
            "flag",
            "--implements",
            "core=https://github.com/owner/core.git@v1.0.0",
        ])
        .assert()
        .failure()
        .code(64)
        .stderr(contains("not both"));
}

#[test]
fn impl_new_rejects_missing_name() {
    let temp = scaffold();

    cli()
        .current_dir(temp.path())
        .args([
            "impl",
            "new",
            "--implements",
            "core=https://github.com/owner/core.git@v1.0.0",
        ])
        .assert()
        .failure()
        .code(64)
        .stderr(contains("name is required"));
}
