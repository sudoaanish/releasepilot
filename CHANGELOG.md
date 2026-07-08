# Changelog

## Unreleased

## v0.1.1 - 2026-07-08

- Added CI workflow for formatting, clippy, tests, and release builds.
- Added release binary packaging workflow for GitHub Releases.
- Added raw Windows `releasepilot.exe` release asset packaging.
- Documented binary installation and PATH usage.
- Fixed cross-platform path safety validation for Windows-style absolute paths in CI.

## v0.1.0 - 2026-07-07

- Added explicit `--target` support for `init`, `check`, and `report`.
- Changed `init` to preview by default and require `--write` for file creation.
- Added `--dry-run`, `--write`, and `--yes` init safety flags.
- Made git inspection use the target path explicitly with `git -C`.
- Added git error classification for missing git, dubious ownership, non-repositories, and other failures.
- Added path safety checks for configured version files, required files, and artifact patterns.
- Fixed readiness recommendations so warnings do not claim the project is fully ready.
- Added BOM-tolerant JSON parsing for version files.
- Added automated tests for target/init safety, git state, report wording, BOM parsing, and path validation.
