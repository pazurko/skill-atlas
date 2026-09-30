use rusqlite::Connection;
use skill_atlas::scanner::Skill;
use skill_atlas::storage::{
    clear_cache, get_cached_repository, init_db, list_all_cached_repositories,
    save_cached_repository,
};

#[test]
fn test_storage_init_and_empty() {
    let conn = Connection::open_in_memory().unwrap();
    init_db(&conn).unwrap();

    let cached = get_cached_repository(&conn, "openai", "swarm", "HEAD").unwrap();
    assert_eq!(cached, None);

    let list = list_all_cached_repositories(&conn).unwrap();
    assert!(list.is_empty());
}

#[test]
fn test_storage_save_and_retrieve() {
    let mut conn = Connection::open_in_memory().unwrap();
    init_db(&conn).unwrap();

    let skills = vec![
        Skill {
            name: "weather-tool".to_string(),
            description: "Fetches weather forecasts".to_string(),
            path: "skills/weather/SKILL.md".to_string(),
            url: "https://github.com/openai/swarm/blob/main/skills/weather/SKILL.md".to_string(),
        },
        Skill {
            name: "search-tool".to_string(),
            description: "Searches web endpoints".to_string(),
            path: "skills/search/skill.json".to_string(),
            url: "https://github.com/openai/swarm/blob/main/skills/search/skill.json".to_string(),
        },
    ];

    save_cached_repository(
        &mut conn,
        "openai",
        "swarm",
        "main",
        Some("abc1234567890"),
        Some("etag_xyz"),
        &skills,
    )
    .unwrap();

    let cached = get_cached_repository(&conn, "openai", "swarm", "main")
        .unwrap()
        .expect("Expected cached repo");

    assert_eq!(cached.0.owner, "openai");
    assert_eq!(cached.0.repo, "swarm");
    assert_eq!(cached.0.branch, "main");
    assert_eq!(cached.0.commit_sha, Some("abc1234567890".to_string()));
    assert_eq!(cached.1.len(), 2);
    assert_eq!(cached.1[0].name, "search-tool"); // ordered by name
    assert_eq!(cached.1[1].name, "weather-tool");
}

#[test]
fn test_storage_update_upsert() {
    let mut conn = Connection::open_in_memory().unwrap();
    init_db(&conn).unwrap();

    let initial_skills = vec![Skill {
        name: "v1-skill".to_string(),
        description: "Version 1 skill".to_string(),
        path: "skills/v1/SKILL.md".to_string(),
        url: "https://github.com/test/repo/blob/main/skills/v1/SKILL.md".to_string(),
    }];

    save_cached_repository(
        &mut conn,
        "test",
        "repo",
        "main",
        Some("commit_v1"),
        None,
        &initial_skills,
    )
    .unwrap();

    let updated_skills = vec![
        Skill {
            name: "v2-skill-a".to_string(),
            description: "Version 2 skill A".to_string(),
            path: "skills/v2a/SKILL.md".to_string(),
            url: "https://github.com/test/repo/blob/main/skills/v2a/SKILL.md".to_string(),
        },
        Skill {
            name: "v2-skill-b".to_string(),
            description: "Version 2 skill B".to_string(),
            path: "skills/v2b/SKILL.md".to_string(),
            url: "https://github.com/test/repo/blob/main/skills/v2b/SKILL.md".to_string(),
        },
    ];

    save_cached_repository(
        &mut conn,
        "test",
        "repo",
        "main",
        Some("commit_v2"),
        None,
        &updated_skills,
    )
    .unwrap();

    let cached = get_cached_repository(&conn, "test", "repo", "main")
        .unwrap()
        .unwrap();

    assert_eq!(cached.0.commit_sha, Some("commit_v2".to_string()));
    assert_eq!(cached.1.len(), 2);
    assert_eq!(cached.1[0].name, "v2-skill-a");
    assert_eq!(cached.1[1].name, "v2-skill-b");
}

#[test]
fn test_storage_clear_cache() {
    let mut conn = Connection::open_in_memory().unwrap();
    init_db(&conn).unwrap();

    let skills = vec![Skill {
        name: "test-skill".to_string(),
        description: "Test".to_string(),
        path: "skills/SKILL.md".to_string(),
        url: "https://github.com/a/b/blob/main/skills/SKILL.md".to_string(),
    }];

    save_cached_repository(&mut conn, "a", "b", "main", Some("sha123"), None, &skills).unwrap();

    assert!(get_cached_repository(&conn, "a", "b", "main")
        .unwrap()
        .is_some());

    clear_cache(&conn).unwrap();

    assert!(get_cached_repository(&conn, "a", "b", "main")
        .unwrap()
        .is_none());
}
