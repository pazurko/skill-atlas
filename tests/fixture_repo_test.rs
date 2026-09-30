//! End-to-end scan of a realistic fixture repository (`tests/fixtures/sample-repo`).
//!
//! `tree.json` is what the GitHub Trees API would return; `files/` holds the raw contents
//! served by the fake `raw.githubusercontent.com`. The fixture covers plugin-shipped skills,
//! duplicate skill names, unusual folder names (spaces, unicode, mixed-case file names),
//! lookalikes of ignored folders, vendored decoys, supporting documents and a folder that is
//! itself named `SKILL.md`.

use serde_json::Value;
use skill_atlas::scanner::{scan_github_repo, ScannerOptions};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

const SHA: &str = "sha-fixture";

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample-repo")
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).expect("valid utf-8 path")
}

/// Serves `files/<path>` for raw-content requests and records every requested path.
struct RawFiles {
    prefix: String,
    requested: Arc<Mutex<Vec<String>>>,
}

impl Respond for RawFiles {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let decoded = percent_decode(request.url.path());
        let Some(rel) = decoded.strip_prefix(&self.prefix) else {
            return ResponseTemplate::new(404);
        };
        self.requested.lock().unwrap().push(rel.to_string());
        match std::fs::read_to_string(fixture_root().join("files").join(rel)) {
            Ok(body) => ResponseTemplate::new(200).set_body_string(body),
            Err(_) => ResponseTemplate::new(404),
        }
    }
}

async fn mount_fixture(server: &MockServer) -> Arc<Mutex<Vec<String>>> {
    let tree: Value =
        serde_json::from_str(&std::fs::read_to_string(fixture_root().join("tree.json")).unwrap())
            .unwrap();
    Mock::given(method("GET"))
        .and(path("/repos/acme/sample"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "default_branch": "master" })),
        )
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/sample/commits/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "sha": SHA })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/sample/git/trees/HEAD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tree))
        .mount(server)
        .await;
    let requested = Arc::new(Mutex::new(Vec::new()));
    Mock::given(method("GET"))
        .and(path_regex(format!("^/acme/sample/{}/", SHA)))
        .respond_with(RawFiles {
            prefix: format!("/acme/sample/{}/", SHA),
            requested: Arc::clone(&requested),
        })
        .mount(server)
        .await;
    requested
}

fn options(server: &MockServer) -> ScannerOptions {
    ScannerOptions {
        token: None,
        branch: None,
        base_api_url: Some(server.uri()),
        base_raw_url: Some(server.uri()),
        no_cache: true,
        db_path: Some(std::env::temp_dir().join(format!(
            "fixture_repo_{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))),
    }
}

#[tokio::test]
async fn test_fixture_repo_finds_exactly_the_real_skills() {
    let server = MockServer::start().await;
    let requested = mount_fixture(&server).await;
    let opts = options(&server);

    let result = scan_github_repo("acme/sample", &opts).await.unwrap();
    let _ = std::fs::remove_file(opts.db_path.unwrap());

    let found: Vec<(&str, &str)> = result
        .skills
        .iter()
        .map(|s| (s.name.as_str(), s.path.as_str()))
        .collect();
    assert_eq!(
        found,
        vec![
            // Duplicate names are both kept, ordered by path.
            ("code-review", ".claude/skills/code-review/SKILL.md"),
            (
                "code-review",
                "plugins/mcp-tools/resources/jetbrains/agents/mcp/skills/code-review/SKILL.md"
            ),
            // Folder with a space, mixed-case file name.
            ("data-sync", ".agents/skills/data sync/Skill.md"),
            ("deploy", ".junie/skills/deploy/skill.yaml"),
            ("Heading Only Skill", "skills/no-frontmatter/SKILL.md"),
            // A folder that only looks like an ignored one is scanned.
            ("lint", "my-venv-tools/skills/lint/skill.json"),
            ("root-skill", "SKILL.md"),
            // Shipped inside a plugin's resources, 9 levels deep.
            (
                "run-tests",
                "plugins/mcp-tools/resources/jetbrains/agents/mcp/skills/run-tests/SKILL.md"
            ),
            ("übersetzen", "docs/ünïcode-skills/übersetzen/SKILL.md"),
        ]
    );
    assert!(!result.truncated);

    // Only the real skill files were downloaded: no supporting docs, no vendored decoys,
    // no tree entries, each file exactly once.
    let mut requested = requested.lock().unwrap().clone();
    requested.sort();
    let mut expected: Vec<String> = result.skills.iter().map(|s| s.path.clone()).collect();
    expected.sort();
    assert_eq!(requested, expected);

    // Descriptions come from each format, links use the real default branch.
    let plugin_review = &result.skills[1];
    assert_eq!(
        plugin_review.description,
        "Plugin-shipped review skill bundled in the MCP tools plugin."
    );
    assert_eq!(
        result.skills[0].description,
        "Reviews a change set for bugs."
    );
    assert_eq!(
        result.skills[3].description,
        "Deploys the service to staging."
    );
    assert_eq!(result.skills[5].description, "Lints the code base.");
    for skill in &result.skills {
        assert!(
            skill
                .url
                .starts_with("https://github.com/acme/sample/blob/master/"),
            "unexpected url {}",
            skill.url
        );
    }
}
