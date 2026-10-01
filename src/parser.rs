use regex::Regex;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RepoIdentifier {
    pub owner: String,
    pub repo: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ScanTarget {
    Repo { owner: String, repo: String },
    Org { org: String },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("Repository identifier cannot be empty")]
    Empty,
    #[error("Invalid GitHub repository format: \"{0}\". Expected \"owner/repo\" or a GitHub URL.")]
    InvalidFormat(String),
}

/// Parses a GitHub repository string into (owner, repo).
/// Supports shorthand `owner/repo`, HTTPS URLs, and SSH URLs.
pub fn parse_repo_identifier(input: &str) -> Result<RepoIdentifier, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }

    // Match HTTPS / HTTP URLs: https://github.com/owner/repo(.git)
    let http_re =
        Regex::new(r"^(?:https?://)?(?:www\.)?github\.com/([^/]+)/([^/#?]+?)(?:\.git)?(?:/.*)?$")
            .unwrap();
    if let Some(caps) = http_re.captures(trimmed) {
        let owner = caps.get(1).unwrap().as_str().to_string();
        let repo = caps.get(2).unwrap().as_str().to_string();
        return Ok(RepoIdentifier { owner, repo });
    }

    // Match SSH URLs: git@github.com:owner/repo(.git)
    let ssh_re = Regex::new(r"^git@github\.com:([^/]+)/([^/#?]+?)(?:\.git)?$").unwrap();
    if let Some(caps) = ssh_re.captures(trimmed) {
        let owner = caps.get(1).unwrap().as_str().to_string();
        let repo = caps.get(2).unwrap().as_str().to_string();
        return Ok(RepoIdentifier { owner, repo });
    }

    // Match shorthand: owner/repo
    let shorthand_re = Regex::new(r"^([a-zA-Z0-9_.-]+)/([a-zA-Z0-9_.-]+)$").unwrap();
    if let Some(caps) = shorthand_re.captures(trimmed) {
        let owner = caps.get(1).unwrap().as_str().to_string();
        let mut repo = caps.get(2).unwrap().as_str().to_string();
        if repo.ends_with(".git") {
            repo = repo[..repo.len() - 4].to_string();
        }
        return Ok(RepoIdentifier { owner, repo });
    }

    Err(ParseError::InvalidFormat(trimmed.to_string()))
}

/// Parses a scan target, which can be a single repository or an organization.
/// Supports `owner/repo`, `org:<org>`, `org/<org>`, `@<org>`, `https://github.com/<org>`, and standard repo URLs.
pub fn parse_scan_target(input: &str) -> Result<ScanTarget, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }

    // 1. Explicit org: prefix (e.g. org:JetBrains or org:openai)
    if let Some(org_name) = trimmed.strip_prefix("org:") {
        let org = org_name.trim().trim_matches('/').to_string();
        if !org.is_empty() {
            return Ok(ScanTarget::Org { org });
        }
    }

    // 2. Explicit @ prefix (e.g. @JetBrains)
    if let Some(org_name) = trimmed.strip_prefix('@') {
        let org = org_name.trim().to_string();
        if !org.is_empty() {
            return Ok(ScanTarget::Org { org });
        }
    }

    // 3. Shorthand org/<org> (e.g. org/JetBrains)
    if let Some(org_name) = trimmed.strip_prefix("org/") {
        let org = org_name.trim().trim_matches('/').to_string();
        if !org.is_empty() {
            return Ok(ScanTarget::Org { org });
        }
    }

    // 4. GitHub Org URL without repo (e.g. https://github.com/JetBrains or github.com/JetBrains/)
    let http_org_re = Regex::new(r"^(?:https?://)?(?:www\.)?github\.com/([^/#?]+)/?$").unwrap();
    if let Some(caps) = http_org_re.captures(trimmed) {
        let org = caps.get(1).unwrap().as_str().to_string();
        return Ok(ScanTarget::Org { org });
    }

    // 5. Try parsing as normal repository
    let repo_id = parse_repo_identifier(trimmed)?;
    Ok(ScanTarget::Repo {
        owner: repo_id.owner,
        repo: repo_id.repo,
    })
}
