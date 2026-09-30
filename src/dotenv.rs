use std::path::Path;

/// Parses `KEY=VALUE` lines of a `.env` file. Blank lines, `#` comments and lines without `=`
/// are skipped; an `export ` prefix and surrounding quotes are removed; empty keys or values
/// are ignored.
pub fn parse_dotenv(content: &str) -> Vec<(String, String)> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            let key = key.strip_prefix("export ").unwrap_or(key).trim();
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(value);
            (!key.is_empty() && !value.is_empty()).then(|| (key.to_string(), value.to_string()))
        })
        .collect()
}

/// Returns the nearest `.env` in `start` or one of its ancestors, if any.
pub fn find_dotenv_upwards(start: &Path) -> Option<std::path::PathBuf> {
    start
        .ancestors()
        .map(|dir| dir.join(".env"))
        .find(|candidate| candidate.is_file())
}

/// Loads `./.env` and then the nearest `.env` above the running executable (e.g. the project
/// root for `target/release/skill-atlas`), so a binary started from Finder or another directory
/// still finds its token. Earlier files win; existing environment variables are never overridden.
pub fn load_default_dotenv() -> Vec<String> {
    let mut loaded = load_dotenv(Path::new(".env"));
    let exe_env = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.canonicalize().ok())
        .and_then(|exe| exe.parent().and_then(find_dotenv_upwards));
    if let Some(path) = exe_env {
        loaded.extend(load_dotenv(&path));
    }
    loaded
}

/// Loads variables from a `.env` file into the process environment without overriding
/// variables that are already set (and non-empty). A missing or unreadable file is ignored.
/// Returns the names of the variables that were set.
pub fn load_dotenv(path: &Path) -> Vec<String> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut loaded = Vec::new();
    for (key, value) in parse_dotenv(&content) {
        let already_set = std::env::var(&key).map(|v| !v.is_empty()).unwrap_or(false);
        if !already_set {
            std::env::set_var(&key, value);
            loaded.push(key);
        }
    }
    loaded
}
