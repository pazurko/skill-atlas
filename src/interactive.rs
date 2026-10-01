use crate::history::OpenHistory;
use crate::scanner::Skill;
use colored::*;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use std::io::{self, IsTerminal, Write};
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum InteractiveResult {
    Opened(Skill),
    Exited,
}

/// Action resulting from a single key press inside the interactive skills menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// Move the selection cursor to the given index.
    Move(usize),
    /// Open the skill at the given index in the browser (the menu stays open).
    Open(usize),
    /// Show similarity information for the skill at the given index.
    Similar(usize),
    /// Leave the menu and return to the caller (e.g. the `skill-atlas>` prompt).
    Back,
    /// Key has no effect.
    Ignore,
}

/// Maps a key press to a menu action. Pure function so the navigation logic is testable.
pub fn handle_menu_key(
    code: KeyCode,
    modifiers: KeyModifiers,
    selected_idx: usize,
    len: usize,
) -> MenuAction {
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        return MenuAction::Back;
    }
    if len == 0 {
        return match code {
            KeyCode::Char('q') | KeyCode::Esc => MenuAction::Back,
            _ => MenuAction::Ignore,
        };
    }
    match code {
        KeyCode::Char('q') | KeyCode::Esc => MenuAction::Back,
        KeyCode::Up | KeyCode::Char('k') => {
            if selected_idx == 0 || selected_idx >= len {
                MenuAction::Move(len - 1)
            } else {
                MenuAction::Move(selected_idx - 1)
            }
        }
        KeyCode::Down | KeyCode::Char('j') => MenuAction::Move((selected_idx + 1) % len),
        KeyCode::Enter => MenuAction::Open(selected_idx.min(len - 1)),
        KeyCode::Char('s') | KeyCode::Char('S') => MenuAction::Similar(selected_idx.min(len - 1)),
        _ => MenuAction::Ignore,
    }
}

/// How many of the most recent opens are shown below the menu.
pub const MENU_HISTORY_LINES: usize = 5;

/// Mutable state of the skills menu between key presses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuState {
    pub selected_idx: usize,
    /// Result of the last open attempt; kept until the next open (moving does not clear it).
    pub status: Option<String>,
}

impl MenuState {
    /// Applies a menu action. Opening a skill calls `opener` with its URL and records the
    /// attempt in `history`. Returns `false` when the menu should close.
    pub fn apply(
        &mut self,
        action: MenuAction,
        skills: &[Skill],
        repo_name: &str,
        history: &mut OpenHistory,
        opener: &mut dyn FnMut(&str) -> io::Result<()>,
    ) -> bool {
        match action {
            MenuAction::Back => return false,
            MenuAction::Move(idx) => self.selected_idx = idx,
            MenuAction::Similar(idx) => {
                let Some(skill) = skills.get(idx) else {
                    return true;
                };
                let matches = crate::similarity::find_similar_skills(skill, skills, 20.0);
                self.status = Some(if let Some(top) = matches.first() {
                    format!(
                        "{} Most similar to {}: [{}] {} ({:.0}% similar)",
                        "⚡".cyan(),
                        skill.name.bold(),
                        top.index,
                        top.skill.name.bold(),
                        top.similarity
                    )
                } else {
                    format!(
                        "ℹ No similar skills found for {} (threshold: >= 20%)",
                        skill.name.bold()
                    )
                });
            }
            MenuAction::Open(idx) => {
                let Some(skill) = skills.get(idx) else {
                    return true;
                };
                self.status = Some(match opener(&skill.url) {
                    Ok(()) => {
                        history.record(repo_name, skill, None);
                        format!(
                            "{} Opened {} in browser: {}",
                            "⚡".green(),
                            skill.name.bold(),
                            skill.url.blue()
                        )
                    }
                    Err(e) => {
                        history.record(repo_name, skill, Some(e.to_string()));
                        format!("{} Failed to open browser: {}", "❌".red(), e)
                    }
                });
            }
            MenuAction::Ignore => {}
        }
        true
    }
}

/// Label of the file badge: the file name, or the full path when several skills share the
/// same name (so duplicates can be told apart).
pub fn skill_badge_label(skill: &Skill, skills: &[Skill]) -> String {
    let lower = skill.name.to_lowercase();
    let duplicated = skills
        .iter()
        .filter(|s| s.name.to_lowercase() == lower)
        .count()
        > 1;
    if duplicated {
        return skill.path.clone();
    }
    Path::new(&skill.path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("SKILL.md")
        .to_string()
}

/// Presents the discovered skills interactively or prints them in non-interactive mode.
/// Skills opened from the menu are recorded in `history`.
pub fn present_skills(
    skills: &[Skill],
    repo_name: Option<&str>,
    force_interactive: bool,
    history: &mut OpenHistory,
) -> io::Result<InteractiveResult> {
    if skills.is_empty() {
        println!("{}", "\nNo skills found in this repository.\n".yellow());
        return Ok(InteractiveResult::Exited);
    }

    let is_tty = io::stdout().is_terminal() && io::stdin().is_terminal();

    // Non-interactive fallback
    if !is_tty && !force_interactive {
        let repo_header = repo_name.unwrap_or("Discovered Agent Skills");
        println!(
            "\n{} {}\n",
            repo_header.bold(),
            format!("({} skills)", skills.len()).green()
        );
        for (idx, skill) in skills.iter().enumerate() {
            let filename = skill_badge_label(skill, skills);

            println!(
                " [{}] › {} [{}]\n     {}\n     {}",
                idx + 1,
                skill.name.bold().cyan(),
                format!("{} ↗", filename).dimmed(),
                skill.description.normal(),
                format!("🔗 {}", skill.url).blue()
            );
        }
        println!();
        return Ok(InteractiveResult::Exited);
    }

    // Interactive mode
    run_interactive_loop(skills, repo_name, history)
}

fn run_interactive_loop(
    skills: &[Skill],
    repo_name: Option<&str>,
    history: &mut OpenHistory,
) -> io::Result<InteractiveResult> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let mut state = MenuState::default();
    let repo = repo_name.unwrap_or_default().to_string();
    let mut opener = |url: &str| open::that(url);

    let res = (|| -> io::Result<InteractiveResult> {
        loop {
            render_menu(&mut stdout, skills, repo_name, &state, history)?;

            if let Event::Key(key_event) = event::read()? {
                if key_event.kind != event::KeyEventKind::Press {
                    continue;
                }
                let action = handle_menu_key(
                    key_event.code,
                    key_event.modifiers,
                    state.selected_idx,
                    skills.len(),
                );
                if !state.apply(action, skills, &repo, history, &mut opener) {
                    return Ok(InteractiveResult::Exited);
                }
            }
        }
    })();

    let _ = disable_raw_mode();
    let _ = execute!(stdout, cursor::Show);
    println!();

    res
}

/// Draws the whole menu frame: skill cards, key hints, the last status line and the most
/// recent opens of the session (which stay visible while navigating).
pub fn render_menu<W: Write>(
    out: &mut W,
    skills: &[Skill],
    repo_name: Option<&str>,
    state: &MenuState,
    history: &OpenHistory,
) -> io::Result<()> {
    let selected_idx = state.selected_idx;
    execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    let term_width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);
    let effective_width = if term_width > 4 { term_width } else { 80 };

    let header_title = repo_name.unwrap_or("Skill Atlas");
    let count_text = format!(" {} skills ", skills.len());

    writeln!(
        out,
        "{} {}\r\n",
        header_title.bold(),
        count_text.on_bright_black().green().bold()
    )?;

    for (idx, skill) in skills.iter().enumerate() {
        let is_selected = idx == selected_idx;
        let num_str = format!("{:>2}", idx + 1);

        let opened_mark = if history.was_opened(&skill.url) {
            " ✓ opened"
        } else {
            ""
        };
        let badge_text = format!(" {} ↗ ", skill_badge_label(skill, skills));

        // Header line for skill card
        let num_badge = if is_selected {
            format!("[{}]", num_str).bold().white().on_blue()
        } else {
            format!("[{}]", num_str).dimmed()
        };

        let chevron = if is_selected {
            "›".bright_cyan().bold()
        } else {
            "›".dimmed()
        };

        let skill_name_colored = if is_selected {
            skill.name.bold().bright_cyan()
        } else {
            skill.name.bright_cyan()
        };

        let file_badge_colored = if is_selected {
            badge_text.bold().white().on_bright_black()
        } else {
            badge_text.dimmed().on_bright_black()
        };

        // Calculate available spacing for right-aligned badge
        let left_plain_len = 4 + 1 + 2 + skill.name.chars().count() + opened_mark.chars().count(); // "[xx] › skill-name ✓ opened"
        let badge_len = badge_text.chars().count();
        let pad_len = if effective_width > left_plain_len + badge_len + 2 {
            effective_width - (left_plain_len + badge_len)
        } else {
            2
        };
        let spacing = " ".repeat(pad_len);

        writeln!(
            out,
            " {} {} {}{}{}{}\r",
            num_badge,
            chevron,
            skill_name_colored,
            opened_mark.green(),
            spacing,
            file_badge_colored
        )?;

        // Description line
        let desc_max_len = if effective_width > 12 {
            effective_width - 10
        } else {
            60
        };
        let formatted_desc = if skill.description.chars().count() > desc_max_len {
            let truncated: String = skill.description.chars().take(desc_max_len - 3).collect();
            format!("{}...", truncated)
        } else {
            skill.description.clone()
        };

        let desc_colored = if is_selected {
            formatted_desc.normal()
        } else {
            formatted_desc.dimmed()
        };

        writeln!(out, "      {}\r\n", desc_colored)?;
    }

    writeln!(
        out,
        "{}\r",
        "↑/↓ navigate • Enter open in GitHub • s similar • q back to prompt".dimmed()
    )?;

    if let Some(status) = &state.status {
        writeln!(out, "\r\n{}\r", status)?;
    }

    if !history.is_empty() {
        writeln!(
            out,
            "\r\n{}\r",
            format!(
                "Opened in this session ({}, 'history' at the prompt shows all):",
                history.len()
            )
            .bold()
        )?;
        for record in history.recent(MENU_HISTORY_LINES) {
            writeln!(out, "  {}\r", record.line())?;
        }
    }

    out.flush()?;
    Ok(())
}
