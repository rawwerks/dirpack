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
    let temp = TempDir::new().expect("tempdir");
    let proj_a = temp.path().join("projA/src");
    let proj_b = temp.path().join("projB/src");
    fs::create_dir_all(&proj_a).expect("mkdir projA");
    fs::create_dir_all(&proj_b).expect("mkdir projB");
    fs::write(proj_a.join("main.rs"), "fn main() {}\n").expect("write projA main");
    fs::write(proj_b.join("main.rs"), "fn main() {}\n").expect("write projB main");

    let output = pack_output(
        temp.path(),
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

#[test]
fn test_duplicate_utils_fixture() {
    let root = fixture_path("duplicate_utils");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("a:{utils.rs}"));
    assert!(output.contains("e:{utils.rs}"));
}

#[test]
fn test_empty_repo_fixture() {
    let root = fixture_path("empty_repo");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(200),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("[empty_repo]"));
    assert!(output.contains("root: tests/fixtures/empty_repo"));
}

#[test]
fn test_flat_1000_files_fixture() {
    let root = fixture_path("flat_1000_files");
    let output = pack_output(
        &root,
        BudgetTarget::Bytes(50_000),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("file_0001.txt"));
    assert!(output.contains("file_0200.txt"));
}

#[test]
fn test_long_names_fixture() {
    let root = fixture_path("long_names");
    let entry = fs::read_dir(&root)
        .expect("read long_names")
        .next()
        .expect("long_names entry")
        .expect("long_names entry");
    let name = entry.file_name().to_string_lossy().to_string();

    let output = pack_output(
        &root,
        BudgetTarget::Tokens(500),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains(&name));
}

#[test]
fn test_signatures_heavy_fixture() {
    let root = fixture_path("signatures_heavy");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        false,
        true,
    );
    assert!(output.contains("func1"));
    assert!(output.contains("func8"));
}

#[test]
fn test_small_project_fixture() {
    let root = fixture_path("small_project");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        false,
        true,
    );
    assert!(output.contains("src:{lib.rs"));
    assert!(output.contains("tests:{integration.rs}"));
}

#[test]
fn test_special_chars_fixture() {
    let root = fixture_path("special_chars");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("space name.rs"));
    assert!(output.contains("bracket[1].rs"));
    assert!(output.contains("quote'file.rs"));
    assert!(output.contains("double\"quote.rs"));
}

#[test]
fn test_spine_budget_fixture() {
    let root = fixture_path("spine_budget");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(1200),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("this_is_a_very_long_filename_alpha.rs"));
    assert!(output.contains("this_is_a_very_long_filename_gamma.rs"));
}

#[test]
fn test_submodule_like_fixture() {
    let root = fixture_path("submodule_like");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("submodule_repo"));
    assert!(output.contains("src:{lib.rs}"));
    assert!(output.contains(".git"));
}

#[test]
fn test_symlinks_fixture() {
    let root = fixture_path("symlinks");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(500),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("target.txt"));
    assert!(!output.contains("link.txt"));
    assert!(!output.contains("link_dir"));
}

#[test]
fn test_tree_heavy_fixture() {
    let root = fixture_path("tree_heavy");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(1200),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("data_001.txt"));
    assert!(output.contains("data_010.txt"));
}

#[test]
fn test_unicode_names_fixture() {
    let root = fixture_path("unicode_names");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("日本語.rs"));
    assert!(output.contains("emoji_😀.rs"));
}

#[test]
fn test_untracked_files_fixture() {
    let root = fixture_path("untracked_files");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(300),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("tracked.txt"));
}

#[test]
fn test_duplicate_utils_disambiguated() {
    let root = fixture_path("duplicate_utils");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("a:{utils.rs}"));
    assert!(output.contains("e:{utils.rs}"));
}

#[test]
fn test_empty_repo_has_no_tree_segments() {
    let root = fixture_path("empty_repo");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(200),
        Config::default(),
        false,
        false,
    );
    assert!(!output.is_empty());
    assert!(!output.contains(":{"));
}

#[test]
fn test_flat_1000_files_root_listing() {
    let root = fixture_path("flat_1000_files");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(2000),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("root:"));
    assert!(!output.contains("file_0001.txt"));
}

#[test]
fn test_long_names_preserved() {
    let root = fixture_path("long_names");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(500),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
}

#[test]
fn test_signatures_heavy_included() {
    let root = fixture_path("signatures_heavy");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(1200),
        Config::default(),
        false,
        true,
    );
    assert!(output.contains("src/lib.rs:pub fn func1()"));
    assert!(output.contains("pub fn func8()"));
}

#[test]
fn test_small_project_contains_core_files() {
    let root = fixture_path("small_project");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(1200),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("src:{lib.rs"));
    assert!(output.contains(".:{Cargo.toml"));
}

#[test]
fn test_special_chars_in_paths() {
    let root = fixture_path("special_chars");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("space name.rs"));
    assert!(output.contains("double\"quote.rs"));
}

#[test]
fn test_spine_budget_truncation() {
    let root = fixture_path("spine_budget");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(60),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("this_is_a_very_long_filename_alpha.rs"));
    assert!(!output.contains("this_is_a_very_long_filename_gamma.rs"));
}

#[test]
fn test_submodule_like_ignores_git_dir() {
    let root = fixture_path("submodule_like");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        true,
        false,
    );
    assert!(output.contains("submodule_repo/src:{lib.rs}"));
    assert!(!output.contains(".git"));
}

#[test]
fn test_symlinks_skipped() {
    let root = fixture_path("symlinks");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("target.txt"));
    assert!(!output.contains("link.txt"));
    assert!(!output.contains("link_dir"));
}

#[test]
fn test_tree_heavy_contains_main_and_data() {
    let root = fixture_path("tree_heavy");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(1500),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("main.rs"));
    assert!(output.contains("data_001.txt"));
}

#[test]
fn test_unicode_names_preserved() {
    let root = fixture_path("unicode_names");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(800),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("日本語.rs"));
    assert!(output.contains("emoji_😀.rs"));
}

#[test]
fn test_untracked_files_fixture_included() {
    let root = fixture_path("untracked_files");
    let output = pack_output(
        &root,
        BudgetTarget::Tokens(400),
        Config::default(),
        false,
        false,
    );
    assert!(output.contains("tracked.txt"));
}
