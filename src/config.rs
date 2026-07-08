use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub project_type: String,
    pub version_files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitConfig {
    pub main_branch: String,
    pub tag_prefix: String,
    pub require_clean_tree: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArtifactsConfig {
    pub required: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChecksConfig {
    pub required_files: Vec<String>,
    pub forbidden_strings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub project: ProjectConfig,
    pub git: GitConfig,
    pub artifacts: ArtifactsConfig,
    pub checks: ChecksConfig,
}

impl Config {
    /// Load config from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file at {:?}", path.as_ref()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file at {:?}", path.as_ref()))?;
        Ok(config)
    }

    /// Save config to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize configuration to TOML")?;
        fs::write(path.as_ref(), content)
            .with_context(|| format!("Failed to write config file to {:?}", path.as_ref()))?;
        Ok(())
    }
}
