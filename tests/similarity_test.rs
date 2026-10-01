use skill_atlas::scanner::Skill;
use skill_atlas::similarity::{
    calculate_similarity, dice_bigram_similarity, filter_skills, find_all_similar_pairs,
    find_similar_skills, jaccard_similarity, tokenize, DEFAULT_SIMILARITY_THRESHOLD,
};
use std::collections::HashSet;

#[test]
fn test_tokenize_filters_stop_words_and_punctuation() {
    let text = "The quick, brown fox jumps over the lazy dog!";
    let tokens_filtered = tokenize(text, true);
    assert!(tokens_filtered.contains("quick"));
    assert!(tokens_filtered.contains("brown"));
    assert!(tokens_filtered.contains("fox"));
    assert!(tokens_filtered.contains("jumps"));
    assert!(tokens_filtered.contains("lazy"));
    assert!(tokens_filtered.contains("dog"));
    // Stop words should be filtered
    assert!(!tokens_filtered.contains("the"));
    assert!(!tokens_filtered.contains("over"));

    let tokens_unfiltered = tokenize(text, false);
    assert!(tokens_unfiltered.contains("the"));
    assert!(tokens_unfiltered.contains("over"));
}

#[test]
fn test_jaccard_similarity_edge_cases() {
    let empty: HashSet<String> = HashSet::new();
    assert_eq!(jaccard_similarity(&empty, &empty), 0.0);

    let mut set_a = HashSet::new();
    set_a.insert("code".to_string());
    set_a.insert("review".to_string());

    let mut set_b = HashSet::new();
    set_b.insert("code".to_string());
    set_b.insert("review".to_string());

    assert_eq!(jaccard_similarity(&set_a, &set_b), 1.0);

    let mut set_c = HashSet::new();
    set_c.insert("deploy".to_string());

    assert_eq!(jaccard_similarity(&set_a, &set_c), 0.0);
}

#[test]
fn test_dice_bigram_similarity() {
    assert_eq!(dice_bigram_similarity("review", "review"), 1.0);
    assert_eq!(dice_bigram_similarity("", ""), 0.0);
    assert_eq!(dice_bigram_similarity("a", "b"), 0.0);

    let sim = dice_bigram_similarity("code-review", "code-reviewer");
    assert!(sim > 0.8, "Expected high bigram similarity for reviewer");
}

#[test]
fn test_calculate_similarity_identical_and_duplicates() {
    let skill_a = Skill {
        name: "code-review".to_string(),
        description: "Automated code review and quality checks.".to_string(),
        path: ".claude/skills/code-review/SKILL.md".to_string(),
        url: "https://github.com/org/repo/blob/main/.claude/skills/code-review/SKILL.md".to_string(),
    };

    let skill_b = Skill {
        name: "code-review".to_string(),
        description: "Automated code review and quality checks.".to_string(),
        path: "plugins/mcp/skills/code-review/SKILL.md".to_string(),
        url: "https://github.com/org/repo/blob/main/plugins/mcp/skills/code-review/SKILL.md"
            .to_string(),
    };

    let sim = calculate_similarity(&skill_a, &skill_b);
    assert_eq!(sim, 100.0);
}

#[test]
fn test_calculate_similarity_high_relatedness() {
    let skill_a = Skill {
        name: "build-bump-gradle-version".to_string(),
        description: "Bumps the Gradle wrapper version used to build the project.".to_string(),
        path: "skills/build-bump-gradle-version/SKILL.md".to_string(),
        url: "https://github.com/org/repo/blob/main/skills/build-bump-gradle-version/SKILL.md"
            .to_string(),
    };

    let skill_b = Skill {
        name: "build-tools-bump-gradle-api".to_string(),
        description: "Bumps the Gradle API version for plugins compilation.".to_string(),
        path: "skills/build-tools-bump-gradle-api/SKILL.md".to_string(),
        url: "https://github.com/org/repo/blob/main/skills/build-tools-bump-gradle-api/SKILL.md"
            .to_string(),
    };

    let sim = calculate_similarity(&skill_a, &skill_b);
    assert!(
        sim >= 40.0,
        "Expected Gradle bumping skills to have significant similarity, got {}",
        sim
    );
}

#[test]
fn test_calculate_similarity_unrelated() {
    let skill_a = Skill {
        name: "code-review".to_string(),
        description: "Perform git pull request code reviews.".to_string(),
        path: "skills/code-review/SKILL.md".to_string(),
        url: "https://github.com/org/repo/blob/main/skills/code-review/SKILL.md".to_string(),
    };

    let skill_b = Skill {
        name: "k8s-cluster-provision".to_string(),
        description: "Provision AWS EKS kubernetes cluster with terraform.".to_string(),
        path: "infra/k8s/skill.yaml".to_string(),
        url: "https://github.com/org/repo/blob/main/infra/k8s/skill.yaml".to_string(),
    };

    let sim = calculate_similarity(&skill_a, &skill_b);
    assert!(
        sim < DEFAULT_SIMILARITY_THRESHOLD,
        "Expected unrelated skills to be below threshold, got {}",
        sim
    );
}

#[test]
fn test_find_similar_skills_and_pairs() {
    let skills = vec![
        Skill {
            name: "code-review".to_string(),
            description: "Review pull requests and code changes.".to_string(),
            path: ".agents/skills/code-review/SKILL.md".to_string(),
            url: "https://github.com/o/r/blob/m/.agents/skills/code-review/SKILL.md".to_string(),
        },
        Skill {
            name: "deploy-k8s".to_string(),
            description: "Deploy services to kubernetes cluster.".to_string(),
            path: ".agents/skills/deploy-k8s/SKILL.md".to_string(),
            url: "https://github.com/o/r/blob/m/.agents/skills/deploy-k8s/SKILL.md".to_string(),
        },
        Skill {
            name: "code-review".to_string(),
            description: "Review pull requests and code changes in plugins.".to_string(),
            path: "plugins/skills/code-review/SKILL.md".to_string(),
            url: "https://github.com/o/r/blob/m/plugins/skills/code-review/SKILL.md".to_string(),
        },
    ];

    let matches = find_similar_skills(&skills[0], &skills, DEFAULT_SIMILARITY_THRESHOLD);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].index, 3);
    assert_eq!(matches[0].skill.name, "code-review");
    assert!(matches[0].similarity >= 90.0);

    let pairs = find_all_similar_pairs(&skills, DEFAULT_SIMILARITY_THRESHOLD);
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].index_a, 1);
    assert_eq!(pairs[0].index_b, 3);
}

#[test]
fn test_filter_skills() {
    let skills = vec![
        Skill {
            name: "build-bump-gradle".to_string(),
            description: "Bumps gradle wrapper version.".to_string(),
            path: "skills/gradle/SKILL.md".to_string(),
            url: "https://github.com/o/r/blob/m/skills/gradle/SKILL.md".to_string(),
        },
        Skill {
            name: "code-review".to_string(),
            description: "Review pull requests and KT fixes.".to_string(),
            path: ".claude/skills/review/SKILL.md".to_string(),
            url: "https://github.com/o/r/blob/m/.claude/skills/review/SKILL.md".to_string(),
        },
        Skill {
            name: "deploy-app".to_string(),
            description: "Deploy to cloud production.".to_string(),
            path: "deploy/skill.yaml".to_string(),
            url: "https://github.com/o/r/blob/m/deploy/skill.yaml".to_string(),
        },
    ];

    // Empty query returns all
    assert_eq!(filter_skills(&skills, "").len(), 3);
    assert_eq!(filter_skills(&skills, "   ").len(), 3);

    // Filter by name
    let filtered_name = filter_skills(&skills, "review");
    assert_eq!(filtered_name.len(), 1);
    assert_eq!(filtered_name[0].name, "code-review");

    // Filter by description
    let filtered_desc = filter_skills(&skills, "wrapper");
    assert_eq!(filtered_desc.len(), 1);
    assert_eq!(filtered_desc[0].name, "build-bump-gradle");

    // Filter by path
    let filtered_path = filter_skills(&skills, "yaml");
    assert_eq!(filtered_path.len(), 1);
    assert_eq!(filtered_path[0].name, "deploy-app");

    // Multi-term filter
    let multi = filter_skills(&skills, "bump gradle");
    assert_eq!(multi.len(), 1);
    assert_eq!(multi[0].name, "build-bump-gradle");

    // No match
    let nomatch = filter_skills(&skills, "nonexistent");
    assert!(nomatch.is_empty());
}
