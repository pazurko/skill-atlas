use rusqlite::Connection;
use skill_atlas::scanner::Skill;
use skill_atlas::storage::{
    clear_cache, get_all_cached_skills, get_cached_repository, get_starred_skills, init_db,
    is_skill_starred, list_all_cached_repositories, save_cached_repository, set_skill_starred,
    toggle_skill_starred,
};

#[test]
fn test_storage_init_and_empty() {
    let conn = Connection::open_in_memory().unwrap();
    init_db(&conn).unwrap();

    let cached = get_cached_repository(&conn, "openai", "swarm", "HEAD").unwrap();
    assert_eq!(cached, None);

    let list = list_all_cached_repositories(&conn).unwrap();
    assert!(list.is_empty());

    let all_skills = get_all_cached_skills(&conn).unwrap();
    assert!(all_skills.is_empty());

    let starred = get_starred_skills(&conn).unwrap();
    assert!(starred.is_empty());
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
            starred: false,
        },
        Skill {
            name: "search-tool".to_string(),
            description: "Searches web endpoints".to_string(),
            path: "skills/search/skill.json".to_string(),
            url: "https://github.com/openai/swarm/blob/main/skills/search/skill.json".to_string(),
            starred: false,
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
        starred: false,
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
            starred: false,
        },
        Skill {
            name: "v2-skill-b".to_string(),
            description: "Version 2 skill B".to_string(),
            path: "skills/v2b/SKILL.md".to_string(),
            url: "https://github.com/test/repo/blob/main/skills/v2b/SKILL.md".to_string(),
            starred: false,
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
        starred: false,
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

#[test]
fn test_storage_get_all_cached_skills_multiple_repos() {
    let mut conn = Connection::open_in_memory().unwrap();
    init_db(&conn).unwrap();

    let repo1_skills = vec![Skill {
        name: "b-skill".to_string(),
        description: "B Skill".to_string(),
        path: "skills/b/SKILL.md".to_string(),
        url: "https://github.com/org/repo1/blob/main/skills/b/SKILL.md".to_string(),
        starred: false,
    }];

    let repo2_skills = vec![Skill {
        name: "a-skill".to_string(),
        description: "A Skill".to_string(),
        path: "skills/a/SKILL.md".to_string(),
        url: "https://github.com/org/repo2/blob/main/skills/a/SKILL.md".to_string(),
        starred: false,
    }];

    save_cached_repository(
        &mut conn,
        "org",
        "repo1",
        "main",
        Some("sha1"),
        None,
        &repo1_skills,
    )
    .unwrap();
    save_cached_repository(
        &mut conn,
        "org",
        "repo2",
        "main",
        Some("sha2"),
        None,
        &repo2_skills,
    )
    .unwrap();

    let all_skills = get_all_cached_skills(&conn).unwrap();
    assert_eq!(all_skills.len(), 2);
    assert_eq!(all_skills[0].name, "b-skill");
    assert_eq!(all_skills[1].name, "a-skill");
}

#[test]
fn test_storage_star_and_unstar_skills() {
    let mut conn = Connection::open_in_memory().unwrap();
    init_db(&conn).unwrap();

    let skill = Skill {
        name: "test-star".to_string(),
        description: "Star test".to_string(),
        path: "skills/star/SKILL.md".to_string(),
        url: "https://github.com/org/repo/blob/main/skills/star/SKILL.md".to_string(),
        starred: false,
    };

    assert!(!is_skill_starred(&conn, &skill.url).unwrap());

    // Toggle to starred
    let is_starred = toggle_skill_starred(&conn, &skill).unwrap();
    assert!(is_starred);
    assert!(is_skill_starred(&conn, &skill.url).unwrap());

    let starred_list = get_starred_skills(&conn).unwrap();
    assert_eq!(starred_list.len(), 1);
    assert_eq!(starred_list[0].name, "test-star");
    assert!(starred_list[0].starred);

    // Save repository cache and check that cached repository retrieves it as starred
    save_cached_repository(
        &mut conn,
        "org",
        "repo",
        "main",
        Some("commit_sha"),
        None,
        std::slice::from_ref(&skill),
    )
    .unwrap();

    let cached = get_cached_repository(&conn, "org", "repo", "main")
        .unwrap()
        .unwrap();
    assert!(cached.1[0].starred);

    // Unstar
    set_skill_starred(&conn, &skill, false).unwrap();
    assert!(!is_skill_starred(&conn, &skill.url).unwrap());
    assert!(get_starred_skills(&conn).unwrap().is_empty());
}
