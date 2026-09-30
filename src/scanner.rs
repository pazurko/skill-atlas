use crate::metadata::parse_skill_metadata;
use crate::parser::{parse_repo_identifier, ParseError};
use crate::storage::{get_cached_repository, open_db, save_cached_repository};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

/// Maximum number of skill files downloaded at the same time.
pub const MAX_CONCURRENT_DOWNLOADS: usize = 16;

/// Vendored, generated or VCS folders that never contain real skills.
const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    "site-packages",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub path: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanResult {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub skills: Vec<Skill>,
    #[serde(default)]
    pub from_cache: bool,
    #[serde(default)]
    pub commit_sha: Option<String>,
    /// True when GitHub truncated the repository tree, so some skills may be missing.
    #[serde(default)]
    pub truncated: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ScannerOptions {
    pub token: Option<String>,
    pub branch: Option<String>,
    pub base_api_url: Option<String>,
    pub base_raw_url: Option<String>,
    pub no_cache: bool,
    pub db_path: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error("Repository \"{0}/{1}\" not found or is private.")]
    NotFound(String, String),

    #[error("Access forbidden for repository \"{0}/{1}\".{2}")]
    Forbidden(String, String, String),

    #[error("GitHub API request failed with status {0}: {1}")]
    ApiFailure(StatusCode, String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Storage error: {0}")]
    Storage(#[from] rusqlite::Error),
}

#[derive(Deserialize)]
struct GitTreeResponse {
    #[serde(default)]
    sha: Option<String>,
    #[serde(default)]
    tree: Vec<GitTreeItem>,
    #[serde(default)]
    truncated: bool,
}

#[derive(Deserialize)]
struct GitTreeItem {
    path: String,
    #[serde(rename = "type")]
    item_type: String,
    #[serde(default)]
    sha: String,
}

#[derive(Deserialize)]
struct CommitResponse {
    sha: String,
}

#[derive(Deserialize)]
struct RepoResponse {
    default_branch: String,
}

/// Builds the GitHub web URL of a file on the given branch or ref.
fn github_blob_url(owner: &str, repo: &str, web_ref: &str, path: &str) -> String {
    format!(
        "https://github.com/{}/{}/blob/{}/{}",
        owner, repo, web_ref, path
    )
}

/// Resolves the ref used in GitHub web links. An explicit branch is used as-is; for `HEAD`
/// the repository's default branch is looked up (e.g. `master`), falling back to `HEAD`,
/// which GitHub also resolves to the default branch.
async fn resolve_web_ref(
    client: &reqwest::Client,
    base_api: &str,
    owner: &str,
    repo: &str,
    branch: &str,
) -> String {
    if branch != "HEAD" {
        return branch.to_string();
    }
    let repo_url = format!("{}/repos/{}/{}", base_api, owner, repo);
    if let Ok(res) = client.get(&repo_url).send().await {
        if res.status().is_success() {
            if let Ok(data) = res.json::<RepoResponse>().await {
                if !data.default_branch.trim().is_empty() {
                    return data.default_branch;
                }
            }
        }
    }
    "HEAD".to_string()
}

/// Determines if a file path qualifies as an agent skill definition: a file named
/// `SKILL.md`, `skill.json`, `skill.yaml` or `skill.yml` (any letter case) that is not inside
/// a vendored/generated folder. Other files in skill folders (e.g. `references/*.md`) are
/// supporting documents, not skills.
pub fn is_skill_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_lowercase();
    let mut parts: Vec<&str> = normalized.split('/').collect();
    let filename = parts.pop().unwrap_or("");

    matches!(
        filename,
        "skill.md" | "skill.json" | "skill.yaml" | "skill.yml"
    ) && !parts.iter().any(|dir| IGNORED_DIRS.contains(dir))
}

/// Downloads a text file, returning `None` on any failure or non-success status.
async fn fetch_text(request: reqwest::RequestBuilder) -> Option<String> {
    let res = request.send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }
    res.text().await.ok()
}

/// Downloads a skill file: first from the raw content host (fast, no API rate limit),
/// falling back to the GitHub Blobs API.
async fn fetch_skill_content(
    client: &reqwest::Client,
    raw_url: &str,
    blob_url: Option<&str>,
) -> String {
    if let Some(content) = fetch_text(client.get(raw_url)).await {
        return content;
    }
    if let Some(blob_url) = blob_url {
        let request = client
            .get(blob_url)
            .header(ACCEPT, "application/vnd.github.v3.raw");
        if let Some(content) = fetch_text(request).await {
            return content;
        }
    }
    String::new()
}

/// Scans a GitHub repository for AI agent skills, using local SQLite caching where possible.
pub async fn scan_github_repo(
    repo_input: &str,
    options: &ScannerOptions,
) -> Result<ScanResult, ScannerError> {
    let repo_id = parse_repo_identifier(repo_input)?;
    let owner = repo_id.owner;
    let repo = repo_id.repo;

    let branch = options.branch.as_deref().unwrap_or("HEAD");
    let token = options
        .token
        .clone()
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .or_else(|| std::env::var("GH_TOKEN").ok());

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("skill-atlas-cli"));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github.v3+json"),
    );

    if let Some(tok) = &token {
        if let Ok(val) = HeaderValue::from_str(&format!("token {}", tok)) {
            headers.insert(AUTHORIZATION, val);
        }
    }

    let client = reqwest::Client::builder()
        .default_headers(headers.clone())
        .build()?;

    let base_api = options
        .base_api_url
        .as_deref()
        .unwrap_or("https://api.github.com");

    let mut db_conn = open_db(options.db_path.as_deref()).ok();

    // 1. Check SQLite Cache
    let cached_entry = if !options.no_cache {
        if let Some(ref conn) = db_conn {
            get_cached_repository(conn, &owner, &repo, branch).unwrap_or(None)
        } else {
            None
        }
    } else {
        None
    };

    // 2. Fetch latest commit SHA to check for updates
    let commit_url = format!("{}/repos/{}/{}/commits/{}", base_api, owner, repo, branch);
    let mut latest_commit_sha: Option<String> = None;

    if let Ok(commit_res) = client.get(&commit_url).send().await {
        if commit_res.status().is_success() {
            if let Ok(commit_data) = commit_res.json::<CommitResponse>().await {
                latest_commit_sha = Some(commit_data.sha);
            }
        }
    }

    // 3. If cache is valid and repository has not changed, reuse cached results
    if let Some((cached_repo, cached_skills)) = cached_entry {
        let is_unchanged = match (&cached_repo.commit_sha, &latest_commit_sha) {
            (Some(cached_sha), Some(current_sha)) => cached_sha == current_sha,
            _ => false,
        };

        if is_unchanged {
            // Rebuild links so results cached by older versions (which assumed `main`) are corrected.
            let web_ref = resolve_web_ref(&client, base_api, &owner, &repo, branch).await;
            let skills = cached_skills
                .into_iter()
                .map(|mut skill| {
                    skill.url = github_blob_url(&owner, &repo, &web_ref, &skill.path);
                    skill
                })
                .collect();
            return Ok(ScanResult {
                owner,
                repo,
                branch: branch.to_string(),
                skills,
                from_cache: true,
                commit_sha: cached_repo.commit_sha,
                truncated: false,
            });
        }
    }

    // 4. Perform fresh repository scan via GitHub Tree API
    let api_url = format!(
        "{}/repos/{}/{}/git/trees/{}?recursive=1",
        base_api, owner, repo, branch
    );

    let res = client.get(&api_url).send().await?;
    let status = res.status();

    if !status.is_success() {
        if status == StatusCode::NOT_FOUND {
            return Err(ScannerError::NotFound(owner, repo));
        }
        if status == StatusCode::FORBIDDEN {
            let is_rate_limited = res
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .map(|v| v == "0")
                .unwrap_or(false);
            let msg = if is_rate_limited {
                " GitHub API rate limit exceeded. Consider providing a token via GITHUB_TOKEN environment variable or --token option."
            } else {
                ""
            };
            return Err(ScannerError::Forbidden(owner, repo, msg.to_string()));
        }
        return Err(ScannerError::ApiFailure(
            status,
            res.text().await.unwrap_or_default(),
        ));
    }

    let tree_data: GitTreeResponse = res.json().await?;
    let tree_sha = tree_data.sha.clone();
    let truncated = tree_data.truncated;
    let current_sha = latest_commit_sha.or(tree_sha);

    let skill_entries: Vec<GitTreeItem> = tree_data
        .tree
        .into_iter()
        .filter(|item| item.item_type == "blob" && is_skill_path(&item.path))
        .collect();

    let base_raw = options
        .base_raw_url
        .as_deref()
        .unwrap_or("https://raw.githubusercontent.com");

    let web_ref = resolve_web_ref(&client, base_api, &owner, &repo, branch).await;

    let raw_ref = current_sha.clone().unwrap_or_else(|| web_ref.clone());

    // Download skill files concurrently (bounded), then restore the tree order.
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));
    let mut downloads = JoinSet::new();
    for (idx, entry) in skill_entries.into_iter().enumerate() {
        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);
        let raw_url = format!("{}/{}/{}/{}/{}", base_raw, owner, repo, raw_ref, entry.path);
        let blob_url = (!entry.sha.is_empty()).then(|| {
            format!(
                "{}/repos/{}/{}/git/blobs/{}",
                base_api, owner, repo, entry.sha
            )
        });
        downloads.spawn(async move {
            let _permit = semaphore.acquire_owned().await.ok();
            let content = fetch_skill_content(&client, &raw_url, blob_url.as_deref()).await;
            (idx, entry.path, content)
        });
    }

    let mut downloaded = Vec::new();
    while let Some(joined) = downloads.join_next().await {
        if let Ok(item) = joined {
            downloaded.push(item);
        }
    }
    downloaded.sort_by_key(|(idx, _, _)| *idx);

    let mut skills: Vec<Skill> = downloaded
        .into_iter()
        .map(|(_, path, content)| {
            let metadata = parse_skill_metadata(&content, &path);
            let url = github_blob_url(&owner, &repo, &web_ref, &path);
            Skill {
                name: metadata.name,
                description: metadata.description,
                path,
                url,
            }
        })
        .collect();

    // Ties (several skills with the same name) are ordered by path so the order is stable.
    skills.sort_by_key(|a| (a.name.to_lowercase(), a.path.clone()));

    // 5. Save to SQLite cache
    if let Some(ref mut conn) = db_conn {
        let _ = save_cached_repository(
            conn,
            &owner,
            &repo,
            branch,
            current_sha.as_deref(),
            None,
            &skills,
        );
    }

    Ok(ScanResult {
        owner,
        repo,
        branch: branch.to_string(),
        skills,
        from_cache: false,
        commit_sha: current_sha,
        truncated,
    })
}
