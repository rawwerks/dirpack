use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use dirpack::budget::BudgetTarget;
use dirpack::config::Config;
use dirpack::packer::pack;
use tempfile::TempDir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from("tests/fixtures").join(name)
}

fn pack_output(root: &Path, budget: BudgetTarget, mut config: Config, use_git: bool, include_signatures: bool) -> String {
    // Keep defaults but allow caller tweaks.
    if config.scanning.max_depth == 0 {
        config.scanning.max_depth = 20;
    }
    let result = pack(root, &config, budget, use_git, include_signatures);
    result.output
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

fn find_index(haystack: &str, needle: &str) -> Option<usize> {
    haystack.find(needle)
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

#[test]
fn test_single_file_directory() {
    let root = fixture_path("single_file_dir");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(500),
        Config::default(),
        false,
        true,
    );
    assert!(output.contains("foo:{bar.rs}"));
}

#[test]
fn test_spine_exceeds_budget() {
    let root = fixture_path("flat_many_files");
    let output = pack_output(
        &root,
        BudgetTarget::Bytes(200),
        Config::default(),
        false,
        false,
    );
    // With a tiny budget and a huge flat dir, tree segments should be omitted.
    assert!(!output.contains("file001.rs"));
}

#[test]
fn test_empty_directories_listed() {
    let temp = TempDir::new().expect("tempdir");
    let src = fixture_path("empty_dirs");
    copy_dir_recursive(&src, temp.path()).expect("copy fixture");
    // Remove the placeholder to create a truly empty directory.
    let placeholder = temp.path().join("empty").join(".gitkeep");
    let _ = fs::remove_file(placeholder);

    let output = pack_output(
        temp.path(),
        BudgetTarget::Tokens(500),
        Config::default(),
        false,
        false,
    );
    // Empty dir should appear in top-level dirs list.
    assert!(output.contains("dirs:{empty"));
}

#[test]
fn test_deep_nesting_priority_order() {
    let root = fixture_path("deep_nesting");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(1000),
        Config::default(),
        false,
        true,
    );
    let shallow_idx = find_index(&output, "shallow.rs:");
    let deep_idx = find_index(&output, "a/b/c/d/e/f/g/deep.rs:");
    assert!(shallow_idx.is_some(), "missing shallow signature");
    assert!(deep_idx.is_some(), "missing deep signature");
    assert!(shallow_idx < deep_idx);
}

#[test]
fn test_root_only_repo_tree_segment() {
    let root = fixture_path("root_only");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(500),
        Config::default(),
        false,
        true,
    );
    assert!(output.contains(".:{"));
    assert!(output.contains("main.rs"));
    assert!(output.contains("utils.rs"));
}

#[test]
fn test_monorepo_dirs_listed() {
    let root = fixture_path("monorepo");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(500),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("projA"));
    assert!(output.contains("projB"));
}

#[test]
fn test_no_extensions_included() {
    let root = fixture_path("no_extensions");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(500),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("Dockerfile"));
    assert!(output.contains("LICENSE"));
    assert!(output.contains("Makefile"));
}

#[test]
fn test_binary_only_handling() {
    let root = fixture_path("binary_only");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(500),
        Config::default(),
        false,
        true,
    );
    assert!(output.contains("image.png"));
    assert!(output.contains("doc.pdf"));
}

#[test]
fn test_config_heavy_included() {
    let root = fixture_path("config_heavy");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("config_01.json"));
    assert!(output.contains("config_01.yaml"));
    assert!(output.contains("app.toml"));
}

#[test]
fn test_generated_dirs_excluded() {
    let root = fixture_path("generated_files");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(500),
        Config::default(),
        true,
        false,
    );
    assert!(output.contains("main.rs"));
    assert!(!output.contains("node_modules"));
    assert!(!output.contains("dist"));
    assert!(!output.contains("build"));
}

#[test]
fn test_hidden_files_default_excluded() {
    let root = fixture_path("hidden_files");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(500),
        Config::default(),
        true,
        false,
    );
    assert!(output.contains("main.rs"));
    assert!(!output.contains(".env"));
    assert!(!output.contains(".github"));
}

#[test]
fn test_hidden_dirs_included_when_enabled() {
    let root = fixture_path("hidden_files");
    let mut config = Config::default();
    config.scanning.include_hidden = true;
    let output = pack_output(&root, BudgetTarget::Tokens(500), config, false, false);
    assert!(output.contains(".github"));
    // Security-sensitive files remain excluded even with include_hidden=true.
    assert!(!output.contains(".env"));
}

#[test]
fn test_special_names_encoded() {
    let root = fixture_path("special_names");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("name with spaces [v1].rs"));
    assert!(output.contains("quote'file.rs"));
    assert!(output.contains("brackets(1).rs"));
    assert!(output.contains("日本語.rs"));
}

#[test]
fn test_duplicate_names_disambiguated() {
    let root = fixture_path("duplicate_names");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(500),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("a:{utils.rs}"));
    assert!(output.contains("b:{utils.rs}"));
}

#[test]
fn test_minimum_budget_no_panic() {
    let root = fixture_path("root_only");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(10),
        Config::default(),
        false,
        false,
    );
    assert!(!output.is_empty());
}

#[test]
fn test_exact_fit_budget_bytes() {
    let root = fixture_path("root_only");
    let config = Config::default();
    let minimal = pack_output(&root, BudgetTarget::Bytes(80), config.clone(), false, false);
    let exact = pack_output(
        &root,
        BudgetTarget::Bytes(minimal.len()),
        config,
        false,
        false,
    );
    assert_eq!(exact, minimal);
}

#[test]
fn test_git_untracked_included_and_ignored_excluded() {
    if !git_available() {
        eprintln!("git not available; skipping");
        return;
    }

    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();

    Command::new("git")
        .arg("init")
        .current_dir(root)
        .output()
        .expect("git init");

    fs::write(root.join("tracked.rs"), "fn tracked() {}\n").expect("write tracked");
    fs::write(root.join("untracked.rs"), "fn untracked() {}\n").expect("write untracked");
    fs::create_dir_all(root.join("ignored")).expect("mkdir ignored");
    fs::write(root.join("ignored/ignored.rs"), "fn ignored() {}\n").expect("write ignored");
    fs::write(root.join(".gitignore"), "ignored/\n").expect("write gitignore");

    Command::new("git")
        .args(["add", "tracked.rs", ".gitignore"])
        .current_dir(root)
        .output()
        .expect("git add");

    let output = pack_output(root, BudgetTarget::Tokens(800), Config::default(), true, false);
    assert!(output.contains("tracked.rs"));
    assert!(output.contains("untracked.rs"));
    assert!(!output.contains("ignored"));
}

#[test]
fn test_no_git_mode_includes_ignored() {
    if !git_available() {
        eprintln!("git not available; skipping");
        return;
    }

    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();

    Command::new("git")
        .arg("init")
        .current_dir(root)
        .output()
        .expect("git init");

    fs::write(root.join("tracked.rs"), "fn tracked() {}\n").expect("write tracked");
    fs::create_dir_all(root.join("ignored")).expect("mkdir ignored");
    fs::write(root.join("ignored/ignored.rs"), "fn ignored() {}\n").expect("write ignored");
    fs::write(root.join(".gitignore"), "ignored/\n").expect("write gitignore");

    let mut config = Config::default();
    config.scanning.use_gitignore = false;

    let output = pack_output(root, BudgetTarget::Tokens(800), config, false, false);
    assert!(output.contains("ignored:{ignored.rs}"));
}
