use crate::checks::{CheckStatus, ReportData, Severity};

/// Print readiness report in user-friendly console format
pub fn render_text(data: &ReportData) {
    let blockers: Vec<_> = data
        .check_results
        .iter()
        .filter(|r| r.status == CheckStatus::Fail && r.severity == Severity::Blocker)
        .collect();
    let warnings: Vec<_> = data
        .check_results
        .iter()
        .filter(|r| r.status == CheckStatus::Fail && r.severity == Severity::Warning)
        .collect();

    let status_str = if !blockers.is_empty() {
        "BLOCKED (Fix blockers before releasing)"
    } else if !warnings.is_empty() {
        "WARNINGS PENDING (Review warnings before releasing)"
    } else {
        "READY FOR RELEASE"
    };

    println!("═════════════════════════════════════════════════════════════");
    println!("             ReleasePilot Release Readiness Report           ");
    println!("═════════════════════════════════════════════════════════════");
    println!();
    println!("Status:            {}", status_str);
    println!(
        "Project:           {} ({})",
        data.project_name, data.project_type
    );
    println!();

    println!("─── Git State ───");
    if let Some(error) = &data.git_error {
        println!("  Git Error:       {}", error);
    }
    if data.in_git_repo {
        println!(
            "  Clean Tree:      {}",
            if data.is_git_clean { "Yes" } else { "No" }
        );
        if !data.is_git_clean {
            for file in &data.dirty_files {
                println!("                   - {}", file);
            }
        }
        println!(
            "  Branch:          {} (Expected: {})",
            data.current_branch, data.expected_branch
        );
        if let Some(ref tag) = data.latest_tag {
            println!("  Latest Tag:      {}", tag);
        } else {
            println!("  Latest Tag:      None matching prefix");
        }
        if let Some(commits) = data.commits_since_tag {
            println!("  Commits Since:   {}", commits);
        }
    } else if data.git_error.is_some() {
        println!("  Git repository state unavailable.");
    } else {
        println!("  Not a Git repository.");
    }
    println!();

    println!("─── Version State ───");
    if data.file_versions.is_empty() {
        println!("  No version files configured.");
    } else {
        for (file, version) in &data.file_versions {
            println!("  {:<16} {}", file, version.as_deref().unwrap_or("Unknown"));
        }
        println!(
            "  Consistent:      {}",
            if data.versions_consistent {
                "Yes"
            } else {
                "No"
            }
        );
        if data.latest_tag.is_some() {
            println!(
                "  Progression:     {}",
                if data.versions_greater_than_tag {
                    "Valid"
                } else {
                    "Invalid / Needs Bump"
                }
            );
        }
    }
    println!();

    println!("─── Required Files ───");
    if data.required_files_status.is_empty() {
        println!("  No required files configured.");
    } else {
        for (file, exists) in &data.required_files_status {
            println!("  [{}] {}", if *exists { "✔" } else { "✘" }, file);
        }
    }
    println!();

    println!("─── Required Artifacts ───");
    if data.artifacts_status.is_empty() {
        println!("  No required artifacts configured.");
    } else {
        for (glob, matched) in &data.artifacts_status {
            println!("  [{}] {}", if *matched { "✔" } else { "✘" }, glob);
        }
    }
    println!();

    println!("─── Forbidden Strings ───");
    if data.forbidden_strings_results.is_empty() {
        println!("  No forbidden strings found.");
    } else {
        for res in &data.forbidden_strings_results {
            println!("  File: {}", res.file_path);
            for (line, pattern) in &res.matches {
                println!("    line {}: Found forbidden string '{}'", line, pattern);
            }
        }
    }
    println!();

    println!("─── GitHub Repository Secrets ───");
    if !data.secrets_info.found_in_workflows.is_empty() {
        println!("  Detected in workflow configurations (make sure these are set in repository settings):");
        for secret in &data.secrets_info.found_in_workflows {
            println!("    - {}", secret);
        }
    } else if !data.secrets_info.recommended_secrets.is_empty() {
        println!(
            "  No workflows analyzed. Recommendations for project type '{}':",
            data.project_type
        );
        for secret in &data.secrets_info.recommended_secrets {
            println!("    - {}", secret);
        }
    } else {
        println!("  No secrets identified or recommended.");
    }
    println!();

    if !blockers.is_empty() {
        println!("─── Blockers ({}) ───", blockers.len());
        for b in &blockers {
            println!("  [BLOCKER] {}: {}", b.name, b.message);
        }
        println!();
    }

    if !warnings.is_empty() {
        println!("─── Warnings ({}) ───", warnings.len());
        for w in &warnings {
            println!("  [WARNING] {}: {}", w.name, w.message);
        }
        println!();
    }

    println!("─── Recommended Next Actions ───");
    let mut actions = Vec::new();
    if !data.is_git_clean {
        actions.push("Commit or stash your local git changes.".to_string());
    }
    if data.in_git_repo && data.current_branch != data.expected_branch {
        actions.push(format!(
            "Switch to the configured main branch '{}'.",
            data.expected_branch
        ));
    }
    if !data.versions_consistent {
        actions.push("Align version numbers across all configured version files.".to_string());
    }
    if data.in_git_repo && data.latest_tag.is_some() && !data.versions_greater_than_tag {
        actions.push("Bump the version numbers in version files past the latest tag.".to_string());
    }
    for (file, exists) in &data.required_files_status {
        if !*exists {
            actions.push(format!("Create the missing required file '{}'.", file));
        }
    }
    for (glob, matched) in &data.artifacts_status {
        if !*matched {
            actions.push(format!(
                "Build the project to generate the required artifact matching '{}'.",
                glob
            ));
        }
    }
    if !data.forbidden_strings_results.is_empty() {
        actions
            .push("Remove all forbidden debug/localhost strings from version files.".to_string());
    }
    if !data.secrets_info.found_in_workflows.is_empty() {
        actions.push(
            "Verify that all detected GitHub Action secrets are set in repository settings."
                .to_string(),
        );
    }

    if actions.is_empty() && blockers.is_empty() && warnings.is_empty() {
        actions.push("Everything is ready! Tag and publish your release.".to_string());
    } else if actions.is_empty() && !blockers.is_empty() {
        actions.push("Resolve all blockers before tagging or publishing this release.".to_string());
    } else if actions.is_empty() {
        actions.push("Review warnings before tagging or publishing this release.".to_string());
    }

    for action in actions {
        println!("  → {}", action);
    }
    println!("═════════════════════════════════════════════════════════════");
}

/// Render readiness report in Markdown format
pub fn render_markdown(data: &ReportData) -> String {
    let blockers: Vec<_> = data
        .check_results
        .iter()
        .filter(|r| r.status == CheckStatus::Fail && r.severity == Severity::Blocker)
        .collect();
    let warnings: Vec<_> = data
        .check_results
        .iter()
        .filter(|r| r.status == CheckStatus::Fail && r.severity == Severity::Warning)
        .collect();

    let status_str = if !blockers.is_empty() {
        "🔴 BLOCKED"
    } else if !warnings.is_empty() {
        "🟡 WARNINGS PENDING"
    } else {
        "🟢 READY FOR RELEASE"
    };

    let mut md = String::new();
    md.push_str("# ReleasePilot - Release Readiness Report\n\n");
    md.push_str(&format!("**Status:** {}\n", status_str));
    md.push_str(&format!(
        "**Project:** {} (`{}`)\n\n",
        data.project_name, data.project_type
    ));

    md.push_str("## Summary\n\n");
    md.push_str("| Check | Status | Details |\n");
    md.push_str("|---|---|---|\n");
    for res in &data.check_results {
        let status_emoji = match res.status {
            CheckStatus::Pass => "✅",
            CheckStatus::Fail => match res.severity {
                Severity::Blocker => "❌",
                Severity::Warning => "⚠️",
                Severity::Info => "ℹ️",
            },
            CheckStatus::Info => "ℹ️",
        };
        md.push_str(&format!(
            "| {} | {} | {} |\n",
            res.name, status_emoji, res.message
        ));
    }
    md.push('\n');

    md.push_str("## Git State\n\n");
    if let Some(error) = &data.git_error {
        md.push_str(&format!("- **Git Error:** {}\n", error));
    }
    if data.in_git_repo {
        md.push_str(&format!(
            "- **Working Tree Clean:** {}\n",
            if data.is_git_clean {
                "Yes"
            } else {
                "No (changes pending)"
            }
        ));
        if !data.is_git_clean {
            for file in &data.dirty_files {
                md.push_str(&format!("  - `{}`\n", file));
            }
        }
        md.push_str(&format!(
            "- **Current Branch:** `{}` (Expected: `{}`)\n",
            data.current_branch, data.expected_branch
        ));
        if let Some(ref tag) = data.latest_tag {
            md.push_str(&format!("- **Latest Tag:** `{}`\n", tag));
        } else {
            md.push_str("- **Latest Tag:** None found\n");
        }
        if let Some(commits) = data.commits_since_tag {
            md.push_str(&format!("- **Commits Since Tag:** {}\n", commits));
        }
    } else if data.git_error.is_some() {
        md.push_str("*Git repository state unavailable.*\n");
    } else {
        md.push_str("*Not inside a Git repository.*\n");
    }
    md.push('\n');

    md.push_str("## Version State\n\n");
    if data.file_versions.is_empty() {
        md.push_str("*No version files configured.*\n");
    } else {
        md.push_str("| File | Version |\n");
        md.push_str("|---|---|\n");
        for (file, version) in &data.file_versions {
            md.push_str(&format!(
                "| `{}` | `{}` |\n",
                file,
                version.as_deref().unwrap_or("Unknown")
            ));
        }
        md.push('\n');
        md.push_str(&format!(
            "- **Versions Consistent:** {}\n",
            if data.versions_consistent {
                "Yes"
            } else {
                "No"
            }
        ));
        if data.latest_tag.is_some() {
            md.push_str(&format!(
                "- **Tag Progression Check:** {}\n",
                if data.versions_greater_than_tag {
                    "Passed"
                } else {
                    "Failed"
                }
            ));
        }
    }
    md.push('\n');

    md.push_str("## Required Files\n\n");
    if data.required_files_status.is_empty() {
        md.push_str("*No required files configured.*\n");
    } else {
        for (file, exists) in &data.required_files_status {
            md.push_str(&format!(
                "- [{}] `{}`\n",
                if *exists { "x" } else { " " },
                file
            ));
        }
    }
    md.push('\n');

    md.push_str("## Artifacts\n\n");
    if data.artifacts_status.is_empty() {
        md.push_str("*No artifacts configured.*\n");
    } else {
        for (glob, matched) in &data.artifacts_status {
            md.push_str(&format!(
                "- [{}] Glob `{}`\n",
                if *matched { "x" } else { " " },
                glob
            ));
        }
    }
    md.push('\n');

    md.push_str("## Forbidden Strings\n\n");
    if data.forbidden_strings_results.is_empty() {
        md.push_str("*No forbidden strings found in version files.*\n");
    } else {
        md.push_str("| File | Line | Forbidden Pattern |\n");
        md.push_str("|---|---|---|\n");
        for res in &data.forbidden_strings_results {
            for (line, pattern) in &res.matches {
                md.push_str(&format!(
                    "| `{}` | {} | `{}` |\n",
                    res.file_path, line, pattern
                ));
            }
        }
    }
    md.push('\n');

    md.push_str("## GitHub Secrets\n\n");
    if !data.secrets_info.found_in_workflows.is_empty() {
        md.push_str("The following secrets were found referenced in GitHub Action workflows. Ensure they are configured in repository settings:\n\n");
        for secret in &data.secrets_info.found_in_workflows {
            md.push_str(&format!("- `{}`\n", secret));
        }
    } else if !data.secrets_info.recommended_secrets.is_empty() {
        md.push_str(&format!("No workflow files found. Recommending the following secrets for project type `{}`:\n\n", data.project_type));
        for secret in &data.secrets_info.recommended_secrets {
            md.push_str(&format!("- `{}`\n", secret));
        }
    } else {
        md.push_str("*No secrets identified or recommended.*\n");
    }
    md.push('\n');

    if !blockers.is_empty() {
        md.push_str("## Blockers 🛑\n\n");
        for b in &blockers {
            md.push_str(&format!("- **{}**: {}\n", b.name, b.message));
        }
        md.push('\n');
    }

    if !warnings.is_empty() {
        md.push_str("## Warnings ⚠️\n\n");
        for w in &warnings {
            md.push_str(&format!("- **{}**: {}\n", w.name, w.message));
        }
        md.push('\n');
    }

    md.push_str("## Recommended Next Actions\n\n");
    let mut actions = Vec::new();
    if !data.is_git_clean {
        actions.push("Commit or stash your local git changes.".to_string());
    }
    if data.in_git_repo && data.current_branch != data.expected_branch {
        actions.push(format!(
            "Switch to the configured main branch `{}`.",
            data.expected_branch
        ));
    }
    if !data.versions_consistent {
        actions.push("Align version numbers across all configured version files.".to_string());
    }
    if data.in_git_repo && data.latest_tag.is_some() && !data.versions_greater_than_tag {
        actions.push("Bump the version numbers in version files past the latest tag.".to_string());
    }
    for (file, exists) in &data.required_files_status {
        if !*exists {
            actions.push(format!("Create the missing required file `{}`.", file));
        }
    }
    for (glob, matched) in &data.artifacts_status {
        if !*matched {
            actions.push(format!(
                "Build the project to generate the required artifact matching `{}`.",
                glob
            ));
        }
    }
    if !data.forbidden_strings_results.is_empty() {
        actions
            .push("Remove all forbidden debug/localhost strings from version files.".to_string());
    }
    if !data.secrets_info.found_in_workflows.is_empty() {
        actions.push(
            "Verify that all detected GitHub Action secrets are set in repository settings."
                .to_string(),
        );
    }

    if actions.is_empty() && blockers.is_empty() && warnings.is_empty() {
        actions.push("Everything is ready! Tag and publish your release.".to_string());
    } else if actions.is_empty() && !blockers.is_empty() {
        actions.push("Resolve all blockers before tagging or publishing this release.".to_string());
    } else if actions.is_empty() {
        actions.push("Review warnings before tagging or publishing this release.".to_string());
    }

    for action in actions {
        md.push_str(&format!("1. {}\n", action));
    }

    md
}

#[cfg(test)]
mod tests {
    use super::render_markdown;
    use crate::checks::{CheckResult, CheckStatus, ReportData, Severity};
    use crate::secrets::SecretsCheck;
    use std::collections::HashMap;

    fn report_with_warning() -> ReportData {
        ReportData {
            project_name: "fixture".to_string(),
            project_type: "rust".to_string(),
            in_git_repo: true,
            git_error: None,
            is_git_clean: true,
            dirty_files: vec![],
            current_branch: "main".to_string(),
            expected_branch: "main".to_string(),
            latest_tag: None,
            commits_since_tag: None,
            file_versions: HashMap::new(),
            versions_consistent: true,
            versions_greater_than_tag: true,
            required_files_status: vec![],
            artifacts_status: vec![],
            forbidden_strings_results: vec![],
            secrets_info: SecretsCheck {
                found_in_workflows: vec![],
                recommended_secrets: vec![],
            },
            check_results: vec![CheckResult {
                name: "Latest Git Tag".to_string(),
                status: CheckStatus::Fail,
                message: "No tags starting with prefix 'v' found.".to_string(),
                severity: Severity::Warning,
            }],
        }
    }

    fn report_with_blocker() -> ReportData {
        let mut report = report_with_warning();
        report.check_results = vec![CheckResult {
            name: "Git Repository".to_string(),
            status: CheckStatus::Fail,
            message: "Git command failed while inspecting target.".to_string(),
            severity: Severity::Blocker,
        }];
        report
    }

    #[test]
    fn warning_report_does_not_claim_everything_ready() {
        let md = render_markdown(&report_with_warning());
        assert!(!md.contains("Everything is ready! Tag and publish your release."));
        assert!(md.contains("Review warnings before tagging or publishing this release."));
    }

    #[test]
    fn blocker_report_does_not_claim_everything_ready() {
        let md = render_markdown(&report_with_blocker());
        assert!(!md.contains("Everything is ready! Tag and publish your release."));
        assert!(md.contains("Resolve all blockers before tagging or publishing this release."));
    }
}
