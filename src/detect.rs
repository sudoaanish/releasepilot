use crate::config::{Config, ProjectConfig, GitConfig, ArtifactsConfig, ChecksConfig};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Tauri,
    Android,
    Go,
    Rust,
    NodeVite,
    Unknown,
}

impl ProjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectType::Tauri => "tauri",
            ProjectType::Android => "android",
            ProjectType::Go => "go",
            ProjectType::Rust => "rust",
            ProjectType::NodeVite => "node-vite",
            ProjectType::Unknown => "unknown",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "tauri" => ProjectType::Tauri,
            "android" => ProjectType::Android,
            "go" => ProjectType::Go,
            "rust" => ProjectType::Rust,
            "node-vite" => ProjectType::NodeVite,
            _ => ProjectType::Unknown,
        }
    }
}

/// Detect project type based on file markers in the directory root
pub fn detect_project_type<P: AsRef<Path>>(root: P) -> ProjectType {
    let root = root.as_ref();

    // 1. Tauri
    if root.join("src-tauri/tauri.conf.json").exists() {
        return ProjectType::Tauri;
    }

    // 2. Android
    let has_android_gradle = root.join("settings.gradle").exists()
        || root.join("settings.gradle.kts").exists()
        || root.join("build.gradle").exists()
        || root.join("build.gradle.kts").exists();
    let has_android_app = root.join("app/src/main/AndroidManifest.xml").exists()
        || root.join("app/build.gradle").exists()
        || root.join("app/build.gradle.kts").exists();
    if has_android_gradle && has_android_app {
        return ProjectType::Android;
    }

    // 3. Go CLI
    if root.join("go.mod").exists() {
        return ProjectType::Go;
    }

    // 4. Rust CLI
    if root.join("Cargo.toml").exists() && root.join("src/main.rs").exists() {
        return ProjectType::Rust;
    }

    // 5. Node/Vite
    let has_package_json = root.join("package.json").exists();
    let has_vite_config = root.join("vite.config.ts").exists()
        || root.join("vite.config.js").exists()
        || root.join("vite.config.mts").exists()
        || root.join("vite.config.mjs").exists()
        || root.join("vite.config.cts").exists()
        || root.join("vite.config.cjs").exists();
    if has_package_json && has_vite_config {
        return ProjectType::NodeVite;
    }

    ProjectType::Unknown
}

/// Generate default configuration structure for a detected ProjectType
pub fn default_config_for(project_type: ProjectType, project_name: String) -> Config {
    let version_files = match project_type {
        ProjectType::Tauri => vec![
            "package.json".to_string(),
            "src-tauri/Cargo.toml".to_string(),
            "src-tauri/tauri.conf.json".to_string(),
        ],
        ProjectType::Android => {
            // Find existing app-level build gradle to determine suffix
            if Path::new("app/build.gradle.kts").exists() {
                vec!["app/build.gradle.kts".to_string()]
            } else if Path::new("app/build.gradle").exists() {
                vec!["app/build.gradle".to_string()]
            } else {
                vec!["app/build.gradle.kts".to_string()]
            }
        }
        ProjectType::Go => vec!["go.mod".to_string()],
        ProjectType::Rust => vec!["Cargo.toml".to_string()],
        ProjectType::NodeVite => vec!["package.json".to_string()],
        ProjectType::Unknown => vec![],
    };

    let required_files = match project_type {
        ProjectType::Android => vec![
            "README.md".to_string(),
            "LICENSE".to_string(),
            "CONTRIBUTORS.md".to_string(),
            "releasepilot.toml".to_string(),
        ],
        _ => vec![
            "README.md".to_string(),
            "LICENSE".to_string(),
            "releasepilot.toml".to_string(),
        ],
    };

    let forbidden_strings = match project_type {
        ProjectType::Tauri => vec!["localhost".to_string(), "/main/update.json".to_string()],
        ProjectType::Android => vec!["localhost".to_string()],
        ProjectType::NodeVite => vec!["localhost".to_string()],
        _ => vec![],
    };

    let artifacts = match project_type {
        ProjectType::Tauri => vec![
            "src-tauri/target/release/bundle/msi/*.msi".to_string(),
            "src-tauri/target/release/bundle/dmg/*.dmg".to_string(),
        ],
        ProjectType::Android => vec!["app/build/outputs/apk/release/*.apk".to_string()],
        _ => vec![],
    };

    Config {
        project: ProjectConfig {
            name: project_name,
            project_type: project_type.as_str().to_string(),
            version_files,
        },
        git: GitConfig {
            main_branch: "main".to_string(),
            tag_prefix: "v".to_string(),
            require_clean_tree: true,
        },
        artifacts: ArtifactsConfig {
            required: artifacts,
        },
        checks: ChecksConfig {
            required_files,
            forbidden_strings,
        },
    }
}
