use clap::{ArgMatches, Command};
use serde::Serialize;
use specman::{install, load_manifest};

use crate::commands::CommandResult;
use crate::context::CliSession;
use crate::error::CliError;

pub fn command() -> Command {
    Command::new("install").about("Resolve and materialize all git-referenced dependencies")
}

/// The result of a dependency install, serialized for `--json` output.
#[derive(Debug, Serialize)]
pub struct InstallReport {
    pub requested: usize,
    pub entries: Vec<specman::LockedEntry>,
    pub materialized: Vec<String>,
}

pub fn run(session: &CliSession, _matches: &ArgMatches) -> Result<CommandResult, CliError> {
    let manifest_path = session.workspace_paths.root().join("specman.json");
    let project = load_manifest(&manifest_path).map_err(CliError::from)?;

    let result = install(&project, &session.workspace_paths).map_err(CliError::from)?;

    let report = InstallReport {
        requested: result.entries.len(),
        entries: result.entries,
        materialized: result
            .materialized
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
    };

    Ok(CommandResult::Install { report })
}
