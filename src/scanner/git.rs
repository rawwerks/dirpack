//! Git-aware scanning using git ls-files.

use std::path::Path;
use std::process::Command;

use crate::scanner::entry::FileEntry;

/// Check if a directory is a git repository.
pub fn is_git_repo(root: &Path) -> bool {
    root.join(".git").exists()
}

/// Scan a directory using git ls-files.
/// Returns None if not a git repo, git command fails, or no tracked files.
pub fn scan_git(root: &Path) -> Option<Vec<FileEntry>> {
    if !is_git_repo(root) {
        return None;
    }

    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();

    for file in stdout.split('\0').filter(|s| !s.is_empty()) {
        let path = root.join(file);

        // Skip if file doesn't exist (might be deleted but staged)
        if !path.exists() {
            continue;
        }

        let metadata = match path.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_dir = metadata.is_dir();
        let size = if is_dir { 0 } else { metadata.len() };

        entries.push(FileEntry::new(&path, root, is_dir, size));
    }

    // If no tracked files, fall back to walkdir
    if entries.is_empty() {
        return None;
    }

    // Sort by path for consistent ordering
    entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Some(entries)
}
