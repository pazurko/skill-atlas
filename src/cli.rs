use crate::interactive::present_skills;
use crate::repl::{
    cache_message, run_repl, truncation_message, write_skill_list, write_welcome, ReplSession,
    TerminalUi,
};
use crate::scanner::{scan_github_repo, ScannerOptions};
use clap::{Parser, Subcommand};
use colored::*;
use std::io::{self, IsTerminal, Write};

#[derive(Parser, Debug)]
#[command(
    name = "skill-atlas",
    about = "Skill Atlas - Discover AI agent skills across GitHub repositories",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Scan target GitHub repository or repositories for AI agent skills
    Scan {
        /// Target repository identifier(s) or URL(s) (e.g. owner/repo or https://github.com/owner/repo)
        #[arg(value_name = "GITHUBREPO")]
        githubrepo: Vec<String>,
    },

    /// Filter cached skills by keyword query across name, description, or path
    Filter {
        /// Search query to filter skills by
        query: Option<String>,
    },

    /// Discover similar skills based on heuristics and similarity percentages
    Similar {
        /// Target skill name or index to find similar counterparts for
        target: Option<String>,
    },

    /// Start localhost web interface for scanning and viewing skills
    #[command(alias = "serve")]
    Web {
        /// Port to listen on (default: 3000)
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
        /// Do not automatically open browser on startup
        #[arg(long, default_value_t = false)]
        no_open: bool,
    },

    /// List all cached skills from the local SQLite database
    #[command(alias = "ls")]
    List,

    /// List all starred/bookmarked skills from the local SQLite database
    #[command(alias = "star", alias = "stars", alias = "bookmarks")]
    Starred,
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
        Some(Commands::Web { port, no_open }) => {
            let web_options = crate::web::WebOptions {
                host: "127.0.0.1".to_string(),
                port,
                open_browser: !no_open,
                scanner_options: ScannerOptions::default(),
            };
            crate::web::start_web_server(web_options).await
        }
        Some(Commands::Scan { githubrepo }) => {
            let targets = extract_targets(&githubrepo);
            if targets.is_empty() {
                if is_tty() {
                    run_session(None, ScannerOptions::default(), None).await
                } else {
                    print_usage();
                    Ok(())
                }
            } else if is_tty() {
                run_session(Some(&targets), ScannerOptions::default(), None).await
            } else {
                scan_and_present_multiple(&targets).await
            }
        }
        Some(Commands::List) => {
            let conn = crate::storage::open_db(None)?;
            let skills = crate::storage::get_all_cached_skills(&conn)?;

            if skills.is_empty() {
                println!(
                    "{}",
                    "No skills found in SQLite database yet. Run 'skill-atlas scan <githubrepo>' to scan a repository."
                        .yellow()
                );
                return Ok(());
            }

            let title = format!("SQLite Database ({} cached skills)", skills.len());
            if is_tty() {
                present_skills(
                    &skills,
                    Some(&title),
                    false,
                    &mut crate::history::OpenHistory::new(),
                )?;
            } else {
                write_skill_list(&mut io::stdout(), &skills, &title)?;
            }
            Ok(())
        }
        Some(Commands::Starred) => {
            let conn = crate::storage::open_db(None)?;
            let skills = crate::storage::get_starred_skills(&conn)?;

            if skills.is_empty() {
                println!(
                    "{}",
                    "No starred skills found in SQLite database yet. Use '*' in the interactive menu or 'star <number|name>' to bookmark skills."
                        .yellow()
                );
                return Ok(());
            }

            let title = format!("⭐ Starred Skills ({} skills)", skills.len());
            if is_tty() {
                present_skills(
                    &skills,
                    Some(&title),
                    false,
                    &mut crate::history::OpenHistory::new(),
                )?;
            } else {
                write_skill_list(&mut io::stdout(), &skills, &title)?;
            }
            Ok(())
        }
        Some(Commands::Filter { query }) => {
            let conn = crate::storage::open_db(None)?;
            let skills = crate::storage::get_all_cached_skills(&conn)?;

            if skills.is_empty() {
                println!(
                    "{}",
                    "No skills found in SQLite database yet. Run 'skill-atlas scan <githubrepo>' to scan a repository."
                        .yellow()
                );
                return Ok(());
            }

            let displayed = match query.as_deref() {
                Some(q) if !q.trim().is_empty() => {
                    let filtered = crate::similarity::filter_skills(&skills, q.trim());
                    if filtered.is_empty() {
                        println!(
                            "{}",
                            format!(
                                "No skills matched filter '{}' (0 of {} skills).",
                                q.trim(),
                                skills.len()
                            )
                            .yellow()
                        );
                        return Ok(());
                    }
                    filtered
                }
                _ => skills.clone(),
            };

            let title = if query.is_some() {
                format!(
                    "SQLite Database (filtered: {} of {})",
                    displayed.len(),
                    skills.len()
                )
            } else {
                format!("SQLite Database ({} cached skills)", displayed.len())
            };

            if is_tty() {
                present_skills(
                    &displayed,
                    Some(&title),
                    false,
                    &mut crate::history::OpenHistory::new(),
                )?;
            } else {
                write_skill_list(&mut io::stdout(), &displayed, &title)?;
            }
            Ok(())
        }
        Some(Commands::Similar { target }) => {
            let conn = crate::storage::open_db(None)?;
            let skills = crate::storage::get_all_cached_skills(&conn)?;

            if skills.is_empty() {
                println!(
                    "{}",
                    "No skills found in SQLite database yet. Run 'skill-atlas scan <githubrepo>' to scan a repository."
                        .yellow()
                );
                return Ok(());
            }

            let threshold = crate::similarity::DEFAULT_SIMILARITY_THRESHOLD;
            match target.as_deref() {
                Some(t) if !t.trim().is_empty() => {
                    let t_clean = t.trim();
                    let target_skill = if let Ok(idx) = t_clean.parse::<usize>() {
                        if (1..=skills.len()).contains(&idx) {
                            Some(&skills[idx - 1])
                        } else {
                            None
                        }
                    } else {
                        skills
                            .iter()
                            .find(|s| s.name.eq_ignore_ascii_case(t_clean))
                            .or_else(|| {
                                skills.iter().find(|s| {
                                    s.name.to_lowercase().contains(&t_clean.to_lowercase())
                                })
                            })
                    };

                    let Some(skill) = target_skill else {
                        println!(
                            "{}",
                            format!("Skill matching '{}' not found in database.", t_clean).yellow()
                        );
                        return Ok(());
                    };

                    let matches = crate::similarity::find_similar_skills(skill, &skills, threshold);
                    if matches.is_empty() {
                        println!(
                            "{}",
                            format!(
                                "No skills similar to '{}' detected above {:.0}% threshold.",
                                skill.name, threshold
                            )
                            .yellow()
                        );
                    } else {
                        println!(
                            "\n{}",
                            format!(
                                "Skills similar to '{}' (threshold: >= {:.0}%):",
                                skill.name, threshold
                            )
                            .bold()
                        );
                        for m in &matches {
                            println!(
                                " [{}] › {} ({:.0}% similar)\n     {}\n     {}",
                                m.index,
                                m.skill.name.bold().cyan(),
                                m.similarity,
                                m.skill.description,
                                format!("📁 {}", m.skill.path).dimmed()
                            );
                        }
                        println!();
                    }
                }
                _ => {
                    let pairs = crate::similarity::find_all_similar_pairs(&skills, threshold);
                    if pairs.is_empty() {
                        println!(
                            "{}",
                            format!(
                                "No similar skills detected above {:.0}% threshold.",
                                threshold
                            )
                            .yellow()
                        );
                    } else {
                        println!(
                            "\n{}",
                            format!(
                                "Similar skills detected across database (threshold: >= {:.0}%):",
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
            }
            Ok(())
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
        "Skill Atlas - Core commands:\n  skill-atlas scan <githubrepo>...\n  skill-atlas list\n  skill-atlas filter <query>\n  skill-atlas similar [target]\n  skill-atlas web\n\nRun `skill-atlas --help` for full details."
            .yellow()
    );
}

pub fn extract_targets(inputs: &[String]) -> Vec<String> {
    inputs
        .iter()
        .flat_map(|item| {
            item.split([',', ';'])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .collect()
}

/// Runs the persistent `skill-atlas>` prompt session, optionally scanning repositories first.
async fn run_session(
    initial_repos: Option<&[String]>,
    options: ScannerOptions,
    filter: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = io::stdout();
    let mut ui = TerminalUi;
    let mut session = ReplSession::new(options);

    write_welcome(&mut out)?;
    if let Some(repos) = initial_repos {
        session
            .scan(&mut out, &mut ui, repos, None, false, filter)
            .await?;
    }
    out.flush()?;

    let mut input = io::stdin().lock();
    run_repl(&mut input, &mut out, &mut ui, &mut session).await?;
    Ok(())
}

async fn scan_and_present_multiple(targets: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if targets.len() == 1 {
        return scan_and_present(&targets[0]).await;
    }

    println!(
        "{}",
        format!(
            "\n🔍 Scanning {} repositories for agent skills...",
            targets.len()
        )
        .cyan()
    );

    let options = ScannerOptions::default();
    let mut all_skills = Vec::new();

    for target in targets {
        let parsed = crate::parser::parse_scan_target(target);
        match parsed {
            Ok(crate::parser::ScanTarget::Org { org }) => {
                println!(
                    "{}",
                    format!(" • Discovering organization {}...", org.bold()).cyan()
                );
                match crate::scanner::scan_github_org(&org, &options).await {
                    Ok(results) => {
                        for r in results {
                            if let Some(msg) = cache_message(&r) {
                                println!("   • {}/{} {}", r.owner, r.repo, msg.green());
                            }
                            if let Some(msg) = truncation_message(&r) {
                                println!("   • {}/{} {}", r.owner, r.repo, msg.yellow());
                            }
                            all_skills.extend(r.skills);
                        }
                    }
                    Err(err) => {
                        eprintln!("{}", format!("   ❌ Error for org {}: {}", org, err).red());
                    }
                }
            }
            _ => {
                println!("{}", format!(" • Scanning {}...", target.bold()).cyan());

                match scan_github_repo(target, &options).await {
                    Ok(result) => {
                        if let Some(msg) = cache_message(&result) {
                            println!("   {}", msg.green());
                        }
                        if let Some(msg) = truncation_message(&result) {
                            println!("   {}", msg.yellow());
                        }
                        all_skills.extend(result.skills);
                    }
                    Err(err) => {
                        eprintln!("{}", format!("   ❌ Error for {}: {}", target, err).red());
                    }
                }
            }
        }
    }

    if all_skills.is_empty() {
        println!(
            "{}",
            "\nNo agent skills found in the scanned targets.\n".yellow()
        );
        return Ok(());
    }

    let title = format!("Multiple Targets ({} skills)", all_skills.len());
    present_skills(
        &all_skills,
        Some(&title),
        false,
        &mut crate::history::OpenHistory::new(),
    )?;
    Ok(())
}

async fn scan_and_present(githubrepo: &str) -> Result<(), Box<dyn std::error::Error>> {
    let options = ScannerOptions::default();
    let parsed = crate::parser::parse_scan_target(githubrepo);

    match parsed {
        Ok(crate::parser::ScanTarget::Org { org }) => {
            println!(
                "{}",
                format!(
                    "\n🏢 Discovering repositories for organization {}...",
                    org.bold()
                )
                .cyan()
            );

            match crate::scanner::scan_github_org(&org, &options).await {
                Ok(results) => {
                    let mut all_skills = Vec::new();
                    for r in &results {
                        if let Some(msg) = cache_message(r) {
                            println!(" • {}/{} {}", r.owner, r.repo, msg.green());
                        }
                        if let Some(msg) = truncation_message(r) {
                            println!(" • {}/{} {}", r.owner, r.repo, msg.yellow());
                        }
                        all_skills.extend(r.skills.clone());
                    }

                    if all_skills.is_empty() {
                        println!(
                            "{}",
                            format!("\nNo agent skills found in organization {}.\n", org.bold())
                                .yellow()
                        );
                        return Ok(());
                    }

                    let title = format!("Organization: {} ({} skills)", org, all_skills.len());
                    present_skills(
                        &all_skills,
                        Some(&title),
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
        _ => {
            println!(
                "{}",
                format!(
                    "\n🔍 Scanning repository {} for agent skills...",
                    githubrepo.bold()
                )
                .cyan()
            );

            match scan_github_repo(githubrepo, &options).await {
                Ok(result) => {
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

                    let repo_name = format!("{}/{}", result.owner, result.repo);
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
    }
}
