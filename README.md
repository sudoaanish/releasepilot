# ReleasePilot

ReleasePilot is a local release-readiness checker for open-source projects. It inspects your local repository state, detects the project type, runs basic release checks, scans for forbidden configurations/strings, checks for required GitHub Secrets in CI/CD workflows, and outputs a clear report.

## Features

- **Project Type Detection**: Auto-detects Tauri, Android, Go, Rust, Node/Vite, or Unknown.
- **Git Inspection**: Checks for clean working trees, branch alignment, latest tag, and commits count since latest tag.
- **Version Verification**: Extract versions from configuration/manifest files (like `package.json`, `Cargo.toml`, Gradle files) and checks for consistency and tag correspondence.
- **Forbidden Strings Scan**: Scans key files for debugging or development values that shouldn't leak to production (e.g., `localhost`).
- **Required Files Verification**: Ensures critical documentation (e.g. `README.md`, `LICENSE`) exists before tagging.
- **GitHub Secrets Extraction**: Scans `.github/workflows` configurations statically to identify required repository secrets (e.g., keystores or signing keys) or recommends them based on project types.

## Commands

### 1. `releasepilot init`
Detects the project type and creates a `releasepilot.toml` if it doesn't already exist.
* Use `--force` to overwrite an existing config.

### 2. `releasepilot check`
Runs all checks based on the `releasepilot.toml` configuration and prints a clean report to stdout.
* Use `--config <PATH>` to point to a custom config file.

### 3. `releasepilot report`
Identical checks as `check`, but outputs formatted as Markdown.
* Use `--format md` to select Markdown format (the default and current only supported format).

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
