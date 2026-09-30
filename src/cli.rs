use crate::interactive::present_skills;
use crate::scanner::{scan_github_repo, ScannerOptions};
use clap::{Parser, Subcommand};
use colored::*;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "skill-atlas",
    about = "Interactive CLI to scan GitHub repositories for AI agent skills",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Commands {
    /// Scan target GitHub repository for AI agent skills
    Scan {
        /// Target repository identifier or URL (e.g. owner/repo or https://github.com/owner/repo)
        githubrepo: Option<String>,

        /// GitHub personal access token (optional, to avoid rate limits)
        #[arg(short, long)]
        token: Option<String>,

        /// Git branch or ref to scan (default: HEAD)
        #[arg(short, long)]
        branch: Option<String>,

        /// Output results as JSON instead of interactive menu
        #[arg(long)]
        json: bool,

        /// Bypass local SQLite cache and re-scan the repository
        #[arg(long, alias = "refresh")]
        no_cache: bool,

        /// Custom path to SQLite database for caching
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
}

pub async fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    execute_cli(cli).await
}

pub async fn execute_cli(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Some(Commands::Scan {
            githubrepo: Some(repo),
            token,
            branch,
            json,
            no_cache,
            db_path,
        }) if !repo.trim().is_empty() => {
            scan_and_present(&repo, token, branch, json, no_cache, db_path).await
        }
        Some(Commands::Scan {
            githubrepo: _,
            token,
            branch,
            json,
            no_cache,
            db_path,
        }) => {
            let is_tty = io::stdout().is_terminal() && io::stdin().is_terminal();
            if is_tty {
                println!("{}", "\n⚡ Welcome to Skill Atlas!\n".bold().cyan());
                print!(
                    "{}",
                    "Enter GitHub repository to scan (e.g. https://github.com/JetBrains/kotlin): "
                        .bold()
                );
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let mut repo = input.trim();
                if repo.starts_with("scan ") {
                    repo = repo[5..].trim();
                }

                if repo.is_empty() {
                    println!("{}", "No repository entered. Exiting.\n".yellow());
                    return Ok(());
                }

                scan_and_present(repo, token, branch, json, no_cache, db_path).await
            } else {
                eprintln!(
                    "{}",
                    "Usage: skill-atlas scan <githubrepo>\nRun `skill-atlas --help` for more information.".yellow()
                );
                Ok(())
            }
        }
        None => {
            let is_tty = io::stdout().is_terminal() && io::stdin().is_terminal();
            if is_tty {
                println!("{}", "\n⚡ Welcome to Skill Atlas!\n".bold().cyan());
                print!(
                    "{}",
                    "Enter command or GitHub repository to scan (e.g. scan https://github.com/JetBrains/kotlin): "
                        .bold()
                );
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let mut repo = input.trim();
                if repo.starts_with("scan ") {
                    repo = repo[5..].trim();
                }

                if repo.is_empty() {
                    println!("{}", "No repository entered. Exiting.\n".yellow());
                    return Ok(());
                }

                scan_and_present(repo, None, None, false, false, None).await
            } else {
                eprintln!(
                    "{}",
                    "Usage: skill-atlas scan <githubrepo>\nRun `skill-atlas --help` for more information.".yellow()
                );
                Ok(())
            }
        }
    }
}

async fn scan_and_present(
    githubrepo: &str,
    token: Option<String>,
    branch: Option<String>,
    json: bool,
    no_cache: bool,
    db_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !json {
        println!(
            "{}",
            format!(
                "\n🔍 Scanning repository {} for agent skills...",
                githubrepo.bold()
            )
            .cyan()
        );
    }

    let options = ScannerOptions {
        token,
        branch,
        base_api_url: None,
        base_raw_url: None,
        no_cache,
        db_path,
    };

    match scan_github_repo(githubrepo, &options).await {
        Ok(result) => {
            if json {
                let json_str = serde_json::to_string_pretty(&result)?;
                println!("{}", json_str);
                return Ok(());
            }

            if result.from_cache {
                let commit_info = result
                    .commit_sha
                    .as_deref()
                    .map(|sha| {
                        let short_sha = if sha.len() >= 7 { &sha[..7] } else { sha };
                        format!(" (commit {})", short_sha)
                    })
                    .unwrap_or_default();
                println!(
                    "{}",
                    format!(
                        "📦 Repository unchanged since last scan{}. Loaded results from local SQLite database.\n",
                        commit_info
                    )
                    .green()
                );
            }

            if result.skills.is_empty() {
                println!(
                    "{}",
                    format!(
                        "\nNo agent skills found in {}.\n",
                        format!("{}/{}", result.owner, result.repo).bold()
                    )
                    .yellow()
                );
                return Ok(());
            }

            let repo_name = format!("{}/{}", result.owner, result.repo);
            present_skills(&result.skills, Some(&repo_name), false)?;
            Ok(())
        }
        Err(err) => {
            eprintln!("{}", format!("\n❌ Error: {}\n", err).red());
            std::process::exit(1);
        }
    }
}
