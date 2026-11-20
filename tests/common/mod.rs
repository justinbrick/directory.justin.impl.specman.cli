use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

pub fn init_git(root: &Path) {
    fs::create_dir_all(root.join(".git")).unwrap();
}

/// Runs `specman init` against the given root (which must already be a git repository).
pub fn init_workspace(root: &Path) {
    assert_cmd::Command::new(env!("CARGO_BIN_EXE_specman"))
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
}

// Marked `#[allow(dead_code)]` because these helpers are shared across integration-test binaries,
// and not every binary references every helper.
#[allow(dead_code)]
pub fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_specman"))
}

/// Creates a git + initialized workspace at a fresh temp directory.
#[allow(dead_code)]
pub fn scaffold() -> TempDir {
    let temp = TempDir::new().unwrap();
    init_git(temp.path());
    init_workspace(temp.path());
    temp
}
