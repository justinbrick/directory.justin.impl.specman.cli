---
spec: ../spec/specman-cli/spec.md
name: specman-cli
version: 3.0.0
location: ../
references:
- ref: ../spec/specman-cli/spec/specman-core/spec.md
  type: specification
  optional: false
- ref: ../../directory.justin.impl.specman.core
  type: implementation
  optional: false
---

# Implementation — SpecMan CLI

## Overview

The `specman-cli` implementation delivers the command-line binary defined in
[`spec/specman-cli/spec.md`](../spec/specman-cli/spec.md). It is a thin Rust binary that orchestrates
the capabilities of the `specman` core library (version 6.0.2), exposing a declarative command
surface for initializing workspaces, creating and deleting specifications and implementations,
resolving and installing dependencies, inspecting dependency graphs, mutating project metadata,
and reporting workspace health and compliance.

Every command emits structured `stdout`/`stderr` suitable for automation, maps outcomes to POSIX
`sysexits.h` exit codes, and references the governing specification section in its actionable
errors. The CLI delegates all data-model, dependency, persistence, and validation behavior to the
`specman` library, so there is a single source of truth for workspace, dependency, compliance, and
metadata behavior.

## Implementation Stack

The CLI is implemented in **Rust (2024 Edition)** and links against the `specman` library crate for
all core logic.

- **`clap@4`** drives the CLI surface with declarative subcommand definitions and global flags.
- **`serde_json@1`** serializes structured `--json` outputs.
- **`thiserror@1`** backs domain error types (within the `specman` library).
- **`tracing@0.1`** / **`tracing-subscriber@0.3`** emit contextual logs under `--verbose`.

Exit-code mapping is implemented directly against `sysexits.h` constants rather than a third-party
crate, so the binary's semantics match the specification exactly.

## References

- [`spec/specman-cli/spec.md`](../spec/specman-cli/spec.md) is the governing specification for this
  implementation (the CLI Invocation Model, Workspace Context Resolution, Lifecycle Command Surface,
  Data Model Activation, Metadata Mutation, Compliance & Validation, and Structure Discovery
  concepts).
- [`spec/specman-cli/spec/specman-core/spec.md`](../spec/specman-cli/spec/specman-core/spec.md)
  defines the canonical data model (`specman.json`, `GitReference`, `ImplementsEntry`,
  `OverrideEntry`, `Lockfile`), dependency mapping, installation, metadata mutation, compliance
  reporting, and workspace status capabilities the CLI invokes through the `specman` crate.
- The **`specman` library crate** (the `directory.justin.impl.specman.core` project, a
  path-dependency at `../directory.justin.impl.specman.core`) is the bundled SpecMan Core adapter
  the CLI links against.

## Implementation Details

### Code Location

Source lives at the repository root, a binary crate listed in `Cargo.toml`. The layout is organized
as:

- `src/main.rs` — entry point that calls `specman_cli::run()`.
- `src/lib.rs` — re-exports `run`, `run_cli`, and `CliError` for consumers and tests.
- `src/cli.rs` — constructs the `clap::Command` tree, defines global flags (`--workspace`,
  `--json`, `--verbose`), and dispatches to command modules.
- `src/context.rs` — builds `CliSession` by resolving the workspace via
  `specman::FilesystemWorkspaceLocator` and wiring the `specman::SpecmanEnv` (whose bundled
  `InMemoryAdapter` is the only supported Data Model Adapter).
- `src/error.rs` — defines `ExitStatus` (`sysexits.h` constants) and `CliError`, with conversions
  from `SpecmanError`, `WorkspaceError`, `clap::Error`, and `std::io::Error`.
- `src/util.rs` — workspace flags (`Verbosity`) and git-reference / override token parsing helpers.
- `src/formatter.rs` — text and JSON emitters plus exit-code mapping shared across commands.
- `src/commands/` — one module per command or command group: `init`, `status`, `spec`,
  `implementation` (`impl`), `install`, `metadata`, `compliance`, `structure`, and shared
  `dependencies` helpers.

Creation commands (`init`, `spec new`, `impl new`) resolve a creation root and write the project
manifest themselves; every other command resolves the active workspace via
`CliSession::bootstrap`, which requires an existing `specman.json` and fails fast when none is
found. All filesystem writes for the manifest and Markdown documents reuse the `specman` data-model
types and `parse_manifest` validation, so generated artifacts satisfy the mutual-exclusion and
data-model rules enforced by SpecMan Core.

## Concept Coverage

### Concept: CLI Invocation Model ([spec/specman-cli/spec.md#concept-cli-invocation-model](../spec/specman-cli/spec.md#concept-cli-invocation-model))

The CLI is a standalone binary (`src/main.rs`) that parses arguments and dispatches via
`run_cli` (`src/cli.rs`). Root and every subcommand/subcommand-group expose `--help`, and global
flags (`--workspace`, `--json`, `--verbose`) are marked `.global(true)` so they may appear before
or after the subcommand. Exit codes are mapped to `sysexits.h` constants in `src/error.rs`.

- `pub fn run` (`src/main.rs`, `src/lib.rs`)
- `pub fn run_cli` (`src/cli.rs`)
- `fn build_cli` (`src/cli.rs`)
- `enum ExitStatus` (`src/error.rs`)

### Concept: Workspace Context Resolution ([spec/specman-cli/spec.md#concept-workspace-context-resolution](../spec/specman-cli/spec.md#concept-workspace-context-resolution))

Workspace discovery is delegated to `specman::FilesystemWorkspaceLocator`, which scans for the
nearest git toplevel containing a `specman.json` manifest. `CliSession::bootstrap` honors an
optional `--workspace` override (or an environment variable) and caches the resolved `WorkspacePaths`
for the lifetime of the invocation. Creation commands use `resolve_creation_root`, which requires
an absolute path (or the current directory) and a `.git/` directory without requiring a pre-existing
`specman.json`.

- `pub fn bootstrap` (`src/context.rs`)
- `pub fn resolve_creation_root` (`src/context.rs`)
- `pub fn ensure_git_repository` (`src/context.rs`)

### Concept: Lifecycle Command Surface ([spec/specman-cli/spec.md#concept-lifecycle-command-surface](../spec/specman-cli/spec.md#concept-lifecycle-command-surface))

Create, delete, and dependency-inspection operations are exposed through the `spec`, `impl`, and
`install` command groups. `spec new` and `impl new` construct the `specman.json` manifest (with
`spec` or `impl` object respectively) using the `specman` data-model types, validating references
and implements entries before any filesystem mutation. `spec delete` / `impl delete` invoke
`specman::delete_artifact`, which performs dependency analysis and refuses deletion when downstream
consumers exist unless `--force` is supplied. `install` delegates to `specman::install`, which
resolves and materializes dependencies and writes `specman.lock`.

- `fn create_spec` / `fn spec_dependencies` / `fn delete_spec` (`src/commands/spec.rs`)
- `fn create_impl` / `fn impl_dependencies` / `fn delete_impl` (`src/commands/implementation.rs`)
- `pub fn run` (`src/commands/install.rs`)
- `fn dependency_command` / `fn parse_view` (`src/commands/dependencies.rs`)

### Concept: Data Model Activation ([spec/specman-cli/spec.md#concept-data-model-activation](../spec/specman-cli/spec.md#concept-data-model-activation))

The CLI depends on the `specman` library as its bundled SpecMan Core adapter. `CliSession::bootstrap`
instantiates `specman::SpecmanEnv` with the built-in `InMemoryAdapter` (the only supported adapter);
no alternative adapters may be registered. Every command serializes entities through the `specman`
data-model types and surfaces validation errors verbatim.

- `pub struct CliSession` (`src/context.rs`)
- `use specman::{SpecmanEnv, ...}` (`src/context.rs`)

### Concept: Metadata Mutation Command Surface ([spec/specman-cli/spec.md#concept-metadata-mutation-command-surface](../spec/specman-cli/spec.md#concept-metadata-mutation-command-surface))

The `metadata` command group exposes partial-update semantics by delegating to
`specman::apply_manifest_update`. Scalar fields (`metadata set --name/--description/--tags`) are
updated via `IdentityUpdate`; list fields (`references`, `implements`, `overrides`) are mutated via
dedicated add/remove subcommands mapped onto `SpecificationUpdate`/`ImplementationUpdate` and
`OverridesUpdate`. Arbitrary key-value insertion is not permitted.

- `pub fn run` (`src/commands/metadata.rs`)
- `fn run_set` / `fn run_references` / `fn run_implements` / `fn run_overrides` (`src/commands/metadata.rs`)

### Concept: Compliance & Validation Command Surface ([spec/specman-cli/spec.md#concept-compliance-validation-command-surface](../spec/specman-cli/spec.md#concept-compliance-validation-command-surface))

The `compliance` command resolves the implementation's governing specifications from
`impl.implements`, extracts their constraint groups (excluding transitive specification
dependencies), scans the source root for `[ENSURES: ...]` tags, and maps coverage via
`specman::generate_report`. Reference validation and complete-workspace validation are surfaced
through `status`, which delegates to `specman::validate_workspace_status`.

- `pub fn run` (`src/commands/compliance.rs`)
- `pub fn run` (`src/commands/status.rs`)

### Concept: Structure Discovery Command Surface ([spec/specman-cli/spec.md#concept-structure-discovery-command-surface](../spec/specman-cli/spec.md#concept-structure-discovery-command-surface))

The `structure` command group delegates to `specman::build_workspace_index` and the
`StructureQuery` trait. `structure ls` enumerates heading slugs and constraint group identifiers;
`structure show` renders Markdown content for a heading slug (`--slug`) or constraint group
(`--constraint`), honoring the library's cycle detection, deduplication, and 50-level recursion
depth.

- `fn run_ls` / `fn run_show` (`src/commands/structure.rs`)
- `fn build_index` (`src/commands/structure.rs`)

### Concept: Observability & Error Surfacing ([spec/specman-cli/spec.md#concept-observability-error-surfacing](../spec/specman-cli/spec.md#concept-observability-error-surfacing))

`--verbose` emits structured `tracing` logs capturing the resolved workspace and `.specman` paths;
`--json` switches output to structured JSON via `emit_result`. Error messages reference the
specification section (Concept or Entity) that mandated the behavior through the mapping in
`src/error.rs`.

- `pub fn emit_result` (`src/formatter.rs`)
- `fn print_json` / `fn status_report_to_json` (`src/formatter.rs`)
- `pub fn print` / `pub fn exit_code` (`src/error.rs`)

## Entity Coverage

### Entity: CliSession ([spec/specman-cli/spec.md#entity-clisession](../spec/specman-cli/spec.md#entity-clisession))

- `pub struct CliSession` (`src/context.rs`)
  - `pub workspace_paths: WorkspacePaths`
  - `pub env: Arc<SpecmanEnv>`
  - `pub verbosity: Verbosity`
- `impl CliSession` (`src/context.rs`)
  - `pub fn bootstrap`

### Entity: WorkspaceContext ([spec/specman-cli/spec.md#entity-workspacecontext](../spec/specman-cli/spec.md#entity-workspacecontext))

The workspace root, `.specman` directory, and git repository root are captured as
`specman::WorkspacePaths`, reused across all subcommands invoked within the same process.

- `pub struct WorkspacePaths` (`specman` library)
  - `pub fn root`
  - `pub fn dot_specman`
  - `pub fn ref_dir`
  - `pub fn cache_dir`
- `pub fn resolve_creation_root` (`src/context.rs`)

### Entity: LifecycleRequest ([spec/specman-cli/spec.md#entity-lifecyclerequest](../spec/specman-cli/spec.md#entity-lifecyclerequest))

Create operations are dispatched through the `spec new` / `impl new` command paths, which validate
project metadata against the data model before writing. The structured result is represented by
`CommandResult::SpecCreated` / `CommandResult::ImplCreated`, and deletion results by
`CommandResult::SpecDeleted` / `CommandResult::ImplDeleted`.

- `enum CommandResult` (`src/commands/mod.rs`)
  - `SpecCreated`
  - `ImplCreated`
  - `SpecDeleted`
  - `ImplDeleted`

### Entity: DeletionPlan ([spec/specman-cli/spec.md#entity-deletionplan](../spec/specman-cli/spec.md#entity-deletionplan))

Dependency analysis for delete requests is delegated to `specman::delete_artifact`, which returns a
`DeletePlan` (blocking dependency tree and `blocked` flag) before any filesystem mutation.

- `fn delete_spec` (`src/commands/spec.rs`)
- `fn delete_impl` (`src/commands/implementation.rs`)

### Entity: MetadataUpdate ([spec/specman-cli/spec.md#entity-metadataupdate](../spec/specman-cli/spec.md#entity-metadataupdate))

Metadata mutations are constrained to the supported fields and delegated to
`specman::apply_manifest_update`, which persists the merged `specman.json` and returns the full
updated document (`CommandResult::ManifestUpdated`).

- `fn run_set` / `fn run_references` / `fn run_implements` / `fn run_overrides` (`src/commands/metadata.rs`)

### Entity: ComplianceReport ([spec/specman-cli/spec.md#entity-compliancereport](../spec/specman-cli/spec.md#entity-compliancereport))

The compliance scan result (resolved governing specification, covered/missing constraint groups,
discovered tag locations, and orphans) is produced by `specman::generate_report` and surfaced as
`CommandResult::Compliance`.

- `pub fn run` (`src/commands/compliance.rs`)

### Entity: InstallReport ([spec/specman-cli/spec.md#entity-installreport](../spec/specman-cli/spec.md#entity-installreport))

The install result (resolved `GitReference` entries, materialized `spec/{alias}/` copies, and the
resulting lockfile state) is produced by `specman::install` and surfaced as
`CommandResult::Install` via the `InstallReport` struct.

- `pub struct InstallReport` (`src/commands/install.rs`)
- `pub fn run` (`src/commands/install.rs`)

## Operational Notes

- The CLI is a thin wrapper over the `specman` library; no workspace, dependency, persistence, or
  validation logic is reimplemented in the binary.
- Creation commands validate all git references before writing, so an invalid `--dependencies` or
  `--implements` entry fails with `EX_USAGE` without mutating the filesystem.
- Tests use full 40-character commit hashes for git references to remain hermetic: the `specman`
  library short-circuits `git ls-remote` for commit hashes, avoiding network or credential prompts
  during the test suite.
- Distribution, install scripts, and binary naming are intentionally unspecified by the
  specification; this crate names its binary `specman` via the `[[bin]]` table in `Cargo.toml`.
