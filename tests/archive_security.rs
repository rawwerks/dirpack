use std::path::Path;

use dirpack::archive::{validate_archive_entry, ArchiveEntryType, ArchiveValidationError};

#[test]
fn test_reject_symlink_entry() {
    let root = Path::new("/tmp");
    let err = validate_archive_entry(
        root,
        Path::new("link"),
        ArchiveEntryType::Symlink,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::UnsupportedType);
}

#[test]
fn test_reject_hardlink_entry() {
    let root = Path::new("/tmp");
    let err = validate_archive_entry(
        root,
        Path::new("hardlink"),
        ArchiveEntryType::Hardlink,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::UnsupportedType);
}

#[test]
fn test_reject_absolute_path() {
    let root = Path::new("/tmp");
    let err = validate_archive_entry(
        root,
        Path::new("/etc/passwd"),
        ArchiveEntryType::File,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::AbsolutePath);
}

#[test]
fn test_reject_traversal_path() {
    let root = Path::new("/tmp");
    let err = validate_archive_entry(
        root,
        Path::new("../escape"),
        ArchiveEntryType::File,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::Traversal);
}

#[test]
fn test_reject_deep_traversal_path() {
    let root = Path::new("/tmp");
    let err = validate_archive_entry(
        root,
        Path::new("../../../etc/passwd"),
        ArchiveEntryType::File,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::Traversal);
}

#[test]
fn test_reject_nested_traversal_path() {
    let root = Path::new("/tmp");
    let err = validate_archive_entry(
        root,
        Path::new("../../../etc/passwd"),
        ArchiveEntryType::File,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::Traversal);
}

#[test]
fn test_reject_path_too_long() {
    let root = Path::new("/tmp");
    let long_name = "a".repeat(300);
    let err = validate_archive_entry(
        root,
        Path::new(&long_name),
        ArchiveEntryType::File,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::PathTooLong);
}

#[test]
fn test_accept_file_and_directory() {
    let root = Path::new("/tmp");
    let file = validate_archive_entry(
        root,
        Path::new("dir/file.rs"),
        ArchiveEntryType::File,
        255,
    )
    .expect("file accepted");
    assert!(file.ends_with("dir/file.rs"));

    let dir = validate_archive_entry(
        root,
        Path::new("dir/sub"),
        ArchiveEntryType::Directory,
        255,
    )
    .expect("dir accepted");
    assert!(dir.ends_with("dir/sub"));
}
