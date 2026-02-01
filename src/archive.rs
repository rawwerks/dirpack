use std::path::{Component, Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveEntryType {
    File,
    Directory,
    Symlink,
    Hardlink,
    Other,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ArchiveValidationError {
    #[error("entry path is empty")]
    EmptyPath,
    #[error("entry path is absolute")]
    AbsolutePath,
    #[error("entry path contains parent traversal")]
    Traversal,
    #[error("entry path too long")]
    PathTooLong,
    #[error("entry type not allowed")]
    UnsupportedType,
    #[error("entry path escapes sandbox")]
    EscapesSandbox,
}

pub fn validate_archive_entry(
    root: &Path,
    entry_path: &Path,
    entry_type: ArchiveEntryType,
    max_path_len: usize,
) -> Result<PathBuf, ArchiveValidationError> {
    if entry_path.as_os_str().is_empty() {
        return Err(ArchiveValidationError::EmptyPath);
    }

    let mut normalized = PathBuf::new();
    for component in entry_path.components() {
        match component {
            Component::ParentDir => return Err(ArchiveValidationError::Traversal),
            Component::RootDir | Component::Prefix(_) => {
                return Err(ArchiveValidationError::AbsolutePath)
            }
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err(ArchiveValidationError::EmptyPath);
    }

    if normalized.as_os_str().len() > max_path_len {
        return Err(ArchiveValidationError::PathTooLong);
    }

    match entry_type {
        ArchiveEntryType::File | ArchiveEntryType::Directory => {}
        ArchiveEntryType::Symlink | ArchiveEntryType::Hardlink | ArchiveEntryType::Other => {
            return Err(ArchiveValidationError::UnsupportedType)
        }
    }

    let target = root.join(&normalized);
    if target.strip_prefix(root).is_err() {
        return Err(ArchiveValidationError::EscapesSandbox);
    }

    Ok(target)
}
