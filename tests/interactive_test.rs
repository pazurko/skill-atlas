use crossterm::event::{KeyCode, KeyModifiers};
use skill_atlas::history::OpenHistory;
use skill_atlas::interactive::{
    handle_menu_key, present_skills, render_menu, skill_badge_label, InteractiveResult, MenuAction,
    MenuState,
};
use skill_atlas::scanner::Skill;

#[test]
fn test_present_skills_empty_list() {
    let skills: Vec<Skill> = vec![];
    let result =
        present_skills(&skills, Some("owner/repo"), false, &mut OpenHistory::new()).unwrap();
    assert_eq!(result, InteractiveResult::Exited);
}

#[test]
fn test_present_skills_non_interactive_fallback() {
    let skills = vec![Skill {
        name: "Test Skill".to_string(),
        description: "Test description".to_string(),
        path: "skills/test/SKILL.md".to_string(),
        url: "https://github.com/owner/repo/blob/main/skills/test/SKILL.md".to_string(),
    }];

    // In non-interactive mode (e.g. CI / test harness), present_skills prints output and exits cleanly
    let result =
        present_skills(&skills, Some("owner/repo"), false, &mut OpenHistory::new()).unwrap();
    assert_eq!(result, InteractiveResult::Exited);
}

#[test]
fn test_menu_navigation_wraps() {
    let none = KeyModifiers::NONE;
    assert_eq!(
        handle_menu_key(KeyCode::Down, none, 0, 3),
        MenuAction::Move(1)
    );
    assert_eq!(
        handle_menu_key(KeyCode::Char('j'), none, 2, 3),
        MenuAction::Move(0)
    );
    assert_eq!(
        handle_menu_key(KeyCode::Up, none, 0, 3),
        MenuAction::Move(2)
    );
    assert_eq!(
        handle_menu_key(KeyCode::Char('k'), none, 2, 3),
        MenuAction::Move(1)
    );
}

#[test]
fn test_menu_enter_opens_and_stays() {
    let none = KeyModifiers::NONE;
    assert_eq!(
        handle_menu_key(KeyCode::Enter, none, 1, 3),
        MenuAction::Open(1)
    );
    // Opening does not leave the menu: navigation and opening another skill still work
    assert_eq!(
        handle_menu_key(KeyCode::Down, none, 1, 3),
        MenuAction::Move(2)
    );
    assert_eq!(
        handle_menu_key(KeyCode::Enter, none, 2, 3),
        MenuAction::Open(2)
    );
}

#[test]
fn test_menu_back_keys() {
    let none = KeyModifiers::NONE;
    assert_eq!(
        handle_menu_key(KeyCode::Char('q'), none, 0, 3),
        MenuAction::Back
    );
    assert_eq!(handle_menu_key(KeyCode::Esc, none, 0, 3), MenuAction::Back);
    assert_eq!(
        handle_menu_key(KeyCode::Char('c'), KeyModifiers::CONTROL, 0, 3),
        MenuAction::Back
    );
    assert_eq!(
        handle_menu_key(KeyCode::Char('x'), none, 0, 3),
        MenuAction::Ignore
    );
}

#[test]
fn test_menu_empty_list_is_safe() {
    let none = KeyModifiers::NONE;
    assert_eq!(
        handle_menu_key(KeyCode::Enter, none, 0, 0),
        MenuAction::Ignore
    );
    assert_eq!(
        handle_menu_key(KeyCode::Down, none, 0, 0),
        MenuAction::Ignore
    );
    assert_eq!(
        handle_menu_key(KeyCode::Char('q'), none, 0, 0),
        MenuAction::Back
    );
}

fn skill(name: &str, path: &str) -> Skill {
    Skill {
        name: name.to_string(),
        description: format!("{} description", name),
        path: path.to_string(),
        url: format!("https://github.com/owner/repo/blob/main/{}", path),
    }
}

fn render(skills: &[Skill], state: &MenuState, history: &OpenHistory) -> String {
    let mut out: Vec<u8> = Vec::new();
    render_menu(&mut out, skills, Some("owner/repo"), state, history).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn test_opened_status_and_history_survive_navigation() {
    let skills = vec![
        skill("alpha", "skills/alpha/SKILL.md"),
        skill("beta", "skills/beta/SKILL.md"),
    ];
    let mut history = OpenHistory::new();
    let mut state = MenuState::default();
    let mut opened_urls = Vec::new();
    let mut opener = |url: &str| {
        opened_urls.push(url.to_string());
        Ok(())
    };

    assert!(state.apply(
        MenuAction::Open(0),
        &skills,
        "owner/repo",
        &mut history,
        &mut opener
    ));
    // Moving with the arrows must not wipe the status line or the history.
    assert!(state.apply(
        MenuAction::Move(1),
        &skills,
        "owner/repo",
        &mut history,
        &mut opener
    ));
    assert!(state.apply(
        MenuAction::Move(0),
        &skills,
        "owner/repo",
        &mut history,
        &mut opener
    ));
    assert!(state.status.as_deref().unwrap().contains("Opened"));

    assert!(state.apply(
        MenuAction::Open(1),
        &skills,
        "owner/repo",
        &mut history,
        &mut opener
    ));
    assert!(state.apply(
        MenuAction::Open(0),
        &skills,
        "owner/repo",
        &mut history,
        &mut opener
    ));
    assert!(state.apply(
        MenuAction::Move(1),
        &skills,
        "owner/repo",
        &mut history,
        &mut opener
    ));
    assert!(!state.apply(
        MenuAction::Back,
        &skills,
        "owner/repo",
        &mut history,
        &mut opener
    ));

    assert_eq!(opened_urls.len(), 3);
    let names: Vec<&str> = history.records().iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "beta", "alpha"]);
    assert!(history
        .records()
        .iter()
        .all(|r| r.repo == "owner/repo" && r.error.is_none()));
    assert!(history.records().iter().all(|r| r.time.len() == 8));

    let frame = render(&skills, &state, &history);
    assert!(frame.contains("Opened in this session (3"));
    assert_eq!(frame.matches("✓ opened").count(), 2);
    assert!(frame.contains(&skills[1].url));
}

#[test]
fn test_failed_open_is_recorded_but_not_marked_opened() {
    let skills = vec![skill("alpha", "skills/alpha/SKILL.md")];
    let mut history = OpenHistory::new();
    let mut state = MenuState::default();
    state.apply(
        MenuAction::Open(0),
        &skills,
        "owner/repo",
        &mut history,
        &mut |_| Err(std::io::Error::other("no browser")),
    );
    assert_eq!(history.len(), 1);
    assert_eq!(history.records()[0].error.as_deref(), Some("no browser"));
    assert!(!history.was_opened(&skills[0].url));
    assert!(state
        .status
        .as_deref()
        .unwrap()
        .contains("Failed to open browser"));

    let frame = render(&skills, &state, &history);
    assert!(!frame.contains("✓ opened"));
    assert!(frame.contains("failed: no browser"));
}

#[test]
fn test_menu_shows_only_recent_history_lines() {
    let skills = vec![skill("alpha", "skills/alpha/SKILL.md")];
    let mut history = OpenHistory::new();
    let mut state = MenuState::default();
    for _ in 0..8 {
        state.apply(
            MenuAction::Open(0),
            &skills,
            "o/r",
            &mut history,
            &mut |_| Ok(()),
        );
    }
    assert_eq!(history.recent(5).len(), 5);
    let frame = render(&skills, &state, &history);
    assert!(frame.contains("Opened in this session (8"));
    assert_eq!(frame.matches("✓").count(), 1 + 5); // card mark + five history lines
}

#[test]
fn test_duplicate_skill_names_show_their_path() {
    let skills = vec![
        skill("review", ".claude/skills/review/SKILL.md"),
        skill("Review", "plugins/foo/resources/skills/review/SKILL.md"),
        skill("unique", "skills/unique/skill.yaml"),
    ];
    assert_eq!(
        skill_badge_label(&skills[0], &skills),
        ".claude/skills/review/SKILL.md"
    );
    assert_eq!(
        skill_badge_label(&skills[1], &skills),
        "plugins/foo/resources/skills/review/SKILL.md"
    );
    assert_eq!(skill_badge_label(&skills[2], &skills), "skill.yaml");
    let frame = render(&skills, &MenuState::default(), &OpenHistory::new());
    assert!(frame.contains("plugins/foo/resources/skills/review/SKILL.md ↗"));
    assert!(!frame.contains("Opened in this session"));
}
