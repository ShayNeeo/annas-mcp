use std::path::Path;
use regex::Regex;

pub fn sanitize_filename(filename: &str) -> String {
    let unsafe_re = Regex::new(r#"[<>:"/\\|?*\x00-\x1f]"#).unwrap();
    let mut safe = unsafe_re.replace_all(filename, "_").to_string();

    // Remove any remaining path traversal components
    safe = safe.replace("..", "_");

    // Trim underscores, dots and whitespace from edges
    safe = safe.trim_matches(|c: char| c.is_whitespace() || c == '.' || c == '_').to_string();

    // Enforce reasonable length limit (200 chars)
    if safe.chars().count() > 200 {
        safe = safe.chars().take(200).collect();
    }

    if safe.is_empty() {
        "untitled".to_string()
    } else {
        safe
    }
}

pub fn make_safe_filepath(folder: &Path, title: &str, format: &str) -> std::path::PathBuf {
    let safe_title = sanitize_filename(title);
    let clean_format = format.trim_start_matches('.').to_lowercase();
    let ext = if clean_format.is_empty() {
        "bin".to_string()
    } else {
        clean_format
    };
    folder.join(format!("{safe_title}.{ext}"))
}
