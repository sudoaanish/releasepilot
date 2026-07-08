mod cli;
mod config;
mod detect;
mod git;
mod version;
mod secrets;
mod checks;
mod report;

use clap::Parser;
use cli::{Cli, Commands, ReportFormat};
use config::Config;
use anyhow::Context;
use std::env;
use std::path::PathBuf;
use std::process;


fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:?}", e);
        process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args = Cli::parse();
    let current_dir = env::current_dir().context("Failed to get current working directory")?;

    match args.command {
        Commands::Init { force } => {
            let config_path = current_dir.join("releasepilot.toml");
            if config_path.exists() && !force {
                return Err(anyhow::anyhow!(
                    "releasepilot.toml already exists. Use --force to overwrite."
                ));
            }

            let project_type = detect::detect_project_type(&current_dir);
            let dir_name = current_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("ReleasePilotProject")
                .to_string();

            let config = detect::default_config_for(project_type, dir_name);
            config.save_to_file(&config_path)?;

            println!(
                "Successfully initialized releasepilot.toml for project type '{}'!",
                project_type.as_str()
            );
        }
        Commands::Check { config } => {
            let config_file = config.unwrap_or_else(|| PathBuf::from("releasepilot.toml"));
            let final_config = if config_file.exists() {
                Config::load_from_file(&config_file)?
            } else {
                let project_type = detect::detect_project_type(&current_dir);
                let dir_name = current_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("ReleasePilotProject")
                    .to_string();
                detect::default_config_for(project_type, dir_name)
            };

            let report_data = checks::run_checks(&final_config, &current_dir)?;
            report::render_text(&report_data);

            let has_blockers = report_data.check_results.iter().any(|r| {
                r.status == checks::CheckStatus::Fail && r.severity == checks::Severity::Blocker
            });

            if has_blockers {
                process::exit(1);
            }
        }
        Commands::Report { config, format } => {
            let config_file = config.unwrap_or_else(|| PathBuf::from("releasepilot.toml"));
            let final_config = if config_file.exists() {
                Config::load_from_file(&config_file)?
            } else {
                let project_type = detect::detect_project_type(&current_dir);
                let dir_name = current_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("ReleasePilotProject")
                    .to_string();
                detect::default_config_for(project_type, dir_name)
            };

            let report_data = checks::run_checks(&final_config, &current_dir)?;
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
