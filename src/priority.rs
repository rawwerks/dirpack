//! Priority calculation from config rules.

use std::path::Path;

use crate::config::{CategoryConfig, PriorityRule, PriorityWeights};
use crate::scanner::entry::FileEntry;

/// Calculate priority for a file entry based on config rules and weights.
pub fn calculate_priority(
    entry: &FileEntry,
    rules: &[PriorityRule],
    categories: &CategoryConfig,
    weights: &PriorityWeights,
) -> i32 {
    let mut priority = None;

    // Check pattern rules first (highest specificity)
    for rule in rules {
        if matches_pattern(&entry.relative_path, &rule.pattern) {
            priority = Some(rule.priority);
            break;
        }
    }

    // Fall back to category-based priority
    if priority.is_none() && !entry.extension.is_empty() {
        priority = category_priority(&entry.extension, categories);
    }

    let mut score = priority.unwrap_or(weights.default_priority);
    score += path_adjustment(entry, categories, weights);
    score.max(0)
}

/// Check if a path matches a glob pattern.
pub fn matches_pattern(path: &Path, pattern: &str) -> bool {
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

fn path_adjustment(entry: &FileEntry, categories: &CategoryConfig, weights: &PriorityWeights) -> i32 {
    let mut delta = 0;
    let file_name = entry.file_name().to_ascii_lowercase();
    let components = path_components_lower(&entry.relative_path);

    let is_code = !entry.extension.is_empty()
        && categories
            .code
            .extensions
            .iter()
            .any(|ext| ext.eq_ignore_ascii_case(&entry.extension));

    let is_entrypoint = is_entrypoint_name(&file_name);
    if is_entrypoint {
        delta += weights.entrypoint_boost;
    }

    if is_code && entry.depth == 0 && is_entrypoint {
        delta += weights.root_code_boost;
    }

    if is_code
        && components
            .iter()
            .any(|c| matches!(c.as_str(), "src" | "cmd" | "lib" | "pkg" | "internal"))
    {
        delta += weights.focus_dir_boost;
    }

    if is_test_like(&file_name, &components) {
        delta += weights.test_penalty;
    }

    if is_fixture_like(&file_name, &components) {
        delta += weights.fixture_penalty;
    }

    if entry.depth > 2 {
        let penalty = ((entry.depth - 2) as i32) * weights.depth_penalty_step;
        delta += penalty.max(weights.max_depth_penalty);
    }

    delta
}

fn path_components_lower(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| component.as_os_str().to_str())
        .filter(|part| !part.is_empty() && *part != ".")
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn is_entrypoint_name(file_name: &str) -> bool {
    matches!(
        file_name,
        "main.rs"
            | "lib.rs"
            | "main.go"
            | "app.go"
            | "server.go"
            | "main.py"
            | "app.py"
            | "__init__.py"
            | "index.ts"
            | "index.tsx"
            | "main.ts"
            | "main.js"
            | "index.js"
            | "cli.js"
    )
}

fn is_test_like(file_name: &str, components: &[String]) -> bool {
    if file_name.starts_with("test_")
        || file_name.contains("_test.")
        || file_name.ends_with("_test")
    {
        return true;
    }

    components.iter().any(|part| {
        matches!(
            part.as_str(),
            "test" | "tests" | "testing" | "spec" | "specs" | "e2e" | "integration"
        )
    })
}

fn is_fixture_like(file_name: &str, components: &[String]) -> bool {
    if file_name.contains("fixture") || file_name.contains("mock") {
        return true;
    }

    components.iter().any(|part| {
        matches!(
            part.as_str(),
            "fixtures" | "fixture" | "mocks" | "mock" | "testdata"
        )
    })
}

/// Sort entries by priority (highest first).
pub fn sort_by_priority(
    entries: &mut [FileEntry],
    rules: &[PriorityRule],
    categories: &CategoryConfig,
    weights: &PriorityWeights,
) {
    entries.sort_by(|a, b| {
        let pa = calculate_priority(a, rules, categories, weights);
        let pb = calculate_priority(b, rules, categories, weights);
        pb.cmp(&pa)
            .then_with(|| a.depth.cmp(&b.depth))
            .then_with(|| a.relative_path.cmp(&b.relative_path))
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
            representation: crate::scanner::entry::Representation::NameOnly,
        }
    }

    #[test]
    fn test_readme_priority() {
        let rules = vec![PriorityRule {
            pattern: "README*".to_string(),
            priority: 200,
        }];
        let categories = CategoryConfig::default();
        let weights = PriorityWeights::default();

        let entry = make_entry("README.md");
        assert_eq!(calculate_priority(&entry, &rules, &categories, &weights), 200);
    }

    #[test]
    fn test_glob_pattern() {
        let rules = vec![PriorityRule {
            pattern: "**/mod.rs".to_string(),
            priority: 130,
        }];
        let categories = CategoryConfig::default();
        let weights = PriorityWeights::default();

        let entry = make_entry("src/scanner/mod.rs");
        assert!(calculate_priority(&entry, &rules, &categories, &weights) >= 130);
    }

    #[test]
    fn test_extension_priority() {
        let rules = vec![];
        let categories = CategoryConfig::default();
        let weights = PriorityWeights::default();

        let entry = make_entry("main.rs");
        assert!(calculate_priority(&entry, &rules, &categories, &weights) >= 100);
    }

    #[test]
    fn test_root_code_boost_over_nested() {
        let rules = vec![];
        let categories = CategoryConfig::default();
        let weights = PriorityWeights::default();

        let root = make_entry("main.rs");
        let nested = make_entry("src/helper.rs");
        assert!(calculate_priority(&root, &rules, &categories, &weights) > calculate_priority(&nested, &rules, &categories, &weights));
    }

    #[test]
    fn test_test_file_penalty() {
        let rules = vec![];
        let categories = CategoryConfig::default();
        let weights = PriorityWeights::default();

        let code = make_entry("cmd/bd/agent.go");
        let test = make_entry("cmd/bd/agent_test.go");
        assert!(calculate_priority(&code, &rules, &categories, &weights) > calculate_priority(&test, &rules, &categories, &weights));
    }

    #[test]
    fn test_custom_weights() {
        let rules = vec![];
        let categories = CategoryConfig::default();
        let mut weights = PriorityWeights::default();

        // Increase test penalty
        weights.test_penalty = -100;

        let code = make_entry("src/lib.rs");
        let test = make_entry("tests/test_lib.rs");

        let code_priority = calculate_priority(&code, &rules, &categories, &weights);
        let test_priority = calculate_priority(&test, &rules, &categories, &weights);

        // Test should have much lower priority with increased penalty
        assert!(code_priority - test_priority > 50);
    }
}
