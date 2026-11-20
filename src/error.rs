use std::fmt;
use std::process::ExitCode;

use clap::error::ErrorKind as ClapErrorKind;
use specman::{LifecycleError, SpecmanError, WorkspaceError};

// POSIX `sysexits.h` constants. Successful execution uses `EX_OK`; failure scenarios
// choose the closest matching constant so automation can rely on consistent semantics
// across commands (spec/specman-cli/spec.md#concept-cli-invocation-model).
const EX_OK: u8 = 0;
const EX_USAGE: u8 = 64;
const EX_DATAERR: u8 = 65;
const EX_NOINPUT: u8 = 66;
const EX_UNAVAILABLE: u8 = 69;
const EX_SOFTWARE: u8 = 70;
const EX_OSERR: u8 = 71;
const EX_CANTCREAT: u8 = 73;
const EX_CONFIG: u8 = 78;

/// The set of exit statuses the CLI can surface, mapped to `sysexits.h` constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    /// `EX_OK` — successful execution.
    Ok,
    /// `EX_USAGE` — command-line usage or validation error.
    Usage,
    /// `EX_DATAERR` — input data was incorrect (validation failures).
    Data,
    /// `EX_NOINPUT` — a required input file was missing.
    NoInput,
    /// `EX_UNAVAILABLE` — a service or dependency could not be located.
    Unavailable,
    /// `EX_OSERR` — an operating-system-level failure (I/O).
    Io,
    /// `EX_CANTCREAT` — a file or directory could not be created.
    CantCreate,
    /// `EX_CONFIG` — a configuration error.
    Config,
    /// `EX_SOFTWARE` — an internal software error.
    Software,
}

impl ExitStatus {
    pub fn code(self) -> u8 {
        match self {
            ExitStatus::Ok => EX_OK,
            ExitStatus::Usage => EX_USAGE,
            ExitStatus::Data => EX_DATAERR,
            ExitStatus::NoInput => EX_NOINPUT,
            ExitStatus::Unavailable => EX_UNAVAILABLE,
            ExitStatus::Io => EX_OSERR,
            ExitStatus::CantCreate => EX_CANTCREAT,
            ExitStatus::Config => EX_CONFIG,
            ExitStatus::Software => EX_SOFTWARE,
        }
    }
}

/// A structured CLI error carrying a human-readable message and a `sysexits` status.
#[derive(Debug)]
pub struct CliError {
    message: String,
    status: ExitStatus,
}

impl CliError {
    pub fn new(message: impl Into<String>, status: ExitStatus) -> Self {
        Self {
            message: message.into(),
            status,
        }
    }

    pub fn status(&self) -> ExitStatus {
        self.status
    }

    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.status.code())
    }

    pub fn print(&self) {
        if !self.message.is_empty() {
            eprintln!("{}", self.message);
        }
    }
}

impl From<SpecmanError> for CliError {
    fn from(err: SpecmanError) -> Self {
        let status = match &err {
            SpecmanError::UnknownWorkType(_) => ExitStatus::Usage,
            SpecmanError::Dependency(message) => {
                // Dependency-cycle errors are data errors; dependency resolution failures
                // that stem from a missing artifact map to `EX_UNAVAILABLE`.
                if message.contains("cycle") {
                    ExitStatus::Data
                } else {
                    ExitStatus::Unavailable
                }
            }
            SpecmanError::MissingTarget(_) => ExitStatus::Data,
            SpecmanError::Lifecycle(err) => match err {
                LifecycleError::DeletionBlocked { .. } => ExitStatus::Data,
                LifecycleError::PlanTargetMismatch { .. } => ExitStatus::Software,
                LifecycleError::Context { source, .. } => match source.as_ref() {
                    LifecycleError::DeletionBlocked { .. } => ExitStatus::Data,
                    LifecycleError::PlanTargetMismatch { .. } => ExitStatus::Software,
                    LifecycleError::Context { .. } => ExitStatus::Software,
                },
            },
            SpecmanError::Workspace(_) => ExitStatus::Usage,
            SpecmanError::Serialization(_) => ExitStatus::Data,
            SpecmanError::Io(_) => ExitStatus::Io,
        };
        CliError::new(err.to_string(), status)
    }
}

impl From<WorkspaceError> for CliError {
    fn from(err: WorkspaceError) -> Self {
        let status = match &err {
            WorkspaceError::NotFound { .. } => ExitStatus::Config,
            WorkspaceError::ManifestMissing { .. } => ExitStatus::Config,
            WorkspaceError::GitMissing { .. } => ExitStatus::Usage,
            WorkspaceError::InvalidStart { .. } => ExitStatus::Usage,
            WorkspaceError::InvalidHandle { .. } => ExitStatus::Usage,
            WorkspaceError::NestedWorkspace { .. } => ExitStatus::Usage,
            WorkspaceError::OutsideWorkspace { .. } => ExitStatus::Usage,
            WorkspaceError::Monorepo { .. } => ExitStatus::Config,
            WorkspaceError::RelativePath { .. } => ExitStatus::Usage,
            WorkspaceError::Io(_) => ExitStatus::Io,
        };
        CliError::new(err.to_string(), status)
    }
}

impl From<clap::Error> for CliError {
    fn from(err: clap::Error) -> Self {
        let status = match err.kind() {
            ClapErrorKind::DisplayHelp | ClapErrorKind::DisplayVersion => ExitStatus::Ok,
            _ => ExitStatus::Usage,
        };
        if status == ExitStatus::Ok {
            let _ = err.print();
            CliError::new(String::new(), status)
        } else {
            CliError::new(err.to_string(), status)
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        let status = match err.kind() {
            std::io::ErrorKind::NotFound => ExitStatus::NoInput,
            std::io::ErrorKind::PermissionDenied => ExitStatus::CantCreate,
            _ => ExitStatus::Io,
        };
        CliError::new(err.to_string(), status)
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_match_sysexits() {
        assert_eq!(ExitStatus::Ok.code(), 0);
        assert_eq!(ExitStatus::Usage.code(), 64);
        assert_eq!(ExitStatus::Data.code(), 65);
        assert_eq!(ExitStatus::NoInput.code(), 66);
        assert_eq!(ExitStatus::Unavailable.code(), 69);
        assert_eq!(ExitStatus::Software.code(), 70);
        assert_eq!(ExitStatus::Io.code(), 71);
        assert_eq!(ExitStatus::CantCreate.code(), 73);
        assert_eq!(ExitStatus::Config.code(), 78);
    }

    #[test]
    fn serialization_maps_to_data_error() {
        let err = CliError::from(SpecmanError::Serialization("bad".into()));
        assert_eq!(err.status(), ExitStatus::Data);
    }

    #[test]
    fn unknown_work_type_maps_to_usage() {
        let err = CliError::from(SpecmanError::UnknownWorkType("nope".to_string()));
        assert_eq!(err.status(), ExitStatus::Usage);
    }
}
