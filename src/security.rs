use std::path::{Component, Path, PathBuf};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PathTraversalError {
    #[error("empty archive path")]
    EmptyPath,
    #[error("absolute paths are not allowed in archives")]
    AbsolutePath,
    #[error("parent directory traversal is not allowed in archives")]
    ParentDir,
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
