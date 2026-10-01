use crate::scanner::Skill;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Default minimum similarity percentage threshold for detecting similar skills.
pub const DEFAULT_SIMILARITY_THRESHOLD: f64 = 30.0;

/// Stop words excluded from description tokenization to emphasize meaningful keywords.
const STOP_WORDS: &[&str] = &[
    "a", "about", "above", "after", "again", "against", "all", "am", "an", "and", "any", "are",
    "as", "at", "be", "because", "been", "before", "being", "below", "between", "both", "but",
    "by", "can", "could", "did", "do", "does", "doing", "down", "during", "each", "few", "for",
    "from", "further", "had", "has", "have", "having", "he", "her", "here", "hers", "herself",
    "him", "himself", "his", "how", "i", "if", "in", "into", "is", "it", "its", "itself", "just",
    "me", "more", "most", "my", "myself", "no", "nor", "not", "now", "of", "off", "on", "once",
    "only", "or", "other", "our", "ours", "ourselves", "out", "over", "own", "same", "she",
    "should", "so", "some", "such", "than", "that", "the", "their", "theirs", "them", "themselves",
    "then", "there", "these", "they", "this", "those", "through", "to", "too", "under", "until",
    "up", "very", "was", "we", "were", "what", "when", "where", "which", "while", "who", "whom",
    "why", "with", "would", "you", "your", "yours", "yourself", "yourselves",
];

/// Pair of similar skills with a calculated similarity percentage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimilarPair {
    pub index_a: usize,
    pub index_b: usize,
    pub skill_a: Skill,
    pub skill_b: Skill,
    pub similarity: f64,
}

/// A matched similar skill with similarity percentage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimilarSkillMatch {
    pub index: usize,
    pub skill: Skill,
    pub similarity: f64,
}

/// Tokenizes text into a set of lowercased alphanumeric keywords, filtering stop words.
pub fn tokenize(text: &str, filter_stop_words: bool) -> HashSet<String> {
    let stop_set: HashSet<&'static str> = STOP_WORDS.iter().copied().collect();
    text.split(|c: char| !c.is_alphanumeric())
        .map(|s| s.to_lowercase())
        .filter(|s| !s.is_empty())
        .filter(|s| !filter_stop_words || (!stop_set.contains(s.as_str()) && s.len() > 1))
        .collect()
}

/// Calculates Jaccard similarity between two token sets (0.0 to 1.0).
pub fn jaccard_similarity(set_a: &HashSet<String>, set_b: &HashSet<String>) -> f64 {
    if set_a.is_empty() && set_b.is_empty() {
        return 0.0;
    }
    let intersection = set_a.intersection(set_b).count();
    let union = set_a.union(set_b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Computes character bigram Dice coefficient for fuzzy string similarity (0.0 to 1.0).
pub fn dice_bigram_similarity(s1: &str, s2: &str) -> f64 {
    let s1_clean: String = s1.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect();
    let s2_clean: String = s2.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect();

    if s1_clean == s2_clean {
        return if s1_clean.is_empty() { 0.0 } else { 1.0 };
    }
    if s1_clean.len() < 2 || s2_clean.len() < 2 {
        return 0.0;
    }

    let mut bigrams1 = Vec::new();
    let chars1: Vec<char> = s1_clean.chars().collect();
    for i in 0..chars1.len() - 1 {
        bigrams1.push((chars1[i], chars1[i + 1]));
    }

    let mut bigrams2 = Vec::new();
    let chars2: Vec<char> = s2_clean.chars().collect();
    for i in 0..chars2.len() - 1 {
        bigrams2.push((chars2[i], chars2[i + 1]));
    }

    let total = bigrams1.len() + bigrams2.len();
    if total == 0 {
        return 0.0;
    }

    let mut matches = 0;
    let mut used2 = vec![false; bigrams2.len()];

    for b1 in &bigrams1 {
        for (idx, b2) in bigrams2.iter().enumerate() {
            if !used2[idx] && b1 == b2 {
                matches += 1;
                used2[idx] = true;
                break;
            }
        }
    }

    (2.0 * matches as f64) / (total as f64)
}

/// Calculates the similarity percentage (0.0% to 100.0%) between two skills.
///
/// Uses heuristic weights combining:
/// - Skill name similarity (token Jaccard + character bigram Dice coefficient)
/// - Description similarity (tokenized keyword Jaccard overlap)
/// - File path / category similarity (directory token overlap)
pub fn calculate_similarity(a: &Skill, b: &Skill) -> f64 {
    // If identical names
    let a_name_lower = a.name.trim().to_lowercase();
    let b_name_lower = b.name.trim().to_lowercase();

    if a_name_lower == b_name_lower {
        let a_desc = a.description.trim();
        let b_desc = b.description.trim();
        if a_desc.eq_ignore_ascii_case(b_desc) {
            return 100.0;
        }
        let desc_a_tokens = tokenize(&a.description, true);
        let desc_b_tokens = tokenize(&b.description, true);
        let desc_sim = jaccard_similarity(&desc_a_tokens, &desc_b_tokens);
        return (90.0 + (desc_sim * 10.0)).min(100.0);
    }

    let name_tokens_a = tokenize(&a.name, false);
    let name_tokens_b = tokenize(&b.name, false);
    let name_jaccard = jaccard_similarity(&name_tokens_a, &name_tokens_b);
    let name_dice = dice_bigram_similarity(&a.name, &b.name);
    let name_sim = name_jaccard.max(name_dice);

    let desc_tokens_a = tokenize(&a.description, true);
    let desc_tokens_b = tokenize(&b.description, true);
    let desc_sim = jaccard_similarity(&desc_tokens_a, &desc_tokens_b);

    let path_tokens_a = tokenize(&a.path, false);
    let path_tokens_b = tokenize(&b.path, false);
    let path_sim = jaccard_similarity(&path_tokens_a, &path_tokens_b);

    let raw_sim = if !desc_tokens_a.is_empty() && !desc_tokens_b.is_empty() {
        (0.50 * name_sim) + (0.40 * desc_sim) + (0.10 * path_sim)
    } else {
        (0.80 * name_sim) + (0.20 * path_sim)
    };

    let percentage = (raw_sim * 100.0).clamp(0.0, 100.0);
    (percentage * 10.0).round() / 10.0
}

/// Finds all skills similar to a target skill among a collection, above `min_pct` threshold.
pub fn find_similar_skills(target: &Skill, all: &[Skill], min_pct: f64) -> Vec<SimilarSkillMatch> {
    let mut matches = Vec::new();
    for (idx, skill) in all.iter().enumerate() {
        // Exclude the target skill itself (comparing path and name)
        if skill.path == target.path && skill.name == target.name {
            continue;
        }
        let sim = calculate_similarity(target, skill);
        if sim >= min_pct {
            matches.push(SimilarSkillMatch {
                index: idx + 1,
                skill: skill.clone(),
                similarity: sim,
            });
        }
    }
    matches.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.skill.name.to_lowercase().cmp(&b.skill.name.to_lowercase()))
    });
    matches
}

/// Finds all pairs of similar skills across a list of skills above `min_pct` threshold.
pub fn find_all_similar_pairs(skills: &[Skill], min_pct: f64) -> Vec<SimilarPair> {
    let mut pairs = Vec::new();
    let len = skills.len();
    for i in 0..len {
        for j in (i + 1)..len {
            let sim = calculate_similarity(&skills[i], &skills[j]);
            if sim >= min_pct {
                pairs.push(SimilarPair {
                    index_a: i + 1,
                    index_b: j + 1,
                    skill_a: skills[i].clone(),
                    skill_b: skills[j].clone(),
                    similarity: sim,
                });
            }
        }
    }
    pairs.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.index_a.cmp(&b.index_a))
    });
    pairs
}

/// Filters skills matching query terms across name, description, or file path.
pub fn filter_skills(skills: &[Skill], query: &str) -> Vec<Skill> {
    let query_clean = query.trim();
    if query_clean.is_empty() {
        return skills.to_vec();
    }
    let terms: Vec<String> = query_clean
        .split_whitespace()
        .map(|s| s.to_lowercase())
        .collect();

    skills
        .iter()
        .filter(|skill| {
            let name_lower = skill.name.to_lowercase();
            let desc_lower = skill.description.to_lowercase();
            let path_lower = skill.path.to_lowercase();

            terms.iter().all(|term| {
                name_lower.contains(term) || desc_lower.contains(term) || path_lower.contains(term)
            })
        })
        .cloned()
        .collect()
}
