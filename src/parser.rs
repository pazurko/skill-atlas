use regex::Regex;

#[derive(Debug, PartialEq, Eq)]
pub struct RepoIdentifier {
    pub owner: String,
    pub repo: String,
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
