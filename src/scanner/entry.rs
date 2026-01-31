//! File entry struct representing a scanned file.

use std::path::{Path, PathBuf};

/// Level of detail for file representation in output.
/// Used for progressive disclosure / mesh decimation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Representation {
    /// Just the name in the tree (default, cheapest)
    #[default]
    NameOnly,
    /// Signatures/headings extracted
    Structure,
    /// First N lines of content
    Snippet,
    /// Full file content
    Full,
}

impl Representation {
    /// Get the next higher detail level, if any.
    pub fn upgrade(&self) -> Option<Representation> {
        match self {
            Representation::NameOnly => Some(Representation::Structure),
            Representation::Structure => Some(Representation::Snippet),
            Representation::Snippet => Some(Representation::Full),
            Representation::Full => None,
        }
    }
}

/// Represents a file or directory found during scanning.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Absolute path to the file
    pub path: PathBuf,
    /// Path relative to the scan root
    pub relative_path: PathBuf,
    /// Whether this is a directory
    pub is_dir: bool,
    /// File size in bytes (0 for directories)
    pub size: u64,
    /// File extension (empty for directories or no extension)
    pub extension: String,
    /// Depth from root (0 = root level)
    pub depth: usize,
    /// Current representation level for output
    pub representation: Representation,
}

impl FileEntry {
    /// Create a new FileEntry from a path and root.
    pub fn new(path: &Path, root: &Path, is_dir: bool, size: u64) -> Self {
        let relative_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();
        let depth = relative_path.components().count().saturating_sub(1);
        let extension = if is_dir {
            String::new()
        } else {
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string()
        };

        Self {
            path: path.to_path_buf(),
            relative_path,
            is_dir,
            size,
            extension,
            depth,
            representation: Representation::NameOnly,
        }
    }

    /// Get the file name as a string.
    pub fn file_name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
    }

    /// Check if this file matches an extension.
    pub fn has_extension(&self, ext: &str) -> bool {
        self.extension.eq_ignore_ascii_case(ext)
    }
}
