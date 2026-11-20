use std::env;
use std::path::{Component, Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use specman::{WorkspaceDiscovery, WorkspaceError};

use crate::commands::CommandResult;
use crate::error::{CliError, ExitStatus};

pub fn command() -> Command {
    Command::new("init")
        .about("Initialize a SpecMan workspace scaffold within a git repository")
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .help("Workspace root to initialize. Defaults to the current directory."),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Resolve and print the workspace root and paths that would be created or reused without writing to disk."),
        )
}

pub fn run(
    workspace_override: Option<String>,
    matches: &ArgMatches,
) -> Result<CommandResult, CliError> {
    let positional = matches.get_one::<String>("path").cloned();
    // `--workspace` takes precedence over the positional value when both are supplied.
    let target = match workspace_override.or(positional) {
        Some(raw) => absolutize(&raw)?,
        // When neither is supplied, the current working directory is the target path.
        None => env::current_dir()?,
    };
    let dry_run = matches.get_flag("dry-run");

    let root = target;

    if dry_run {
        let already = root.join("specman.json").is_file();
        let ctx = plan(&root, already)?;
        return Ok(CommandResult::WorkspaceInitialized {
            root: ctx.root,
            dot_specman: ctx.dot_specman,
            created: !already,
            dry_run: true,
        });
    }

    let already = root.join("specman.json").is_file();
    if already {
        // Idempotent: reuse an existing workspace without mutating files.
        let dot_specman = root.join(".specman");
        let created = ensure_scaffold(&root, &dot_specman)?;
        return Ok(CommandResult::WorkspaceInitialized {
            root: root.display().to_string(),
            dot_specman: dot_specman.display().to_string(),
            created,
            dry_run: false,
        });
    }

    // Delegate workspace creation to the SpecMan Core initializer.
    let context = WorkspaceDiscovery::create(root.clone()).map_err(map_workspace_error)?;
    let paths = context.into_paths();

    Ok(CommandResult::WorkspaceInitialized {
        root: paths.root().display().to_string(),
        dot_specman: paths.dot_specman().display().to_string(),
        created: true,
        dry_run: false,
    })
}

/// Dry-run result describing the resolved root and `.specman` directory.
struct InitContext {
    root: String,
    dot_specman: String,
}

fn plan(root: &Path, _already: bool) -> Result<InitContext, CliError> {
    // Validate the target without writing anything.
    if root.exists() && !root.is_dir() {
        return Err(CliError::new(
            format!("workspace root {} is not a directory", root.display()),
            ExitStatus::Usage,
        ));
    }

    // Refuse nested workspace creation by checking ancestors.
    for ancestor in root.ancestors().skip(1) {
        if ancestor.join("specman.json").is_file() {
            return Err(CliError::new(
                format!(
                    "cannot initialize workspace at {}; ancestor workspace already exists at {}",
                    root.display(),
                    ancestor.join("specman.json").display()
                ),
                ExitStatus::Usage,
            ));
        }
    }

    // Verify a git repository exists at the target root.
    if !root.join(".git").is_dir() {
        return Err(CliError::new(
            format!(
                "workspace root {} is missing a .git directory; run `git init` there first",
                root.display()
            ),
            ExitStatus::Usage,
        ));
    }

    Ok(InitContext {
        root: root.display().to_string(),
        dot_specman: root.join(".specman").display().to_string(),
    })
}

/// Ensures `.specman` and required subdirectories exist when a manifest is already present.
fn ensure_scaffold(root: &Path, dot_specman: &Path) -> Result<bool, CliError> {
    let missing = !dot_specman.is_dir();
    if missing {
        std::fs::create_dir_all(dot_specman.join("cache"))?;
        std::fs::create_dir_all(root.join("spec"))?;
    }
    Ok(missing)
}

fn absolutize(raw: &str) -> Result<PathBuf, CliError> {
    let candidate = PathBuf::from(raw);
    if !candidate.is_absolute() {
        return Err(CliError::new(
            format!("workspace root '{raw}' must be an absolute path"),
            ExitStatus::Usage,
        ));
    }

    // Reject path traversal (`..`) that could escape the intended root.
    if candidate
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(CliError::new(
            format!("workspace root '{raw}' must not traverse parent directories (..)"),
            ExitStatus::Usage,
        ));
    }

    Ok(lexical_normalize(&candidate))
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Component::RootDir.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if normalized
                    .components()
                    .next_back()
                    .is_some_and(|c| matches!(c, Component::Normal(_)))
                {
                    normalized.pop();
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    if normalized.as_os_str().is_empty() {
        PathBuf::from("/")
    } else {
        normalized
    }
}

fn map_workspace_error(err: WorkspaceError) -> CliError {
    let status = match &err {
        WorkspaceError::GitMissing { .. } => ExitStatus::Usage,
        WorkspaceError::NestedWorkspace { .. } => ExitStatus::Usage,
        WorkspaceError::RelativePath { .. } => ExitStatus::Usage,
        WorkspaceError::Io(_) => ExitStatus::Io,
        _ => ExitStatus::Usage,
    };
    CliError::new(err.to_string(), status)
}
