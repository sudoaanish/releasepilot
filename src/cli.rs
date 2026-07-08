use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "releasepilot")]
#[command(version, about = "A local release-readiness checker for open-source projects", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Detect project type and create a default releasepilot.toml config
    Init {
        /// Project directory to initialize
        #[arg(short, long)]
        target: Option<PathBuf>,

        /// Preview the generated config without writing it
        #[arg(long)]
        dry_run: bool,

        /// Write releasepilot.toml to the target directory
        #[arg(long)]
        write: bool,

        /// Force overwrite of existing configuration file
        #[arg(short, long)]
        force: bool,

        /// Confirm destructive/noninteractive overwrite when used with --write --force
        #[arg(long)]
        yes: bool,
    },
    /// Run release readiness checks and output a plain text summary
    Check {
        /// Project directory to inspect
        #[arg(short, long)]
        target: Option<PathBuf>,

        /// Path to custom releasepilot.toml config
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Run release readiness checks and output a formatted markdown report
    Report {
        /// Project directory to inspect
        #[arg(short, long)]
        target: Option<PathBuf>,

        /// Path to custom releasepilot.toml config
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Output report format
        #[arg(short, long, value_enum, default_value_t = ReportFormat::Md)]
        format: ReportFormat,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportFormat {
    /// Markdown format
    Md,
}
