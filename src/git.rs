use std::process::Command;
use std::str;
use anyhow::{Result, anyhow, Context};

/// Run a git command and return stdout as string
pub fn run_git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("Failed to execute git process. Is git installed and in PATH?")?;
    
    if !output.status.success() {
        let err_msg = str::from_utf8(&output.stderr).unwrap_or("Unknown git error");
        return Err(anyhow!("Git execution failed: {}", err_msg));
    }
    
    let stdout = str::from_utf8(&output.stdout)
        .context("Git stdout contains invalid UTF-8")?
        .trim()
        .to_string();
    Ok(stdout)
}

/// Check if the current directory is within a Git repository
pub fn is_in_git_repo() -> bool {
    run_git(&["rev-parse", "--is-inside-work-tree"])
        .map(|val| val == "true")
        .unwrap_or(false)
}

/// Check if the Git working tree is clean. Returns (is_clean, dirty_file_list)
pub fn is_clean() -> Result<(bool, Vec<String>)> {
    let output = run_git(&["status", "--porcelain"])?;
    if output.is_empty() {
        Ok((true, vec![]))
    } else {
        let dirty_files = output
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        Ok((false, dirty_files))
    }
}

/// Retrieve the active branch name
pub fn current_branch() -> Result<String> {
    run_git(&["rev-parse", "--abbrev-ref", "HEAD"])
}

/// Get the latest tag matching the specified prefix using version-sort
pub fn latest_tag(prefix: &str) -> Result<Option<String>> {
    let pattern = format!("{}*", prefix);
    let output = run_git(&["tag", "-l", &pattern, "--sort=-v:refname"])?;
    let tags: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
    if tags.is_empty() {
        Ok(None)
    } else {
        Ok(Some(tags[0].to_string()))
    }
}

/// Count commits from the tag to HEAD
pub fn commits_since_tag(tag: &str) -> Result<usize> {
    let revision_range = format!("{}..HEAD", tag);
    let output = run_git(&["rev-list", &revision_range, "--count"])?;
    let count = output.trim().parse::<usize>()
        .context("Failed to parse commit count from git rev-list")?;
    Ok(count)
}
