pub mod cli;
pub mod dotenv;
pub mod history;
pub mod interactive;
pub mod metadata;
pub mod parser;
pub mod repl;
pub mod scanner;
pub mod similarity;
pub mod storage;
pub mod web;

pub use cli::Cli;
pub use interactive::present_skills;
pub use metadata::{extract_first_sentence, parse_skill_metadata, SkillMetadata};
pub use parser::{parse_repo_identifier, RepoIdentifier};
pub use scanner::{scan_github_repo, ScanResult, ScannerOptions, Skill};
pub use similarity::{
    calculate_similarity, filter_skills, find_all_similar_pairs, find_similar_skills, SimilarPair,
    SimilarSkillMatch, DEFAULT_SIMILARITY_THRESHOLD,
};
pub use storage::{
    clear_cache, default_db_path, get_cached_repository, list_all_cached_repositories, open_db,
    save_cached_repository, CachedRepo,
};
pub use web::{create_router, start_web_server, AppState, WebOptions};
