//! Priority calculation from config rules.

use std::path::Path;

use crate::config::{CategoryConfig, PriorityRule};
use crate::scanner::entry::FileEntry;

/// Default priority for files that don't match any rule.
const DEFAULT_PRIORITY: i32 = 50;

/// Calculate priority for a file entry based on config rules.
pub fn calculate_priority(
    entry: &FileEntry,
    rules: &[PriorityRule],
    categories: &CategoryConfig,
) -> i32 {
    // Check pattern rules first (highest specificity)
    for rule in rules {
        if matches_pattern(&entry.relative_path, &rule.pattern) {
            return rule.priority;
        }
    }

    // Fall back to category-based priority
    if !entry.extension.is_empty() {
        if let Some(priority) = category_priority(&entry.extension, categories) {
            return priority;
        }
    }

    DEFAULT_PRIORITY
}

/// Check if a path matches a glob pattern.
fn matches_pattern(path: &Path, pattern: &str) -> bool {
    let path_str = path.to_string_lossy();

    // Handle ** prefix (matches any path)
    if pattern.starts_with("**/") {
        let suffix = &pattern[3..];
        return matches_simple(&path_str, suffix)
            || path_str
                .split('/')
                .any(|part| matches_simple(part, suffix));
    }

    // Handle simple wildcards
    matches_simple(&path_str, pattern)
}

/// Simple pattern matching with * wildcard.
fn matches_simple(text: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if !pattern.contains('*') {
        return text == pattern || text.ends_with(&format!("/{}", pattern));
    }

    // Handle trailing wildcard: "README*" matches "README.md"
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        let file_name = text.rsplit('/').next().unwrap_or(text);
        return file_name.starts_with(prefix);
    }

    // Handle leading wildcard: "*.rs" matches "main.rs"
    if pattern.starts_with('*') {
        let suffix = &pattern[1..];
        return text.ends_with(suffix);
    }

    // Handle middle wildcard: "src/*.rs"
    if let Some(star_pos) = pattern.find('*') {
        let prefix = &pattern[..star_pos];
        let suffix = &pattern[star_pos + 1..];
        return text.starts_with(prefix) && text.ends_with(suffix);
    }

    false
}

/// Get priority from category config based on extension.
fn category_priority(extension: &str, categories: &CategoryConfig) -> Option<i32> {
    let ext_lower = extension.to_lowercase();

    if categories.code.extensions.iter().any(|e| e == &ext_lower) {
        return Some(categories.code.priority);
    }
    if categories.docs.extensions.iter().any(|e| e == &ext_lower) {
        return Some(categories.docs.priority);
    }
    if categories
        .config
        .extensions
        .iter()
        .any(|e| e == &ext_lower)
    {
        return Some(categories.config.priority);
    }
    if categories.build.extensions.iter().any(|e| e == &ext_lower) {
        return Some(categories.build.priority);
    }
    if categories.data.extensions.iter().any(|e| e == &ext_lower) {
        return Some(categories.data.priority);
    }

    None
}

/// Sort entries by priority (highest first).
pub fn sort_by_priority(
    entries: &mut [FileEntry],
    rules: &[PriorityRule],
    categories: &CategoryConfig,
) {
    entries.sort_by(|a, b| {
        let pa = calculate_priority(a, rules, categories);
        let pb = calculate_priority(b, rules, categories);
        pb.cmp(&pa) // Descending order
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_entry(path: &str) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            relative_path: PathBuf::from(path),
            is_dir: false,
            size: 100,
            extension: path.rsplit('.').next().unwrap_or("").to_string(),
            depth: 0,
        }
    }

    #[test]
    fn test_readme_priority() {
        let rules = vec![PriorityRule {
            pattern: "README*".to_string(),
            priority: 200,
        }];
        let categories = CategoryConfig::default();

        let entry = make_entry("README.md");
        assert_eq!(calculate_priority(&entry, &rules, &categories), 200);
    }

    #[test]
    fn test_glob_pattern() {
        let rules = vec![PriorityRule {
            pattern: "**/mod.rs".to_string(),
            priority: 130,
        }];
        let categories = CategoryConfig::default();

        let entry = make_entry("src/scanner/mod.rs");
        assert_eq!(calculate_priority(&entry, &rules, &categories), 130);
    }

    #[test]
    fn test_extension_priority() {
        let rules = vec![];
        let categories = CategoryConfig::default();

        let entry = make_entry("main.rs");
        assert_eq!(calculate_priority(&entry, &rules, &categories), 100); // code priority
    }
}
