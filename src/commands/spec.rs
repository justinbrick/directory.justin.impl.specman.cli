use std::fs;
use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::json;
use specman::{
    ArtifactId, ArtifactKind, DeleteOptions, DeleteResult, GitReference, SpecManProjectManifest,
    delete_artifact, load_manifest, parse_manifest,
};

use crate::commands::dependencies;
use crate::commands::{CommandResult, DependencyScope};
use crate::context::CliSession;
use crate::error::{CliError, ExitStatus};
use crate::util;

pub fn command() -> Command {
    Command::new("spec")
        .about("Manage the specification project at the active workspace root")
        .subcommand(spec_new_command())
        .subcommand(dependencies::dependency_command(
            "dependencies",
            "specification",
        ))
        .subcommand(spec_delete_command())
}

pub fn run(session: &CliSession, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    match matches.subcommand() {
        Some(("new", sub)) => create_spec(session.workspace_paths.root(), sub),
        Some(("dependencies", sub)) => spec_dependencies(session, sub),
        Some(("delete", sub)) => delete_spec(session, sub),
        _ => Err(CliError::new("unsupported spec command", ExitStatus::Usage)),
    }
}

/// Entry point for `spec new` when no session has been bootstrapped (the project manifest does
/// not yet exist). Resolves the creation root and dispatches to the shared create routine.
pub fn run_new(root: &std::path::Path, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    crate::context::ensure_git_repository(root)?;
    create_spec(root, matches)
}

fn spec_new_command() -> Command {
    Command::new("new")
        .about("Create a new specification project at the workspace root")
        .arg(
            Arg::new("name")
                .value_name("NAME")
                .help("Project name (positional)."),
        )
        .arg(
            Arg::new("name-flag")
                .long("name")
                .value_name("VALUE")
                .help("Explicit project name. Overrides the positional value."),
        )
        .arg(
            Arg::new("description")
                .long("description")
                .value_name("VALUE"),
        )
        .arg(Arg::new("tags").long("tags").value_name("A,B,C"))
        .arg(
            Arg::new("index")
                .long("index")
                .value_name("PATH")
                .help("Workspace-relative path to the primary specification document."),
        )
        .arg(
            Arg::new("dependencies")
                .long("dependencies")
                .value_name("ALIAS=URL@REF,...")
                .help("Comma-separated alias=url@ref entries added to spec.references."),
        )
}

fn create_spec(root: &Path, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    ensure_uninitialized_at(root)?;

    let positional = matches.get_one::<String>("name").cloned();
    let named = matches.get_one::<String>("name-flag").cloned();
    // `--name` overrides the positional value.
    let name = named.or(positional).ok_or_else(|| {
        CliError::new(
            "spec name is required (positional or --name)",
            ExitStatus::Usage,
        )
    })?;

    let description = matches.get_one::<String>("description").cloned();
    let tags = util::split_csv(matches.get_one::<String>("tags"));
    let index = matches.get_one::<String>("index").cloned();

    // Validate references before any filesystem mutation (spec new MUST fail with EX_USAGE on
    // invalid entries before touching disk).
    let references: Vec<GitReference> = matches
        .get_one::<String>("dependencies")
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|tok| util::parse_reference(tok, "dependency"))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    // Build the manifest as JSON and validate it against the bundled data model before writing.
    let mut spec_obj = json!({});
    if let Some(index) = &index {
        spec_obj["index"] = json!(index);
    }
    if !references.is_empty() {
        spec_obj["references"] = serde_json::to_value(&references).map_err(|e| {
            CliError::new(
                format!("failed to encode references: {e}"),
                ExitStatus::Software,
            )
        })?;
    }

    let mut manifest = json!({
        "name": name,
        "spec": spec_obj,
    });
    if let Some(description) = &description {
        manifest["description"] = json!(description);
    }
    if !tags.is_empty() {
        manifest["tags"] = json!(tags);
    }

    persist_manifest_value(&root, manifest)?;

    // Persist the rendered document only when `spec.index` is present. When absent, the CLI MUST
    // NOT assume a default filename (spec/specman-core/spec.md#entity-specproject).
    if let Some(index) = &index {
        let body_path = root.join(index);
        if let Some(parent) = body_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&body_path, specification_body(&name))?;
    }

    Ok(CommandResult::SpecCreated {
        name,
        index: index.clone(),
        path: match &index {
            Some(i) => root.join(i).display().to_string(),
            None => root.join("specman.json").display().to_string(),
        },
    })
}

fn spec_delete_command() -> Command {
    Command::new("delete")
        .about("Delete the specification project after dependency checks")
        .arg(
            Arg::new("force")
                .long("force")
                .action(ArgAction::SetTrue)
                .help("Override dependency blockers after printing the dependency tree."),
        )
}

fn delete_spec(session: &CliSession, matches: &ArgMatches) -> Result<CommandResult, CliError> {
    let forced = matches.get_flag("force");
    let manifest = load_manifest(&session.workspace_paths.root().join("specman.json"))
        .map_err(CliError::from)?;
    if !manifest.is_spec() {
        return Err(CliError::new(
            "active project is not a specification",
            ExitStatus::Usage,
        ));
    }
    let name = manifest.name().to_string();
    let artifact = ArtifactId {
        kind: ArtifactKind::Specification,
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

    Ok(CommandResult::SpecDeleted {
        name,
        forced,
        removed_path: removed.directory.display().to_string(),
    })
}

fn spec_dependencies(
    session: &CliSession,
    matches: &ArgMatches,
) -> Result<CommandResult, CliError> {
    let view = dependencies::parse_view(matches)?;
    let manifest = load_manifest(&session.workspace_paths.root().join("specman.json"))
        .map_err(CliError::from)?;
    let artifact = ArtifactId {
        kind: ArtifactKind::Specification,
        name: manifest.name().to_string(),
    };
    let tree = session
        .env
        .mapping
        .dependency_tree(&artifact)
        .map_err(CliError::from)?;

    Ok(CommandResult::DependencyTree {
        scope: DependencyScope::Specification,
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
    // Validate the manifest against the bundled data model before persisting.
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

fn specification_body(name: &str) -> String {
    format!("# {name}\n\nSpecification for `{name}`.\n")
}
