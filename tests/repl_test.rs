use serde_json::json;
use skill_atlas::history::OpenHistory;
use skill_atlas::interactive::{MenuAction, MenuState};
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
    /// Skill indices "opened with Enter" inside the menu, consumed one per menu visit.
    menu_opens: Vec<Vec<usize>>,
}

impl ReplUi for RecordingUi {
    fn show_skills(
        &mut self,
        out: &mut dyn Write,
        skills: &[Skill],
        repo_name: &str,
        history: &mut OpenHistory,
    ) -> io::Result<()> {
        self.shown.push((repo_name.to_string(), skills.len()));
        write_skill_list(out, skills, repo_name)?;
        if !self.menu_opens.is_empty() {
            let opens = self.menu_opens.remove(0);
            let mut state = MenuState::default();
            for idx in opens {
                state.apply(
                    MenuAction::Open(idx),
                    skills,
                    repo_name,
                    history,
                    &mut |_| Ok(()),
                );
                state.apply(MenuAction::Move(0), skills, repo_name, history, &mut |_| {
                    Ok(())
                });
            }
        }
        Ok(())
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

fn rand_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn options_for(server: &MockServer, db_name: &str) -> (ScannerOptions, std::path::PathBuf) {
    let nanos = rand_nanos();
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
fn test_parse_web_command() {
    assert_eq!(
        parse_command("web"),
        ReplCommand::Web {
            port: None,
            no_open: false
        }
    );
    assert_eq!(
        parse_command("serve"),
        ReplCommand::Web {
            port: None,
            no_open: false
        }
    );
    assert_eq!(
        parse_command("web -p 8080"),
        ReplCommand::Web {
            port: Some(8080),
            no_open: false
        }
    );
    assert_eq!(
        parse_command("web --port 8080 --no-open"),
        ReplCommand::Web {
            port: Some(8080),
            no_open: true
        }
    );
    assert_eq!(
        parse_command("web --invalid-flag"),
        ReplCommand::Invalid("Usage: web [--port <PORT>] [--no-open]".to_string())
    );
    assert_eq!(
        parse_command("web --port notanumber"),
        ReplCommand::Invalid(
            "Invalid port number. Usage: web [--port <PORT>] [--no-open]".to_string()
        )
    );
}

#[test]
fn test_parse_scan_variants() {
    assert_eq!(
        parse_command("scan acme/skills"),
        ReplCommand::Scan {
            repos: vec!["acme/skills".into()],
            branch: None,
            refresh: false,
            filter: None,
        }
    );
    assert_eq!(
        parse_command("scan https://github.com/acme/skills -b dev --refresh"),
        ReplCommand::Scan {
            repos: vec!["https://github.com/acme/skills".into()],
            branch: Some("dev".into()),
            refresh: true,
            filter: None,
        }
    );
    assert_eq!(
        parse_command("scan --branch=main acme/skills --no-cache -f review"),
        ReplCommand::Scan {
            repos: vec!["acme/skills".into()],
            branch: Some("main".into()),
            refresh: true,
            filter: Some("review".into()),
        }
    );
    assert_eq!(
        parse_command("scan acme/skills other/repo"),
        ReplCommand::Scan {
            repos: vec!["acme/skills".into(), "other/repo".into()],
            branch: None,
            refresh: false,
            filter: None,
        }
    );
    assert_eq!(
        parse_command("scan acme/skills, other/repo"),
        ReplCommand::Scan {
            repos: vec!["acme/skills".into(), "other/repo".into()],
            branch: None,
            refresh: false,
            filter: None,
        }
    );
    // Bare repository input is treated as a scan
    assert_eq!(
        parse_command("https://github.com/acme/skills"),
        ReplCommand::Scan {
            repos: vec!["https://github.com/acme/skills".into()],
            branch: None,
            refresh: false,
            filter: None,
        }
    );
    assert_eq!(
        parse_command("git@github.com:acme/skills.git"),
        ReplCommand::Scan {
            repos: vec!["git@github.com:acme/skills.git".into()],
            branch: None,
            refresh: false,
            filter: None,
        }
    );
}

#[test]
fn test_parse_filter_and_similar_commands() {
    assert_eq!(parse_command("filter"), ReplCommand::Filter(None));
    assert_eq!(parse_command("f"), ReplCommand::Filter(None));
    assert_eq!(
        parse_command("filter code review"),
        ReplCommand::Filter(Some("code review".to_string()))
    );
    assert_eq!(
        parse_command("f deploy"),
        ReplCommand::Filter(Some("deploy".to_string()))
    );
    assert_eq!(parse_command("similar"), ReplCommand::Similar(None));
    assert_eq!(
        parse_command("similar 1"),
        ReplCommand::Similar(Some("1".to_string()))
    );
    assert_eq!(
        parse_command("similar code-review"),
        ReplCommand::Similar(Some("code-review".to_string()))
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
    let temp_db = std::env::temp_dir().join(format!("test_repl_empty_{}.db", rand_nanos()));
    let options = ScannerOptions {
        db_path: Some(temp_db.clone()),
        ..Default::default()
    };
    let mut session = ReplSession::new(options);
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
    let _ = std::fs::remove_file(temp_db);
}

#[tokio::test]
async fn test_session_list_loads_from_sqlite_database() {
    let temp_db = std::env::temp_dir().join(format!("test_repl_db_list_{}.db", rand_nanos()));
    let mut conn = skill_atlas::storage::open_db(Some(&temp_db)).unwrap();
    let skills = vec![Skill {
        name: "cached-skill".to_string(),
        description: "From database".to_string(),
        path: "skills/test/SKILL.md".to_string(),
        url: "https://github.com/test/repo/blob/main/skills/test/SKILL.md".to_string(),
    }];
    skill_atlas::storage::save_cached_repository(
        &mut conn,
        "test",
        "repo",
        "main",
        Some("sha_db"),
        None,
        &skills,
    )
    .unwrap();

    let options = ScannerOptions {
        db_path: Some(temp_db.clone()),
        ..Default::default()
    };
    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi::default();
    let out = run_script(&mut session, &mut ui, "list\nexit\n").await;

    assert_eq!(ui.shown.len(), 1);
    assert_eq!(session.skills.len(), 1);
    assert_eq!(session.skills[0].name, "cached-skill");
    assert!(out.contains("cached-skill"));

    let _ = std::fs::remove_file(temp_db);
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

#[test]
fn test_parse_history_command() {
    assert_eq!(parse_command("history"), ReplCommand::History);
    assert_eq!(parse_command("OPENED"), ReplCommand::History);
}

#[tokio::test]
async fn test_history_is_an_audit_of_menu_and_open_across_repos() {
    let server = MockServer::start().await;
    mount_repo(
        &server,
        "acme",
        "one",
        "sha1",
        &[("alpha", "alpha"), ("beta", "beta")],
    )
    .await;
    mount_repo(&server, "acme", "two", "sha2", &[("gamma", "gamma")]).await;
    let (options, db) = options_for(&server, "history");

    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi {
        // First menu visit: Enter on beta, then on alpha. Later visits open nothing.
        menu_opens: vec![vec![1, 0]],
        ..Default::default()
    };
    let out = run_script(
        &mut session,
        &mut ui,
        "history\nscan acme/one\nscan acme/two\nopen gamma\nlist\nhistory\nexit\n",
    )
    .await;

    assert!(out.contains("Nothing opened yet in this session."));
    assert!(out.contains("Opened in this session (3):"));
    let names: Vec<(&str, &str)> = session
        .history
        .records()
        .iter()
        .map(|r| (r.repo.as_str(), r.name.as_str()))
        .collect();
    assert_eq!(
        names,
        vec![
            ("acme/one", "beta"),
            ("acme/one", "alpha"),
            ("acme/two", "gamma")
        ]
    );
    // The history survives new scans and further menu visits.
    assert_eq!(ui.shown.len(), 3);
    let history_part = &out[out.rfind("Opened in this session").unwrap()..];
    for url_part in [
        "skills/beta/SKILL.md",
        "skills/alpha/SKILL.md",
        "skills/gamma/SKILL.md",
    ] {
        assert!(history_part.contains(url_part), "missing {url_part}");
    }
    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn test_history_records_failed_browser_launch() {
    let server = MockServer::start().await;
    mount_repo(&server, "acme", "one", "sha1", &[("alpha", "alpha")]).await;
    let (options, db) = options_for(&server, "history_fail");

    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi {
        fail_open: true,
        ..Default::default()
    };
    let out = run_script(
        &mut session,
        &mut ui,
        "scan acme/one\nopen 1\nhistory\nexit\n",
    )
    .await;

    assert_eq!(session.history.len(), 1);
    assert_eq!(
        session.history.records()[0].error.as_deref(),
        Some("no browser")
    );
    assert!(out.contains("failed: no browser"));
    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn test_open_by_name_warns_about_duplicate_names() {
    let server = MockServer::start().await;
    // Two different folders declaring the same skill name.
    mount_repo(
        &server,
        "acme",
        "dups",
        "shad",
        &[("review-a", "review"), ("review-b", "review")],
    )
    .await;
    let (options, db) = options_for(&server, "dups");

    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi::default();
    let out = run_script(
        &mut session,
        &mut ui,
        "scan acme/dups\nopen review\nopen 2\nexit\n",
    )
    .await;

    // Both are kept, ordered by path, and listed with their path so they can be told apart.
    assert_eq!(session.skills.len(), 2);
    assert_eq!(session.skills[0].path, "skills/review-a/SKILL.md");
    assert_eq!(session.skills[1].path, "skills/review-b/SKILL.md");
    assert!(out.contains("skills/review-b/SKILL.md ↗"));
    assert!(out.contains("Note: 2 skills are named 'review'; opened skills/review-a/SKILL.md"));
    assert_eq!(
        out.matches("Note: 2 skills").count(),
        1,
        "no note when opening by number"
    );
    assert_eq!(ui.opened.len(), 2);
    assert!(ui.opened[1].ends_with("skills/review-b/SKILL.md"));
    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn test_session_filter_command() {
    let server = MockServer::start().await;
    mount_repo(
        &server,
        "acme",
        "filter-test",
        "sha_f",
        &[
            ("build-gradle", "build-gradle"),
            ("code-review", "code-review"),
            ("deploy-app", "deploy-app"),
        ],
    )
    .await;
    let (options, db) = options_for(&server, "filter_session");

    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi::default();
    let out = run_script(
        &mut session,
        &mut ui,
        "filter\nscan acme/filter-test\nfilter review\nlist\nfilter nomatch\nfilter clear\nexit\n",
    )
    .await;

    // Filter before scanning
    assert!(out.contains("No skills listed yet in this session. Run 'scan <githubrepo>' first."));

    // Filter review matches 1 skill
    assert!(out.contains("acme/filter-test (filtered: 1 of 3)"));

    // Filter nomatch
    assert!(out.contains("No skills matched filter 'nomatch' (0 of 3 skills)."));

    // Filter cleared
    assert!(out.contains("Filter cleared. Showing all skills."));

    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn test_session_similar_command() {
    let server = MockServer::start().await;
    mount_repo(
        &server,
        "acme",
        "similar-test",
        "sha_s",
        &[
            ("code-review-1", "code-review"),
            ("deploy-prod", "deploy-prod"),
            ("code-review-2", "code-review"),
        ],
    )
    .await;
    let (options, db) = options_for(&server, "similar_session");

    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi::default();
    let out = run_script(
        &mut session,
        &mut ui,
        "similar\nscan acme/similar-test\nsimilar\nsimilar 1\nsimilar 2\nexit\n",
    )
    .await;

    // Similar before scanning
    assert!(out.contains("No skills listed yet in this session. Run 'scan <githubrepo>' first."));

    // Similar across repo finds the code-review duplicate pair
    assert!(out.contains("Similar skills in acme/similar-test (threshold: >= 30%):"));
    assert!(out.contains("[1] code-review <-> [2] code-review (100% similar)"));

    // Similar to skill [1] finds skill [2]
    assert!(out.contains("Skills similar to 'code-review' (threshold: >= 30%):"));
    assert!(out.contains("[2] › code-review (100% similar)"));

    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn test_session_scan_multiple_repositories() {
    let server = MockServer::start().await;
    mount_repo(
        &server,
        "acme",
        "repo-one",
        "sha_1",
        &[("skill-a", "skill-a")],
    )
    .await;
    mount_repo(
        &server,
        "acme",
        "repo-two",
        "sha_2",
        &[("skill-b", "skill-b"), ("skill-c", "skill-c")],
    )
    .await;
    let (options, db) = options_for(&server, "multi_session");

    let mut session = ReplSession::new(options);
    let mut ui = RecordingUi::default();
    let out = run_script(
        &mut session,
        &mut ui,
        "scan acme/repo-one acme/repo-two\nrescan\nexit\n",
    )
    .await;

    assert!(out.contains("Scanning 2 repositories for agent skills..."));
    assert!(out.contains("Scanning acme/repo-one..."));
    assert!(out.contains("Scanning acme/repo-two..."));
    assert_eq!(session.skills.len(), 3);
    assert_eq!(session.skills[0].name, "skill-a");
    assert_eq!(session.skills[1].name, "skill-b");
    assert_eq!(session.skills[2].name, "skill-c");

    let _ = std::fs::remove_file(db);
}
