use clap::{Arg, ArgAction, ArgMatches, Command};
use specman::{WorkspaceStatusConfig, validate_workspace_status};

use crate::commands::CommandResult;
use crate::context::CliSession;
use crate::error::{CliError, ExitStatus};

pub fn command() -> Command {
    Command::new("status")
        .about("Validate workspace structure, references, cycles, and compliance")
        .arg(
            Arg::new("local")
                .long("local")
                .action(ArgAction::SetTrue)
                .help("Disable network reachability checks, restricting reference validation to syntax and filesystem checks only."),
        )
        .arg(
            Arg::new("skip")
                .long("skip")
                .value_name("CATEGORY,...")
                .help("Comma-separated validation categories to disable (structure, references, cycles, compliance)."),
        )
}

pub fn run(session: &CliSession, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    // `--local` is accepted for spec compliance; the bundled adapter's Workspace Status
    // capability performs syntax and filesystem reference checks by default.
    let _local = matches.get_flag("local");
    let skip: Vec<String> = matches
        .get_one::<String>("skip")
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default();

    let mut config = WorkspaceStatusConfig::default();

    // Disable categories requested via `--skip`.
    for category in &skip {
        match category.as_str() {
            "structure" => config.structure = false,
            "references" => config.references = false,
            "cycles" => config.cycles = false,
            "compliance" => config.compliance = false,
            other => {
                return Err(CliError::new(
                    format!(
                        "unknown status category '{other}' (expected structure, references, cycles, or compliance)"
                    ),
                    ExitStatus::Usage,
                ));
            }
        }
    }

    let report = validate_workspace_status(&session.workspace_paths, &config)?;

    Ok(CommandResult::Status(report))
}
