use std::path::Path;
use std::process::Command;
use std::str;

/// Run a git command and return stdout as string
pub fn run_git(root: &Path, args: &[&str]) -> GitResult<String> {
    let output = Command::new("git").arg("-C").arg(root).args(args).output();

    let output = match output {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(GitFailure::new(
                GitErrorKind::GitMissing,
                "git executable was not found in PATH".to_string(),
            ));
        }
        Err(err) => {
            return Err(GitFailure::new(
                GitErrorKind::Other,
                format!("failed to execute git: {err}"),
            ));
        }
    };

    if !output.status.success() {
        let err_msg = str::from_utf8(&output.stderr).unwrap_or("Unknown git error");
        return Err(classify_git_error(err_msg));
    }

    let stdout = str::from_utf8(&output.stdout)
        .map_err(|_| {
            GitFailure::new(
                GitErrorKind::Other,
                "git stdout contains invalid UTF-8".to_string(),
            )
        })?
        .trim()
        .to_string();
    Ok(stdout)
}

/// Check if the current directory is within a Git repository
pub fn is_in_git_repo(root: &Path) -> GitResult<bool> {
    run_git(root, &["rev-parse", "--is-inside-work-tree"]).map(|val| val == "true")
}

/// Check if the Git working tree is clean. Returns (is_clean, dirty_file_list)
pub fn is_clean(root: &Path) -> GitResult<(bool, Vec<String>)> {
    let output = run_git(root, &["status", "--porcelain"])?;
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
pub fn current_branch(root: &Path) -> GitResult<String> {
    run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"])
}

/// Get the latest tag matching the specified prefix using version-sort
pub fn latest_tag(root: &Path, prefix: &str) -> GitResult<Option<String>> {
    let pattern = format!("{}*", prefix);
    let output = run_git(root, &["tag", "-l", &pattern, "--sort=-v:refname"])?;
    let tags: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
    if tags.is_empty() {
        Ok(None)
    } else {
        Ok(Some(tags[0].to_string()))
    }
}

/// Count commits from the tag to HEAD
pub fn commits_since_tag(root: &Path, tag: &str) -> GitResult<usize> {
    let revision_range = format!("{}..HEAD", tag);
    let output = run_git(root, &["rev-list", &revision_range, "--count"])?;
    let count = output.trim().parse::<usize>().map_err(|_| {
        GitFailure::new(
            GitErrorKind::Other,
            "failed to parse commit count from git rev-list".to_string(),
        )
    })?;
    Ok(count)
}

pub type GitResult<T> = Result<T, GitFailure>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitErrorKind {
    NotRepository,
    GitMissing,
    DubiousOwnership,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFailure {
    pub kind: GitErrorKind,
    pub message: String,
}

impl GitFailure {
    fn new(kind: GitErrorKind, message: String) -> Self {
        Self {
            kind,
            message: concise_git_message(&message),
        }
    }
}

fn classify_git_error(stderr: &str) -> GitFailure {
    let lower = stderr.to_lowercase();
    if lower.contains("dubious ownership") || lower.contains("safe.directory") {
        GitFailure::new(GitErrorKind::DubiousOwnership, stderr.to_string())
    } else if lower.contains("not a git repository")
        || lower.contains("not a git repo")
        || lower.contains("outside repository")
    {
        GitFailure::new(GitErrorKind::NotRepository, stderr.to_string())
    } else {
        GitFailure::new(GitErrorKind::Other, stderr.to_string())
    }
}

fn concise_git_message(message: &str) -> String {
    message
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("unknown git error")
        .chars()
        .take(180)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{classify_git_error, GitErrorKind};

    #[test]
    fn classifies_dubious_ownership() {
        let failure = classify_git_error("fatal: detected dubious ownership in repository");
        assert_eq!(failure.kind, GitErrorKind::DubiousOwnership);
    }

    #[test]
    fn classifies_not_repository() {
        let failure = classify_git_error("fatal: not a git repository");
        assert_eq!(failure.kind, GitErrorKind::NotRepository);
    }
}
