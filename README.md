# SpecMan CLI

The SpecMan CLI exposes the SpecMan Core runtime capabilities (dependency mapping, lifecycle
automation, metadata mutation, compliance reporting, and workspace status) through a command-line
binary for managing SpecMan workspaces.

## Features

- Workspace initialization (`init`) and lifecycle commands (`spec`, `impl`)
- Dependency tree inspection with `--upstream`, `--downstream`, and `--all`
- Dependency installation and lockfile generation (`install`)
- `specman.json` metadata mutation (`metadata set`, `metadata references/implements/overrides`)
- Compliance reporting via `[ENSURES: ...]` validation tags (`compliance`)
- Workspace health checks (`status`) and structure discovery (`structure ls` / `structure show`)

## Installation

Install via Cargo:

```bash
cargo install specman-cli
```

## Usage

For the authoritative list of commands, semantics, and exit-code guarantees, see the colocated
CLI specification at `spec/specman-cli/spec.md`.

Run `specman --help` to list available commands and `specman <command> --help` for per-command
usage.

### Initialize a workspace

```bash
# Initialize at the current directory
specman init

# Initialize a specific path
specman init /path/to/workspace

# Plan without writing
specman init /path/to/workspace --dry-run
```

### Create a specification

```bash
specman spec new my-spec --index docs/spec.md --description "..." --tags a,b
```

### Create an implementation

```bash
specman impl new my-impl --implements core=https://github.com/owner/core.git@v1.0.0
```

## Repository

Development and roadmap updates live in the main SpecMan repository:

<https://github.com/justinbrick/specman>
