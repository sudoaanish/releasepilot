mod checks;
mod cli;
mod config;
mod detect;
mod git;
mod path_safety;
mod report;
mod secrets;
mod version;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands, ReportFormat};
use config::Config;
use std::env;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:?}", e);
        process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Init {
            target,
            dry_run,
            write,
            force,
            yes,
        } => {
            if dry_run && write {
                return Err(anyhow::anyhow!(
                    "--dry-run and --write cannot be used together."
                ));
            }

            let target_root = resolve_target(target)?;
            let config_path = target_root.join("releasepilot.toml");
            if config_path.exists() && !force && write {
                return Err(anyhow::anyhow!(
                    "releasepilot.toml already exists. Use --force to overwrite."
                ));
            }
            if config_path.exists() && force && write && !yes {
                return Err(anyhow::anyhow!(
                    "Refusing to overwrite {:?} without --yes.",
                    config_path
                ));
            }

            let project_type = detect::detect_project_type(&target_root);
            let config = init_config_for_target(&target_root, project_type);
            let preview = config.to_toml_with_header()?;

            if write {
                write_init_config(&config, &config_path, force, yes)?;
                println!("Wrote ReleasePilot config to: {}", config_path.display());
            } else {
                println!(
                    "ReleasePilot init preview for target: {}",
                    target_root.display()
                );
                println!("Planned config path: {}", config_path.display());
                println!("No files were written. Re-run with --write to create this file.");
                println!();
                print!("{}", preview);
            }

            if write {
                println!(
                    "Successfully initialized releasepilot.toml for project type '{}'!",
                    project_type.as_str()
                );
            }
        }
        Commands::Check { target, config } => {
            let target_root = resolve_target(target)?;
            let config_file = resolve_config_path(&target_root, config)?;
            let final_config = if config_file.exists() {
                Config::load_from_file(&config_file)?
            } else {
                let project_type = detect::detect_project_type(&target_root);
                let dir_name = target_root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("ReleasePilotProject")
                    .to_string();
                detect::default_config_for(project_type, dir_name)
            };

            let report_data = checks::run_checks(&final_config, &target_root)?;
            report::render_text(&report_data);

            let has_blockers = report_data.check_results.iter().any(|r| {
                r.status == checks::CheckStatus::Fail && r.severity == checks::Severity::Blocker
            });

            if has_blockers {
                process::exit(1);
            }
        }
        Commands::Report {
            target,
            config,
            format,
        } => {
            let target_root = resolve_target(target)?;
            let config_file = resolve_config_path(&target_root, config)?;
            let final_config = if config_file.exists() {
                Config::load_from_file(&config_file)?
            } else {
                let project_type = detect::detect_project_type(&target_root);
                let dir_name = target_root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("ReleasePilotProject")
                    .to_string();
                detect::default_config_for(project_type, dir_name)
            };

            let report_data = checks::run_checks(&final_config, &target_root)?;
            match format {
                ReportFormat::Md => {
                    let md_output = report::render_markdown(&report_data);
                    println!("{}", md_output);
                }
            }

            let has_blockers = report_data.check_results.iter().any(|r| {
                r.status == checks::CheckStatus::Fail && r.severity == checks::Severity::Blocker
            });

            if has_blockers {
                process::exit(1);
            }
        }
    }

    Ok(())
}

fn resolve_target(target: Option<PathBuf>) -> Result<PathBuf> {
    let candidate = match target {
        Some(path) => path,
        None => env::current_dir().context("Failed to get current working directory")?,
    };

    if !candidate.exists() {
        return Err(anyhow::anyhow!(
            "Target path does not exist: {}",
            candidate.display()
        ));
    }
    if !candidate.is_dir() {
        return Err(anyhow::anyhow!(
            "Target path is not a directory: {}",
            candidate.display()
        ));
    }

    candidate
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize target path {}", candidate.display()))
}

fn resolve_config_path(target_root: &Path, config: Option<PathBuf>) -> Result<PathBuf> {
    let config_path = config.unwrap_or_else(|| PathBuf::from("releasepilot.toml"));
    let resolved = if config_path.is_absolute() {
        config_path
    } else {
        target_root.join(config_path)
    };

    path_safety::ensure_path_within_root(target_root, &resolved, "config file")?;
    Ok(resolved)
}

fn init_config_for_target(target_root: &Path, project_type: detect::ProjectType) -> Config {
    let dir_name = target_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ReleasePilotProject")
        .to_string();
    detect::default_config_for(project_type, dir_name)
}

fn write_init_config(config: &Config, config_path: &Path, force: bool, yes: bool) -> Result<()> {
    if config_path.exists() && !force {
        return Err(anyhow::anyhow!(
            "releasepilot.toml already exists. Use --force to overwrite."
        ));
    }
    if config_path.exists() && force && !yes {
        return Err(anyhow::anyhow!(
            "Refusing to overwrite {:?} without --yes.",
            config_path
        ));
    }
    config.save_to_file(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_dir(name: &str) -> PathBuf {
        let root = env::var_os("RELEASEPILOT_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(env::temp_dir);
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = root.join(format!("releasepilot-{name}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_target_honors_explicit_path() {
        let dir = test_dir("target");
        let resolved = resolve_target(Some(dir.clone())).unwrap();
        assert_eq!(resolved, dir.canonicalize().unwrap());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn init_preview_does_not_write_config() {
        let dir = test_dir("preview");
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname='demo'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();

        let project_type = detect::detect_project_type(&dir);
        let config = init_config_for_target(&dir, project_type);
        let preview = config.to_toml_with_header().unwrap();

        assert!(preview.contains("Generated by ReleasePilot"));
        assert!(!dir.join("releasepilot.toml").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn init_write_writes_inside_target_only() {
        let dir = test_dir("write");
        let config = detect::default_config_for(detect::ProjectType::Unknown, "write".to_string());
        let config_path = dir.join("releasepilot.toml");

        write_init_config(&config, &config_path, false, false).unwrap();

        assert!(config_path.exists());
        assert!(fs::read_to_string(config_path)
            .unwrap()
            .contains("Generated by ReleasePilot"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn force_requires_yes_and_overwrites_with_yes() {
        let dir = test_dir("force");
        let config_path = dir.join("releasepilot.toml");
        fs::write(&config_path, "old").unwrap();
        let config = detect::default_config_for(detect::ProjectType::Unknown, "force".to_string());

        assert!(write_init_config(&config, &config_path, true, false).is_err());
        write_init_config(&config, &config_path, true, true).unwrap();
        assert!(fs::read_to_string(config_path)
            .unwrap()
            .contains("Generated by ReleasePilot"));
        let _ = fs::remove_dir_all(dir);
    }
}
