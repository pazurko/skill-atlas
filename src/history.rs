use crate::scanner::Skill;
use colored::*;
use std::io::{self, Write};

/// One skill-open attempt (from the menu or the `open` command).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenRecord {
    /// Local time of the attempt, `HH:MM:SS`.
    pub time: String,
    /// `owner/repo` the skill belongs to.
    pub repo: String,
    pub name: String,
    pub path: String,
    pub url: String,
    /// Browser launch error, if the skill could not be opened.
    pub error: Option<String>,
}

impl OpenRecord {
    /// Single-line, human-readable form used in the menu and by the `history` command.
    pub fn line(&self) -> String {
        match &self.error {
            None => format!(
                "{} {} {} {} {}",
                self.time.dimmed(),
                "✓".green(),
                self.repo.dimmed(),
                self.name.bold(),
                self.url.blue()
            ),
            Some(err) => format!(
                "{} {} {} {} {} ({})",
                self.time.dimmed(),
                "✗".red(),
                self.repo.dimmed(),
                self.name.bold(),
                self.url.blue(),
                format!("failed: {}", err).red()
            ),
        }
    }
}

/// Audit trail of every skill opened during one session, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenHistory {
    records: Vec<OpenRecord>,
}

impl OpenHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an open attempt (successful when `error` is `None`) and returns it.
    pub fn record(&mut self, repo: &str, skill: &Skill, error: Option<String>) -> &OpenRecord {
        self.records.push(OpenRecord {
            time: chrono::Local::now().format("%H:%M:%S").to_string(),
            repo: repo.to_string(),
            name: skill.name.clone(),
            path: skill.path.clone(),
            url: skill.url.clone(),
            error,
        });
        self.records.last().expect("just pushed")
    }

    pub fn records(&self) -> &[OpenRecord] {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Whether the skill with this URL was opened successfully at least once.
    pub fn was_opened(&self, url: &str) -> bool {
        self.records
            .iter()
            .any(|r| r.url == url && r.error.is_none())
    }

    /// The last `n` records, oldest first.
    pub fn recent(&self, n: usize) -> &[OpenRecord] {
        &self.records[self.records.len().saturating_sub(n)..]
    }

    /// Prints the full history (the `history` prompt command).
    pub fn write_all<W: Write + ?Sized>(&self, out: &mut W) -> io::Result<()> {
        if self.records.is_empty() {
            return writeln!(out, "{}", "Nothing opened yet in this session.".yellow());
        }
        writeln!(
            out,
            "\n{}",
            format!("Opened in this session ({}):", self.records.len()).bold()
        )?;
        for (idx, record) in self.records.iter().enumerate() {
            writeln!(out, " {:>3}. {}", idx + 1, record.line())?;
        }
        writeln!(out)
    }
}
