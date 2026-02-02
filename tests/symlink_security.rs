use std::path::Path;

use dirpack::budget::BudgetTarget;
use dirpack::config::Config;
use dirpack::packer::pack;
use tempfile::TempDir;

fn pack_output(root: &Path, config: Config, use_git: bool) -> String {
    let result = pack(root, &config, BudgetTarget::Tokens(500), use_git, false, None);
    result.output
}

#[cfg(unix)]
fn make_symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) {
    std::os::unix::fs::symlink(src, dst).expect("create symlink");
}

#[cfg(unix)]
#[test]
fn test_symlink_to_passwd_skipped() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();

    std::fs::write(root.join("ok.rs"), "fn ok() {}\n").expect("write ok file");
    make_symlink("/etc/passwd", root.join("passwd_link"));

    let output = pack_output(root, Config::default(), false);
    assert!(output.contains("ok.rs"));
    assert!(!output.contains("passwd_link"));
}

#[cfg(unix)]
#[test]
fn test_symlink_chain_skipped() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();

    std::fs::write(root.join("target.rs"), "fn target() {}\n").expect("write target");
    make_symlink("target.rs", root.join("link2"));
    make_symlink("link2", root.join("link1"));

    let output = pack_output(root, Config::default(), false);
    assert!(output.contains("target.rs"));
    assert!(!output.contains("link1"));
    assert!(!output.contains("link2"));
}

#[cfg(unix)]
#[test]
fn test_relative_symlink_escape_skipped() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();

    let outside = temp.path().parent().unwrap().join("outside.txt");
    std::fs::write(&outside, "outside\n").expect("write outside");
    make_symlink("../outside.txt", root.join("escape"));

    let output = pack_output(root, Config::default(), false);
    assert!(!output.contains("escape"));
    let _ = std::fs::remove_file(outside);
}

#[cfg(unix)]
#[test]
fn test_follow_symlinks_forced_off() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();

    std::fs::create_dir_all(root.join("real_dir")).expect("mkdir real_dir");
    std::fs::write(root.join("real_dir/inside.rs"), "fn inside() {}\n").expect("write inside");
    make_symlink("real_dir", root.join("link_dir"));

    let mut config = Config::default();
    config.scanning.follow_symlinks = true;

    let output = pack_output(root, config, false);
    assert!(output.contains("real_dir:{inside.rs}"));
    assert!(!output.contains("link_dir"));
}
