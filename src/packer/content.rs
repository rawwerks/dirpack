//! Full file content inclusion.

use std::fs;
use std::path::Path;

use crate::budget::Budget;
use crate::scanner::entry::FileEntry;

/// Read file content, returning None if file is binary or too large.
pub fn read_content(path: &Path) -> Option<String> {
    let content = fs::read(path).ok()?;

    // Check if file appears to be binary (contains null bytes)
    if content.contains(&0) {
        return None;
    }

    String::from_utf8(content).ok()
}

/// Read file content for an entry.
pub fn read_entry_content(entry: &FileEntry) -> Option<String> {
    if entry.is_dir {
        return None;
    }
    read_content(&entry.path)
}

/// Include file contents that fit within budget.
/// Returns a list of (relative_path, content) tuples.
pub fn include_contents(
    entries: &[FileEntry],
    budget: &mut Budget,
) -> Vec<(String, String)> {
    let mut included = Vec::new();

    for entry in entries {
        if entry.is_dir {
            continue;
        }

        if let Some(content) = read_entry_content(entry) {
            let path_str = entry.relative_path.to_string_lossy().to_string();
            let full_content = format!("### {}\n```\n{}\n```\n", path_str, content);

            if budget.try_add(&full_content) {
                included.push((path_str, content));
            } else {
                // Budget exhausted, stop including content
                break;
            }
        }
    }

    included
}

/// Extract first N lines from content as a summary.
pub fn extract_summary(content: &str, max_lines: usize) -> String {
    content
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_summary() {
        let content = "line1\nline2\nline3\nline4\nline5";
        let summary = extract_summary(content, 3);
        assert_eq!(summary, "line1\nline2\nline3");
    }
}
