use crossterm::event::{KeyCode, KeyModifiers};
use skill_atlas::interactive::{handle_menu_key, present_skills, InteractiveResult, MenuAction};
use skill_atlas::scanner::Skill;

#[test]
fn test_present_skills_empty_list() {
    let skills: Vec<Skill> = vec![];
    let result = present_skills(&skills, Some("owner/repo"), false).unwrap();
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
    let result = present_skills(&skills, Some("owner/repo"), false).unwrap();
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
