use serde_json::json;
use skill_atlas::scanner::{is_skill_path, scan_github_repo, ScannerError, ScannerOptions, Skill};
use std::env;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_is_skill_path_positive() {
    assert!(is_skill_path("skills/weather/SKILL.md"));
    assert!(is_skill_path("skills/code-review/skill.yaml"));
    assert!(is_skill_path(".agents/skills/deploy/skill.json"));
    assert!(is_skill_path(".claude/skills/test/Skill.md"));
    assert!(is_skill_path(".junie/skills/analyze/skill.yml"));
    assert!(is_skill_path("SKILL.md"));
    assert!(is_skill_path("skill.json"));
    // Deeply nested skills are still found (no depth limit)
    assert!(is_skill_path(
        "plugins/mcp-tools/resources/jetbrains/mps/agents/mcp/skills/x/SKILL.md"
    ));
}

#[test]
fn test_is_skill_path_ignores_supporting_docs_in_skill_folders() {
    assert!(!is_skill_path(
        ".agents/skills/bugfix-workflow/references/branch-naming.md"
    ));
    assert!(!is_skill_path(".claude/skills/test.md"));
    assert!(!is_skill_path(".agents/skills/deploy.json"));
    assert!(!is_skill_path("skills/weather/config.yaml"));
    assert!(!is_skill_path("skills/MY_SKILL.md"));
}

#[test]
fn test_is_skill_path_ignores_vendored_folders() {
    assert!(!is_skill_path("node_modules/pkg/skills/x/SKILL.md"));
    assert!(!is_skill_path("web/node_modules/pkg/SKILL.md"));
    assert!(!is_skill_path(".venv/lib/site-packages/pkg/SKILL.md"));
    assert!(!is_skill_path(".git/SKILL.md"));
    assert!(!is_skill_path("a/__pycache__/SKILL.md"));
    // A folder merely containing an ignored name as a substring is fine
    assert!(is_skill_path("my-venv-tools/skills/x/SKILL.md"));
}

#[test]
fn test_is_skill_path_negative() {
    assert!(!is_skill_path("src/index.js"));
    assert!(!is_skill_path("README.md"));
    assert!(!is_skill_path("skills/weather/icon.png"));
    assert!(!is_skill_path("package.json"));
    assert!(!is_skill_path("Cargo.toml"));
}

#[tokio::test]
async fn test_scan_github_repo_success_and_caching() {
    let mock_server = MockServer::start().await;
    let temp_dir = env::temp_dir();
    let db_path = temp_dir.join(format!("test_cache_{}.db", uuid_or_timestamp()));

    let commit_response_v1 = json!({
        "sha": "commit_sha_111"
    });

    Mock::given(method("GET"))
        .and(path("/repos/acme/agent-skills/commits/main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(commit_response_v1))
        .mount(&mock_server)
        .await;

    let tree_response = json!({
        "sha": "tree_sha_111",
        "tree": [
            { "path": "README.md", "type": "blob", "sha": "blob0" },
            { "path": "skills/weather/SKILL.md", "type": "blob", "sha": "blob1" },
            { "path": "skills/calculator/SKILL.md", "type": "blob", "sha": "blob2" }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/repos/acme/agent-skills/git/trees/main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tree_response))
        .mount(&mock_server)
        .await;

    let weather_content =
        "---\nname: Weather Forecast\ndescription: Delivers weather updates\n---\n";
    Mock::given(method("GET"))
        .and(path(
            "/acme/agent-skills/commit_sha_111/skills/weather/SKILL.md",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(weather_content))
        .mount(&mock_server)
        .await;

    let calc_content = "# Calculator\nPerforms mathematical equations\n";
    Mock::given(method("GET"))
        .and(path(
            "/acme/agent-skills/commit_sha_111/skills/calculator/SKILL.md",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(calc_content))
        .mount(&mock_server)
        .await;

    let options = ScannerOptions {
        token: None,
        branch: Some("main".to_string()),
        base_api_url: Some(mock_server.uri()),
        base_raw_url: Some(mock_server.uri()),
        no_cache: false,
        db_path: Some(db_path.clone()),
    };

    // First scan: Not from cache, saves to SQLite
    let result1 = scan_github_repo("acme/agent-skills", &options)
        .await
        .unwrap();

    assert_eq!(result1.owner, "acme");
    assert_eq!(result1.repo, "agent-skills");
    assert_eq!(result1.branch, "main");
    assert_eq!(result1.skills.len(), 2);
    assert!(!result1.from_cache);
    assert_eq!(result1.commit_sha, Some("commit_sha_111".to_string()));

    // Sorted alphabetically
    assert_eq!(
        result1.skills[0],
        Skill {
            name: "Calculator".to_string(),
            description: "Performs mathematical equations".to_string(),
            path: "skills/calculator/SKILL.md".to_string(),
            url: "https://github.com/acme/agent-skills/blob/main/skills/calculator/SKILL.md"
                .to_string(),
        }
    );
    assert_eq!(
        result1.skills[1],
        Skill {
            name: "Weather Forecast".to_string(),
            description: "Delivers weather updates".to_string(),
            path: "skills/weather/SKILL.md".to_string(),
            url: "https://github.com/acme/agent-skills/blob/main/skills/weather/SKILL.md"
                .to_string(),
        }
    );

    // Second scan: Commit unchanged -> Loaded from SQLite cache!
    let result2 = scan_github_repo("acme/agent-skills", &options)
        .await
        .unwrap();

    assert!(result2.from_cache);
    assert_eq!(result2.skills.len(), 2);
    assert_eq!(result2.skills[0].name, "Calculator");
    assert_eq!(result2.skills[1].name, "Weather Forecast");

    // Clean up test db file
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_scan_github_repo_not_found_404() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/acme/nonexistent/git/trees/HEAD"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let options = ScannerOptions {
        token: None,
        branch: None,
        base_api_url: Some(mock_server.uri()),
        base_raw_url: Some(mock_server.uri()),
        no_cache: true,
        db_path: None,
    };

    let result = scan_github_repo("acme/nonexistent", &options).await;
    assert!(
        matches!(result, Err(ScannerError::NotFound(owner, repo)) if owner == "acme" && repo == "nonexistent")
    );
}

#[tokio::test]
async fn test_scan_github_repo_forbidden_rate_limit_403() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/acme/limited/git/trees/HEAD"))
        .respond_with(
            ResponseTemplate::new(403)
                .append_header("x-ratelimit-remaining", "0")
                .set_body_string("Rate limit exceeded"),
        )
        .mount(&mock_server)
        .await;

    let options = ScannerOptions {
        token: None,
        branch: None,
        base_api_url: Some(mock_server.uri()),
        base_raw_url: Some(mock_server.uri()),
        no_cache: true,
        db_path: None,
    };

    let result = scan_github_repo("acme/limited", &options).await;
    match result {
        Err(ScannerError::Forbidden(owner, repo, msg)) => {
            assert_eq!(owner, "acme");
            assert_eq!(repo, "limited");
            assert!(msg.contains("rate limit"));
        }
        _ => panic!("Expected Forbidden error with rate limit message"),
    }
}

async fn mount_head_repo(server: &MockServer, default_branch: Option<&str>) {
    if let Some(default_branch) = default_branch {
        Mock::given(method("GET"))
            .and(path("/repos/jetbrains/kotlin"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "default_branch": default_branch })),
            )
            .mount(server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/repos/jetbrains/kotlin/commits/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "sha": "sha_k" })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/jetbrains/kotlin/git/trees/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sha": "tree_k",
            "tree": [{ "path": ".claude/skills/cherry-pick/SKILL.md", "type": "blob", "sha": "b1" }]
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/jetbrains/kotlin/sha_k/.claude/skills/cherry-pick/SKILL.md",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("---\nname: cherry-pick\ndescription: Cherry pick.\n---\n"),
        )
        .mount(server)
        .await;
}

fn head_options(
    server: &MockServer,
    no_cache: bool,
    db_path: Option<std::path::PathBuf>,
) -> ScannerOptions {
    ScannerOptions {
        token: None,
        branch: None,
        base_api_url: Some(server.uri()),
        base_raw_url: Some(server.uri()),
        no_cache,
        db_path,
    }
}

const MASTER_URL: &str =
    "https://github.com/jetbrains/kotlin/blob/master/.claude/skills/cherry-pick/SKILL.md";

#[tokio::test]
async fn test_scan_head_uses_repository_default_branch_in_urls() {
    let server = MockServer::start().await;
    mount_head_repo(&server, Some("master")).await;

    let result = scan_github_repo(
        "https://github.com/jetbrains/kotlin",
        &head_options(&server, true, None),
    )
    .await
    .unwrap();

    assert_eq!(result.skills.len(), 1);
    assert_eq!(result.skills[0].url, MASTER_URL);
}

#[tokio::test]
async fn test_scan_head_falls_back_to_head_ref_when_default_branch_unknown() {
    let server = MockServer::start().await;
    mount_head_repo(&server, None).await;

    let result = scan_github_repo("jetbrains/kotlin", &head_options(&server, true, None))
        .await
        .unwrap();

    assert_eq!(
        result.skills[0].url,
        "https://github.com/jetbrains/kotlin/blob/HEAD/.claude/skills/cherry-pick/SKILL.md"
    );
}

#[tokio::test]
async fn test_scan_cached_results_with_stale_main_urls_are_corrected() {
    let server = MockServer::start().await;
    mount_head_repo(&server, Some("master")).await;
    let db_path = env::temp_dir().join(format!("test_cache_stale_{}.db", uuid_or_timestamp()));

    // Simulate a cache entry written by an older version that assumed `main`.
    let mut conn = skill_atlas::open_db(Some(&db_path)).unwrap();
    let stale = Skill {
        name: "cherry-pick".to_string(),
        description: "Cherry pick.".to_string(),
        path: ".claude/skills/cherry-pick/SKILL.md".to_string(),
        url: "https://github.com/jetbrains/kotlin/blob/main/.claude/skills/cherry-pick/SKILL.md"
            .to_string(),
    };
    skill_atlas::save_cached_repository(
        &mut conn,
        "jetbrains",
        "kotlin",
        "HEAD",
        Some("sha_k"),
        None,
        &[stale],
    )
    .unwrap();
    drop(conn);

    let result = scan_github_repo(
        "jetbrains/kotlin",
        &head_options(&server, false, Some(db_path.clone())),
    )
    .await
    .unwrap();

    assert!(result.from_cache);
    assert_eq!(result.skills[0].url, MASTER_URL);
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_scan_downloads_only_skill_files_concurrently_in_order() {
    let server = MockServer::start().await;
    let count = 40;
    let mut tree = vec![
        json!({ "path": ".agents/skills/s00/references/doc.md", "type": "blob", "sha": "r" }),
        json!({ "path": "node_modules/pkg/SKILL.md", "type": "blob", "sha": "n" }),
    ];
    for i in 0..count {
        tree.push(json!({ "path": format!(".agents/skills/s{:02}/SKILL.md", i), "type": "blob", "sha": format!("b{}", i) }));
    }
    Mock::given(method("GET"))
        .and(path("/repos/acme/many/commits/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "sha": "sha_m" })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/many"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "default_branch": "main" })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/many/git/trees/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "sha": "t", "tree": tree })))
        .mount(&server)
        .await;
    for i in 0..count {
        Mock::given(method("GET"))
            .and(path(format!(
                "/acme/many/sha_m/.agents/skills/s{:02}/SKILL.md",
                i
            )))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(std::time::Duration::from_millis(100))
                    .set_body_string(format!(
                        "---\nname: skill-{:02}\ndescription: Skill {}.\n---\n",
                        i, i
                    )),
            )
            .expect(1)
            .mount(&server)
            .await;
    }

    let options = ScannerOptions {
        token: None,
        branch: None,
        base_api_url: Some(server.uri()),
        base_raw_url: Some(server.uri()),
        no_cache: true,
        db_path: None,
    };
    let started = std::time::Instant::now();
    let result = scan_github_repo("acme/many", &options).await.unwrap();
    let elapsed = started.elapsed();

    assert_eq!(result.skills.len(), count);
    assert!(!result.truncated);
    for (i, skill) in result.skills.iter().enumerate() {
        assert_eq!(skill.name, format!("skill-{:02}", i));
        assert_eq!(skill.path, format!(".agents/skills/s{:02}/SKILL.md", i));
    }
    // 40 downloads of 100 ms each take ~4 s sequentially; concurrently well under 2 s.
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "scan took {:?}",
        elapsed
    );

    // Only real skill files were downloaded: no reference docs, no vendored files, no blob API.
    let requests = server.received_requests().await.unwrap();
    assert!(!requests.iter().any(|r| r.url.path().contains("references")));
    assert!(!requests
        .iter()
        .any(|r| r.url.path().contains("node_modules")));
    assert!(!requests
        .iter()
        .any(|r| r.url.path().contains("/git/blobs/")));
}

#[tokio::test]
async fn test_scan_falls_back_to_blob_api_and_reports_truncated_tree() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/big/commits/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "sha": "sha_b" })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/big/git/trees/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sha": "t",
            "truncated": true,
            "tree": [{ "path": "skills/deploy/SKILL.md", "type": "blob", "sha": "blob_d" }]
        })))
        .mount(&server)
        .await;
    // Raw download is not mocked (404), so the Blobs API is used.
    Mock::given(method("GET"))
        .and(path("/repos/acme/big/git/blobs/blob_d"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("---\nname: deploy\ndescription: Deploys it.\n---\n"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let options = ScannerOptions {
        token: None,
        branch: None,
        base_api_url: Some(server.uri()),
        base_raw_url: Some(server.uri()),
        no_cache: true,
        db_path: None,
    };
    let result = scan_github_repo("acme/big", &options).await.unwrap();

    assert!(result.truncated);
    assert_eq!(result.skills.len(), 1);
    assert_eq!(result.skills[0].name, "deploy");
    assert_eq!(result.skills[0].description, "Deploys it.");
}

fn uuid_or_timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
