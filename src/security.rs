use std::fs::Metadata;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};

const MAX_ARCHIVE_PATH_LEN: usize = 4096;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PathTraversalError {
    #[error("empty archive path")]
    EmptyPath,
    #[error("absolute paths are not allowed in archives")]
    AbsolutePath,
    #[error("parent directory traversal is not allowed in archives")]
    ParentDir,
    #[error("archive path exceeds maximum length")]
    PathTooLong,
    #[error("path escapes sandbox root")]
    EscapesRoot,
}

fn log_security_event(entry: &Path, reason: &PathTraversalError) {
    eprintln!(
        "SECURITY: rejected archive path '{}' ({})",
        entry.display(),
        reason
    );
}

/// Normalize an archive entry path and reject any traversal/absolute paths.
pub fn normalize_archive_entry(entry: &Path) -> Result<PathBuf, PathTraversalError> {
    if entry.as_os_str().is_empty() {
        log_security_event(entry, &PathTraversalError::EmptyPath);
        return Err(PathTraversalError::EmptyPath);
    }

    let mut normalized = PathBuf::new();
    for component in entry.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                log_security_event(entry, &PathTraversalError::AbsolutePath);
                return Err(PathTraversalError::AbsolutePath);
            }
            Component::ParentDir => {
                log_security_event(entry, &PathTraversalError::ParentDir);
                return Err(PathTraversalError::ParentDir);
            }
            Component::CurDir => {}
            Component::Normal(part) => {
                normalized.push(part);
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        log_security_event(entry, &PathTraversalError::EmptyPath);
        return Err(PathTraversalError::EmptyPath);
    }

    if archive_path_len(&normalized) > MAX_ARCHIVE_PATH_LEN {
        log_security_event(entry, &PathTraversalError::PathTooLong);
        return Err(PathTraversalError::PathTooLong);
    }

    Ok(normalized)
}

/// Resolve an archive entry path within a sandbox root.
/// Caller should create the path after validation.
pub fn resolve_entry_within_root(
    root: &Path,
    entry: &Path,
) -> Result<PathBuf, PathTraversalError> {
    let normalized = normalize_archive_entry(entry)?;
    let root_canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let resolved = root_canon.join(&normalized);
    if !resolved.starts_with(&root_canon) {
        log_security_event(entry, &PathTraversalError::EscapesRoot);
        return Err(PathTraversalError::EscapesRoot);
    }
    Ok(resolved)
}

fn archive_path_len(path: &Path) -> usize {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        return path.as_os_str().as_bytes().len();
    }
    #[cfg(not(unix))]
    {
        path.to_string_lossy().len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveEntryKind {
    RegularFile,
    Directory,
    Symlink,
    Hardlink,
    BlockDevice,
    CharDevice,
    Fifo,
    Socket,
    Special,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ArchiveEntryTypeError {
    #[error("symlinks are not allowed in archives")]
    Symlink,
    #[error("hardlinks are not allowed in archives")]
    Hardlink,
    #[error("block devices are not allowed in archives")]
    BlockDevice,
    #[error("character devices are not allowed in archives")]
    CharDevice,
    #[error("FIFOs (named pipes) are not allowed in archives")]
    Fifo,
    #[error("sockets are not allowed in archives")]
    Socket,
    #[error("special file types are not allowed in archives")]
    Special,
}

fn log_entry_type_event(entry: &Path, reason: &ArchiveEntryTypeError) {
    eprintln!(
        "SECURITY: rejected archive entry '{}' ({})",
        entry.display(),
        reason
    );
}

impl ArchiveEntryKind {
    pub fn is_allowed(self) -> bool {
        matches!(self, ArchiveEntryKind::RegularFile | ArchiveEntryKind::Directory)
    }
}

pub fn validate_archive_entry_kind(
    entry: &Path,
    kind: ArchiveEntryKind,
) -> Result<(), ArchiveEntryTypeError> {
    if kind.is_allowed() {
        return Ok(());
    }

    let err = match kind {
        ArchiveEntryKind::Symlink => ArchiveEntryTypeError::Symlink,
        ArchiveEntryKind::Hardlink => ArchiveEntryTypeError::Hardlink,
        ArchiveEntryKind::BlockDevice => ArchiveEntryTypeError::BlockDevice,
        ArchiveEntryKind::CharDevice => ArchiveEntryTypeError::CharDevice,
        ArchiveEntryKind::Fifo => ArchiveEntryTypeError::Fifo,
        ArchiveEntryKind::Socket => ArchiveEntryTypeError::Socket,
        ArchiveEntryKind::Special => ArchiveEntryTypeError::Special,
        ArchiveEntryKind::RegularFile | ArchiveEntryKind::Directory => ArchiveEntryTypeError::Special,
    };

    log_entry_type_event(entry, &err);
    Err(err)
}

pub fn classify_fs_entry(metadata: &Metadata) -> ArchiveEntryKind {
    let file_type = metadata.file_type();

    if file_type.is_dir() {
        return ArchiveEntryKind::Directory;
    }

    if file_type.is_file() {
        #[cfg(unix)]
        {
            if metadata.nlink() > 1 {
                return ArchiveEntryKind::Hardlink;
            }
        }
        return ArchiveEntryKind::RegularFile;
    }

    if file_type.is_symlink() {
        return ArchiveEntryKind::Symlink;
    }

    #[cfg(unix)]
    {
        if file_type.is_block_device() {
            return ArchiveEntryKind::BlockDevice;
        }
        if file_type.is_char_device() {
            return ArchiveEntryKind::CharDevice;
        }
        if file_type.is_fifo() {
            return ArchiveEntryKind::Fifo;
        }
        if file_type.is_socket() {
            return ArchiveEntryKind::Socket;
        }
    }

    ArchiveEntryKind::Special
}

pub fn validate_archive_entry_metadata(
    entry: &Path,
    metadata: &Metadata,
) -> Result<(), ArchiveEntryTypeError> {
    let kind = classify_fs_entry(metadata);
    validate_archive_entry_kind(entry, kind)
}
