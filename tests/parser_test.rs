use skill_atlas::parser::{parse_repo_identifier, ParseError, RepoIdentifier};

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
