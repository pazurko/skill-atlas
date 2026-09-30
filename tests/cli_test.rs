use clap::Parser;
use skill_atlas::cli::{Cli, Commands};
use std::path::PathBuf;

#[test]
fn test_cli_parse_scan_basic() {
    let args = vec!["skill-atlas", "scan", "openai/swarm"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Scan {
            githubrepo,
            token,
            branch,
            json,
            no_cache,
            db_path,
        }) => {
            assert_eq!(githubrepo, Some("openai/swarm".to_string()));
            assert_eq!(token, None);
            assert_eq!(branch, None);
            assert!(!json);
            assert!(!no_cache);
            assert_eq!(db_path, None);
        }
        None => panic!("Expected Scan command"),
    }
}

#[test]
fn test_cli_parse_scan_with_flags() {
    let args = vec![
        "skill-atlas",
        "scan",
        "https://github.com/anthropics/anthropic-sdk-python",
        "--token",
        "ghp_test123",
        "--branch",
        "develop",
        "--json",
        "--no-cache",
        "--db-path",
        "/tmp/custom_skills.db",
    ];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Scan {
            githubrepo,
            token,
            branch,
            json,
            no_cache,
            db_path,
        }) => {
            assert_eq!(
                githubrepo,
                Some("https://github.com/anthropics/anthropic-sdk-python".to_string())
            );
            assert_eq!(token, Some("ghp_test123".to_string()));
            assert_eq!(branch, Some("develop".to_string()));
            assert!(json);
            assert!(no_cache);
            assert_eq!(db_path, Some(PathBuf::from("/tmp/custom_skills.db")));
        }
        None => panic!("Expected Scan command"),
    }
}

#[test]
fn test_cli_parse_no_args() {
    let args = vec!["skill-atlas"];
    let cli = Cli::try_parse_from(args).unwrap();
    assert_eq!(cli.command, None);
}

#[test]
fn test_cli_parse_scan_without_repo() {
    let args = vec!["skill-atlas", "scan"];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Some(Commands::Scan { githubrepo, .. }) => {
            assert_eq!(githubrepo, None);
        }
        None => panic!("Expected Scan command"),
    }
}
