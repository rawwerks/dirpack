use std::path::Path;

use dirpack::security::{resolve_entry_within_root, PathTraversalError};
use tempfile::TempDir;

#[test]
fn test_rejects_parent_dir_traversal() {
    let root = TempDir::new().expect("tempdir");
    let err = resolve_entry_within_root(root.path(), Path::new("../../../etc/passwd"))
        .expect_err("expected traversal to be rejected");
    assert_eq!(err, PathTraversalError::ParentDir);
}

#[test]
fn test_rejects_absolute_path() {
    let root = TempDir::new().expect("tempdir");
    let err = resolve_entry_within_root(root.path(), Path::new("/etc/shadow"))
        .expect_err("expected absolute path to be rejected");
    assert_eq!(err, PathTraversalError::AbsolutePath);
}

#[test]
fn test_rejects_mixed_traversal() {
    let root = TempDir::new().expect("tempdir");
    let err = resolve_entry_within_root(root.path(), Path::new("foo/../../../bar"))
        .expect_err("expected traversal to be rejected");
    assert_eq!(err, PathTraversalError::ParentDir);
}

#[test]
fn test_resolves_safe_path() {
    let root = TempDir::new().expect("tempdir");
    let resolved =
        resolve_entry_within_root(root.path(), Path::new("safe/dir/file.txt"))
            .expect("expected safe path to resolve");
    assert!(resolved.starts_with(root.path()));
    assert!(resolved.ends_with(Path::new("safe/dir/file.txt")));
}
