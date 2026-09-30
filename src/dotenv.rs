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
