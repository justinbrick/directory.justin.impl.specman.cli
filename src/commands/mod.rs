use serde::Serialize;
use specman::{ComplianceReport, DependencyTree, SpecManProjectManifest, WorkspaceStatusReport};

use crate::error::ExitStatus;

pub mod compliance;
pub mod dependencies;
pub mod implementation;
pub mod init;
pub mod install;
pub mod metadata;
pub mod spec;
pub mod status;
pub mod structure;

/// The structured outcome of a CLI command, serialized for `--json` output and rendered as
/// human-readable text otherwise. Each variant maps to a stable exit status.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandResult {
    Status(WorkspaceStatusReport),
    WorkspaceInitialized {
        root: String,
        dot_specman: String,
        created: bool,
        dry_run: bool,
    },
    SpecCreated {
        name: String,
        index: Option<String>,
        path: String,
    },
    SpecDeleted {
        name: String,
        forced: bool,
        removed_path: String,
    },
    ImplCreated {
        name: String,
        path: String,
    },
    ImplDeleted {
        name: String,
        forced: bool,
        removed_path: String,
    },
    Install {
        #[serde(flatten)]
        report: install::InstallReport,
    },
    ManifestUpdated {
        manifest: SpecManProjectManifest,
    },
    Compliance(ComplianceReport),
    StructureList {
        headings: Vec<String>,
        constraints: Vec<String>,
    },
    StructureShow {
        content: String,
    },
    DependencyTree {
        scope: DependencyScope,
        view: DependencyView,
        tree: DependencyTree,
    },
}

/// Which lifecycle project a dependency query addresses.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyScope {
    Specification,
    Implementation,
}

/// The traversal direction of a dependency query.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyView {
    Downstream,
    Upstream,
    All,
}

impl CommandResult {
    pub fn exit_status(&self) -> ExitStatus {
        match self {
            CommandResult::Status(report) => match report.global_status {
                specman::StatusResult::Pass => ExitStatus::Ok,
                specman::StatusResult::Fail => ExitStatus::Data,
            },
            CommandResult::Compliance(report) => {
                if report.missing.is_empty() {
                    ExitStatus::Ok
                } else {
                    ExitStatus::Data
                }
            }
            _ => ExitStatus::Ok,
        }
    }
}
