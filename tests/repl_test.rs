use serde_json::json;
use skill_atlas::repl::{
    parse_command, run_repl, write_skill_list, ReplCommand, ReplSession, ReplUi, OPEN_USAGE,
    SCAN_USAGE,
};
use skill_atlas::scanner::{ScannerOptions, Skill};
use std::io::{self, Cursor, Write};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Records UI side effects instead of opening a browser or a raw-mode menu.
#[derive(Default)]
struct RecordingUi {
    shown: Vec<(String, usize)>,
    opened: Vec<String>,
    clears: usize,
    fail_open: bool,
}

impl ReplUi for RecordingUi {
    fn show_skills(
        &mut self,
        out: &mut dyn Write,
        skills: &[Skill],
        repo_name: &str,
    ) -> io::Result<()> {
        self.shown.push((repo_name.to_string(), skills.len()));
        write_skill_list(out, skills, repo_name)
    }

    fn open_url(&mut self, url: &str) -> io::Result<()> {
        self.opened.push(url.to_string());
        if self.fail_open {
            Err(io::Error::other("no browser"))
        } else {
            Ok(())
        }
    }

    fn clear_screen(&mut self, _out: &mut dyn Write) -> io::Result<()> {
        self.clears += 1;
        Ok(())
    }
}

async fn mount_repo(
    server: &MockServer,
    owner: &str,
    repo: &str,
    sha: &str,
    skills: &[(&str, &str)],
) {
    Mock::given(method("GET"))
        .and(path(format!("/repos/{}/{}", owner, repo)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "default_branch": "main" })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/repos/{}/{}/commits/HEAD", owner, repo)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "sha": sha })))
        .mount(server)
        .await;

    let tree: Vec<_> = skills
        .iter()
        .map(|(dir, _)| json!({ "path": format!("skills/{}/SKILL.md", dir), "type": "blob", "sha": dir }))
        .collect();
    Mock::given(method("GET"))
        .and(path(format!("/repos/{}/{}/git/trees/HEAD", owner, repo)))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "sha": "tree", "tree": tree })),
        )
        .mount(server)
        .await;

    for (dir, name) in skills {
        Mock::given(method("GET"))
            .and(path(format!(
                "/{}/{}/{}/skills/{}/SKILL.md",
                owner, repo, sha, dir
            )))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                "---\nname: {}\ndescription: Does {} things.\n---\n",
                name, dir
            )))
            .mount(server)
            .await;
    }
}

fn options_for(server: &MockServer, db_name: &str) -> (ScannerOptions, std::path::PathBuf) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = std::env::temp_dir().join(format!("repl_{}_{}.db", db_name, nanos));
    (
        ScannerOptions {
            token: None,
            branch: None,
            base_api_url: Some(server.uri()),
            base_raw_url: Some(server.uri()),
            no_cache: false,
            db_path: Some(db_path.clone()),
        },
        db_path,
    )
}

async fn run_script(session: &mut ReplSession, ui: &mut RecordingUi, script: &str) -> String {
    let mut input = Cursor::new(script.as_bytes().to_vec());
    let mut out: Vec<u8> = Vec::new();
    run_repl(&mut input, &mut out, ui, session).await.unwrap();
    String::from_utf8(out).unwrap()
}

// ---------- parse_command ----------

#[test]
fn test_parse_basic_commands() {
    assert_eq!(parse_command(""), ReplCommand::Empty);
    assert_eq!(parse_command("   \n"), ReplCommand::Empty);
    for cmd in ["exit", "quit", "q", "EXIT"] {
        assert_eq!(parse_command(cmd), ReplCommand::Exit);
    }
    assert_eq!(parse_command("help"), ReplCommand::Help);
    assert_eq!(parse_command("?"), ReplCommand::Help);
    assert_eq!(parse_command("clear"), ReplCommand::Clear);
    assert_eq!(parse_command("list"), ReplCommand::List);
    assert_eq!(parse_command("ls"), ReplCommand::List);
    assert_eq!(parse_command("rescan"), ReplCommand::Rescan);
    assert_eq!(parse_command("refresh"), ReplCommand::Rescan);
}

#[test]
fn test_parse_scan_variants() {
    assert_eq!(
        parse_command("scan acme/skills"),
        ReplCommand::Scan {
            repo: "acme/skills".into(),
            branch: None,
            refresh: false
        }
    );
    assert_eq!(
        parse_command("scan https://github.com/acme/skills -b dev --refresh"),
        ReplCommand::Scan {
            repo: "https://github.com/acme/skills".into(),
            branch: Some("dev".into()),
            refresh: true
        }
    );
    assert_eq!(
        parse_command("scan --branch=main acme/skills --no-cache"),
        ReplCommand::Scan {
            repo: "acme/skills".into(),
            branch: Some("main".into()),
            refresh: true
        }
    );
    // Bare repository input is treated as a scan
    assert_eq!(
        parse_command("https://github.com/acme/skills"),
        ReplCommand::Scan {
            repo: "https://github.com/acme/skills".into(),
            branch: None,
            refresh: false
        }
    );
    assert_eq!(
        parse_command("git@github.com:acme/skills.git"),
        ReplCommand::Scan {
            repo: "git@github.com:acme/skills.git".into(),
            branch: None,
            refresh: false
        }
    );
}

#[test]
fn test_parse_invalid_commands() {
    assert_eq!(
        parse_command("scan"),
        ReplCommand::Invalid(SCAN_USAGE.into())
    );
    assert_eq!(
        parse_command("scan a/b -b"),
        ReplCommand::Invalid(SCAN_USAGE.into())
    );
    assert_eq!(
        parse_command("scan a/b c/d"),
        ReplCommand::Invalid(SCAN_USAGE.into())
    );
    assert!(
        matches!(parse_command("scan a/b --bogus"), ReplCommand::Invalid(m) if m.contains("--bogus"))
    );
    assert_eq!(
        parse_command("open"),
        ReplCommand::Invalid(OPEN_USAGE.into())
    );
    assert_eq!(
        parse_command("open My Skill"),
        ReplCommand::Open("My Skill".into())
    );
    assert!(
        matches!(parse_command("frobnicate"), ReplCommand::Invalid(m) if m.contains("Unknown command"))
    );
}

// ---------- session loop ----------

#[tokio::test]
async fn test_session_keeps_running_across_multiple_scans_and_opens() {
    let server = MockServer::start().await;
    mount_repo(
        &server,
        "acme",
        "one",
        "sha1",
        &[("alpha", "Alpha"), ("beta", "Beta")],
    )
    .await;
    mount_repo(&server, "acme", "two", "sha2", &[("gamma", "Gamma")]).await;
    let (options, db) = options_for(&server, "multi");

    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi::default();
    let out = run_script(
        &mut session,
        &mut ui,
        "scan acme/one\nopen 1\nopen beta\nacme/two\nopen 1\nlist\nexit\n",
    )
    .await;

    assert_eq!(
        ui.shown,
        vec![
            ("acme/one".into(), 2),
            ("acme/two".into(), 1),
            ("acme/two".into(), 1)
        ]
    );
    assert_eq!(
        ui.opened,
        vec![
            "https://github.com/acme/one/blob/main/skills/alpha/SKILL.md",
            "https://github.com/acme/one/blob/main/skills/beta/SKILL.md",
            "https://github.com/acme/two/blob/main/skills/gamma/SKILL.md",
        ]
    );
    assert!(out.contains("Goodbye!"));
    assert_eq!(session.repo_name.as_deref(), Some("acme/two"));
    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn test_session_rescan_uses_cache_bypass_and_scan_again_uses_cache() {
    let server = MockServer::start().await;
    mount_repo(&server, "acme", "one", "sha1", &[("alpha", "Alpha")]).await;
    let (options, db) = options_for(&server, "rescan");

    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi::default();
    let out = run_script(
        &mut session,
        &mut ui,
        "scan acme/one\nscan acme/one\nrescan\nquit\n",
    )
    .await;

    // Second scan is served from the SQLite cache, rescan re-fetches (no cache message).
    assert_eq!(
        out.matches("Loaded results from local SQLite database")
            .count(),
        1
    );
    assert_eq!(ui.shown.len(), 3);
    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn test_session_survives_scan_error_and_continues() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/missing/git/trees/HEAD"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/missing/commits/HEAD"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    mount_repo(&server, "acme", "one", "sha1", &[("alpha", "Alpha")]).await;
    let (options, db) = options_for(&server, "error");

    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi::default();
    let out = run_script(
        &mut session,
        &mut ui,
        "scan acme/missing\nscan not a repo\nscan acme/one\nopen 1\nexit\n",
    )
    .await;

    assert!(out.contains("Error:"));
    assert!(out.contains("not found"));
    assert_eq!(ui.shown, vec![("acme/one".into(), 1)]);
    assert_eq!(ui.opened.len(), 1);
    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn test_session_repo_without_skills() {
    let server = MockServer::start().await;
    mount_repo(&server, "acme", "empty", "sha0", &[]).await;
    let (options, db) = options_for(&server, "empty");

    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi::default();
    let out = run_script(&mut session, &mut ui, "scan acme/empty\nopen 1\nexit\n").await;

    assert!(out.contains("No agent skills found in"));
    assert!(out.contains("No skills listed yet"));
    assert!(ui.shown.is_empty());
    assert!(ui.opened.is_empty());
    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn test_session_open_edge_cases() {
    let server = MockServer::start().await;
    mount_repo(
        &server,
        "acme",
        "one",
        "sha1",
        &[("alpha", "Alpha"), ("beta", "Beta")],
    )
    .await;
    let (options, db) = options_for(&server, "open");

    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi {
        fail_open: true,
        ..Default::default()
    };
    let out = run_script(
        &mut session,
        &mut ui,
        "open 1\nscan acme/one\nopen 0\nopen 3\nopen zeta\nopen ALPHA\nexit\n",
    )
    .await;

    assert!(out.contains("No skills listed yet in this session"));
    assert_eq!(
        out.matches("Invalid index. Please choose between 1 and 2.")
            .count(),
        2
    );
    assert!(out.contains("Skill matching 'zeta' not found"));
    assert!(out.contains("Failed to open browser: no browser"));
    assert_eq!(
        ui.opened,
        vec!["https://github.com/acme/one/blob/main/skills/alpha/SKILL.md"]
    );
    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn test_session_help_clear_unknown_and_nothing_to_rescan() {
    let mut session = ReplSession::new(ScannerOptions::default());
    let mut ui = RecordingUi::default();
    let out = run_script(
        &mut session,
        &mut ui,
        "\nhelp\nclear\nfoo\nrescan\nlist\nexit\n",
    )
    .await;

    assert!(out.contains("Available commands:"));
    assert!(out.contains("open <number|name>"));
    assert_eq!(ui.clears, 1);
    assert!(out.contains("Unknown command: 'foo'"));
    assert!(out.contains("Nothing to rescan yet"));
    assert!(out.contains("No skills listed yet"));
    assert!(out.contains("Goodbye!"));
}

#[tokio::test]
async fn test_session_ends_on_eof() {
    let mut session = ReplSession::new(ScannerOptions::default());
    let mut ui = RecordingUi::default();
    let out = run_script(&mut session, &mut ui, "help\n").await;
    assert!(out.contains("Available commands:"));
    assert!(out.contains("Goodbye!"));
}

#[tokio::test]
async fn test_session_stops_reading_after_exit() {
    let mut session = ReplSession::new(ScannerOptions::default());
    let mut ui = RecordingUi::default();
    let out = run_script(&mut session, &mut ui, "exit\nhelp\n").await;
    assert!(!out.contains("Available commands:"));
}

#[tokio::test]
async fn test_session_warns_when_tree_is_truncated() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/huge/git/trees/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sha": "tree",
            "truncated": true,
            "tree": [{ "path": "skills/alpha/SKILL.md", "type": "blob", "sha": "alpha" }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/huge/commits/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "sha": "shah" })))
        .mount(&server)
        .await;
    let (options, db) = options_for(&server, "truncated");

    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi::default();
    let out = run_script(&mut session, &mut ui, "scan acme/huge\nexit\n").await;

    assert!(out.contains("results may be incomplete"));
    assert_eq!(ui.shown.len(), 1);
    let _ = std::fs::remove_file(db);
}
