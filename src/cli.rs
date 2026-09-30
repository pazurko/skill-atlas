use crate::interactive::present_skills;
use crate::repl::{cache_message, run_repl, write_welcome, ReplSession, TerminalUi};
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
    // Pick up GITHUB_TOKEN (and friends) from ./.env or the .env above the executable, without
    // overriding variables already set in the environment.
    crate::dotenv::load_default_dotenv();
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
            if !json && is_tty() {
                let options = scanner_options(token, branch, no_cache, db_path);
                run_session(Some(repo.trim()), options).await
            } else {
                scan_and_present(&repo, token, branch, json, no_cache, db_path).await
            }
        }
        Some(Commands::Scan {
            githubrepo: _,
            token,
            branch,
            json: _,
            no_cache,
            db_path,
        }) => {
            if is_tty() {
                run_session(None, scanner_options(token, branch, no_cache, db_path)).await
            } else {
                print_usage();
                Ok(())
            }
        }
        None => {
            if is_tty() {
                run_session(None, ScannerOptions::default()).await
            } else {
                print_usage();
                Ok(())
            }
        }
    }
}

fn is_tty() -> bool {
    io::stdout().is_terminal() && io::stdin().is_terminal()
}

fn print_usage() {
    eprintln!(
        "{}",
        "Usage: skill-atlas scan <githubrepo>\nRun `skill-atlas --help` for more information."
            .yellow()
    );
}

fn scanner_options(
    token: Option<String>,
    branch: Option<String>,
    no_cache: bool,
    db_path: Option<PathBuf>,
) -> ScannerOptions {
    ScannerOptions {
        token,
        branch,
        base_api_url: None,
        base_raw_url: None,
        no_cache,
        db_path,
    }
}

/// Runs the persistent `skill-atlas>` prompt session, optionally scanning a repository first.
async fn run_session(
    initial_repo: Option<&str>,
    options: ScannerOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = io::stdout();
    let mut ui = TerminalUi;
    let mut session = ReplSession::new(options);

    write_welcome(&mut out)?;
    if let Some(repo) = initial_repo {
        session.scan(&mut out, &mut ui, repo, None, false).await?;
    }
    out.flush()?;

    let mut input = io::stdin().lock();
    run_repl(&mut input, &mut out, &mut ui, &mut session).await?;
    Ok(())
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

            if let Some(msg) = cache_message(&result) {
                println!("{}", msg.green());
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
