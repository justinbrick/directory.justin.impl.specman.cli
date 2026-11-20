use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use specman::{FilesystemWorkspaceLocator, SpecmanEnv, WorkspaceLocator, WorkspacePaths};

use crate::error::{CliError, ExitStatus};
use crate::util::Verbosity;

/// The shared environment assembled for a single CLI invocation. It captures the resolved
/// workspace context, the data-model adapter, and logging preferences so downstream commands
/// reuse the same state without recomputing filesystem details (see
/// spec/specman-cli/spec.md#entity-clisession and #entity-workspacecontext).
pub struct CliSession {
    pub workspace_paths: WorkspacePaths,
    pub env: Arc<SpecmanEnv>,
    pub verbosity: Verbosity,
}

impl CliSession {
    /// Resolves the active workspace (honoring an optional `--workspace` override) and wires the
    /// bundled data-model adapter so downstream commands satisfy the Workspace Context Resolution
    /// and Data Model Activation concepts.
    pub fn bootstrap(
        workspace_override: Option<String>,
        verbosity: Verbosity,
    ) -> Result<Self, CliError> {
        let locator: Arc<FilesystemWorkspaceLocator> = match workspace_override {
            Some(path) => Arc::new(FilesystemWorkspaceLocator::new(PathBuf::from(path))),
            None => Arc::new(FilesystemWorkspaceLocator::from_current_dir()?),
        };

        let workspace_paths = locator.workspace()?;
        // The bundled InMemoryAdapter is the only supported adapter (Data Model Activation).
        let env = SpecmanEnv::new(locator)?;

        Ok(Self {
            workspace_paths,
            env: Arc::new(env),
            verbosity,
        })
    }
}

/// Resolves the target root for a creation command (`init`, `spec new`, `impl new`).
///
/// Unlike [`CliSession::bootstrap`], this does NOT require a `specman.json` manifest to already
/// exist — creation commands are precisely the ones that first write it. The target root must be
/// an absolute path (or the current working directory when no override is supplied), and must
/// contain a `.git/` directory, so creation refuses to run outside a git repository.
pub fn resolve_creation_root(workspace_override: Option<String>) -> Result<PathBuf, CliError> {
    let raw = match workspace_override {
        Some(path) => {
            let candidate = PathBuf::from(&path);
            if !candidate.is_absolute() {
                return Err(CliError::new(
                    format!("workspace root '{path}' must be an absolute path"),
                    ExitStatus::Usage,
                ));
            }
            candidate
        }
        None => std::env::current_dir()?,
    };

    let root = lexical_normalize(&raw);

    if root.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(CliError::new(
            format!(
                "workspace root '{}' must not traverse parent directories (..)",
                raw.display()
            ),
            ExitStatus::Usage,
        ));
    }

    if root.exists() && !root.is_dir() {
        return Err(CliError::new(
            format!("workspace root {} is not a directory", root.display()),
            ExitStatus::Usage,
        ));
    }

    Ok(root)
}

/// Verifies a git repository exists at `root`, failing with a descriptive error otherwise.
pub fn ensure_git_repository(root: &Path) -> Result<(), CliError> {
    if !root.join(".git").is_dir() {
        return Err(CliError::new(
            format!(
                "workspace root {} is missing a .git directory; run `git init` there first",
                root.display()
            ),
            ExitStatus::Usage,
        ));
    }
    Ok(())
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
