use clap::{ArgMatches, Command};
use specman::{
    ArtifactId, ArtifactKind, ComplianceReport, StructureQuery, WorkspacePaths,
    build_workspace_index, ensure_reference, generate_report, load_manifest, scan_source_root,
};

use crate::commands::CommandResult;
use crate::context::CliSession;
use crate::error::CliError;

pub fn command() -> Command {
    Command::new("compliance")
        .about("Generate a compliance report mapping constraint groups to validation tags")
}

pub fn run(session: &CliSession, _matches: &ArgMatches) -> Result<CommandResult, CliError> {
    let root = session.workspace_paths.root().to_path_buf();
    let manifest = load_manifest(&root.join("specman.json")).map_err(CliError::from)?;

    if !manifest.is_impl() {
        return Err(crate::error::CliError::new(
            "compliance reporting requires an implementation project",
            crate::error::ExitStatus::Usage,
        ));
    }

    let implementation = ArtifactId {
        kind: ArtifactKind::Implementation,
        name: manifest.name().to_string(),
    };

    // Resolve governing specifications from `impl.implements` and collect their constraint
    // groups. Transitive specification dependencies are intentionally excluded.
    let impl_ = manifest.impl_().expect("checked is_impl");
    let mut constraint_groups = Vec::new();
    let mut primary_spec: Option<ArtifactId> = None;

    for entry in &impl_.implements {
        let spec_dir =
            ensure_reference(&entry.ref_, &session.workspace_paths).map_err(CliError::from)?;
        let spec_manifest =
            load_manifest(&spec_dir.join("specman.json")).map_err(CliError::from)?;
        let spec_id = ArtifactId {
            kind: ArtifactKind::Specification,
            name: spec_manifest.name().to_string(),
        };
        if primary_spec.is_none() {
            primary_spec = Some(spec_id);
        }

        if let Some(index) = spec_manifest.spec().and_then(|s| s.index.as_deref()) {
            let body = spec_dir.join(index);
            if body.is_file() {
                let spec_workspace =
                    WorkspacePaths::new(spec_dir.clone(), spec_dir.join(".specman"));
                let workspace_index =
                    build_workspace_index(&spec_workspace).map_err(CliError::from)?;
                constraint_groups.extend(workspace_index.list_constraint_groups());
            }
        }
    }

    constraint_groups.sort();
    constraint_groups.dedup();

    let specification = primary_spec.unwrap_or_else(|| ArtifactId {
        kind: ArtifactKind::Specification,
        name: "unknown".to_string(),
    });

    let tags = scan_source_root(&root).map_err(CliError::from)?;

    let report: ComplianceReport = generate_report(
        specification,
        implementation,
        root.clone(),
        &constraint_groups,
        tags,
    );

    Ok(CommandResult::Compliance(report))
}
