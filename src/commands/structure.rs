use clap::{Arg, ArgGroup, ArgMatches, Command};
use specman::{StructureQuery, build_workspace_index};

use crate::commands::CommandResult;
use crate::context::CliSession;
use crate::error::{CliError, ExitStatus};

pub fn command() -> Command {
    Command::new("structure")
        .about("Read-only structure indexing and discovery for the active project")
        .subcommand(
            Command::new("ls").about("Enumerate heading slugs and constraint group identifiers"),
        )
        .subcommand(
            Command::new("show")
                .about("Render markdown content for a heading slug or constraint group")
                .arg(
                    Arg::new("slug")
                        .long("slug")
                        .value_name("SLUG")
                        .help("A heading slug to render."),
                )
                .arg(
                    Arg::new("constraint")
                        .long("constraint")
                        .value_name("GROUP")
                        .help("A constraint group identifier to render."),
                )
                .group(
                    ArgGroup::new("target")
                        .args(["slug", "constraint"])
                        .required(true),
                ),
        )
}

pub fn run(session: &CliSession, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    match matches.subcommand() {
        Some(("ls", _)) => run_ls(session),
        Some(("show", sub)) => run_show(session, sub),
        _ => Err(CliError::new(
            "unsupported structure command",
            ExitStatus::Usage,
        )),
    }
}

fn build_index(session: &CliSession) -> Result<specman::WorkspaceIndex, CliError> {
    build_workspace_index(&session.workspace_paths).map_err(CliError::from)
}

fn run_ls(session: &CliSession) -> Result<CommandResult, CliError> {
    let index = build_index(session)?;

    let mut headings = index.list_heading_slugs();
    let mut constraints = index.list_constraint_groups();
    headings.sort();
    constraints.sort();

    Ok(CommandResult::StructureList {
        headings,
        constraints,
    })
}

fn run_show(session: &CliSession, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    let index = build_index(session)?;

    let content = if let Some(slug) = matches.get_one::<String>("slug") {
        index.render_heading(slug)
    } else if let Some(group) = matches.get_one::<String>("constraint") {
        index.render_constraint_group(group)
    } else {
        None
    };

    match content {
        Some(content) => Ok(CommandResult::StructureShow { content }),
        None => Err(CliError::new(
            "requested identifier could not be resolved",
            ExitStatus::Data,
        )),
    }
}
