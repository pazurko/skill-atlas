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
        _ => panic!("Expected Scan command"),
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
        _ => panic!("Expected Scan command"),
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
        _ => panic!("Expected Scan command"),
    }
}

#[test]
fn test_cli_parse_web_default() {
    let args = vec!["skill-atlas", "web"];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Some(Commands::Web {
            host,
            port,
            no_open,
            open,
            token,
            branch,
            no_cache,
            db_path,
        }) => {
            assert_eq!(host, "127.0.0.1");
            assert_eq!(port, 3000);
            assert!(!no_open);
            assert!(!open);
            assert_eq!(token, None);
            assert_eq!(branch, None);
            assert!(!no_cache);
            assert_eq!(db_path, None);
        }
        _ => panic!("Expected Web command"),
    }
}

#[test]
fn test_cli_parse_web_with_flags_and_serve_alias() {
    let args = vec![
        "skill-atlas",
        "serve",
        "-H",
        "0.0.0.0",
        "-p",
        "8080",
        "--no-open",
        "--token",
        "ghp_web123",
        "--branch",
        "main",
        "--refresh",
        "--db-path",
        "/tmp/web_skills.db",
    ];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Some(Commands::Web {
            host,
            port,
            no_open,
            token,
            branch,
            no_cache,
            db_path,
            ..
        }) => {
            assert_eq!(host, "0.0.0.0");
            assert_eq!(port, 8080);
            assert!(no_open);
            assert_eq!(token, Some("ghp_web123".to_string()));
            assert_eq!(branch, Some("main".to_string()));
            assert!(no_cache);
            assert_eq!(db_path, Some(PathBuf::from("/tmp/web_skills.db")));
        }
        _ => panic!("Expected Web command"),
    }
}
