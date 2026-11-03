mod doctor;
mod preset_cmd;
mod vault_cmd;

use clap::{Args, Parser, Subcommand};
use doctor::DoctorEngine;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "agy")]
#[command(about = "Antigravity Multi-User Server Management & Diagnostic CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run 9-category health diagnostics across the server
    Doctor(DoctorArgs),
    /// Manage skill presets and workspace symlink projections
    #[command(subcommand)]
    Preset(PresetCommands),
    /// Manage Google Antigravity account vault and failover slots
    #[command(subcommand)]
    Vault(VaultCommands),
}

#[derive(Args)]
pub struct DoctorArgs {
    /// Output diagnostics report in JSON format
    #[arg(long)]
    pub json: bool,

    /// Verbose output showing extra debug details
    #[arg(short, long)]
    pub verbose: bool,

    /// Override path to Caddyfile
    #[arg(long)]
    pub caddyfile: Option<String>,

    /// Override path to skills library
    #[arg(long)]
    pub library: Option<String>,

    /// Override path to presets.yaml
    #[arg(long)]
    pub presets: Option<String>,

    /// Override URL for agy-hub status check
    #[arg(long)]
    pub hub_url: Option<String>,

    /// Override URL for agy-proxy status check
    #[arg(long)]
    pub proxy_url: Option<String>,
}

#[derive(Subcommand)]
enum PresetCommands {
    /// List all available domain presets and ALWAYS_ACTIVE skills
    List {
        #[arg(long)]
        presets_file: Option<PathBuf>,
    },
    /// Switch workspace skill symlinks to the specified preset
    Switch {
        /// Target preset name (e.g., 'web', 'backend', 'devops')
        preset: String,

        /// Path to developer workspace (defaults to current directory)
        #[arg(short, long)]
        workspace: Option<PathBuf>,

        /// Path to skills library
        #[arg(long)]
        library: Option<PathBuf>,

        /// Path to presets.yaml
        #[arg(long)]
        presets_file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum VaultCommands {
    /// Initialize an empty encrypted vault
    Init {
        #[arg(short, long, default_value = "/etc/antigravity/vault.enc")]
        vault_path: PathBuf,

        #[arg(short, long)]
        key_path: Option<PathBuf>,
    },
    /// Add or update an OAuth account slot in the vault
    Add {
        #[arg(short, long, default_value = "/etc/antigravity/vault.enc")]
        vault_path: PathBuf,

        #[arg(short, long)]
        key_path: Option<PathBuf>,

        #[arg(short, long)]
        email: String,

        #[arg(short, long)]
        refresh_token: String,
    },
    /// List all configured account slots in the vault
    List {
        #[arg(short, long, default_value = "/etc/antigravity/vault.enc")]
        vault_path: PathBuf,

        #[arg(short, long)]
        key_path: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor(args) => {
            let engine = DoctorEngine {
                skills_library_path: args.library,
                presets_yaml_path: args.presets,
                vault_path: None,
                caddyfile_path: args.caddyfile,
                pam_config_path: None,
                proxy_url: args.proxy_url,
                hub_url: args.hub_url,
            };

            let report = engine.run_diagnostics().await;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("========================================================");
                println!(" 🩺 Antigravity Server Doctor Diagnostic Report");
                println!(" Total Checks: {} | Passed: {} | Failed: {}",
                    report.total_checks, report.passed_checks, report.failed_checks);
                println!("========================================================");

                for check in &report.checks {
                    let status_str = if check.passed { "[PASS]" } else { "[FAIL]" };
                    println!("{} {:<32} :: {}", status_str, check.category, check.name);
                    println!("       Details: {}", check.details);
                    if let Some(ref rec) = check.recommendation {
                        println!("       Recommendation: {}", rec);
                    }
                }
                println!("========================================================");
                if report.all_passed {
                    println!("🎉 All server components healthy and operational.");
                } else {
                    println!("⚠️  Some checks failed. Review recommendations above.");
                }
            }

            if !report.all_passed {
                std::process::exit(1);
            }
        }
        Commands::Preset(cmd) => match cmd {
            PresetCommands::List { presets_file } => {
                preset_cmd::run_preset_list(presets_file.as_deref())?;
            }
            PresetCommands::Switch {
                preset,
                workspace,
                library,
                presets_file,
            } => {
                let ws = workspace.unwrap_or_else(|| PathBuf::from("."));
                preset_cmd::run_preset_switch(
                    &preset,
                    &ws,
                    library.as_deref(),
                    presets_file.as_deref(),
                )?;
            }
        },
        Commands::Vault(cmd) => match cmd {
            VaultCommands::Init {
                vault_path,
                key_path,
            } => {
                vault_cmd::run_vault_init(&vault_path, key_path.as_deref()).await?;
            }
            VaultCommands::Add {
                vault_path,
                key_path,
                email,
                refresh_token,
            } => {
                vault_cmd::run_vault_add(&vault_path, key_path.as_deref(), &email, &refresh_token)
                    .await?;
            }
            VaultCommands::List {
                vault_path,
                key_path,
            } => {
                vault_cmd::run_vault_list(&vault_path, key_path.as_deref()).await?;
            }
        },
    }

    Ok(())
}
