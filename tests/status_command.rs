use predicates::str::contains;

mod common;
use common::{cli, scaffold};

#[test]
fn status_of_initialized_spec_workspace_is_ok() {
    let temp = scaffold();

    cli()
        .current_dir(temp.path())
        .args(["spec", "new", "status-spec", "--index", "spec.md"])
        .assert()
        .success();

    cli()
        .current_dir(temp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(contains("Global Status: PASS"));
}

#[test]
fn status_supports_json_output() {
    let temp = scaffold();

    cli()
        .current_dir(temp.path())
        .args(["spec", "new", "json-spec", "--index", "spec.md"])
        .assert()
        .success();

    cli()
        .current_dir(temp.path())
        .args(["status", "--json"])
        .assert()
        .success()
        .stdout(contains("\"status\""));
}

#[test]
fn status_rejects_unknown_skip_category() {
    let temp = scaffold();

    cli()
        .current_dir(temp.path())
        .args(["spec", "new", "skip-spec", "--index", "spec.md"])
        .assert()
        .success();

    cli()
        .current_dir(temp.path())
        .args(["status", "--skip", "bogus"])
        .assert()
        .failure()
        .code(64)
        .stderr(contains("unknown status category"));
}
