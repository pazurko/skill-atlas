use crate::history::OpenHistory;
use crate::interactive::present_skills;
use crate::scanner::{scan_github_repo, ScanResult, ScannerOptions, Skill};
use colored::*;
use crossterm::{
    cursor, execute,
    terminal::{Clear, ClearType},
};
use std::io::{self, BufRead, Write};

pub const PROMPT: &str = "skill-atlas> ";

/// A single command entered at the `skill-atlas>` prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplCommand {
    /// Blank line, nothing to do.
    Empty,
    /// Scan one or more repositories (bare `owner/repo` or URLs are treated as `scan <repo>`).
    Scan {
        repos: Vec<String>,
        branch: Option<String>,
        refresh: bool,
        filter: Option<String>,
    },
    /// Re-scan the last scanned repository or repositories, bypassing the cache.
    Rescan,
    /// Show the skills of the last scan again.
    List,
    /// Filter skills by keyword query across name, description, or path.
    Filter(Option<String>),
    /// Discover similar skills based on heuristics and similarity percentages.
    Similar(Option<String>),
    /// Open a skill from the last scan by number or name.
    Open(String),
    /// Show every skill opened in this session (audit trail).
    History,
    /// Launch localhost web interface.
    Web {
        port: Option<u16>,
        no_open: bool,
    },
    Help,
    Clear,
    Exit,
    /// Malformed or unknown command, with the message to show to the user.
    Invalid(String),
}

pub const SCAN_USAGE: &str =
    "Usage: scan <githubrepo>... [--branch <BRANCH>] [--refresh] [--filter <QUERY>]";
pub const OPEN_USAGE: &str = "Usage: open <number|name>";
pub const WEB_USAGE: &str = "Usage: web [--port <PORT>] [--no-open]";
pub const FILTER_USAGE: &str = "Usage: filter [query|clear]";
pub const SIMILAR_USAGE: &str = "Usage: similar [<number|name>]";

/// Parses one line typed at the prompt into a command.
pub fn parse_command(line: &str) -> ReplCommand {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let Some(first) = parts.first() else {
        return ReplCommand::Empty;
    };
    let args = &parts[1..];

    match first.to_lowercase().as_str() {
        "exit" | "quit" | "q" => ReplCommand::Exit,
        "help" | "?" => ReplCommand::Help,
        "clear" | "cls" => ReplCommand::Clear,
        "list" | "ls" => {
            if args.is_empty() {
                ReplCommand::List
            } else {
                ReplCommand::Filter(Some(args.join(" ")))
            }
        }
        "filter" | "f" => {
            if args.is_empty() {
                ReplCommand::Filter(None)
            } else {
                ReplCommand::Filter(Some(args.join(" ")))
            }
        }
        "similar" => {
            if args.is_empty() {
                ReplCommand::Similar(None)
            } else {
                ReplCommand::Similar(Some(args.join(" ")))
            }
        }
        "rescan" | "refresh" => ReplCommand::Rescan,
        "history" | "opened" => ReplCommand::History,
        "web" | "serve" => parse_web_args(args),
        "open" => {
            if args.is_empty() {
                ReplCommand::Invalid(OPEN_USAGE.to_string())
            } else {
                ReplCommand::Open(args.join(" "))
            }
        }
        "scan" => parse_scan_args(args),
        _ if looks_like_repo(first) => parse_scan_args(&parts),
        other => ReplCommand::Invalid(format!(
            "Unknown command: '{}'. Type 'help' for available commands.",
            other
        )),
    }
}

fn parse_web_args(args: &[&str]) -> ReplCommand {
    let mut port: Option<u16> = None;
    let mut no_open = false;
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        match *arg {
            "-p" | "--port" => match iter.next() {
                Some(val) => match val.parse::<u16>() {
                    Ok(p) => port = Some(p),
                    Err(_) => {
                        return ReplCommand::Invalid(
                            "Invalid port number. Usage: web [--port <PORT>] [--no-open]"
                                .to_string(),
                        )
                    }
                },
                None => return ReplCommand::Invalid(WEB_USAGE.to_string()),
            },
            "--no-open" => no_open = true,
            "--open" => no_open = false,
            _ => return ReplCommand::Invalid(WEB_USAGE.to_string()),
        }
    }

    ReplCommand::Web { port, no_open }
}

fn looks_like_repo(token: &str) -> bool {
    token.contains('/') || token.starts_with("git@")
}

fn parse_scan_args(args: &[&str]) -> ReplCommand {
    let mut repos: Vec<String> = Vec::new();
    let mut branch: Option<String> = None;
    let mut refresh = false;
    let mut filter: Option<String> = None;
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        match *arg {
            "-b" | "--branch" => match iter.next() {
                Some(value) => branch = Some(value.to_string()),
                None => return ReplCommand::Invalid(SCAN_USAGE.to_string()),
            },
            "-f" | "--filter" => match iter.next() {
                Some(value) => filter = Some(value.to_string()),
                None => return ReplCommand::Invalid(SCAN_USAGE.to_string()),
            },
            "--refresh" | "--no-cache" => refresh = true,
            flag if flag.starts_with("--branch=") => {
                branch = Some(flag["--branch=".len()..].to_string());
            }
            flag if flag.starts_with("--filter=") => {
                filter = Some(flag["--filter=".len()..].to_string());
            }
            flag if flag.starts_with('-') => {
                return ReplCommand::Invalid(format!("Unknown option '{}'. {}", flag, SCAN_USAGE));
            }
            value => {
                for part in value.split([',', ';']) {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        repos.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    if repos.is_empty() {
        ReplCommand::Invalid(SCAN_USAGE.to_string())
    } else {
        ReplCommand::Scan {
            repos,
            branch,
            refresh,
            filter,
        }
    }
}

/// Terminal side effects of the prompt session, abstracted so the loop can be tested.
pub trait ReplUi {
    /// Presents the skills of a scan (interactive menu in a TTY). Skills opened from the menu
    /// must be recorded in `history`.
    fn show_skills(
        &mut self,
        out: &mut dyn Write,
        skills: &[Skill],
        repo_name: &str,
        history: &mut OpenHistory,
    ) -> io::Result<()>;
    /// Opens a URL in the default browser.
    fn open_url(&mut self, url: &str) -> io::Result<()>;
    /// Clears the terminal screen.
    fn clear_screen(&mut self, out: &mut dyn Write) -> io::Result<()>;
}

/// Real terminal UI: arrow-key menu, system browser, screen clearing.
pub struct TerminalUi;

impl ReplUi for TerminalUi {
    fn show_skills(
        &mut self,
        out: &mut dyn Write,
        skills: &[Skill],
        repo_name: &str,
        history: &mut OpenHistory,
    ) -> io::Result<()> {
        out.flush()?;
        present_skills(skills, Some(repo_name), false, history)?;
        Ok(())
    }

    fn open_url(&mut self, url: &str) -> io::Result<()> {
        open::that(url)
    }

    fn clear_screen(&mut self, out: &mut dyn Write) -> io::Result<()> {
        let mut stdout = io::stdout();
        out.flush()?;
        execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))
    }
}

/// State kept across commands in one prompt session.
#[derive(Debug, Clone, Default)]
pub struct ReplSession {
    /// Base scanner options (token, default branch, cache settings, db path).
    pub options: ScannerOptions,
    /// Repository input of the last successful scan.
    pub last_repo: Option<String>,
    /// Repository inputs of the last successful scan.
    pub last_repos: Vec<String>,
    /// Branch override used by the last successful scan.
    pub last_branch: Option<String>,
    /// `owner/repo` of the last successful scan.
    pub repo_name: Option<String>,
    /// Skills of the last successful scan.
    pub skills: Vec<Skill>,
    /// Active search filter if any.
    pub active_filter: Option<String>,
    /// Every skill opened in this session, across repositories.
    pub history: OpenHistory,
}

impl ReplSession {
    pub fn new(options: ScannerOptions) -> Self {
        Self {
            options,
            ..Default::default()
        }
    }

    /// Scans one or more repositories, reports errors without aborting, and presents results.
    pub async fn scan<W: Write, U: ReplUi>(
        &mut self,
        out: &mut W,
        ui: &mut U,
        repos: &[String],
        branch: Option<String>,
        refresh: bool,
        filter: Option<String>,
    ) -> io::Result<()> {
        if repos.is_empty() {
            return Ok(());
        }

        let mut options = self.options.clone();
        if branch.is_some() {
            options.branch = branch.clone();
        }
        options.no_cache = options.no_cache || refresh;

        if repos.len() == 1 {
            let repo = &repos[0];
            writeln!(
                out,
                "{}",
                format!(
                    "\n🔍 Scanning repository {} for agent skills...",
                    repo.bold()
                )
                .cyan()
            )?;
            out.flush()?;

            match scan_github_repo(repo, &options).await {
                Ok(result) => {
                    if let Some(msg) = cache_message(&result) {
                        writeln!(out, "{}", msg.green())?;
                    }
                    if let Some(msg) = truncation_message(&result) {
                        writeln!(out, "{}", msg.yellow())?;
                    }
                    let repo_name = format!("{}/{}", result.owner, result.repo);
                    self.last_repo = Some(repo.to_string());
                    self.last_repos = repos.to_vec();
                    self.last_branch = branch;
                    self.repo_name = Some(repo_name.clone());
                    self.skills = result.skills;
                    self.active_filter = filter;

                    let displayed = if let Some(f) = &self.active_filter {
                        crate::similarity::filter_skills(&self.skills, f)
                    } else {
                        self.skills.clone()
                    };

                    if self.skills.is_empty() {
                        writeln!(
                            out,
                            "{}",
                            format!("\nNo agent skills found in {}.\n", repo_name.bold()).yellow()
                        )?;
                    } else if displayed.is_empty() {
                        writeln!(
                            out,
                            "{}",
                            format!(
                                "\nNo agent skills matched filter '{}' (0 of {} skills in {}).\n",
                                self.active_filter.as_deref().unwrap_or_default(),
                                self.skills.len(),
                                repo_name.bold()
                            )
                            .yellow()
                        )?;
                    } else {
                        let title = if self.active_filter.is_some() {
                            format!(
                                "{} (filtered: {} of {})",
                                repo_name,
                                displayed.len(),
                                self.skills.len()
                            )
                        } else {
                            repo_name
                        };
                        ui.show_skills(out, &displayed, &title, &mut self.history)?;
                        writeln!(
                            out,
                            "{}",
                            "Type 'open <number|name>', 'filter [query]', 'similar', 'list', 'history', 'scan <githubrepo>', 'rescan' or 'help'."
                                .dimmed()
                        )?;
                    }
                }
                Err(err) => {
                    writeln!(out, "{}", format!("\n❌ Error: {}\n", err).red())?;
                }
            }
        } else {
            writeln!(
                out,
                "{}",
                format!(
                    "\n🔍 Scanning {} repositories for agent skills...",
                    repos.len()
                )
                .cyan()
            )?;
            out.flush()?;

            let mut all_skills = Vec::new();

            for repo in repos {
                writeln!(out, "{}", format!(" • Scanning {}...", repo.bold()).cyan())?;
                out.flush()?;

                match scan_github_repo(repo, &options).await {
                    Ok(result) => {
                        if let Some(msg) = cache_message(&result) {
                            writeln!(out, "   {}", msg.green())?;
                        }
                        if let Some(msg) = truncation_message(&result) {
                            writeln!(out, "   {}", msg.yellow())?;
                        }
                        all_skills.extend(result.skills);
                    }
                    Err(err) => {
                        writeln!(
                            out,
                            "{}",
                            format!("   ❌ Error for {}: {}", repo, err).red()
                        )?;
                    }
                }
            }

            self.last_repo = Some(repos.join(" "));
            self.last_repos = repos.to_vec();
            self.last_branch = branch;
            self.repo_name = Some(format!(
                "Multiple Repositories ({} repos, {} skills)",
                repos.len(),
                all_skills.len()
            ));
            self.skills = all_skills;
            self.active_filter = filter;

            let displayed = if let Some(f) = &self.active_filter {
                crate::similarity::filter_skills(&self.skills, f)
            } else {
                self.skills.clone()
            };

            if self.skills.is_empty() {
                writeln!(
                    out,
                    "{}",
                    "\nNo agent skills found in the scanned repositories.\n".yellow()
                )?;
            } else if displayed.is_empty() {
                writeln!(
                    out,
                    "{}",
                    format!(
                        "\nNo agent skills matched filter '{}' (0 of {} skills across {} repositories).\n",
                        self.active_filter.as_deref().unwrap_or_default(),
                        self.skills.len(),
                        repos.len()
                    )
                    .yellow()
                )?;
            } else {
                let title = if self.active_filter.is_some() {
                    format!(
                        "Multiple Repositories (filtered: {} of {})",
                        displayed.len(),
                        self.skills.len()
                    )
                } else {
                    format!("Multiple Repositories ({} skills)", self.skills.len())
                };
                ui.show_skills(out, &displayed, &title, &mut self.history)?;
                writeln!(
                    out,
                    "{}",
                    "Type 'open <number|name>', 'filter [query]', 'similar', 'list', 'history', 'scan <githubrepo>', 'rescan' or 'help'."
                        .dimmed()
                )?;
            }
        }
        Ok(())
    }

    fn find_skill(&self, query: &str) -> Result<&Skill, String> {
        if self.skills.is_empty() {
            return Err(
                "No skills listed yet in this session. Run 'scan <githubrepo>' first.".to_string(),
            );
        }
        let query = query.trim();
        if let Ok(idx) = query.parse::<usize>() {
            return if (1..=self.skills.len()).contains(&idx) {
                Ok(&self.skills[idx - 1])
            } else {
                Err(format!(
                    "Invalid index. Please choose between 1 and {}.",
                    self.skills.len()
                ))
            };
        }
        let lower = query.to_lowercase();
        self.skills
            .iter()
            .find(|s| s.name.to_lowercase() == lower)
            .or_else(|| {
                self.skills
                    .iter()
                    .find(|s| s.name.to_lowercase().contains(&lower))
            })
            .ok_or_else(|| format!("Skill matching '{}' not found in recent results.", query))
    }

    /// Executes one command. Returns `false` when the session should end.
    pub async fn execute<W: Write, U: ReplUi>(
        &mut self,
        out: &mut W,
        ui: &mut U,
        command: ReplCommand,
    ) -> io::Result<bool> {
        match command {
            ReplCommand::Empty => {}
            ReplCommand::Exit => {
                writeln!(out, "{}", "Goodbye!".yellow())?;
                return Ok(false);
            }
            ReplCommand::Help => write_help(out)?,
            ReplCommand::Clear => ui.clear_screen(out)?,
            ReplCommand::Invalid(msg) => writeln!(out, "{}", msg.yellow())?,
            ReplCommand::Scan {
                repos,
                branch,
                refresh,
                filter,
            } => self.scan(out, ui, &repos, branch, refresh, filter).await?,
            ReplCommand::Rescan => {
                if !self.last_repos.is_empty() {
                    let repos = self.last_repos.clone();
                    let branch = self.last_branch.clone();
                    self.scan(out, ui, &repos, branch, true, self.active_filter.clone())
                        .await?;
                } else if let Some(repo) = self.last_repo.clone() {
                    let branch = self.last_branch.clone();
                    self.scan(out, ui, &[repo], branch, true, self.active_filter.clone())
                        .await?;
                } else {
                    writeln!(
                        out,
                        "{}",
                        "Nothing to rescan yet. Run 'scan <githubrepo>' first.".yellow()
                    )?;
                }
            }
            ReplCommand::List => {
                if self.skills.is_empty() {
                    // Try to load from SQLite database
                    if let Ok(conn) = crate::storage::open_db(self.options.db_path.as_deref()) {
                        if let Ok(db_skills) = crate::storage::get_all_cached_skills(&conn) {
                            if !db_skills.is_empty() {
                                self.skills = db_skills;
                                self.repo_name =
                                    Some("SQLite Database (All Cached Skills)".to_string());
                            }
                        }
                    }
                }

                if self.skills.is_empty() {
                    writeln!(
                        out,
                        "{}",
                        "No skills listed yet in this session. Run 'scan <githubrepo>' first."
                            .yellow()
                    )?;
                } else {
                    let displayed = if let Some(f) = &self.active_filter {
                        crate::similarity::filter_skills(&self.skills, f)
                    } else {
                        self.skills.clone()
                    };
                    let title = if self.active_filter.is_some() {
                        format!(
                            "{} (filtered: {} of {})",
                            self.repo_name.as_deref().unwrap_or_default(),
                            displayed.len(),
                            self.skills.len()
                        )
                    } else {
                        self.repo_name.clone().unwrap_or_default()
                    };
                    ui.show_skills(out, &displayed, &title, &mut self.history)?;
                }
            }
            ReplCommand::Filter(query) => {
                if self.skills.is_empty() {
                    // Try to load from SQLite database
                    if let Ok(conn) = crate::storage::open_db(self.options.db_path.as_deref()) {
                        if let Ok(db_skills) = crate::storage::get_all_cached_skills(&conn) {
                            if !db_skills.is_empty() {
                                self.skills = db_skills;
                                self.repo_name =
                                    Some("SQLite Database (All Cached Skills)".to_string());
                            }
                        }
                    }
                }

                if self.skills.is_empty() {
                    writeln!(
                        out,
                        "{}",
                        "No skills listed yet in this session. Run 'scan <githubrepo>' first."
                            .yellow()
                    )?;
                } else {
                    match query {
                        None => {
                            self.active_filter = None;
                            writeln!(out, "{}", "Filter cleared. Showing all skills.".green())?;
                            let repo_name = self.repo_name.clone().unwrap_or_default();
                            ui.show_skills(out, &self.skills, &repo_name, &mut self.history)?;
                        }
                        Some(ref q)
                            if q.trim().eq_ignore_ascii_case("clear") || q.trim().is_empty() =>
                        {
                            self.active_filter = None;
                            writeln!(out, "{}", "Filter cleared. Showing all skills.".green())?;
                            let repo_name = self.repo_name.clone().unwrap_or_default();
                            ui.show_skills(out, &self.skills, &repo_name, &mut self.history)?;
                        }
                        Some(q) => {
                            let filtered = crate::similarity::filter_skills(&self.skills, &q);
                            if filtered.is_empty() {
                                writeln!(
                                    out,
                                    "{}",
                                    format!(
                                        "No skills matched filter '{}' (0 of {} skills).",
                                        q.trim(),
                                        self.skills.len()
                                    )
                                    .yellow()
                                )?;
                            } else {
                                self.active_filter = Some(q.trim().to_string());
                                let repo_name = format!(
                                    "{} (filtered: {} of {})",
                                    self.repo_name.as_deref().unwrap_or_default(),
                                    filtered.len(),
                                    self.skills.len()
                                );
                                ui.show_skills(out, &filtered, &repo_name, &mut self.history)?;
                            }
                        }
                    }
                }
            }
            ReplCommand::Similar(query) => {
                if self.skills.is_empty() {
                    // Try to load from SQLite database
                    if let Ok(conn) = crate::storage::open_db(self.options.db_path.as_deref()) {
                        if let Ok(db_skills) = crate::storage::get_all_cached_skills(&conn) {
                            if !db_skills.is_empty() {
                                self.skills = db_skills;
                                self.repo_name =
                                    Some("SQLite Database (All Cached Skills)".to_string());
                            }
                        }
                    }
                }

                if self.skills.is_empty() {
                    writeln!(
                        out,
                        "{}",
                        "No skills listed yet in this session. Run 'scan <githubrepo>' first."
                            .yellow()
                    )?;
                } else {
                    match query {
                        None => {
                            let pairs = crate::similarity::find_all_similar_pairs(
                                &self.skills,
                                crate::similarity::DEFAULT_SIMILARITY_THRESHOLD,
                            );
                            if pairs.is_empty() {
                                writeln!(
                                    out,
                                    "{}",
                                    format!(
                                        "No similar skills found in {} (threshold: >= {:.0}%).",
                                        self.repo_name.as_deref().unwrap_or_default(),
                                        crate::similarity::DEFAULT_SIMILARITY_THRESHOLD
                                    )
                                    .yellow()
                                )?;
                            } else {
                                writeln!(
                                    out,
                                    "\n{}",
                                    format!(
                                        "Similar skills in {} (threshold: >= {:.0}%):",
                                        self.repo_name.as_deref().unwrap_or_default(),
                                        crate::similarity::DEFAULT_SIMILARITY_THRESHOLD
                                    )
                                    .bold()
                                )?;
                                for p in &pairs {
                                    writeln!(
                                        out,
                                        " • [{}] {} <-> [{}] {} ({:.0}% similar)",
                                        p.index_a,
                                        p.skill_a.name.cyan(),
                                        p.index_b,
                                        p.skill_b.name.cyan(),
                                        p.similarity
                                    )?;
                                }
                                writeln!(out)?;
                            }
                        }
                        Some(target_query) => match self.find_skill(&target_query) {
                            Ok(skill) => {
                                let skill = skill.clone();
                                let matches = crate::similarity::find_similar_skills(
                                    &skill,
                                    &self.skills,
                                    crate::similarity::DEFAULT_SIMILARITY_THRESHOLD,
                                );
                                if matches.is_empty() {
                                    writeln!(
                                        out,
                                        "{}",
                                        format!(
                                            "No skills similar to '{}' found (threshold: >= {:.0}%).",
                                            skill.name,
                                            crate::similarity::DEFAULT_SIMILARITY_THRESHOLD
                                        )
                                        .yellow()
                                    )?;
                                } else {
                                    writeln!(
                                        out,
                                        "\n{}",
                                        format!(
                                            "Skills similar to '{}' (threshold: >= {:.0}%):",
                                            skill.name,
                                            crate::similarity::DEFAULT_SIMILARITY_THRESHOLD
                                        )
                                        .bold()
                                    )?;
                                    for m in &matches {
                                        writeln!(
                                            out,
                                            " [{}] › {} ({:.0}% similar)\n     {}\n     {}",
                                            m.index,
                                            m.skill.name.bold().cyan(),
                                            m.similarity,
                                            m.skill.description,
                                            format!("📁 {}", m.skill.path).dimmed()
                                        )?;
                                    }
                                    writeln!(out)?;
                                }
                            }
                            Err(msg) => writeln!(out, "{}", msg.yellow())?,
                        },
                    }
                }
            }
            ReplCommand::History => self.history.write_all(out)?,
            ReplCommand::Web { port, no_open } => {
                let port = port.unwrap_or(3000);
                let web_opts = crate::web::WebOptions {
                    host: "127.0.0.1".to_string(),
                    port,
                    open_browser: !no_open,
                    scanner_options: self.options.clone(),
                };
                if let Err(e) = crate::web::start_web_server(web_opts).await {
                    writeln!(out, "{} Failed to start web server: {}", "❌".red(), e)?;
                }
            }
            ReplCommand::Open(query) => match self.find_skill(&query) {
                Ok(skill) => {
                    let skill = skill.clone();
                    writeln!(
                        out,
                        "{} Opening {} in browser: {}",
                        "⚡".green(),
                        skill.name.bold(),
                        skill.url.blue()
                    )?;
                    let same_name = self
                        .skills
                        .iter()
                        .filter(|s| s.name.eq_ignore_ascii_case(&skill.name))
                        .count();
                    if same_name > 1 && query.trim().parse::<usize>().is_err() {
                        writeln!(
                            out,
                            "{}",
                            format!(
                                "Note: {} skills are named '{}'; opened {}. Use 'open <number>' to pick another.",
                                same_name, skill.name, skill.path
                            )
                            .yellow()
                        )?;
                    }
                    let repo_name = self.repo_name.clone().unwrap_or_default();
                    let error = ui.open_url(&skill.url).err();
                    if let Some(e) = &error {
                        writeln!(out, "{} Failed to open browser: {}", "❌".red(), e)?;
                    }
                    self.history
                        .record(&repo_name, &skill, error.map(|e| e.to_string()));
                }
                Err(msg) => writeln!(out, "{}", msg.yellow())?,
            },
        }
        Ok(true)
    }
}

/// Runs the `skill-atlas>` prompt loop until `exit`/`quit` or end of input.
pub async fn run_repl<R: BufRead, W: Write, U: ReplUi>(
    input: &mut R,
    out: &mut W,
    ui: &mut U,
    session: &mut ReplSession,
) -> io::Result<()> {
    loop {
        write!(out, "{}", PROMPT.bold())?;
        out.flush()?;

        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            writeln!(out, "{}", "\nGoodbye!".yellow())?;
            return Ok(());
        }

        if !session.execute(out, ui, parse_command(&line)).await? {
            return Ok(());
        }
    }
}

/// Writes the welcome banner shown when the prompt session starts.
pub fn write_welcome<W: Write>(out: &mut W) -> io::Result<()> {
    writeln!(out, "{}", "\n⚡ Welcome to Skill Atlas!".bold().cyan())?;
    writeln!(
        out,
        "Type 'scan <githubrepo>' (e.g. scan https://github.com/JetBrains/kotlin), 'help' for commands, or 'exit' to quit.\n"
    )
}

pub fn write_help<W: Write>(out: &mut W) -> io::Result<()> {
    writeln!(out, "\n{}", "Available commands:".bold())?;
    let rows = [
        (
            "scan <githubrepo>",
            "Scan a GitHub repository for AI agent skills",
        ),
        (
            "list",
            "Show all cached skills or skills of the current scan",
        ),
        (
            "filter [query|clear]",
            "Filter listed skills by name, description, or path",
        ),
        (
            "similar [<number|name>]",
            "Discover similar skills and duplicate definitions by percentage",
        ),
        (
            "web [--port <PORT>]",
            "Start localhost web interface to scan and view skills",
        ),
        (
            "open <number|name>",
            "Open a skill definition on GitHub in your default browser",
        ),
        ("history", "Show skills opened in this session (audit log)"),
        ("help", "Show this help message"),
        ("clear", "Clear the terminal screen"),
        ("exit, quit, q", "Exit Skill Atlas"),
    ];
    for (cmd, desc) in rows {
        writeln!(out, "  {:<30} {}", cmd.cyan(), desc)?;
    }
    writeln!(out)
}

/// Warning shown when GitHub truncated the repository tree of a fresh scan.
pub fn truncation_message(result: &ScanResult) -> Option<String> {
    result.truncated.then(|| {
        "⚠️  Warning: GitHub truncated the repository tree for this large repository; results may be incomplete."
            .to_string()
    })
}

/// Message shown when results were loaded from the SQLite cache.
pub fn cache_message(result: &ScanResult) -> Option<String> {
    if !result.from_cache {
        return None;
    }
    let commit_info = result
        .commit_sha
        .as_deref()
        .map(|sha| {
            let short_sha = if sha.len() >= 7 { &sha[..7] } else { sha };
            format!(" (commit {})", short_sha)
        })
        .unwrap_or_default();
    Some(format!(
        "📦 Repository unchanged since last scan{}. Loaded results from local SQLite database.\n",
        commit_info
    ))
}

/// Plain numbered list of skills (used by non-TTY presenters and tests).
pub fn write_skill_list<W: Write + ?Sized>(
    out: &mut W,
    skills: &[Skill],
    repo_name: &str,
) -> io::Result<()> {
    writeln!(
        out,
        "\n{} {}\n",
        repo_name.bold(),
        format!("({} skills)", skills.len()).green()
    )?;
    for (idx, skill) in skills.iter().enumerate() {
        let filename = crate::interactive::skill_badge_label(skill, skills);
        writeln!(
            out,
            " [{}] › {} [{}]\n     {}\n     {}",
            idx + 1,
            skill.name.bold().cyan(),
            format!("{} ↗", filename).dimmed(),
            skill.description,
            format!("🔗 {}", skill.url).blue()
        )?;
    }
    writeln!(out)
}
