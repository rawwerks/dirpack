//! Directory walking using the ignore crate (gitignore-aware).

use std::path::Path;

use ignore::WalkBuilder;

use crate::config::ScanningConfig;
use crate::scanner::entry::FileEntry;

/// Scan a directory using the ignore crate (respects .gitignore).
pub fn scan_walk(root: &Path, config: &ScanningConfig) -> Vec<FileEntry> {
    let mut entries = Vec::new();

    let max_depth = if config.max_depth == 0 {
        None
    } else {
        Some(config.max_depth)
    };

    let walker = WalkBuilder::new(root)
        .hidden(!config.include_hidden)
        .git_ignore(config.use_gitignore)
        .git_global(config.use_gitignore)
        .git_exclude(config.use_gitignore)
        .follow_links(config.follow_symlinks)
        .max_depth(max_depth)
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // Skip the root directory itself
        if path == root {
            continue;
        }

        let is_dir = entry.file_type().is_some_and(|ft| ft.is_dir());
        let size = if is_dir {
            0
        } else {
            entry.metadata().map(|m| m.len()).unwrap_or(0)
        };

        entries.push(FileEntry::new(path, root, is_dir, size));
    }

    // Sort by path for consistent ordering
    entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    entries
}
