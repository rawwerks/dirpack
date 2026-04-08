//! Integration tests for the on-disk pack cache and max_file_size_bytes.

use std::fs;
use std::thread::sleep;
use std::time::Duration;

use tempfile::TempDir;

use dirpack::budget::BudgetTarget;
use dirpack::cache;
use dirpack::config::{Config, OutputFormat};
use dirpack::packer;
use dirpack::scanner;

/// Helper: create a small fixture directory with a known layout.
fn create_fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("README.md"), "# Fixture\nHello world\n").unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {}\nfn compute(x: i32) -> i32 { x + 1 }\n",
    )
    .unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "pub mod utils;\n").unwrap();
    dir
}

/// Helper: pack a fixture directory and return (output, budget_used).
fn pack(root: &std::path::Path, config: &Config) -> dirpack::packer::PackResult {
    packer::pack(
        root,
        config,
        BudgetTarget::Tokens(2000),
        false, // no git in temp dirs
        true,
        Some("."),
    )
}

#[test]
fn cache_hit_returns_identical_output() {
    let fixture = create_fixture();
    let config = Config::default();
    let budget = BudgetTarget::Tokens(2000);
    let format = OutputFormat::Pipe;

    // Scan
    let entries = scanner::scan(fixture.path(), &config, false);
    let key = cache::compute_key(
        fixture.path(),
        &entries,
        &config,
        budget,
        format,
        false,
        true,
        Some("."),
    );

    // Cold pack
    let result1 = pack(fixture.path(), &config);
    cache::write(&key, &result1);

    // Hot read
    let cached = cache::read(&key).expect("cache miss on immediate re-read");
    let result2 = cached.into_pack_result();

    assert_eq!(result1.output, result2.output);
    assert_eq!(result1.budget_used, result2.budget_used);
    assert_eq!(result1.budget_limit, result2.budget_limit);
    assert_eq!(result1.files_included, result2.files_included);
}

#[test]
fn cache_invalidates_on_file_change() {
    let fixture = create_fixture();
    let config = Config::default();
    let budget = BudgetTarget::Tokens(2000);
    let format = OutputFormat::Pipe;

    // Build key before modification
    let entries1 = scanner::scan(fixture.path(), &config, false);
    let key1 = cache::compute_key(
        fixture.path(),
        &entries1,
        &config,
        budget,
        format,
        false,
        true,
        Some("."),
    );

    let result1 = pack(fixture.path(), &config);
    cache::write(&key1, &result1);

    // Modify a file (sleep 1s to ensure mtime changes on filesystems with
    // second-granularity timestamps)
    sleep(Duration::from_millis(1100));
    fs::write(
        fixture.path().join("main.rs"),
        "fn main() { println!(\"changed\"); }\n",
    )
    .unwrap();

    // Rebuild key after modification
    let entries2 = scanner::scan(fixture.path(), &config, false);
    let key2 = cache::compute_key(
        fixture.path(),
        &entries2,
        &config,
        budget,
        format,
        false,
        true,
        Some("."),
    );

    // Keys must differ
    assert_ne!(key1.as_str(), key2.as_str());
    // Old key still returns old result
    assert!(cache::read(&key1).is_some());
    // New key has no cached entry yet
    assert!(cache::read(&key2).is_none());
}

#[test]
fn cache_invalidates_on_budget_change() {
    let fixture = create_fixture();
    let config = Config::default();
    let format = OutputFormat::Pipe;

    let entries = scanner::scan(fixture.path(), &config, false);

    let key_2k = cache::compute_key(
        fixture.path(),
        &entries,
        &config,
        BudgetTarget::Tokens(2000),
        format,
        false,
        true,
        Some("."),
    );
    let key_4k = cache::compute_key(
        fixture.path(),
        &entries,
        &config,
        BudgetTarget::Tokens(4000),
        format,
        false,
        true,
        Some("."),
    );

    assert_ne!(key_2k.as_str(), key_4k.as_str());
}

#[test]
fn max_file_size_skips_large_files_for_content_but_keeps_in_spine() {
    let dir = TempDir::new().unwrap();

    // Create a small file and a "large" file
    fs::write(dir.path().join("small.rs"), "fn small() {}\n").unwrap();
    // 3 MiB of text — above the 2 MiB default limit
    let big_content: String = "x".repeat(3 * 1024 * 1024);
    fs::write(dir.path().join("big.rs"), &big_content).unwrap();

    let config = Config::default();
    let result = packer::pack(
        dir.path(),
        &config,
        BudgetTarget::Tokens(10000),
        false,
        true,
        Some("."),
    );

    // The output should mention big.rs in the tree (spine)
    assert!(result.output.contains("big.rs"), "big.rs should appear in spine");
    // But should NOT contain signature from big.rs (it's above max_file_size_bytes)
    // The small.rs signature should be present
    assert!(
        result.output.contains("fn small()"),
        "small.rs signature should be extracted"
    );
    // big.rs should not have content included
    assert!(
        !result.output.contains(&"x".repeat(100)),
        "big.rs content should not be included"
    );
}

#[test]
fn max_file_size_zero_disables_limit() {
    let dir = TempDir::new().unwrap();

    // 3 MiB file
    let big_content = format!("fn big_func() {{}}\n{}", "// pad\n".repeat(200_000));
    fs::write(dir.path().join("big.rs"), &big_content).unwrap();

    let mut config = Config::default();
    config.scanning.max_file_size_bytes = 0; // disable limit

    let result = packer::pack(
        dir.path(),
        &config,
        BudgetTarget::Tokens(10000),
        false,
        true,
        Some("."),
    );

    // With limit disabled, signatures from big.rs should be extracted
    assert!(
        result.output.contains("fn big_func()"),
        "big.rs signatures should be extracted when limit is disabled"
    );
}
