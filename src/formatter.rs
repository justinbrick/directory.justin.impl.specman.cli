use std::collections::BTreeMap;
use std::process::ExitCode;

use specman::{ArtifactKind, ArtifactSummary, DependencyEdge};

use crate::commands::install::InstallReport;
use crate::commands::{CommandResult, DependencyScope, DependencyView};
use crate::error::CliError;

pub enum OutputFormat {
    Text,
    Json,
}

/// Renders a `CommandResult` as human-readable text or JSON, converting outcomes into
/// deterministic `sysexits` exit codes.
pub fn emit_result(result: CommandResult, format: OutputFormat) -> Result<ExitCode, CliError> {
    match format {
        OutputFormat::Text => print_text(&result),
        OutputFormat::Json => print_json(&result)?,
    };
    Ok(ExitCode::from(result.exit_status().code()))
}

fn print_text(result: &CommandResult) {
    match result {
        CommandResult::Status(report) => {
            println!(
                "Global Status: {}",
                match report.global_status {
                    specman::StatusResult::Pass => "PASS",
                    specman::StatusResult::Fail => "FAIL",
                }
            );
            println!(
                "Spec/Impl Status: {}",
                match report.spec_impl_status {
                    specman::StatusResult::Pass => "PASS",
                    specman::StatusResult::Fail => "FAIL",
                }
            );
            println!("Artifact Count: {}", report.artifact_count);

            if !report.structure_errors.is_empty() {
                println!("\nStructure Errors:");
                for error in &report.structure_errors {
                    println!("  - {error}");
                }
            }
            if !report.cycle_errors.is_empty() {
                println!("\nCycle Errors:");
                for error in &report.cycle_errors {
                    println!("  - {error}");
                }
            }
            if !report.artifacts.is_empty() {
                println!("\nArtifacts:");
                for (id, status) in &report.artifacts {
                    let label = if status.reference_errors.is_empty()
                        && status.compliance_missing.is_empty()
                    {
                        "OK"
                    } else {
                        "FAIL"
                    };
                    println!(
                        "  [{label}] {name} ({kind:?})",
                        name = id.name,
                        kind = id.kind
                    );
                    for err in &status.structure_errors {
                        println!("    [Structure] {err}");
                    }
                    for issue in &status.reference_errors {
                        let loc = match (issue.line, issue.column) {
                            (Some(line), Some(col)) => format!(":{line}:{col}"),
                            (Some(line), None) => format!(":{line}"),
                            _ => String::new(),
                        };
                        println!(
                            "    [{issue:?}] {msg}{loc}",
                            issue = issue.issue_type,
                            msg = issue.destination
                        );
                    }
                    for missing in &status.compliance_missing {
                        println!("    [Compliance] Missing: {missing}");
                    }
                    for orphan in &status.compliance_orphans {
                        println!(
                            "    [Compliance] Orphan tag: {id} at {file}:{line}",
                            id = orphan.identifier,
                            file = orphan.location.file_path,
                            line = orphan.location.line_number
                        );
                    }
                }
            }
        }
        CommandResult::WorkspaceInitialized {
            root,
            dot_specman,
            created,
            dry_run,
        } => {
            if *dry_run {
                if *created {
                    println!(
                        "Dry run: would initialize workspace at {root} (.specman at {dot_specman})"
                    );
                } else {
                    println!(
                        "Dry run: workspace already initialized at {root} (.specman at {dot_specman})"
                    );
                }
            } else if *created {
                println!("Initialized workspace at {root} (.specman: {dot_specman})");
            } else {
                println!("Workspace already initialized at {root} (.specman: {dot_specman})");
            }
        }
        CommandResult::SpecCreated { name, index, path } => match index {
            Some(index) => println!("Created specification '{name}' (index: {index}) at {path}"),
            None => println!("Created specification '{name}' at {path}"),
        },
        CommandResult::SpecDeleted {
            name,
            forced,
            removed_path,
        } => {
            if *forced {
                println!("Deleted specification '{name}' with --force (removed: {removed_path})");
            } else {
                println!("Deleted specification '{name}' (removed: {removed_path})");
            }
        }
        CommandResult::ImplCreated { name, path } => {
            println!("Created implementation '{name}' at {path}");
        }
        CommandResult::ImplDeleted {
            name,
            forced,
            removed_path,
        } => {
            if *forced {
                println!("Deleted implementation '{name}' with --force (removed: {removed_path})");
            } else {
                println!("Deleted implementation '{name}' (removed: {removed_path})");
            }
        }
        CommandResult::Install { report } => print_install(report),
        CommandResult::ManifestUpdated { manifest } => {
            println!("Updated specman.json manifest for '{}'", manifest.name());
        }
        CommandResult::Compliance(report) => {
            println!(
                "Compliance report for '{}' (spec: '{}')",
                report.implementation.name, report.specification.name
            );
            println!("Coverage:");
            if report.coverage.is_empty() {
                println!("  (none)");
            }
            for (group, tags) in &report.coverage {
                println!("  - {group}: {} tag(s)", tags.len());
                for tag in tags {
                    println!(
                        "      {file}:{line} ({tag_type:?})",
                        file = tag.location.file_path,
                        line = tag.location.line_number,
                        tag_type = tag.tag_type
                    );
                }
            }
            if !report.missing.is_empty() {
                println!("Missing coverage:");
                for group in &report.missing {
                    println!("  - {group}");
                }
            }
            if !report.orphans.is_empty() {
                println!("Orphaned tags:");
                for tag in &report.orphans {
                    println!(
                        "  - {id} at {file}:{line}",
                        id = tag.identifier,
                        file = tag.location.file_path,
                        line = tag.location.line_number
                    );
                }
            }
        }
        CommandResult::StructureList {
            headings,
            constraints,
        } => {
            println!("Headings ({}):", headings.len());
            for slug in headings {
                println!("  - {slug}");
            }
            println!("Constraint groups ({}):", constraints.len());
            for group in constraints {
                println!("  - {group}");
            }
        }
        CommandResult::StructureShow { content } => {
            println!("{content}");
        }
        CommandResult::DependencyTree { scope, view, tree } => {
            print_dependency_view(*scope, *view, tree);
        }
    }
}

fn print_install(report: &InstallReport) {
    println!("Installed {} reference(s)", report.requested);
    for materialized in &report.materialized {
        println!("  - {materialized}");
    }
}

fn print_json(result: &CommandResult) -> Result<(), CliError> {
    let payload = match result {
        // `WorkspaceStatusReport.artifacts` is keyed by `ArtifactId`, which is not a JSON string,
        // so we serialize it manually with stringified keys.
        CommandResult::Status(report) => status_report_to_json(report)?,
        other => serde_json::to_value(other).map_err(|e| {
            CliError::new(
                format!("failed to serialize output: {e}"),
                crate::error::ExitStatus::Software,
            )
        })?,
    };
    println!("{payload}");
    Ok(())
}

/// Serializes a [`specman::WorkspaceStatusReport`] to JSON, stringifying the `ArtifactId` keys of
/// the `artifacts` map (serialized as `"kind://name"`).
fn status_report_to_json(
    report: &specman::WorkspaceStatusReport,
) -> Result<serde_json::Value, CliError> {
    let mut artifacts = serde_json::Map::new();
    for (id, status) in &report.artifacts {
        let status_value = serde_json::to_value(status).map_err(|e| {
            CliError::new(
                format!("failed to serialize artifact status: {e}"),
                crate::error::ExitStatus::Software,
            )
        })?;
        artifacts.insert(id.to_string(), status_value);
    }

    let global_status = match report.global_status {
        specman::StatusResult::Pass => "Pass",
        specman::StatusResult::Fail => "Fail",
    };
    let spec_impl_status = match report.spec_impl_status {
        specman::StatusResult::Pass => "Pass",
        specman::StatusResult::Fail => "Fail",
    };

    Ok(serde_json::json!({
        "type": "status",
        "global_status": global_status,
        "spec_impl_status": spec_impl_status,
        "artifacts": artifacts,
        "cycle_errors": report.cycle_errors,
        "structure_errors": report.structure_errors,
        "artifact_count": report.artifact_count,
    }))
}

fn print_dependency_view(
    scope: DependencyScope,
    view: DependencyView,
    tree: &specman::DependencyTree,
) {
    println!(
        "Dependency tree ({}) for {} '{}'",
        view_label(view),
        scope_label(scope),
        tree.root.id.name
    );

    match view {
        DependencyView::All => {
            render_direction_section("Downstream", &tree.downstream, tree);
            render_direction_section("Upstream", &tree.upstream, tree);
        }
        DependencyView::Downstream => {
            render_direction_section("Downstream", &tree.downstream, tree)
        }
        DependencyView::Upstream => render_direction_section("Upstream", &tree.upstream, tree),
    }
}

fn render_direction_section(title: &str, edges: &[DependencyEdge], tree: &specman::DependencyTree) {
    println!("  {title} ({} edge(s)):", edges.len());
    println!("    {}", artifact_label(&tree.root));
    if edges.is_empty() {
        println!("      (none)");
        return;
    }

    let children_map = build_children_map(edges);
    let mut stack = Vec::new();
    render_children(&tree.root, &children_map, "      ".to_string(), &mut stack);
}

// Edges are directional: `from` is the artifact that depends on `to`. We build a parent ->
// children map keyed by the `from` name so the root artifact's dependencies render as a tree.
fn build_children_map(edges: &[DependencyEdge]) -> BTreeMap<String, Vec<ArtifactSummary>> {
    let mut map: BTreeMap<String, Vec<ArtifactSummary>> = BTreeMap::new();

    for edge in edges {
        map.entry(edge.from.id.name.clone())
            .or_default()
            .push(edge.to.clone());
    }

    for children in map.values_mut() {
        children.sort_by(|a, b| a.id.name.cmp(&b.id.name));
    }
    map
}

fn render_children(
    parent: &ArtifactSummary,
    children_map: &BTreeMap<String, Vec<ArtifactSummary>>,
    prefix: String,
    stack: &mut Vec<String>,
) {
    if let Some(children) = children_map.get(&parent.id.name) {
        stack.push(parent.id.name.clone());
        for (index, child) in children.iter().enumerate() {
            let connector = if index + 1 == children.len() {
                "└──"
            } else {
                "├──"
            };
            let cycle = stack.contains(&child.id.name);
            if cycle {
                println!(
                    "{prefix}{connector} {} (cycle detected)",
                    artifact_label(child)
                );
                continue;
            }
            println!("{prefix}{connector} {}", artifact_label(child));
            let next_prefix = format!(
                "{prefix}{}",
                if index + 1 == children.len() {
                    "    "
                } else {
                    "│   "
                }
            );
            render_children(child, children_map, next_prefix, stack);
        }
        stack.pop();
    }
}

fn artifact_label(summary: &ArtifactSummary) -> String {
    let kind = match summary.id.kind {
        ArtifactKind::Specification => "spec",
        ArtifactKind::Implementation => "impl",
    };
    format!("{} {}", kind, summary.id.name)
}

fn view_label(view: DependencyView) -> &'static str {
    match view {
        DependencyView::Downstream => "downstream",
        DependencyView::Upstream => "upstream",
        DependencyView::All => "complete",
    }
}

fn scope_label(scope: DependencyScope) -> &'static str {
    match scope {
        DependencyScope::Specification => "specification",
        DependencyScope::Implementation => "implementation",
    }
}
