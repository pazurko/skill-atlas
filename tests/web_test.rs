use serde_json::json;
use skill_atlas::scanner::{ScanResult, ScannerOptions, Skill};
use skill_atlas::storage::{open_db, save_cached_repository};
use skill_atlas::web::{create_router, AppState, ErrorResponse, HealthResponse};
use std::sync::Arc;
use tokio::net::TcpListener;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn spawn_test_app(options: ScannerOptions) -> (String, std::path::PathBuf) {
    let db_path = options
        .db_path
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join(format!("web_test_{}.db", rand_nanos())));

    let mut final_options = options;
    final_options.db_path = Some(db_path.clone());

    let state = Arc::new(AppState {
        default_options: final_options,
    });

    let app = create_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{}", addr), db_path)
}

fn rand_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

#[tokio::test]
async fn test_web_index_html_served() {
    let (base_url, db_path) = spawn_test_app(ScannerOptions::default()).await;
    let client = reqwest::Client::new();

    let res = client.get(&base_url).send().await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let content_type = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(content_type.contains("text/html"));

    let body = res.text().await.unwrap();
    assert!(body.contains("Skill Atlas"));
    assert!(body.contains("AI Agent Skills Explorer"));
    assert!(body.contains("Scan Repository"));
    assert!(body.contains("id=\"repo-input\""));
    assert!(body.contains("id=\"filter-input\""));

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_web_api_health() {
    let (base_url, db_path) = spawn_test_app(ScannerOptions::default()).await;
    let client = reqwest::Client::new();

    let res = client
        .get(format!("{}/api/health", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let health: HealthResponse = res.json().await.unwrap();
    assert_eq!(health.status, "ok");
    assert!(!health.version.is_empty());

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_web_api_cached_repositories() {
    let db_path = std::env::temp_dir().join(format!("web_test_cached_{}.db", rand_nanos()));
    let mut conn = open_db(Some(&db_path)).unwrap();

    let sample_skill = Skill {
        name: "test-skill".to_string(),
        description: "Test description.".to_string(),
        path: "SKILL.md".to_string(),
        url: "https://github.com/test-org/test-repo/blob/main/SKILL.md".to_string(),
    };

    save_cached_repository(
        &mut conn,
        "test-org",
        "test-repo",
        "main",
        Some("commit_sha_12345"),
        None,
        &[sample_skill],
    )
    .unwrap();

    let options = ScannerOptions {
        db_path: Some(db_path.clone()),
        ..Default::default()
    };

    let (base_url, _) = spawn_test_app(options).await;
    let client = reqwest::Client::new();

    let res = client
        .get(format!("{}/api/cached", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let repos: Vec<skill_atlas::storage::CachedRepo> = res.json().await.unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].owner, "test-org");
    assert_eq!(repos[0].repo, "test-repo");
    assert_eq!(repos[0].branch, "main");
    assert_eq!(repos[0].commit_sha.as_deref(), Some("commit_sha_12345"));

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_web_api_scan_post_and_get() {
    let mock_server = MockServer::start().await;

    // Mock commit SHA
    Mock::given(method("GET"))
        .and(path("/repos/acme/agent-tools/commits/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sha": "1234567890abcdef"
        })))
        .mount(&mock_server)
        .await;

    // Mock Git Tree
    Mock::given(method("GET"))
        .and(path("/repos/acme/agent-tools/git/trees/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sha": "tree_sha",
            "truncated": false,
            "tree": [
                {
                    "path": "skills/code-review/SKILL.md",
                    "mode": "100644",
                    "type": "blob",
                    "sha": "blob_review",
                    "size": 120
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    // Mock raw skill file download
    Mock::given(method("GET"))
        .and(path(
            "/acme/agent-tools/1234567890abcdef/skills/code-review/SKILL.md",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "---\nname: code-review\ndescription: Automated code review assistant.\n---\n# Review",
        ))
        .mount(&mock_server)
        .await;

    let db_path = std::env::temp_dir().join(format!("web_test_scan_{}.db", rand_nanos()));
    let options = ScannerOptions {
        base_api_url: Some(mock_server.uri()),
        base_raw_url: Some(mock_server.uri()),
        db_path: Some(db_path.clone()),
        ..Default::default()
    };

    let (base_url, _) = spawn_test_app(options).await;
    let client = reqwest::Client::new();

    // 1. Test POST /api/scan
    let post_res = client
        .post(format!("{}/api/scan", base_url))
        .json(&json!({
            "repo": "acme/agent-tools"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(post_res.status(), reqwest::StatusCode::OK);
    let result: ScanResult = post_res.json().await.unwrap();
    assert_eq!(result.owner, "acme");
    assert_eq!(result.repo, "agent-tools");
    assert_eq!(result.skills.len(), 1);
    assert_eq!(result.skills[0].name, "code-review");
    assert_eq!(
        result.skills[0].description,
        "Automated code review assistant."
    );
    assert!(!result.from_cache);

    // 2. Test GET /api/scan (cached scan)
    let get_res = client
        .get(format!("{}/api/scan?repo=acme/agent-tools", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(get_res.status(), reqwest::StatusCode::OK);
    let cached_result: ScanResult = get_res.json().await.unwrap();
    assert_eq!(cached_result.skills.len(), 1);
    assert!(cached_result.from_cache);

    // 3. Test force refresh
    let refresh_res = client
        .post(format!("{}/api/scan", base_url))
        .json(&json!({
            "repo": "acme/agent-tools",
            "refresh": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(refresh_res.status(), reqwest::StatusCode::OK);
    let refreshed_result: ScanResult = refresh_res.json().await.unwrap();
    assert!(!refreshed_result.from_cache);

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_web_api_scan_errors() {
    let mock_server = MockServer::start().await;

    // 404 repo
    Mock::given(method("GET"))
        .and(path("/repos/acme/nonexistent/commits/HEAD"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "message": "Not Found"
        })))
        .mount(&mock_server)
        .await;

    let db_path = std::env::temp_dir().join(format!("web_test_err_{}.db", rand_nanos()));
    let options = ScannerOptions {
        base_api_url: Some(mock_server.uri()),
        base_raw_url: Some(mock_server.uri()),
        db_path: Some(db_path.clone()),
        ..Default::default()
    };

    let (base_url, _) = spawn_test_app(options).await;
    let client = reqwest::Client::new();

    // 1. Empty repository -> 400
    let empty_res = client
        .post(format!("{}/api/scan", base_url))
        .json(&json!({ "repo": "   " }))
        .send()
        .await
        .unwrap();
    assert_eq!(empty_res.status(), reqwest::StatusCode::BAD_REQUEST);

    // 2. Invalid repository format -> 400
    let invalid_res = client
        .post(format!("{}/api/scan", base_url))
        .json(&json!({ "repo": "justaname" }))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid_res.status(), reqwest::StatusCode::BAD_REQUEST);

    // 3. Not found repository -> 404
    let not_found_res = client
        .post(format!("{}/api/scan", base_url))
        .json(&json!({ "repo": "acme/nonexistent" }))
        .send()
        .await
        .unwrap();
    assert_eq!(not_found_res.status(), reqwest::StatusCode::NOT_FOUND);
    let err: ErrorResponse = not_found_res.json().await.unwrap();
    assert!(err.error.contains("not found"));

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_web_api_scan_with_filter_and_similar() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/acme/skills-hub"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "default_branch": "main"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/repos/acme/skills-hub/commits/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sha": "sha_web_sim"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/repos/acme/skills-hub/git/trees/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sha": "tree_sim",
            "tree": [
                { "path": "skills/review-a/SKILL.md", "type": "blob", "sha": "blob1" },
                { "path": "skills/deploy/skill.yaml", "type": "blob", "sha": "blob2" },
                { "path": "skills/review-b/SKILL.md", "type": "blob", "sha": "blob3" }
            ]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/acme/skills-hub/sha_web_sim/skills/review-a/SKILL.md"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "---\nname: code-review\ndescription: Automated code review.\n---\n",
        ))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/acme/skills-hub/sha_web_sim/skills/deploy/skill.yaml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "name: deploy-prod\ndescription: Production deployment.\n",
        ))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/acme/skills-hub/sha_web_sim/skills/review-b/SKILL.md"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "---\nname: code-review\ndescription: Automated code review.\n---\n",
        ))
        .mount(&mock_server)
        .await;

    let db_path = std::env::temp_dir().join(format!("web_test_sim_{}.db", rand_nanos()));
    let options = ScannerOptions {
        base_api_url: Some(mock_server.uri()),
        base_raw_url: Some(mock_server.uri()),
        db_path: Some(db_path.clone()),
        ..Default::default()
    };

    let (base_url, _) = spawn_test_app(options).await;
    let client = reqwest::Client::new();

    // 1. Scan with filter parameter
    let filter_res = client
        .post(format!("{}/api/scan", base_url))
        .json(&json!({
            "repo": "acme/skills-hub",
            "filter": "deploy"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(filter_res.status(), reqwest::StatusCode::OK);
    let filter_result: ScanResult = filter_res.json().await.unwrap();
    assert_eq!(filter_result.skills.len(), 1);
    assert_eq!(filter_result.skills[0].name, "deploy-prod");

    // 2. GET /api/similar endpoint
    let similar_res = client
        .get(format!("{}/api/similar?repo=acme/skills-hub", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(similar_res.status(), reqwest::StatusCode::OK);
    let similar_data: skill_atlas::web::SimilarResponse = similar_res.json().await.unwrap();
    assert_eq!(similar_data.pairs.len(), 1);
    assert_eq!(similar_data.pairs[0].skill_a.name, "code-review");
    assert_eq!(similar_data.pairs[0].skill_b.name, "code-review");
    assert_eq!(similar_data.pairs[0].similarity, 100.0);

    // 3. POST /api/similar with target
    let target_res = client
        .post(format!("{}/api/similar", base_url))
        .json(&json!({
            "repo": "acme/skills-hub",
            "target": "code-review"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(target_res.status(), reqwest::StatusCode::OK);
    let target_data: skill_atlas::web::SimilarResponse = target_res.json().await.unwrap();
    assert_eq!(target_data.target_matches.len(), 1);
    assert_eq!(target_data.target_matches[0].skill.name, "code-review");

    let _ = std::fs::remove_file(db_path);
}
