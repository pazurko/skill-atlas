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

/// Presents the discovered skills interactively or prints them in non-interactive mode.
pub fn present_skills(
    skills: &[Skill],
    repo_name: Option<&str>,
    force_interactive: bool,
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
            let filename = Path::new(&skill.path)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("SKILL.md");

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
    run_interactive_loop(skills, repo_name)
}

fn run_interactive_loop(
    skills: &[Skill],
    repo_name: Option<&str>,
) -> io::Result<InteractiveResult> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let mut selected_idx = 0;

    let res = (|| -> io::Result<InteractiveResult> {
        loop {
            render_menu(&mut stdout, skills, repo_name, selected_idx)?;

            if let Event::Key(key_event) = event::read()? {
                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && key_event.code == KeyCode::Char('c')
                {
                    return Ok(InteractiveResult::Exited);
                }

                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(InteractiveResult::Exited);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if selected_idx == 0 {
                            selected_idx = skills.len() - 1;
                        } else {
                            selected_idx -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        selected_idx = (selected_idx + 1) % skills.len();
                    }
                    KeyCode::Enter => {
                        let selected_skill = skills[selected_idx].clone();
                        return Ok(InteractiveResult::Opened(selected_skill));
                    }
                    _ => {}
                }
            }
        }
    })();

    let _ = disable_raw_mode();
    let _ = execute!(stdout, cursor::Show);

    match res {
        Ok(InteractiveResult::Opened(skill)) => {
            println!(
                "\n{} Opening {} in browser: {}",
                "⚡".green(),
                skill.name.bold(),
                skill.url.blue()
            );
            if let Err(e) = open::that(&skill.url) {
                eprintln!("{} Failed to open browser: {}", "❌".red(), e);
            }
            Ok(InteractiveResult::Opened(skill))
        }
        Ok(InteractiveResult::Exited) => {
            println!("{}", "\nExiting Skill Atlas.".yellow());
            Ok(InteractiveResult::Exited)
        }
        Err(e) => Err(e),
    }
}

fn render_menu<W: Write>(
    out: &mut W,
    skills: &[Skill],
    repo_name: Option<&str>,
    selected_idx: usize,
) -> io::Result<()> {
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

        let filename = Path::new(&skill.path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("SKILL.md");
        let badge_text = format!(" {} ↗ ", filename);

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
        let left_plain_len = 4 + 1 + 2 + skill.name.len(); // "[xx] › skill-name"
        let badge_len = badge_text.len();
        let pad_len = if effective_width > left_plain_len + badge_len + 2 {
            effective_width - (left_plain_len + badge_len)
        } else {
            2
        };
        let spacing = " ".repeat(pad_len);

        writeln!(
            out,
            " {} {} {}{}{}\r",
            num_badge, chevron, skill_name_colored, spacing, file_badge_colored
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
        "↑/↓ navigate • Enter open in GitHub • q quit".dimmed()
    )?;

    out.flush()?;
    Ok(())
}
