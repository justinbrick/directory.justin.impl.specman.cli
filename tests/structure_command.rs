use predicates::str::contains;
use std::fs;

mod common;
use common::{cli, scaffold};

fn create_spec_with_index(temp: &tempfile::TempDir) {
    cli()
        .current_dir(temp.path())
        .args(["spec", "new", "struct-spec", "--index", "spec.md"])
        .assert()
        .success();

    let body = r#"# Overview

Intro text.

## Concept: SpecMan Structure

!concept-specman-structure.indexing.headings:
- Each heading MUST be indexed.

## Details

Detail text.
"#;
    fs::write(temp.path().join("spec.md"), body).unwrap();
}

#[test]
fn structure_ls_enumerates_headings_and_constraints() {
    let temp = scaffold();
    create_spec_with_index(&temp);

    cli()
        .current_dir(temp.path())
        .args(["structure", "ls"])
        .assert()
        .success()
        .stdout(contains("overview"))
        .stdout(contains("concept-specman-structure"))
        .stdout(contains("concept-specman-structure.indexing.headings"));
}

#[test]
fn structure_show_renders_heading() {
    let temp = scaffold();
    create_spec_with_index(&temp);

    cli()
        .current_dir(temp.path())
        .args(["structure", "show", "--slug", "overview"])
        .assert()
        .success()
        .stdout(contains("Intro text."));
}

#[test]
fn structure_show_unknown_slug_fails() {
    let temp = scaffold();
    create_spec_with_index(&temp);

    cli()
        .current_dir(temp.path())
        .args(["structure", "show", "--slug", "does-not-exist"])
        .assert()
        .failure()
        .code(65)
        .stderr(contains("could not be resolved"));
}

#[test]
fn structure_show_renders_constraint_group() {
    let temp = scaffold();
    create_spec_with_index(&temp);

    cli()
        .current_dir(temp.path())
        .args([
            "structure",
            "show",
            "--constraint",
            "concept-specman-structure.indexing.headings",
        ])
        .assert()
        .success()
        .stdout(contains("Each heading MUST be indexed."));
}
