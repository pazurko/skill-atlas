use skill_atlas::metadata::{
    clean_markdown, extract_first_sentence, infer_name_from_path, parse_skill_metadata,
    SkillMetadata,
};

#[test]
fn test_yaml_frontmatter_extraction() {
    let content = r#"---
name: Weather Forecast
description: Fetches real-time weather information for any city
---
# Additional Notes
Some body text
"#;
    let meta = parse_skill_metadata(content, "skills/weather/SKILL.md");
    assert_eq!(
        meta,
        SkillMetadata {
            name: "Weather Forecast".to_string(),
            description: "Fetches real-time weather information for any city".to_string(),
        }
    );
}

#[test]
fn test_yaml_frontmatter_quoted_values() {
    let content = r#"---
name: "Code Reviewer"
description: 'Automated code review assistant'
---
"#;
    let meta = parse_skill_metadata(content, "skills/review/SKILL.md");
    assert_eq!(
        meta,
        SkillMetadata {
            name: "Code Reviewer".to_string(),
            description: "Automated code review assistant".to_string(),
        }
    );
}

#[test]
fn test_markdown_heading_extraction() {
    let content = r#"# Database Migrator

A skill that automates SQL migrations across PostgreSQL environments.

## Details
More info here.
"#;
    let meta = parse_skill_metadata(content, "skills/db-migrator/SKILL.md");
    assert_eq!(
        meta,
        SkillMetadata {
            name: "Database Migrator".to_string(),
            description: "A skill that automates SQL migrations across PostgreSQL environments."
                .to_string(),
        }
    );
}

#[test]
fn test_markdown_heading_with_formatting_stripping() {
    let content = r#"# **Search** `Skill`

Enables agent to search with [Google](https://google.com) and **Bing** APIs.
"#;
    let meta = parse_skill_metadata(content, "skills/search/SKILL.md");
    assert_eq!(
        meta,
        SkillMetadata {
            name: "Search Skill".to_string(),
            description: "Enables agent to search with Google and Bing APIs.".to_string(),
        }
    );
}

#[test]
fn test_json_skill_extraction() {
    let content = r#"{
  "name": "Translation Service",
  "description": "Translates text between 50 languages"
}"#;
    let meta = parse_skill_metadata(content, "skills/translate/skill.json");
    assert_eq!(
        meta,
        SkillMetadata {
            name: "Translation Service".to_string(),
            description: "Translates text between 50 languages".to_string(),
        }
    );
}

#[test]
fn test_yaml_skill_file_extraction() {
    let content = r#"
name: Sentiment Analyzer
summary: Evaluates sentiment and emotion in customer reviews.
"#;
    let meta = parse_skill_metadata(content, "skills/sentiment/skill.yaml");
    assert_eq!(
        meta,
        SkillMetadata {
            name: "Sentiment Analyzer".to_string(),
            description: "Evaluates sentiment and emotion in customer reviews.".to_string(),
        }
    );
}

#[test]
fn test_frontmatter_name_with_markdown_body_description() {
    let content = r#"---
name: Cloud Deployer
---
Deploys containers to AWS ECS clusters and manages autoscaling.
"#;
    let meta = parse_skill_metadata(content, "skills/deploy/SKILL.md");
    assert_eq!(
        meta,
        SkillMetadata {
            name: "Cloud Deployer".to_string(),
            description: "Deploys containers to AWS ECS clusters and manages autoscaling."
                .to_string(),
        }
    );
}

#[test]
fn test_fallback_metadata_from_parent_folder() {
    let content = "";
    let meta = parse_skill_metadata(content, "skills/summarizer/SKILL.md");
    assert_eq!(
        meta,
        SkillMetadata {
            name: "summarizer".to_string(),
            description: "No description provided.".to_string(),
        }
    );
}

#[test]
fn test_infer_name_from_path_variations() {
    assert_eq!(infer_name_from_path("skills/search/SKILL.md"), "search");
    assert_eq!(
        infer_name_from_path(".agents/skills/pdf-parser.json"),
        "pdf-parser"
    );
    assert_eq!(infer_name_from_path("skill.yaml"), "skill");
}

#[test]
fn test_clean_markdown_utility() {
    assert_eq!(
        clean_markdown("This is **bold** and `code` with [link](https://example.com)"),
        "This is bold and code with link"
    );
}

#[test]
fn test_extract_first_sentence_variations() {
    // Single sentence with period
    assert_eq!(
        extract_first_sentence(
            "Fetches real-time weather information for any city. Supports 100+ countries."
        ),
        "Fetches real-time weather information for any city."
    );

    // Single sentence with exclamation mark
    assert_eq!(
        extract_first_sentence(
            "Translates text between 50 languages! It is very fast and reliable."
        ),
        "Translates text between 50 languages!"
    );

    // Single sentence with question mark
    assert_eq!(
        extract_first_sentence(
            "Need automated SQL migrations? This skill handles it automatically. Read more below."
        ),
        "Need automated SQL migrations?"
    );

    // Abbreviations should not split sentence prematurely
    assert_eq!(
        extract_first_sentence(
            "Supports cloud providers (e.g. AWS, GCP). Configure your credentials in .env."
        ),
        "Supports cloud providers (e.g. AWS, GCP)."
    );

    // No trailing punctuation
    assert_eq!(
        extract_first_sentence("Simple one line summary"),
        "Simple one line summary"
    );
}

#[test]
fn test_multi_sentence_markdown_heading_extraction() {
    let content = r#"# Database Migrator

A skill that automates SQL migrations across PostgreSQL environments. It also checks schema health and rollbacks.

## Usage
Run migrations easily.
"#;
    let meta = parse_skill_metadata(content, "skills/db-migrator/SKILL.md");
    assert_eq!(
        meta,
        SkillMetadata {
            name: "Database Migrator".to_string(),
            description: "A skill that automates SQL migrations across PostgreSQL environments."
                .to_string(),
        }
    );
}

#[test]
fn test_multi_sentence_json_extraction() {
    let content = r#"{
  "name": "Translation Service",
  "description": "Translates text between 50 languages. Highly optimized for speed and accuracy."
}"#;
    let meta = parse_skill_metadata(content, "skills/translate/skill.json");
    assert_eq!(
        meta,
        SkillMetadata {
            name: "Translation Service".to_string(),
            description: "Translates text between 50 languages.".to_string(),
        }
    );
}
