use skill_atlas::interactive::{present_skills, InteractiveResult};
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
