use std::path::{Path, PathBuf};

use dirpack::budget::BudgetTarget;
use dirpack::config::Config;
use dirpack::packer;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn pack_fixture(name: &str, budget: BudgetTarget) -> packer::PackResult {
    let root = fixture_path(name);
    let config = Config::default();
    packer::pack(&root, &config, budget, true, true)
}

#[test]
fn test_single_file_directory() {
    // Single file under a subdir should be listed in the tree output.
    let result = pack_fixture("single_file", BudgetTarget::Bytes(256));
    assert!(result.output.contains("sub:{only.rs}"));
}

#[test]
fn test_very_deep_nesting() {
    // Deep paths should not crash and should mention the deep file.
    let result = pack_fixture("deep", BudgetTarget::Bytes(512));
    assert!(result.output.contains("deep.rs"));
}

#[test]
fn test_minimal_budget() {
    // Tiny budgets should still emit a header without crashing.
    let result = pack_fixture("single_file", BudgetTarget::Tokens(5));
    assert!(result.output.starts_with('['));
    assert!(result.budget_used <= result.budget_limit);
}

#[test]
fn test_empty_directory() {
    // Empty repos should produce a minimal header-only output.
    let root = fixture_path("empty_repo");
    let config = Config::default();
    let result = packer::pack(&root, &config, BudgetTarget::Bytes(128), true, true);

    assert!(result.output.contains("[empty_repo]"));
    assert!(result.output.contains("root: "));
}
