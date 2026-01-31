//! Directory scanning module.
//!
//! Provides git-aware and fallback directory scanning.

pub mod entry;
pub mod git;
pub mod walk;

use std::path::Path;

use crate::config::ScanningConfig;
pub use entry::{FileEntry, Representation};

/// Scan a directory, preferring git ls-files if available.
pub fn scan(root: &Path, config: &ScanningConfig, use_git: bool) -> Vec<FileEntry> {
    // Try git first if enabled
    if use_git {
        if let Some(entries) = git::scan_git(root) {
            return entries;
        }
    }

    // Fall back to walking
    walk::scan_walk(root, config)
}

/// Filter entries to only include files (not directories).
pub fn files_only(entries: Vec<FileEntry>) -> Vec<FileEntry> {
    entries.into_iter().filter(|e| !e.is_dir).collect()
}

/// Filter entries to only include directories.
pub fn dirs_only(entries: Vec<FileEntry>) -> Vec<FileEntry> {
    entries.into_iter().filter(|e| e.is_dir).collect()
}
