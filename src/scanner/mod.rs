//! Directory scanning module.
//!
//! Provides git-aware and fallback directory scanning.

pub mod entry;
pub mod git;
pub mod walk;

use std::path::{Path, PathBuf};

use crate::config::{security_exclude_patterns, Config, ScanningConfig};
pub use entry::{FileEntry, Representation};
use ignore::gitignore::{Gitignore, GitignoreBuilder};

/// Scan a directory, preferring git ls-files if available.
pub fn scan(root: &Path, config: &Config, use_git: bool) -> Vec<FileEntry> {
    let mut effective_scanning = config.scanning.clone();
    if !use_git {
        effective_scanning.use_gitignore = false;
        if effective_scanning.no_git_safety {
            effective_scanning.include_hidden = true;
        }
    }

    // Try git first if enabled
    let entries = if use_git {
        if let Some(entries) = git::scan_git(root) {
            entries
        } else {
            walk::scan_walk(root, &effective_scanning)
        }
    } else {
        // Fall back to walking
        walk::scan_walk(root, &effective_scanning)
    };

    let exclude_patterns = build_exclude_patterns(config, use_git);
    let entries = apply_excludes(entries, root, &exclude_patterns);

    filter_hidden_entries(entries, root, effective_scanning.include_hidden)
}

/// Filter entries to only include files (not directories).
pub fn files_only(entries: Vec<FileEntry>) -> Vec<FileEntry> {
    entries.into_iter().filter(|e| !e.is_dir).collect()
}

/// Filter entries to only include directories.
pub fn dirs_only(entries: Vec<FileEntry>) -> Vec<FileEntry> {
    entries.into_iter().filter(|e| e.is_dir).collect()
}

fn filter_hidden_entries(
    entries: Vec<FileEntry>,
    root: &Path,
    include_hidden: bool,
) -> Vec<FileEntry> {
    if entries.is_empty() {
        return entries;
    }

    let mut output: Vec<FileEntry> = Vec::new();
    let mut hidden_roots: std::collections::BTreeSet<PathBuf> =
        std::collections::BTreeSet::new();
    let mut seen_paths: std::collections::BTreeSet<PathBuf> =
        std::collections::BTreeSet::new();

    for mut entry in entries {
        let rel = entry.relative_path.clone();
        if let Some(hidden_root) = first_hidden_prefix(&rel) {
            hidden_roots.insert(hidden_root.clone());
            if include_hidden {
                if entry.is_dir && rel == hidden_root {
                    entry.representation = Representation::NameOnly;
                }
                seen_paths.insert(rel.clone());
                output.push(entry);
            }
            continue;
        }

        seen_paths.insert(rel);
        output.push(entry);
    }

    if include_hidden {
        for hidden_root in hidden_roots {
            if seen_paths.contains(&hidden_root) {
                continue;
            }
            let mut entry = FileEntry::new(&root.join(&hidden_root), root, true, 0);
            entry.representation = Representation::NameOnly;
            output.push(entry);
        }
    }

    output.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    output
}

fn build_exclude_patterns(config: &Config, use_git: bool) -> Vec<String> {
    let mut patterns = Vec::new();
    if use_git {
        patterns.extend(config.exclude.patterns.clone());
    }
    patterns.extend(security_exclude_patterns());
    patterns
}

fn apply_excludes(entries: Vec<FileEntry>, root: &Path, patterns: &[String]) -> Vec<FileEntry> {
    if patterns.is_empty() {
        return entries;
    }

    let matcher = build_matcher(root, patterns);
    if matcher.is_none() {
        return entries;
    }
    let matcher = matcher.unwrap();

    entries
        .into_iter()
        .filter(|entry| {
            let matched = matcher.matched_path_or_any_parents(&entry.relative_path, entry.is_dir);
            !matched.is_ignore()
        })
        .collect()
}

fn build_matcher(root: &Path, patterns: &[String]) -> Option<Gitignore> {
    if patterns.is_empty() {
        return None;
    }

    let mut builder = GitignoreBuilder::new(root);
    for pattern in patterns {
        let _ = builder.add_line(None, pattern);
    }
    builder.build().ok()
}

fn first_hidden_prefix(path: &Path) -> Option<PathBuf> {
    let mut prefix = PathBuf::new();
    for component in path.components() {
        let os = component.as_os_str();
        let name = os.to_string_lossy();
        prefix.push(os);
        if name.starts_with('.') && name != "." && name != ".." {
            return Some(prefix);
        }
    }
    None
}
