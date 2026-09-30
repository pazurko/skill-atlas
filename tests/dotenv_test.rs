use skill_atlas::dotenv::{load_dotenv, parse_dotenv};

#[test]
fn test_parse_dotenv_handles_comments_export_and_quotes() {
    let content = "\n# comment\nGITHUB_TOKEN=abc123\nexport A_KEY = \"quoted value\"\nB_KEY='single'\nNO_EQUALS_LINE\nEMPTY=\n=novalue\nURL=https://x.y/?a=b\n";
    let pairs = parse_dotenv(content);
    assert_eq!(
        pairs,
        vec![
            ("GITHUB_TOKEN".to_string(), "abc123".to_string()),
            ("A_KEY".to_string(), "quoted value".to_string()),
            ("B_KEY".to_string(), "single".to_string()),
            ("URL".to_string(), "https://x.y/?a=b".to_string()),
        ]
    );
}

#[test]
fn test_load_dotenv_sets_missing_vars_without_overriding_existing() {
    let dir = std::env::temp_dir().join(format!(
        "skill_atlas_dotenv_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(".env");
    std::fs::write(
        &path,
        "SKILL_ATLAS_TEST_NEW=from_file\nSKILL_ATLAS_TEST_EXISTING=from_file\n",
    )
    .unwrap();
    std::env::set_var("SKILL_ATLAS_TEST_EXISTING", "from_env");
    std::env::remove_var("SKILL_ATLAS_TEST_NEW");

    let loaded = load_dotenv(&path);

    assert_eq!(loaded, vec!["SKILL_ATLAS_TEST_NEW".to_string()]);
    assert_eq!(std::env::var("SKILL_ATLAS_TEST_NEW").unwrap(), "from_file");
    assert_eq!(
        std::env::var("SKILL_ATLAS_TEST_EXISTING").unwrap(),
        "from_env"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_load_dotenv_missing_file_is_ignored() {
    let loaded = load_dotenv(std::path::Path::new("/nonexistent/skill-atlas/.env"));
    assert!(loaded.is_empty());
}
