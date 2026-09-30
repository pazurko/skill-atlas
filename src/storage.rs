use crate::scanner::Skill;
use chrono::Utc;
use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedRepo {
    pub id: i64,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub commit_sha: Option<String>,
    pub etag: Option<String>,
    pub last_scanned_at: String,
}

/// Returns the default database path `~/.skill-atlas/skills.db`.
pub fn default_db_path() -> PathBuf {
    if let Some(home_dir) = dirs::home_dir() {
        home_dir.join(".skill-atlas").join("skills.db")
    } else {
        PathBuf::from("skills.db")
    }
}

/// Opens an existing SQLite database or creates a new one at the given path.
pub fn open_db(path: Option<&Path>) -> Result<Connection> {
    let conn = match path {
        Some(p) => {
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = fs::create_dir_all(parent);
                }
            }
            Connection::open(p)?
        }
        None => {
            let p = default_db_path();
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }
            Connection::open(&p)?
        }
    };

    init_db(&conn)?;
    Ok(conn)
}

/// Initializes database tables and indices if they do not exist.
pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS repositories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            owner TEXT NOT NULL,
            repo TEXT NOT NULL,
            branch TEXT NOT NULL,
            commit_sha TEXT,
            etag TEXT,
            last_scanned_at TEXT NOT NULL,
            UNIQUE(owner, repo, branch)
        );

        CREATE TABLE IF NOT EXISTS skills (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            path TEXT NOT NULL,
            url TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_repo_lookup ON repositories(owner, repo, branch);
        CREATE INDEX IF NOT EXISTS idx_skill_repo ON skills(repository_id);
        "#,
    )?;
    Ok(())
}

/// Retrieves cached repository information and its associated skills.
pub fn get_cached_repository(
    conn: &Connection,
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<Option<(CachedRepo, Vec<Skill>)>> {
    let mut stmt = conn.prepare(
        "SELECT id, owner, repo, branch, commit_sha, etag, last_scanned_at 
         FROM repositories 
         WHERE LOWER(owner) = LOWER(?1) AND LOWER(repo) = LOWER(?2) AND LOWER(branch) = LOWER(?3)",
    )?;

    let repo_opt = stmt
        .query_row(params![owner, repo, branch], |row| {
            Ok(CachedRepo {
                id: row.get(0)?,
                owner: row.get(1)?,
                repo: row.get(2)?,
                branch: row.get(3)?,
                commit_sha: row.get(4)?,
                etag: row.get(5)?,
                last_scanned_at: row.get(6)?,
            })
        })
        .ok();

    if let Some(cached_repo) = repo_opt {
        let mut skill_stmt = conn.prepare(
            "SELECT name, description, path, url 
             FROM skills 
             WHERE repository_id = ?1 
             ORDER BY LOWER(name) ASC",
        )?;

        let skill_iter = skill_stmt.query_map(params![cached_repo.id], |row| {
            Ok(Skill {
                name: row.get(0)?,
                description: row.get(1)?,
                path: row.get(2)?,
                url: row.get(3)?,
            })
        })?;

        let mut skills = Vec::new();
        for skill in skill_iter {
            skills.push(skill?);
        }

        Ok(Some((cached_repo, skills)))
    } else {
        Ok(None)
    }
}

/// Saves or updates a repository and its discovered skills in SQLite.
pub fn save_cached_repository(
    conn: &mut Connection,
    owner: &str,
    repo: &str,
    branch: &str,
    commit_sha: Option<&str>,
    etag: Option<&str>,
    skills: &[Skill],
) -> Result<()> {
    let tx = conn.transaction()?;
    let now = Utc::now().to_rfc3339();

    tx.execute(
        r#"
        INSERT INTO repositories (owner, repo, branch, commit_sha, etag, last_scanned_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(owner, repo, branch) DO UPDATE SET
            commit_sha = excluded.commit_sha,
            etag = excluded.etag,
            last_scanned_at = excluded.last_scanned_at
        "#,
        params![owner, repo, branch, commit_sha, etag, now],
    )?;

    let repo_id: i64 = tx.query_row(
        "SELECT id FROM repositories WHERE LOWER(owner) = LOWER(?1) AND LOWER(repo) = LOWER(?2) AND LOWER(branch) = LOWER(?3)",
        params![owner, repo, branch],
        |row| row.get(0),
    )?;

    // Clear old skills for this repository
    tx.execute(
        "DELETE FROM skills WHERE repository_id = ?1",
        params![repo_id],
    )?;

    // Insert new skills
    {
        let mut insert_skill = tx.prepare(
            "INSERT INTO skills (repository_id, name, description, path, url) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;

        for skill in skills {
            insert_skill.execute(params![
                repo_id,
                skill.name,
                skill.description,
                skill.path,
                skill.url
            ])?;
        }
    }

    tx.commit()?;
    Ok(())
}

/// Lists all cached repositories in the database.
pub fn list_all_cached_repositories(conn: &Connection) -> Result<Vec<CachedRepo>> {
    let mut stmt = conn.prepare(
        "SELECT id, owner, repo, branch, commit_sha, etag, last_scanned_at 
         FROM repositories 
         ORDER BY last_scanned_at DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(CachedRepo {
            id: row.get(0)?,
            owner: row.get(1)?,
            repo: row.get(2)?,
            branch: row.get(3)?,
            commit_sha: row.get(4)?,
            etag: row.get(5)?,
            last_scanned_at: row.get(6)?,
        })
    })?;

    let mut repos = Vec::new();
    for row in rows {
        repos.push(row?);
    }
    Ok(repos)
}

/// Clears all cached repository and skill data.
pub fn clear_cache(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM skills", [])?;
    conn.execute("DELETE FROM repositories", [])?;
    Ok(())
}
