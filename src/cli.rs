use std::ffi::OsString;
use std::process::ExitCode;

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::commands;
use crate::context::CliSession;
use crate::error::{CliError, ExitStatus};
use crate::formatter::{OutputFormat, emit_result};
use crate::util::Verbosity;

const NAME: &str = "specman";

pub fn run() -> ExitCode {
    init_tracing();
    match run_cli(std::env::args()) {
        Ok(code) => code,
        Err(err) => {
            err.print();
            err.exit_code()
        }
    }
}

/// Parses CLI arguments, resolves the workspace, and dispatches to the appropriate command while
/// enforcing the CLI Invocation Model (spec/specman-cli/spec.md#concept-cli-invocation-model).
pub fn run_cli<I, S>(args: I) -> Result<ExitCode, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString> + Clone,
{
    let command = build_cli();
    let matches = command.try_get_matches_from(args)?;

    let verbosity = Verbosity {
        json: matches.get_flag("json"),
        verbose: matches.get_flag("verbose"),
    };
    let output = if verbosity.json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    };

    let workspace_override = matches.get_one::<String>("workspace").cloned();

    let result = dispatch(&matches, workspace_override, verbosity)?;
    emit_result(result, output)
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}

/// Defines the root `clap::Command` tree with global flags and subcommands for every command in
/// the Lifecycle Command Surface.
fn build_cli() -> Command {
    Command::new(NAME)
        .about("SpecMan CLI")
        .arg(
            Arg::new("workspace")
                .long("workspace")
                .global(true)
                .value_name("PATH")
                .help("Specify the workspace root. Defaults to the nearest ancestor git toplevel with a specman.json manifest."),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Emit newline-delimited JSON instead of human-readable text."),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Emit additional logging about workspace paths and adapters."),
        )
        .subcommand_required(true)
        .subcommand(commands::init::command())
        .subcommand(commands::status::command())
        .subcommand(commands::spec::command())
        .subcommand(commands::implementation::command())
        .subcommand(commands::install::command())
        .subcommand(commands::metadata::command())
        .subcommand(commands::compliance::command())
        .subcommand(commands::structure::command())
}

/// Delegates parsed subcommands to their modules. `init`, `spec new`, and `impl new` run without
/// a resolved workspace (they are the commands that create it); every other command requires
/// workspace discovery and fails fast when none is found.
fn dispatch(
    matches: &ArgMatches,
    workspace_override: Option<String>,
    verbosity: Verbosity,
) -> Result<commands::CommandResult, CliError> {
    match matches.subcommand() {
        Some(("init", sub)) => commands::init::run(workspace_override, sub),
        Some(("spec", sub)) if sub.subcommand_name() == Some("new") => {
            let root = crate::context::resolve_creation_root(workspace_override)?;
            let (_, new_matches) = sub.subcommand().unwrap();
            commands::spec::run_new(&root, new_matches)
        }
        Some(("impl", sub)) if sub.subcommand_name() == Some("new") => {
            let root = crate::context::resolve_creation_root(workspace_override)?;
            let (_, new_matches) = sub.subcommand().unwrap();
            commands::implementation::run_new(&root, new_matches)
        }
        _ => {
            let session = CliSession::bootstrap(workspace_override, verbosity)?;
            if session.verbosity.verbose {
                tracing::info!(
                    workspace = %session.workspace_paths.root().display(),
                    dot_specman = %session.workspace_paths.dot_specman().display(),
                    "resolved workspace context"
                );
            }
            dispatch_with_session(&session, matches)
        }
    }
}

fn dispatch_with_session(
    session: &CliSession,
    matches: &ArgMatches,
) -> Result<commands::CommandResult, CliError> {
    match matches.subcommand() {
        Some(("status", sub)) => commands::status::run(session, sub),
        Some(("spec", sub)) => commands::spec::run(session, sub),
        Some(("impl", sub)) => commands::implementation::run(session, sub),
        Some(("install", sub)) => commands::install::run(session, sub),
        Some(("metadata", sub)) => commands::metadata::run(session, sub),
        Some(("compliance", sub)) => commands::compliance::run(session, sub),
        Some(("structure", sub)) => commands::structure::run(session, sub),
        _ => Err(CliError::new("missing command", ExitStatus::Usage)),
    }
}
