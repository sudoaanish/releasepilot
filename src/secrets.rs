use std::fs;
use std::path::Path;
use std::collections::HashSet;
use anyhow::Result;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct SecretsCheck {
    pub found_in_workflows: Vec<String>,
    pub recommended_secrets: Vec<String>,
}

/// Scan `.github/workflows` to find required secrets references, and suggest recommended secrets for project type.
pub fn check_github_secrets<P: AsRef<Path>>(root: P, project_type: &str) -> Result<SecretsCheck> {
    let root = root.as_ref();
    let workflows_dir = root.join(".github/workflows");
    let mut found_secrets = HashSet::new();

    if workflows_dir.is_dir() {
        let re_secret = Regex::new(r#"secrets\.([a-zA-Z0-9_-]+)"#)?;
        if let Ok(entries) = fs::read_dir(workflows_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if ext == "yml" || ext == "yaml" {
                            if let Ok(content) = fs::read_to_string(&path) {
                                for cap in re_secret.captures_iter(&content) {
                                    let secret_name = cap[1].to_string();
                                    if secret_name != "GITHUB_TOKEN" {
                                        found_secrets.insert(secret_name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut recommended_secrets = Vec::new();
    match project_type {
        "tauri" => {
            recommended_secrets.push("TAURI_SIGNING_PRIVATE_KEY".to_string());
            recommended_secrets.push("TAURI_SIGNING_PRIVATE_KEY_PASSWORD".to_string());
        }
        "android" => {
            // Android uses a keystore to sign release APKs
            recommended_secrets.push("ANDROID_KEYSTORE_BASE64 (or project-specific keystore secret)".to_string());
            recommended_secrets.push("ANDROID_KEYSTORE_PASSWORD".to_string());
            recommended_secrets.push("ANDROID_KEY_ALIAS".to_string());
            recommended_secrets.push("ANDROID_KEY_PASSWORD".to_string());
        }
        "rust" => {
            recommended_secrets.push("CARGO_REGISTRY_TOKEN (if publishing to crates.io)".to_string());
        }
        "node-vite" => {
            recommended_secrets.push("NPM_TOKEN (if publishing to npm registry)".to_string());
        }
        _ => {}
    }

    let mut found_in_workflows: Vec<String> = found_secrets.into_iter().collect();
    found_in_workflows.sort();

    Ok(SecretsCheck {
        found_in_workflows,
        recommended_secrets,
    })
}
