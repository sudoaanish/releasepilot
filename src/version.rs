use anyhow::{anyhow, Context, Result};
use regex::Regex;
use std::fs;
use std::path::Path;

/// Extract the version string from a supported project file
pub fn extract_version_from_file<P: AsRef<Path>>(path: P) -> Result<Option<String>> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(anyhow!("File does not exist: {:?}", path));
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("Invalid filename: {:?}", path))?;

    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read file: {:?}", path))?;

    let content = content.trim_start_matches('\u{feff}');

    if filename == "package.json" || filename == "update.json" {
        let json: serde_json::Value = serde_json::from_str(content)
            .with_context(|| format!("Failed to parse JSON file {:?}", path))?;
        if let Some(v) = json.get("version").and_then(|v| v.as_str()) {
            return Ok(Some(v.to_string()));
        }
    } else if filename == "tauri.conf.json" {
        let json: serde_json::Value = serde_json::from_str(content)
            .with_context(|| format!("Failed to parse Tauri config {:?}", path))?;
        // Tauri v2 version
        if let Some(v) = json.get("version").and_then(|v| v.as_str()) {
            return Ok(Some(v.to_string()));
        }
        // Tauri v1 version
        if let Some(v) = json
            .get("package")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(Some(v.to_string()));
        }
    } else if filename == "Cargo.toml" {
        let toml: toml::Value = toml::from_str(content)
            .with_context(|| format!("Failed to parse Cargo.toml {:?}", path))?;
        if let Some(v) = toml
            .get("package")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(Some(v.to_string()));
        }
    } else if filename == "go.mod" {
        // Go versions are managed via git tags. No version inside go.mod
        return Ok(None);
    } else if filename.contains("build.gradle") {
        // Look for versionName = "..." or versionName = '...'
        let re_version_name = Regex::new(r#"versionName\s*=\s*["']([^"']+)["']"#)?;
        if let Some(caps) = re_version_name.captures(content) {
            return Ok(Some(caps[1].to_string()));
        }
        // Look for version = "..." or version = '...'
        let re_version = Regex::new(r#"version\s*=\s*["']([^"']+)["']"#)?;
        if let Some(caps) = re_version.captures(content) {
            return Ok(Some(caps[1].to_string()));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::extract_version_from_file;
    use std::fs;

    #[test]
    fn parses_bom_prefixed_package_json() {
        let dir =
            std::env::temp_dir().join(format!("releasepilot-bom-package-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("package.json");
        fs::write(&path, "\u{feff}{\"version\":\"1.2.3\"}\n").unwrap();
        let version = extract_version_from_file(&path).unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(version.as_deref(), Some("1.2.3"));
    }
}
