use crate::interactive::present_skills;
use crate::repl::{
    cache_message, run_repl, truncation_message, write_welcome, ReplSession, TerminalUi,
};
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

#[derive(Subcommand, Debug, PartialEq)]
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

        /// Filter scanned skills by query across name, description, or path
        #[arg(short, long)]
        filter: Option<String>,

        /// Highlight and report similar skills found in the repository
        #[arg(long)]
        similar: bool,

        /// Minimum similarity percentage threshold for detection (default: 30.0)
        #[arg(long)]
        min_similarity: Option<f64>,
    },

    /// Start localhost web interface for scanning and viewing skills
    #[command(alias = "serve")]
    Web {
        /// Host address to bind to (default: 127.0.0.1)
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on (default: 3000)
        #[arg(short, long, default_value_t = 3000)]
        port: u16,

        /// Do not automatically open the web browser on startup
        #[arg(long)]
        no_open: bool,

        /// Automatically open the web interface in the default browser
        #[arg(long, conflicts_with = "no_open")]
        open: bool,

        /// GitHub personal access token (optional, to avoid rate limits)
        #[arg(short, long)]
        token: Option<String>,

        /// Git branch or ref to scan by default (default: HEAD)
        #[arg(short, long)]
        branch: Option<String>,

        /// Bypass local SQLite cache by default
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
        Some(Commands::Web {
            host,
            port,
            no_open,
            open: _,
            token,
            branch,
            no_cache,
            db_path,
        }) => {
            let options = ScannerOptions {
                token,
                branch,
                base_api_url: None,
                base_raw_url: None,
                no_cache,
                db_path,
            };
            let web_options = crate::web::WebOptions {
                host,
                port,
                open_browser: !no_open,
                scanner_options: options,
            };
            crate::web::start_web_server(web_options).await
        }
        Some(Commands::Scan {
            githubrepo: Some(repo),
            token,
            branch,
            json,
            no_cache,
            db_path,
            filter,
            similar,
            min_similarity,
        }) if !repo.trim().is_empty() => {
            if !json && is_tty() {
                let options = scanner_options(token, branch, no_cache, db_path);
                run_session(Some(repo.trim()), options, filter).await
            } else {
                let cli_opts = ScanCliOptions {
                    token,
                    branch,
                    json,
                    no_cache,
                    db_path,
                    filter,
                    similar,
                    min_similarity,
                };
                scan_and_present(&repo, cli_opts).await
            }
        }
        Some(Commands::Scan {
            githubrepo: _,
            token,
            branch,
            json: _,
            no_cache,
            db_path,
            filter,
            similar: _,
            min_similarity: _,
        }) => {
            if is_tty() {
                run_session(None, scanner_options(token, branch, no_cache, db_path), filter).await
            } else {
                print_usage();
                Ok(())
            }
        }
        None => {
            if is_tty() {
                run_session(None, ScannerOptions::default(), None).await
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
    filter: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = io::stdout();
    let mut ui = TerminalUi;
    let mut session = ReplSession::new(options);

    write_welcome(&mut out)?;
    if let Some(repo) = initial_repo {
        session.scan(&mut out, &mut ui, repo, None, false, filter).await?;
    }
    out.flush()?;

    let mut input = io::stdin().lock();
    run_repl(&mut input, &mut out, &mut ui, &mut session).await?;
    Ok(())
}

#[derive(Debug, Clone, Default)]
struct ScanCliOptions {
    token: Option<String>,
    branch: Option<String>,
    json: bool,
    no_cache: bool,
    db_path: Option<PathBuf>,
    filter: Option<String>,
    similar: bool,
    min_similarity: Option<f64>,
}

async fn scan_and_present(
    githubrepo: &str,
    opts: ScanCliOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    if !opts.json {
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
        token: opts.token,
        branch: opts.branch,
        base_api_url: None,
        base_raw_url: None,
        no_cache: opts.no_cache,
        db_path: opts.db_path,
    };

    match scan_github_repo(githubrepo, &options).await {
        Ok(mut result) => {
            if let Some(f) = &opts.filter {
                result.skills = crate::similarity::filter_skills(&result.skills, f);
            }

            if opts.json {
                let json_str = serde_json::to_string_pretty(&result)?;
                println!("{}", json_str);
                return Ok(());
            }

            if let Some(msg) = cache_message(&result) {
                println!("{}", msg.green());
            }
            if let Some(msg) = truncation_message(&result) {
                println!("{}", msg.yellow());
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

            let threshold = opts
                .min_similarity
                .unwrap_or(crate::similarity::DEFAULT_SIMILARITY_THRESHOLD);
            if opts.similar {
                let pairs = crate::similarity::find_all_similar_pairs(&result.skills, threshold);
                if !pairs.is_empty() {
                    println!(
                        "\n{}",
                        format!(
                            "Similar skills detected (threshold: >= {:.0}%):",
                            threshold
                        )
                        .bold()
                    );
                    for p in &pairs {
                        println!(
                            " • [{}] {} <-> [{}] {} ({:.0}% similar)",
                            p.index_a,
                            p.skill_a.name.cyan(),
                            p.index_b,
                            p.skill_b.name.cyan(),
                            p.similarity
                        );
                    }
                    println!();
                }
            }

            let repo_name = if let Some(f) = &opts.filter {
                format!("{}/{} (filtered by '{}')", result.owner, result.repo, f)
            } else {
                format!("{}/{}", result.owner, result.repo)
            };

            present_skills(
                &result.skills,
                Some(&repo_name),
                false,
                &mut crate::history::OpenHistory::new(),
            )?;
            Ok(())
        }
        Err(err) => {
            eprintln!("{}", format!("\n❌ Error: {}\n", err).red());
            std::process::exit(1);
        }
    }
}
