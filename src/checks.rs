use crate::config::Config;
use crate::git;
use crate::version;
use crate::secrets::{self, SecretsCheck};
use std::path::Path;
use std::collections::HashMap;
use std::fs;
use anyhow::Result;
use glob::glob;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Blocker,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone)]
pub struct FileForbiddenStrings {
    pub file_path: String,
    pub matches: Vec<(usize, String)>, // (line_number, matched_string)
}

#[derive(Debug, Clone)]
pub struct ReportData {
    pub project_name: String,
    pub project_type: String,
    pub in_git_repo: bool,
    pub is_git_clean: bool,
    pub dirty_files: Vec<String>,
    pub current_branch: String,
    pub expected_branch: String,
    pub latest_tag: Option<String>,
    pub commits_since_tag: Option<usize>,
    pub file_versions: HashMap<String, Option<String>>,
    pub versions_consistent: bool,
    pub versions_greater_than_tag: bool,
    pub required_files_status: Vec<(String, bool)>, // (file_path, exists)
    pub artifacts_status: Vec<(String, bool)>, // (glob_pattern, has_match)
    pub forbidden_strings_results: Vec<FileForbiddenStrings>,
    pub secrets_info: SecretsCheck,
    pub check_results: Vec<CheckResult>,
}

/// Helper function to compare version strings component-wise (e.g. 1.2.3 vs 1.2.2)
pub fn compare_versions(v1: &str, v2: &str) -> std::cmp::Ordering {
    let clean_v1 = v1.trim_start_matches('v');
    let clean_v2 = v2.trim_start_matches('v');

    let parts1: Vec<&str> = clean_v1.split('.').collect();
    let parts2: Vec<&str> = clean_v2.split('.').collect();

    for i in 0..std::cmp::max(parts1.len(), parts2.len()) {
        let p1 = parts1.get(i).unwrap_or(&"0");
        let p2 = parts2.get(i).unwrap_or(&"0");

        let n1 = p1.parse::<u32>();
        let n2 = p2.parse::<u32>();

        match (n1, n2) {
            (Ok(num1), Ok(num2)) => {
                if num1 != num2 {
                    return num1.cmp(&num2);
                }
            }
            _ => {
                if p1 != p2 {
                    return p1.cmp(p2);
                }
            }
        }
    }
    std::cmp::Ordering::Equal
}

/// Run all checkers based on config
pub fn run_checks(config: &Config, root: &Path) -> Result<ReportData> {
    let mut check_results = Vec::new();

    // 1. Git State Check
    let in_git_repo = git::is_in_git_repo();
    let mut is_git_clean = true;
    let mut dirty_files = Vec::new();
    let mut current_branch = String::new();
    let mut latest_tag_val = None;
    let mut commits_since_tag = None;

    if in_git_repo {
        // Clean working tree check
        if let Ok((clean, files)) = git::is_clean() {
            is_git_clean = clean;
            dirty_files = files;
            if config.git.require_clean_tree && !is_git_clean {
                check_results.push(CheckResult {
                    name: "Git Working Tree Clean".to_string(),
                    status: CheckStatus::Fail,
                    message: format!("Git working tree has {} uncommitted change(s).", dirty_files.len()),
                    severity: Severity::Blocker,
                });
            } else if !is_git_clean {
                check_results.push(CheckResult {
                    name: "Git Working Tree Clean".to_string(),
                    status: CheckStatus::Fail,
                    message: "Git working tree is dirty (non-blocking).".to_string(),
                    severity: Severity::Warning,
                });
            } else {
                check_results.push(CheckResult {
                    name: "Git Working Tree Clean".to_string(),
                    status: CheckStatus::Pass,
                    message: "Git working tree is clean.".to_string(),
                    severity: Severity::Info,
                });
            }
        }

        // Branch check
        if let Ok(branch) = git::current_branch() {
            current_branch = branch;
            if current_branch == config.git.main_branch {
                check_results.push(CheckResult {
                    name: "Git Branch".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("On the configured main branch '{}'.", current_branch),
                    severity: Severity::Info,
                });
            } else {
                check_results.push(CheckResult {
                    name: "Git Branch".to_string(),
                    status: CheckStatus::Fail,
                    message: format!("On branch '{}', expected '{}'.", current_branch, config.git.main_branch),
                    severity: Severity::Warning,
                });
            }
        }

        // Tag check
        if let Ok(tag) = git::latest_tag(&config.git.tag_prefix) {
            latest_tag_val = tag;
            if let Some(ref tag_str) = latest_tag_val {
                check_results.push(CheckResult {
                    name: "Latest Git Tag".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("Found latest tag '{}'.", tag_str),
                    severity: Severity::Info,
                });

                if let Ok(commits) = git::commits_since_tag(tag_str) {
                    commits_since_tag = Some(commits);
                    if commits > 0 {
                        check_results.push(CheckResult {
                            name: "Commits Since Tag".to_string(),
                            status: CheckStatus::Pass,
                            message: format!("{} commit(s) since latest tag '{}'.", commits, tag_str),
                            severity: Severity::Info,
                        });
                    } else {
                        check_results.push(CheckResult {
                            name: "Commits Since Tag".to_string(),
                            status: CheckStatus::Fail,
                            message: format!("No commits since latest tag '{}'. Version is already tagged.", tag_str),
                            severity: Severity::Warning,
                        });
                    }
                }
            } else {
                check_results.push(CheckResult {
                    name: "Latest Git Tag".to_string(),
                    status: CheckStatus::Fail,
                    message: format!("No tags starting with prefix '{}' found.", config.git.tag_prefix),
                    severity: Severity::Info,
                });
            }
        }
    } else {
        check_results.push(CheckResult {
            name: "Git Repository".to_string(),
            status: CheckStatus::Fail,
            message: "Not inside a Git repository. Git state checks skipped.".to_string(),
            severity: Severity::Warning,
        });
    }

    // 2. Version State Checks
    let mut file_versions = HashMap::new();
    let mut extracted_versions = Vec::new();
    for file in &config.project.version_files {
        let path = root.join(file);
        if path.exists() {
            match version::extract_version_from_file(&path) {
                Ok(Some(v)) => {
                    file_versions.insert(file.clone(), Some(v.clone()));
                    extracted_versions.push((file.clone(), v));
                }
                Ok(None) => {
                    file_versions.insert(file.clone(), None);
                }
                Err(e) => {
                    check_results.push(CheckResult {
                        name: format!("Version Extraction ({})", file),
                        status: CheckStatus::Fail,
                        message: format!("Error reading version: {}", e),
                        severity: Severity::Warning,
                    });
                }
            }
        } else {
            check_results.push(CheckResult {
                name: format!("Version File Presence ({})", file),
                status: CheckStatus::Fail,
                message: format!("Configured version file '{}' does not exist.", file),
                severity: Severity::Warning,
            });
        }
    }

    let mut versions_consistent = true;
    if extracted_versions.len() > 1 {
        let first_ver = &extracted_versions[0].1;
        for (file, ver) in &extracted_versions[1..] {
            if ver != first_ver {
                versions_consistent = false;
                check_results.push(CheckResult {
                    name: "Version Consistency".to_string(),
                    status: CheckStatus::Fail,
                    message: format!("Mismatched versions: '{}' has '{}' vs '{}' has '{}'", 
                        extracted_versions[0].0, first_ver, file, ver),
                    severity: Severity::Warning,
                });
                break;
            }
        }
    }

    if versions_consistent && !extracted_versions.is_empty() {
        check_results.push(CheckResult {
            name: "Version Consistency".to_string(),
            status: CheckStatus::Pass,
            message: format!("All version files are consistent at version '{}'.", extracted_versions[0].1),
            severity: Severity::Info,
        });
    }

    let mut versions_greater_than_tag = true;
    if !extracted_versions.is_empty() {
        if let Some(ref tag) = latest_tag_val {
            let file_ver = &extracted_versions[0].1;
            match compare_versions(file_ver, tag) {
                std::cmp::Ordering::Less => {
                    versions_greater_than_tag = false;
                    check_results.push(CheckResult {
                        name: "Version Progress".to_string(),
                        status: CheckStatus::Fail,
                        message: format!("Current project version '{}' is older than latest tag '{}'.", file_ver, tag),
                        severity: Severity::Warning,
                    });
                }
                std::cmp::Ordering::Equal => {
                    if commits_since_tag.unwrap_or(0) > 0 {
                        versions_greater_than_tag = false;
                        check_results.push(CheckResult {
                            name: "Version Progress".to_string(),
                            status: CheckStatus::Fail,
                            message: format!("Current project version '{}' equals the latest tag '{}', but commits exist since the tag. Bump the version.", file_ver, tag),
                            severity: Severity::Warning,
                        });
                    }
                }
                std::cmp::Ordering::Greater => {
                    check_results.push(CheckResult {
                        name: "Version Progress".to_string(),
                        status: CheckStatus::Pass,
                        message: format!("Current project version '{}' is bumped past latest tag '{}'.", file_ver, tag),
                        severity: Severity::Info,
                    });
                }
            }
        }
    }

    // 3. Required Files check
    let mut required_files_status = Vec::new();
    for file in &config.checks.required_files {
        let path = root.join(file);
        let exists = path.exists();
        required_files_status.push((file.clone(), exists));
        if exists {
            check_results.push(CheckResult {
                name: format!("Required File ({})", file),
                status: CheckStatus::Pass,
                message: format!("Required file '{}' exists.", file),
                severity: Severity::Info,
            });
        } else {
            check_results.push(CheckResult {
                name: format!("Required File ({})", file),
                status: CheckStatus::Fail,
                message: format!("Required file '{}' is missing.", file),
                severity: Severity::Blocker,
            });
        }
    }

    // 4. Artifacts check
    let mut artifacts_status = Vec::new();
    for pattern in &config.artifacts.required {
        let glob_path = root.join(pattern);
        let glob_str = glob_path.to_string_lossy().into_owned();
        let has_match = match glob(&glob_str) {
            Ok(paths) => paths.filter_map(|r| r.ok()).count() > 0,
            Err(_) => false,
        };
        artifacts_status.push((pattern.clone(), has_match));

        if has_match {
            check_results.push(CheckResult {
                name: format!("Required Artifact ({})", pattern),
                status: CheckStatus::Pass,
                message: format!("Artifact glob '{}' matched one or more files.", pattern),
                severity: Severity::Info,
            });
        } else {
            check_results.push(CheckResult {
                name: format!("Required Artifact ({})", pattern),
                status: CheckStatus::Fail,
                message: format!("Artifact glob '{}' did not match any files.", pattern),
                severity: Severity::Warning,
            });
        }
    }

    // 5. Forbidden Strings Scan
    let mut forbidden_strings_results = Vec::new();
    for file in &config.project.version_files {
        let path = root.join(file);
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path) {
                let mut matches = Vec::new();
                for (idx, line) in content.lines().enumerate() {
                    for forbidden in &config.checks.forbidden_strings {
                        if line.contains(forbidden) {
                            matches.push((idx + 1, forbidden.clone()));
                        }
                    }
                }
                if !matches.is_empty() {
                    forbidden_strings_results.push(FileForbiddenStrings {
                        file_path: file.clone(),
                        matches: matches.clone(),
                    });

                    for (line_num, pattern) in matches {
                        check_results.push(CheckResult {
                            name: "Forbidden String Scan".to_string(),
                            status: CheckStatus::Fail,
                            message: format!("Forbidden string '{}' found in {} at line {}.", pattern, file, line_num),
                            severity: Severity::Blocker,
                        });
                    }
                }
            }
        }
    }
    if forbidden_strings_results.is_empty() && !config.checks.forbidden_strings.is_empty() {
        check_results.push(CheckResult {
            name: "Forbidden String Scan".to_string(),
            status: CheckStatus::Pass,
            message: "No forbidden strings found in version files.".to_string(),
            severity: Severity::Info,
        });
    }

    // 6. GitHub Secrets Check
    let secrets_info = secrets::check_github_secrets(root, &config.project.project_type)?;
    
    // Add check results for secrets
    if !secrets_info.found_in_workflows.is_empty() {
        check_results.push(CheckResult {
            name: "GitHub Repository Secrets".to_string(),
            status: CheckStatus::Info,
            message: format!("Detected {} secret(s) referenced in GitHub Actions workflows.", secrets_info.found_in_workflows.len()),
            severity: Severity::Info,
        });
    } else if !secrets_info.recommended_secrets.is_empty() {
        check_results.push(CheckResult {
            name: "GitHub Repository Secrets".to_string(),
            status: CheckStatus::Info,
            message: format!("No workflow files found. Recommending {} secret(s) based on project type '{}'.", 
                secrets_info.recommended_secrets.len(), config.project.project_type),
            severity: Severity::Info,
        });
    }

    Ok(ReportData {
        project_name: config.project.name.clone(),
        project_type: config.project.project_type.clone(),
        in_git_repo,
        is_git_clean,
        dirty_files,
        current_branch,
        expected_branch: config.git.main_branch.clone(),
        latest_tag: latest_tag_val,
        commits_since_tag,
        file_versions,
        versions_consistent,
        versions_greater_than_tag,
        required_files_status,
        artifacts_status,
        forbidden_strings_results,
        secrets_info,
        check_results,
    })
}
