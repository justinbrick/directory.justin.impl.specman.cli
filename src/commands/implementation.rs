use std::fs;
use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::json;
use specman::{
    ArtifactId, ArtifactKind, DeleteOptions, DeleteResult, GitReference, ImplementsEntry,
    SpecManProjectManifest, delete_artifact, load_manifest, parse_manifest,
};

use crate::commands::dependencies;
use crate::commands::{CommandResult, DependencyScope};
use crate::context::CliSession;
use crate::error::{CliError, ExitStatus};
use crate::util;

pub fn command() -> Command {
    Command::new("impl")
        .about("Manage the implementation project at the active workspace root")
        .subcommand(impl_new_command())
        .subcommand(dependencies::dependency_command(
            "dependencies",
            "implementation",
        ))
        .subcommand(impl_delete_command())
}

pub fn run(session: &CliSession, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    match matches.subcommand() {
        Some(("new", sub)) => create_impl(session.workspace_paths.root(), sub),
        Some(("dependencies", sub)) => impl_dependencies(session, sub),
        Some(("delete", sub)) => delete_impl(session, sub),
        _ => Err(CliError::new("unsupported impl command", ExitStatus::Usage)),
    }
}

/// Entry point for `impl new` when no session has been bootstrapped (the project manifest does
/// not yet exist). Resolves the creation root and dispatches to the shared create routine.
pub fn run_new(root: &std::path::Path, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    crate::context::ensure_git_repository(root)?;
    create_impl(root, matches)
}

fn impl_new_command() -> Command {
    Command::new("new")
        .about("Create a new implementation project at the workspace root")
        .arg(Arg::new("name").value_name("NAME").help("Implementation name (positional)."))
        .arg(
            Arg::new("name-flag")
                .long("name")
                .value_name("VALUE")
                .help("Explicit implementation name. Must not be combined with a positional name."),
        )
        .arg(Arg::new("description").long("description").value_name("VALUE"))
        .arg(Arg::new("tags").long("tags").value_name("A,B,C"))
        .arg(
            Arg::new("implements")
                .long("implements")
                .value_name("ALIAS=URL@REF")
                .action(ArgAction::Append)
                .help("A specification to implement (alias=url@ref). May be repeated; at least one is required."),
        )
        .arg(
            Arg::new("constraints")
                .long("constraints")
                .value_name("REGEX,...")
                .action(ArgAction::Append)
                .help("Comma-separated constraint regexes attached to the preceding --implements entry."),
        )
        .arg(
            Arg::new("language")
                .long("language")
                .value_name("LANG[@VERSION]")
                .help("The implementing language (e.g. rust, rust@1.80)."),
        )
}

fn create_impl(root: &Path, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    ensure_uninitialized_at(root)?;

    let positional = matches.get_one::<String>("name").cloned();
    let named = matches.get_one::<String>("name-flag").cloned();

    // `impl new` MUST require exactly one name: positional XOR --name, error on both or neither.
    let name = match (positional, named) {
        (Some(_), Some(_)) => {
            return Err(CliError::new(
                "provide the implementation name either positionally or via --name, not both",
                ExitStatus::Usage,
            ));
        }
        (Some(name), None) | (None, Some(name)) => name,
        (None, None) => {
            return Err(CliError::new(
                "implementation name is required (positional or --name)",
                ExitStatus::Usage,
            ));
        }
    };

    let description = matches.get_one::<String>("description").cloned();
    let tags = util::split_csv(matches.get_one::<String>("tags"));
    let language = matches.get_one::<String>("language").cloned();

    // Gather parallel `--implements` and `--constraints` values. Constraints associate
    // positionally with the implements entries (constraints[0] applies to implements[0], etc.).
    let implements_raw: Vec<String> = matches
        .get_many::<String>("implements")
        .map(|vals| vals.cloned().collect())
        .unwrap_or_default();
    let constraints_raw: Vec<String> = matches
        .get_many::<String>("constraints")
        .map(|vals| vals.cloned().collect())
        .unwrap_or_default();

    if implements_raw.is_empty() {
        return Err(CliError::new(
            "at least one --implements entry is required",
            ExitStatus::Usage,
        ));
    }

    // Validate every `--implements` entry before any filesystem mutation.
    let mut implements = Vec::with_capacity(implements_raw.len());
    for (i, token) in implements_raw.iter().enumerate() {
        let reference: GitReference = util::parse_reference(token, "--implements")?;
        let constraint_regexes = constraints_raw
            .get(i)
            .map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        implements.push(ImplementsEntry {
            ref_: reference,
            constraints: constraint_regexes,
        });
    }

    // Build the manifest as JSON and validate it against the bundled data model before writing.
    let implements_value = serde_json::to_value(&implements).map_err(|e| {
        CliError::new(
            format!("failed to encode implements: {e}"),
            ExitStatus::Software,
        )
    })?;

    let mut manifest = json!({
        "name": name,
        "impl": { "implements": implements_value },
    });
    if let Some(description) = &description {
        manifest["description"] = json!(description);
    }
    if !tags.is_empty() {
        manifest["tags"] = json!(tags);
    }

    persist_manifest_value(&root, manifest)?;

    // Render the implementation document at `impl.md` (the repository root).
    let mut body = format!("# {name}\n\nImplementation of `{name}`.\n");
    if let Some(language) = &language {
        body.push_str(&format!("\n## Implementing Language\n\n{language}\n"));
    }
    fs::write(root.join("impl.md"), body)?;

    Ok(CommandResult::ImplCreated {
        name,
        path: root.join("impl.md").display().to_string(),
    })
}

fn impl_delete_command() -> Command {
    Command::new("delete")
        .about("Delete the implementation project after dependency checks")
        .arg(
            Arg::new("force")
                .long("force")
                .action(ArgAction::SetTrue)
                .help("Override dependency blockers after printing the dependency tree."),
        )
}

fn delete_impl(session: &CliSession, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    let forced = matches.get_flag("force");
    let manifest = load_manifest(&session.workspace_paths.root().join("specman.json"))
        .map_err(CliError::from)?;
    if !manifest.is_impl() {
        return Err(CliError::new(
            "active project is not an implementation",
            ExitStatus::Usage,
        ));
    }
    let name = manifest.name().to_string();
    let artifact = ArtifactId {
        kind: ArtifactKind::Implementation,
        name: name.clone(),
    };

    let result = delete_artifact(
        &session.env,
        &artifact,
        DeleteOptions {
            force: forced,
            dry_run: false,
        },
    )
    .map_err(CliError::from)?;

    let removed = match result {
        DeleteResult::Removed(r) => r,
        DeleteResult::DryRun(_) => unreachable!("dry-run not requested"),
    };

    Ok(CommandResult::ImplDeleted {
        name,
        forced,
        removed_path: removed.directory.display().to_string(),
    })
}

fn impl_dependencies(
    session: &CliSession,
    matches: &ArgMatches,
) -> Result<CommandResult, CliError> {
    let view = dependencies::parse_view(matches)?;
    let manifest = load_manifest(&session.workspace_paths.root().join("specman.json"))
        .map_err(CliError::from)?;
    let artifact = ArtifactId {
        kind: ArtifactKind::Implementation,
        name: manifest.name().to_string(),
    };
    let tree = session
        .env
        .mapping
        .dependency_tree(&artifact)
        .map_err(CliError::from)?;

    Ok(CommandResult::DependencyTree {
        scope: DependencyScope::Implementation,
        view,
        tree,
    })
}

/// Ensures the project has NOT yet declared a `spec` or `impl` object.
fn ensure_uninitialized_at(root: &Path) -> Result<(), CliError> {
    let manifest_path = root.join("specman.json");
    if manifest_path.is_file() {
        return Err(CliError::new(
            "workspace already declares a project; refusing to overwrite",
            ExitStatus::Usage,
        ));
    }
    Ok(())
}

/// Validates and writes a manifest JSON value to `specman.json` at the workspace root.
pub(crate) fn persist_manifest_value(
    root: &Path,
    value: serde_json::Value,
) -> Result<(), CliError> {
    let _: SpecManProjectManifest = parse_manifest(&value.to_string()).map_err(CliError::from)?;
    let serialized = serde_json::to_string_pretty(&value).map_err(|e| {
        CliError::new(
            format!("failed to serialize manifest: {e}"),
            ExitStatus::Software,
        )
    })?;
    fs::write(root.join("specman.json"), serialized)?;
    Ok(())
}
