use skill_atlas::parser::{
    parse_repo_identifier, parse_scan_target, ParseError, RepoIdentifier, ScanTarget,
};

#[test]
fn test_parse_scan_target_repos_and_orgs() {
    // 1. Repo shorthand
    assert_eq!(
        parse_scan_target("openai/swarm").unwrap(),
        ScanTarget::Repo {
            owner: "openai".to_string(),
            repo: "swarm".to_string()
        }
    );

    // 2. Org with org: prefix
    assert_eq!(
        parse_scan_target("org:JetBrains").unwrap(),
        ScanTarget::Org {
            org: "JetBrains".to_string()
        }
    );

    // 3. Org with @ prefix
    assert_eq!(
        parse_scan_target("@openai").unwrap(),
        ScanTarget::Org {
            org: "openai".to_string()
        }
    );

    // 4. Org with org/ prefix
    assert_eq!(
        parse_scan_target("org/google").unwrap(),
        ScanTarget::Org {
            org: "google".to_string()
        }
    );

    // 5. Org GitHub URL
    assert_eq!(
        parse_scan_target("https://github.com/JetBrains").unwrap(),
        ScanTarget::Org {
            org: "JetBrains".to_string()
        }
    );
    assert_eq!(
        parse_scan_target("https://github.com/JetBrains/").unwrap(),
        ScanTarget::Org {
            org: "JetBrains".to_string()
        }
    );

    // 6. Repo GitHub URL
    assert_eq!(
        parse_scan_target("https://github.com/JetBrains/kotlin").unwrap(),
        ScanTarget::Repo {
            owner: "JetBrains".to_string(),
            repo: "kotlin".to_string()
        }
    );
}

#[test]
fn test_parse_shorthand() {
    let result = parse_repo_identifier("openai/swarm").unwrap();
    assert_eq!(
        result,
        RepoIdentifier {
            owner: "openai".to_string(),
            repo: "swarm".to_string(),
        }
    );
}

#[test]
fn test_parse_shorthand_with_git_suffix() {
    let result = parse_repo_identifier("facebook/react.git").unwrap();
    assert_eq!(
        result,
        RepoIdentifier {
            owner: "facebook".to_string(),
            repo: "react".to_string(),
        }
    );
}

#[test]
fn test_parse_https_url() {
    let result = parse_repo_identifier("https://github.com/rust-lang/rust").unwrap();
    assert_eq!(
        result,
        RepoIdentifier {
            owner: "rust-lang".to_string(),
            repo: "rust".to_string(),
        }
    );
}

#[test]
fn test_parse_https_url_with_git_and_trailing_path() {
    let result =
        parse_repo_identifier("https://github.com/rust-lang/cargo.git/tree/master").unwrap();
    assert_eq!(
        result,
        RepoIdentifier {
            owner: "rust-lang".to_string(),
            repo: "cargo".to_string(),
        }
    );
}

#[test]
fn test_parse_ssh_url() {
    let result = parse_repo_identifier("git@github.com:pazurko/skill-atlas.git").unwrap();
    assert_eq!(
        result,
        RepoIdentifier {
            owner: "pazurko".to_string(),
            repo: "skill-atlas".to_string(),
        }
    );
}

#[test]
fn test_parse_empty_string() {
    let result = parse_repo_identifier("   ");
    assert_eq!(result, Err(ParseError::Empty));
}

#[test]
fn test_parse_invalid_format() {
    let result = parse_repo_identifier("invalid_repo_name");
    assert!(matches!(result, Err(ParseError::InvalidFormat(_))));
}
