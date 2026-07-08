# ReleasePilot

[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![ReleasePilot](https://img.shields.io/badge/status-v0.1.0--prep-yellow.svg)](#)

ReleasePilot is a local release-readiness checker for open-source projects. It inspects your local repository state, detects the project type, runs basic release checks, scans for forbidden configurations/strings, checks for required GitHub Secrets in CI/CD workflows, and outputs a clear report.

## Features

- **Project Type Detection**: Auto-detects Tauri, Android, Go, Rust, Node/Vite, or Unknown.
- **Git Inspection**: Checks for clean working trees, branch alignment, latest tag, and commits count since latest tag.
- **Version Verification**: Extract versions from configuration/manifest files (like `package.json`, `Cargo.toml`, Gradle files) and checks for consistency and tag correspondence.
- **Forbidden Strings Scan**: Scans key files for debugging or development values that shouldn't leak to production (e.g., `localhost`).
- **Required Files Verification**: Ensures critical documentation (e.g. `README.md`, `LICENSE`) exists before tagging.
- **GitHub Secrets Extraction**: Scans `.github/workflows` configurations statically to identify required repository secrets (e.g., keystores or signing keys) or recommends them based on project types.

## Install and Build

From the ReleasePilot repository:

```powershell
cargo build --release
```

The release binary will be written by Cargo under `target/release/`. During local validation you can keep build artifacts outside the repo:

```powershell
$env:CARGO_TARGET_DIR = "C:\tmp\releasepilot-target"
cargo build --release
```

## Safe Usage

ReleasePilot is a local tool. Prefer explicit target paths so you always know which project is being inspected:

```powershell
releasepilot check --target D:\Projs\my-project
releasepilot report --target D:\Projs\my-project --format md
```

`check` and `report` inspect the target and print to stdout. `init` is safer by default: it previews the generated config unless you explicitly pass `--write`.

### 1. `releasepilot init`
Detects the project type and previews a `releasepilot.toml` for the target.

```powershell
releasepilot init --target D:\Projs\my-project
releasepilot init --target D:\Projs\my-project --dry-run
releasepilot init --target D:\Projs\my-project --write
releasepilot init --target D:\Projs\my-project --write --force --yes
```

Notes:

* Without `--write`, no files are created or changed.
* `--force` only matters with `--write`.
* Overwriting an existing config requires `--write --force --yes`.
* The generated config includes a comment header and is meant to be reviewed and edited.

### 2. `releasepilot check`
Runs all checks based on the `releasepilot.toml` configuration and prints a clean report to stdout.
* Use `--config <PATH>` to point to a custom config file.
* Use `--target <PATH>` to inspect a specific project.

### 3. `releasepilot report`
Identical checks as `check`, but outputs formatted as Markdown.
* Use `--format md` to select Markdown format (the default and current only supported format).
* Use `--target <PATH>` to inspect a specific project.

## Configuration File (`releasepilot.toml`)

Here is an example layout:

```toml
[project]
name = "MyProject"
type = "rust"
version_files = ["Cargo.toml"]

[git]
main_branch = "main"
tag_prefix = "v"
require_clean_tree = true

[artifacts]
required = ["target/release/myproject.exe"]

[checks]
required_files = ["README.md", "LICENSE"]
forbidden_strings = ["localhost", "TODO"]
```

Configured project paths must be relative to the target root. Absolute paths and parent-directory escapes such as `..` are rejected by default.

## Known Limitations

- ReleasePilot does not call the GitHub API; it only scans local workflow files for `secrets.NAME` references.
- Artifact checks are glob-based and only verify whether matching files exist.
- The current report format is text or Markdown on stdout.
- Project detection is intentionally simple and based on root-level marker files.
