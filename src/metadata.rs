use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
}

/// Parses metadata (name, description) from skill file content and path.
pub fn parse_skill_metadata(content: &str, file_path: &str) -> SkillMetadata {
    if content.trim().is_empty() {
        return infer_fallback_metadata(file_path);
    }

    // 1. JSON parsing
    if file_path.ends_with(".json") {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
            let name = value
                .get("name")
                .or_else(|| value.get("title"))
                .or_else(|| value.get("id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| infer_name_from_path(file_path));

            let description = value
                .get("description")
                .or_else(|| value.get("desc"))
                .or_else(|| value.get("summary"))
                .or_else(|| value.get("details"))
                .and_then(|v| v.as_str())
                .map(extract_first_sentence)
                .unwrap_or_else(|| "No description provided.".to_string());

            return SkillMetadata {
                name: clean_value(&name),
                description,
            };
        }
    }

    // 2. YAML frontmatter parsing (--- ... ---) or pure YAML file
    if file_path.ends_with(".yaml") || file_path.ends_with(".yml") {
        if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(content) {
            if let Some(meta) = extract_yaml_metadata(&value, file_path) {
                return meta;
            }
        }
    }

    let frontmatter_re = Regex::new(r"(?s)^---\r?\n(.*?)\r?\n---").unwrap();
    if let Some(caps) = frontmatter_re.captures(content) {
        let yaml_str = caps.get(1).unwrap().as_str();
        if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
            let name = value
                .get("name")
                .or_else(|| value.get("title"))
                .or_else(|| value.get("id"))
                .and_then(|v| v.as_str())
                .map(clean_value);

            let description = value
                .get("description")
                .or_else(|| value.get("desc"))
                .or_else(|| value.get("summary"))
                .or_else(|| value.get("details"))
                .and_then(|v| v.as_str())
                .map(extract_first_sentence);

            if let Some(n) = name {
                let desc = description.unwrap_or_else(|| {
                    // Try to extract description from the markdown body below frontmatter
                    let body = &content[caps.get(0).unwrap().end()..];
                    extract_description_from_markdown(body)
                });

                return SkillMetadata {
                    name: n,
                    description: desc,
                };
            }
        }
    }

    // 3. Markdown Heading 1 (# Title)
    let header_re = Regex::new(r"(?m)^#\s+(.+)$").unwrap();
    if let Some(caps) = header_re.captures(content) {
        let name = caps.get(1).unwrap().as_str().trim().to_string();
        let body = &content[caps.get(0).unwrap().end()..];
        let description = extract_description_from_markdown(body);

        return SkillMetadata {
            name: clean_markdown(&name),
            description,
        };
    }

    // 4. Any other non-empty markdown content
    let description = extract_description_from_markdown(content);
    if description != "No description provided." {
        return SkillMetadata {
            name: infer_name_from_path(file_path),
            description,
        };
    }

    infer_fallback_metadata(file_path)
}

fn extract_yaml_metadata(value: &serde_yaml::Value, file_path: &str) -> Option<SkillMetadata> {
    let name = value
        .get("name")
        .or_else(|| value.get("title"))
        .or_else(|| value.get("id"))
        .and_then(|v| v.as_str())
        .map(clean_value)
        .unwrap_or_else(|| infer_name_from_path(file_path));

    let description = value
        .get("description")
        .or_else(|| value.get("desc"))
        .or_else(|| value.get("summary"))
        .or_else(|| value.get("details"))
        .and_then(|v| v.as_str())
        .map(extract_first_sentence)
        .unwrap_or_else(|| "No description provided.".to_string());

    Some(SkillMetadata { name, description })
}

fn extract_description_from_markdown(content: &str) -> String {
    let mut paragraph = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }

        // Skip headers, badges, comments, code blocks, horizontal rules
        if trimmed.starts_with('#')
            || trimmed.starts_with("---")
            || trimmed.starts_with("```")
            || trimmed.starts_with("<!--")
            || trimmed.starts_with("[![")
            || trimmed.starts_with("![")
        {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }

        paragraph.push(trimmed);
    }

    if paragraph.is_empty() {
        "No description provided.".to_string()
    } else {
        extract_first_sentence(&paragraph.join(" "))
    }
}

/// Extracts only the first sentence from a description string.
pub fn extract_first_sentence(val: &str) -> String {
    let cleaned = clean_markdown(val);
    if cleaned == "No description provided." || cleaned.is_empty() {
        return "No description provided.".to_string();
    }

    // Check for sentence terminator (. ! ?) followed by whitespace, newline, or EOF,
    // while skipping common abbreviations (e.g., i.e., etc., vs.).
    let chars: Vec<char> = cleaned.chars().collect();
    let len = chars.len();

    for i in 0..len {
        let c = chars[i];
        if c == '.' || c == '!' || c == '?' {
            let is_at_end = i + 1 == len || chars[i + 1].is_whitespace();
            if is_at_end {
                if c == '.' {
                    let prefix: String = chars[..=i].iter().collect();
                    let lower_prefix = prefix.to_lowercase();
                    if lower_prefix.ends_with("e.g.")
                        || lower_prefix.ends_with("i.e.")
                        || lower_prefix.ends_with("etc.")
                        || lower_prefix.ends_with("vs.")
                    {
                        continue;
                    }
                }
                let sentence: String = chars[..=i].iter().collect();
                let trimmed = sentence.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    cleaned
}

pub fn clean_markdown(val: &str) -> String {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return "No description provided.".to_string();
    }

    // Strip outer quotes
    let unquoted = clean_value(trimmed);

    // Replace Markdown links [text](url) -> text
    let link_re = Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap();
    let without_links = link_re.replace_all(&unquoted, "$1");

    // Replace bold/italic markers **text** or *text* or `text`
    let formatted = without_links
        .replace("**", "")
        .replace("__", "")
        .replace('`', "")
        .replace("*", "");

    let cleaned = formatted.trim().to_string();
    if cleaned.is_empty() {
        "No description provided.".to_string()
    } else {
        cleaned
    }
}

pub fn clean_value(val: &str) -> String {
    let trimmed = val.trim();
    if ((trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
        && trimmed.len() >= 2
    {
        return trimmed[1..trimmed.len() - 1].trim().to_string();
    }
    trimmed.to_string()
}

pub fn infer_name_from_path(file_path: &str) -> String {
    let parts: Vec<&str> = file_path.split('/').collect();
    if parts.len() > 1 {
        let parent = parts[parts.len() - 2];
        if parent != "skills"
            && parent != ".skills"
            && parent != ".agents"
            && parent != ".claude"
            && parent != ".junie"
        {
            return parent.to_string();
        }
    }
    let filename = parts.last().unwrap_or(&"");
    if let Some(dot_idx) = filename.rfind('.') {
        filename[..dot_idx].to_string()
    } else {
        filename.to_string()
    }
}

pub fn infer_fallback_metadata(file_path: &str) -> SkillMetadata {
    SkillMetadata {
        name: infer_name_from_path(file_path),
        description: "No description provided.".to_string(),
    }
}
