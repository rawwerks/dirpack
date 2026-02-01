use std::fs;
use std::path::Path;

use dirpack::security::{
    validate_archive_entry_kind, validate_archive_entry_metadata, ArchiveEntryKind,
    ArchiveEntryTypeError,
};
use tempfile::TempDir;

#[test]
fn test_allows_regular_file_and_directory_kinds() {
    let file_ok =
        validate_archive_entry_kind(Path::new("file.txt"), ArchiveEntryKind::RegularFile);
    assert!(file_ok.is_ok());

    let dir_ok = validate_archive_entry_kind(Path::new("dir/"), ArchiveEntryKind::Directory);
    assert!(dir_ok.is_ok());
}

#[test]
fn test_rejects_block_device_kind() {
    let err = validate_archive_entry_kind(Path::new("dev/block"), ArchiveEntryKind::BlockDevice)
        .expect_err("expected block device to be rejected");
    assert_eq!(err, ArchiveEntryTypeError::BlockDevice);
}

#[test]
fn test_rejects_char_device_kind() {
    let err = validate_archive_entry_kind(Path::new("dev/char"), ArchiveEntryKind::CharDevice)
        .expect_err("expected char device to be rejected");
    assert_eq!(err, ArchiveEntryTypeError::CharDevice);
}

#[test]
fn test_rejects_fifo_kind() {
    let err = validate_archive_entry_kind(Path::new("pipe"), ArchiveEntryKind::Fifo)
        .expect_err("expected fifo to be rejected");
    assert_eq!(err, ArchiveEntryTypeError::Fifo);
}

#[test]
fn test_validate_archive_entry_metadata_accepts_regular_entries() {
    let temp = TempDir::new().expect("tempdir");

    let file_path = temp.path().join("file.txt");
    fs::write(&file_path, "data").expect("write file");
    let file_meta = fs::symlink_metadata(&file_path).expect("file metadata");
    validate_archive_entry_metadata(&file_path, &file_meta).expect("file allowed");

    let dir_path = temp.path().join("dir");
    fs::create_dir(&dir_path).expect("mkdir");
    let dir_meta = fs::symlink_metadata(&dir_path).expect("dir metadata");
    validate_archive_entry_metadata(&dir_path, &dir_meta).expect("dir allowed");
}
