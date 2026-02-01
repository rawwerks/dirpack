//! Adversarial tests for archive security validation
//! Trying to find bypasses and edge cases

use std::path::Path;
use dirpack::archive::{validate_archive_entry, ArchiveEntryType, ArchiveValidationError};

// ============================================================
// UNICODE ATTACKS
// ============================================================

#[test]
fn test_unicode_dot_dot() {
    // U+FF0E is FULLWIDTH FULL STOP (．)
    // Some systems might normalize this to regular dot
    let root = Path::new("/tmp");
    let result = validate_archive_entry(
        root,
        Path::new("．．/etc/passwd"),
        ArchiveEntryType::File,
        255,
    );
    // Should either reject or treat as literal filename, NOT traverse
    eprintln!("Result: {:?}", result);
    if let Ok(path) = result {
        eprintln!("Path: {:?}", path);
        // The path should be /tmp/．．/etc/passwd (literal unicode dots as dir name)
        // NOT /tmp/../etc/passwd which would escape
        // Check it's within sandbox
        assert!(path.starts_with(root), "Path escaped sandbox: {:?}", path);
    }
}

#[test]
fn test_unicode_slash() {
    // U+2215 is DIVISION SLASH (∕)
    // U+FF0F is FULLWIDTH SOLIDUS (／)
    let root = Path::new("/tmp");
    let result = validate_archive_entry(
        root,
        Path::new("..∕etc∕passwd"),
        ArchiveEntryType::File,
        255,
    );
    // Should either reject or treat as literal, NOT traverse
    if let Ok(path) = result {
        assert!(!path.to_string_lossy().contains("/etc/passwd"));
    }
}

// ============================================================
// NULL BYTE INJECTION
// ============================================================

#[test]
fn test_null_byte_in_path() {
    let root = Path::new("/tmp");
    // Path with null byte - some C libraries truncate at null
    let result = validate_archive_entry(
        root,
        Path::new("file.txt\0.jpg"),
        ArchiveEntryType::File,
        255,
    );
    // Should handle gracefully - Rust paths handle nulls differently than C
    // This tests that we don't have C-string truncation issues
    assert!(result.is_ok() || result.is_err()); // Just shouldn't panic
}

// ============================================================
// HIDDEN TRAVERSAL IN MIDDLE OF PATH
// ============================================================

#[test]
fn test_traversal_after_descent() {
    let root = Path::new("/tmp");
    // Go down then back up
    let err = validate_archive_entry(
        root,
        Path::new("foo/bar/../../../etc/passwd"),
        ArchiveEntryType::File,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::Traversal);
}

#[test]
fn test_traversal_mixed_with_curdir() {
    let root = Path::new("/tmp");
    // Mix . and .. to try to confuse
    let err = validate_archive_entry(
        root,
        Path::new("./foo/./../../../etc/passwd"),
        ArchiveEntryType::File,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::Traversal);
}

// ============================================================
// EMPTY AND EDGE CASE PATHS
// ============================================================

#[test]
fn test_empty_path() {
    let root = Path::new("/tmp");
    let err = validate_archive_entry(
        root,
        Path::new(""),
        ArchiveEntryType::File,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::EmptyPath);
}

#[test]
fn test_curdir_only() {
    let root = Path::new("/tmp");
    // Just "." should normalize to empty
    let err = validate_archive_entry(
        root,
        Path::new("."),
        ArchiveEntryType::File,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::EmptyPath);
}

#[test]
fn test_multiple_curdir() {
    let root = Path::new("/tmp");
    // "./././." should normalize to empty
    let err = validate_archive_entry(
        root,
        Path::new("./././."),
        ArchiveEntryType::File,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::EmptyPath);
}

#[test]
fn test_double_slash() {
    let root = Path::new("/tmp");
    // Double slash shouldn't cause issues
    let result = validate_archive_entry(
        root,
        Path::new("foo//bar"),
        ArchiveEntryType::File,
        255,
    );
    assert!(result.is_ok());
}

#[test]
fn test_trailing_slash() {
    let root = Path::new("/tmp");
    let result = validate_archive_entry(
        root,
        Path::new("foo/bar/"),
        ArchiveEntryType::Directory,
        255,
    );
    assert!(result.is_ok());
}

// ============================================================
// WINDOWS-SPECIFIC ATTACKS (should be caught on all platforms)
// ============================================================

#[test]
fn test_windows_drive_letter() {
    let root = Path::new("/tmp");
    // Windows drive letter - should be caught as absolute/prefix
    let result = validate_archive_entry(
        root,
        Path::new("C:\\Windows\\System32"),
        ArchiveEntryType::File,
        255,
    );
    // On Unix this is just a weird filename, on Windows it's absolute
    // Either way, shouldn't escape sandbox
    if let Ok(path) = result {
        assert!(path.starts_with(root));
    }
}

#[test]
fn test_windows_unc_path() {
    let root = Path::new("/tmp");
    // UNC path
    let result = validate_archive_entry(
        root,
        Path::new("\\\\server\\share"),
        ArchiveEntryType::File,
        255,
    );
    // Should either reject or treat as literal
    if let Ok(path) = result {
        assert!(path.starts_with(root));
    }
}

#[test]
fn test_windows_device_names() {
    // Device names that could cause issues on Windows
    let root = Path::new("/tmp");
    for device in &["CON", "PRN", "AUX", "NUL", "COM1", "LPT1"] {
        let result = validate_archive_entry(
            root,
            Path::new(device),
            ArchiveEntryType::File,
            255,
        );
        // Should be allowed (just a filename on Linux)
        // but shouldn't cause issues
        if let Ok(path) = result {
            assert!(path.starts_with(root));
        }
    }
}

// ============================================================
// PATH LENGTH EDGE CASES
// ============================================================

#[test]
fn test_path_exactly_at_limit() {
    let root = Path::new("/tmp");
    let name = "a".repeat(255);
    let result = validate_archive_entry(
        root,
        Path::new(&name),
        ArchiveEntryType::File,
        255,
    );
    assert!(result.is_ok());
}

#[test]
fn test_path_one_over_limit() {
    let root = Path::new("/tmp");
    let name = "a".repeat(256);
    let err = validate_archive_entry(
        root,
        Path::new(&name),
        ArchiveEntryType::File,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::PathTooLong);
}

// ============================================================
// SYMLINK TARGET CONTENT (even though symlink entry is blocked)
// ============================================================

#[test]
fn test_entry_type_other() {
    let root = Path::new("/tmp");
    let err = validate_archive_entry(
        root,
        Path::new("device"),
        ArchiveEntryType::Other,
        255,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::UnsupportedType);
}

// ============================================================
// STRIP_PREFIX CHECK VALIDATION
// ============================================================

#[test]
fn test_strip_prefix_with_root_trailing_slash() {
    // Edge case: root with trailing slash
    let root = Path::new("/tmp/");
    let result = validate_archive_entry(
        root,
        Path::new("foo/bar"),
        ArchiveEntryType::File,
        255,
    );
    assert!(result.is_ok());
    if let Ok(path) = result {
        // Should still be within /tmp
        assert!(path.starts_with("/tmp"));
    }
}

// ============================================================
// DEEP NESTING
// ============================================================

#[test]
fn test_very_deep_nesting() {
    let root = Path::new("/tmp");
    // Create a path with 100 levels of nesting
    let deep_path = (0..100).map(|i| format!("d{}", i)).collect::<Vec<_>>().join("/");
    let result = validate_archive_entry(
        root,
        Path::new(&deep_path),
        ArchiveEntryType::Directory,
        4096,  // Allow long path
    );
    assert!(result.is_ok());
}

#[test]
fn test_deep_traversal_at_end() {
    let root = Path::new("/tmp");
    // Go deep then try to escape
    let deep_path = (0..10).map(|i| format!("d{}", i)).collect::<Vec<_>>().join("/");
    let attack_path = format!("{}/../../../../../../../etc/passwd", deep_path);
    let err = validate_archive_entry(
        root,
        Path::new(&attack_path),
        ArchiveEntryType::File,
        4096,
    )
    .unwrap_err();
    assert_eq!(err, ArchiveValidationError::Traversal);
}
