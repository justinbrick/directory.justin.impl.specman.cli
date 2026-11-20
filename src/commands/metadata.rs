use std::path::Path;

use clap::{Arg, ArgMatches, Command};
use specman::{
    ImplementationUpdate, ImplementsEntry, ManifestUpdate, OverrideEntry, OverrideEntryRef,
    OverrideEntrySite, OverridesUpdate, SpecificationUpdate, apply_manifest_update, load_manifest,
};

use crate::commands::CommandResult;
use crate::context::CliSession;
use crate::error::{CliError, ExitStatus};
use crate::util;

pub fn command() -> Command {
    Command::new("metadata")
        .about("Mutate the active project's specman.json via partial updates")
        .subcommand(set_command())
        .subcommand(
            Command::new("references")
                .about("Add/remove spec.references entries (specification projects only)")
                .subcommand(reference_add_command())
                .subcommand(reference_remove_command()),
        )
        .subcommand(
            Command::new("implements")
                .about("Add/remove impl.implements entries (implementation projects only)")
                .subcommand(implements_add_command())
                .subcommand(implements_remove_command()),
        )
        .subcommand(
            Command::new("overrides")
                .about("Add/remove top-level overrides entries")
                .subcommand(override_add_command())
                .subcommand(override_remove_command()),
        )
}

pub fn run(session: &CliSession, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    let manifest_path = session.workspace_paths.root().join("specman.json");
    match matches.subcommand() {
        Some(("set", sub)) => run_set(session, &manifest_path, sub),
        Some(("references", sub)) => run_references(session, &manifest_path, sub),
        Some(("implements", sub)) => run_implements(session, &manifest_path, sub),
        Some(("overrides", sub)) => run_overrides(session, &manifest_path, sub),
        _ => Err(CliError::new(
            "unsupported metadata command",
            ExitStatus::Usage,
        )),
    }
}

fn set_command() -> Command {
    Command::new("set")
        .about("Update scalar metadata fields (name, description, tags)")
        .arg(Arg::new("name").long("name").value_name("VALUE"))
        .arg(
            Arg::new("description")
                .long("description")
                .value_name("VALUE"),
        )
        .arg(Arg::new("tags").long("tags").value_name("A,B,C"))
}

fn reference_add_command() -> Command {
    Command::new("add")
        .about("Append a validated alias=url@ref entry to spec.references")
        .arg(Arg::new("value").required(true).value_name("ALIAS=URL@REF"))
}

fn reference_remove_command() -> Command {
    Command::new("remove")
        .about("Remove a spec.references entry by alias")
        .arg(
            Arg::new("alias")
                .long("alias")
                .required(true)
                .value_name("ALIAS"),
        )
}

fn implements_add_command() -> Command {
    Command::new("add")
        .about("Append a validated alias=url@ref entry to impl.implements")
        .arg(Arg::new("value").required(true).value_name("ALIAS=URL@REF"))
}

fn implements_remove_command() -> Command {
    Command::new("remove")
        .about("Remove an impl.implements entry by alias")
        .arg(
            Arg::new("alias")
                .long("alias")
                .required(true)
                .value_name("ALIAS"),
        )
}

fn override_add_command() -> Command {
    Command::new("add")
        .about("Append a validated site=<site> ref=<ref> entry to overrides")
        .arg(
            Arg::new("value")
                .required(true)
                .value_name("SITE=<site> REF=<ref>"),
        )
}

fn override_remove_command() -> Command {
    Command::new("remove")
        .about("Remove an overrides entry by site")
        .arg(
            Arg::new("site")
                .long("site")
                .required(true)
                .value_name("SITE"),
        )
}

fn run_set(
    session: &CliSession,
    manifest_path: &Path,
    matches: &ArgMatches,
) -> Result<CommandResult, CliError> {
    let manifest = load_manifest(manifest_path).map_err(CliError::from)?;
    let name = matches.get_one::<String>("name").cloned();
    let description = matches.get_one::<String>("description").cloned();
    let tags = matches
        .get_one::<String>("tags")
        .map(|v| util::split_csv(Some(v)));

    let update = if manifest.is_spec() {
        ManifestUpdate::Specification(SpecificationUpdate {
            identity: specman::IdentityUpdate {
                name,
                description,
                tags,
            },
            ..Default::default()
        })
    } else {
        ManifestUpdate::Implementation(ImplementationUpdate {
            identity: specman::IdentityUpdate {
                name,
                description,
                tags,
            },
            ..Default::default()
        })
    };

    finish_metadata(session, manifest_path, update)
}

fn run_references(
    session: &CliSession,
    manifest_path: &Path,
    matches: &ArgMatches,
) -> Result<CommandResult, CliError> {
    let manifest = load_manifest(manifest_path).map_err(CliError::from)?;
    if !manifest.is_spec() {
        return Err(CliError::new(
            "`metadata references` applies to specification projects only",
            ExitStatus::Usage,
        ));
    }

    let update = match matches.subcommand() {
        Some(("add", sub)) => {
            let value = sub.get_one::<String>("value").unwrap();
            let reference = util::parse_reference(value, "--references add")?;
            ManifestUpdate::Specification(SpecificationUpdate {
                add_references: vec![reference],
                ..Default::default()
            })
        }
        Some(("remove", sub)) => {
            let alias = sub.get_one::<String>("alias").unwrap().clone();
            ManifestUpdate::Specification(SpecificationUpdate {
                remove_references: vec![alias],
                ..Default::default()
            })
        }
        _ => {
            return Err(CliError::new(
                "unsupported references command",
                ExitStatus::Usage,
            ));
        }
    };

    finish_metadata(session, manifest_path, update)
}

fn run_implements(
    session: &CliSession,
    manifest_path: &Path,
    matches: &ArgMatches,
) -> Result<CommandResult, CliError> {
    let manifest = load_manifest(manifest_path).map_err(CliError::from)?;
    if !manifest.is_impl() {
        return Err(CliError::new(
            "`metadata implements` applies to implementation projects only",
            ExitStatus::Usage,
        ));
    }

    let update = match matches.subcommand() {
        Some(("add", sub)) => {
            let value = sub.get_one::<String>("value").unwrap();
            let reference = util::parse_reference(value, "--implements add")?;
            ManifestUpdate::Implementation(ImplementationUpdate {
                add_implements: vec![ImplementsEntry {
                    ref_: reference,
                    constraints: Vec::new(),
                }],
                ..Default::default()
            })
        }
        Some(("remove", sub)) => {
            let alias = sub.get_one::<String>("alias").unwrap().clone();
            // Refuse to leave `implements` empty (data model requires at least one entry).
            if let Some(impl_) = manifest.impl_()
                && impl_.implements.len() <= 1
            {
                return Err(CliError::new(
                    "refusing to remove the last impl.implements entry",
                    ExitStatus::Data,
                ));
            }
            ManifestUpdate::Implementation(ImplementationUpdate {
                remove_implements: vec![alias],
                ..Default::default()
            })
        }
        _ => {
            return Err(CliError::new(
                "unsupported implements command",
                ExitStatus::Usage,
            ));
        }
    };

    finish_metadata(session, manifest_path, update)
}

fn run_overrides(
    session: &CliSession,
    manifest_path: &Path,
    matches: &ArgMatches,
) -> Result<CommandResult, CliError> {
    let manifest = load_manifest(manifest_path).map_err(CliError::from)?;

    let overrides = match matches.subcommand() {
        Some(("add", sub)) => {
            let value = sub.get_one::<String>("value").unwrap();
            let (site, reference) = util::parse_override(value, "--overrides add")?;
            let entry = OverrideEntry {
                site: site
                    .parse::<OverrideEntrySite>()
                    .map_err(|e| CliError::new(format!("invalid site: {e}"), ExitStatus::Usage))?,
                ref_: reference
                    .parse::<OverrideEntryRef>()
                    .map_err(|e| CliError::new(format!("invalid ref: {e}"), ExitStatus::Usage))?,
            };
            OverridesUpdate {
                add_overrides: vec![entry],
                ..Default::default()
            }
        }
        Some(("remove", sub)) => {
            let site = sub.get_one::<String>("site").unwrap().clone();
            OverridesUpdate {
                remove_overrides: vec![site],
                ..Default::default()
            }
        }
        _ => {
            return Err(CliError::new(
                "unsupported overrides command",
                ExitStatus::Usage,
            ));
        }
    };

    let update = if manifest.is_spec() {
        ManifestUpdate::Specification(SpecificationUpdate {
            overrides,
            ..Default::default()
        })
    } else {
        ManifestUpdate::Implementation(ImplementationUpdate {
            overrides,
            ..Default::default()
        })
    };

    finish_metadata(session, manifest_path, update)
}

fn finish_metadata(
    _session: &CliSession,
    manifest_path: &Path,
    update: ManifestUpdate,
) -> Result<CommandResult, CliError> {
    let manifest = apply_manifest_update(manifest_path, &update).map_err(CliError::from)?;
    Ok(CommandResult::ManifestUpdated { manifest })
}
