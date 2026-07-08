use anyhow::{anyhow, Result};
use std::path::{Component, Path, PathBuf};

pub fn validate_config_path(value: &str, label: &str) -> Result<()> {
    let path = Path::new(value);
    if path.is_absolute() || looks_like_windows_absolute_path(value) {
        return Err(anyhow!(
            "{label} must be relative and stay inside the target: {value}"
        ));
    }

    for component in path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(anyhow!(
                    "{label} cannot contain absolute or parent-directory segments: {value}"
                ));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }

    Ok(())
}

fn looks_like_windows_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    let drive_absolute = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    let unc_absolute = value.starts_with("\\\\") || value.starts_with("//");

    drive_absolute || unc_absolute
}

pub fn safe_join(root: &Path, value: &str, label: &str) -> Result<PathBuf> {
    validate_config_path(value, label)?;
    Ok(root.join(value))
}

pub fn ensure_path_within_root(root: &Path, path: &Path, label: &str) -> Result<()> {
    let root = root.canonicalize()?;
    let candidate = if path.exists() {
        path.canonicalize()?
    } else {
        let parent = path
            .parent()
            .ok_or_else(|| anyhow!("{label} has no parent path: {}", path.display()))?;
        let canonical_parent = parent.canonicalize()?;
        canonical_parent.join(path.file_name().unwrap_or_default())
    };

    if !candidate.starts_with(&root) {
        return Err(anyhow!(
            "{label} must stay inside target root {}: {}",
            root.display(),
            candidate.display()
        ));
    }

    Ok(())
}

pub fn validate_config(config: &crate::config::Config) -> Result<()> {
    for path in &config.project.version_files {
        validate_config_path(path, "version file")?;
    }
    for path in &config.checks.required_files {
        validate_config_path(path, "required file")?;
    }
    for pattern in &config.artifacts.required {
        validate_config_path(pattern, "artifact pattern")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_config_path;

    #[test]
    fn rejects_parent_escape() {
        assert!(validate_config_path("../Cargo.toml", "version file").is_err());
    }

    #[test]
    fn rejects_absolute_path() {
        assert!(validate_config_path("C:\\secrets\\.env", "version file").is_err());
        assert!(validate_config_path("C:/secrets/.env", "version file").is_err());
        assert!(validate_config_path("\\\\server\\share\\.env", "version file").is_err());
    }

    #[test]
    fn allows_relative_glob() {
        assert!(validate_config_path("target/release/*.exe", "artifact pattern").is_ok());
    }
}
