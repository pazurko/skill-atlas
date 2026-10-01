use clap::Parser;
use skill_atlas::cli::{Cli, Commands};

#[test]
fn test_cli_parse_scan_basic() {
    let args = vec!["skill-atlas", "scan", "openai/swarm"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Scan { githubrepo }) => {
            assert_eq!(githubrepo, Some("openai/swarm".to_string()));
        }
        _ => panic!("Expected Scan command"),
    }
}

#[test]
fn test_cli_parse_scan_without_repo() {
    let args = vec!["skill-atlas", "scan"];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Some(Commands::Scan { githubrepo }) => {
            assert_eq!(githubrepo, None);
        }
        _ => panic!("Expected Scan command"),
    }
}

#[test]
fn test_cli_parse_list_and_alias() {
    let args = vec!["skill-atlas", "list"];
    let cli = Cli::try_parse_from(args).unwrap();
    assert_eq!(cli.command, Some(Commands::List));

    let args_alias = vec!["skill-atlas", "ls"];
    let cli_alias = Cli::try_parse_from(args_alias).unwrap();
    assert_eq!(cli_alias.command, Some(Commands::List));
}

#[test]
fn test_cli_parse_filter() {
    let args = vec!["skill-atlas", "filter", "gradle"];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Some(Commands::Filter { query }) => {
            assert_eq!(query, Some("gradle".to_string()));
        }
        _ => panic!("Expected Filter command"),
    }

    let args_empty = vec!["skill-atlas", "filter"];
    let cli_empty = Cli::try_parse_from(args_empty).unwrap();
    match cli_empty.command {
        Some(Commands::Filter { query }) => {
            assert_eq!(query, None);
        }
        _ => panic!("Expected Filter command"),
    }
}

#[test]
fn test_cli_parse_similar() {
    let args = vec!["skill-atlas", "similar", "test-skill"];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Some(Commands::Similar { target }) => {
            assert_eq!(target, Some("test-skill".to_string()));
        }
        _ => panic!("Expected Similar command"),
    }

    let args_all = vec!["skill-atlas", "similar"];
    let cli_all = Cli::try_parse_from(args_all).unwrap();
    match cli_all.command {
        Some(Commands::Similar { target }) => {
            assert_eq!(target, None);
        }
        _ => panic!("Expected Similar command"),
    }
}

#[test]
fn test_cli_parse_web_default_and_alias() {
    let args = vec!["skill-atlas", "web"];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Some(Commands::Web { port }) => {
            assert_eq!(port, 3000);
        }
        _ => panic!("Expected Web command"),
    }

    let args_serve = vec!["skill-atlas", "serve", "-p", "8080"];
    let cli_serve = Cli::try_parse_from(args_serve).unwrap();
    match cli_serve.command {
        Some(Commands::Web { port }) => {
            assert_eq!(port, 8080);
        }
        _ => panic!("Expected Web command with port 8080"),
    }
}

#[test]
fn test_cli_parse_no_args() {
    let args = vec!["skill-atlas"];
    let cli = Cli::try_parse_from(args).unwrap();
    assert_eq!(cli.command, None);
}
