//! Packer module for progressive disclosure algorithm.
//!
//! Phases:
//! 1. SPINE - Directory tree structure (always included)
//! 2. SIGNATURES - Code signatures for high-priority files
//! 3. SUMMARIES - Doc excerpts for README, etc.
//! 4. CONTENT - Full file contents (budget permitting)
//!
//! ## History
//!
//! This module was reverted to v3-incremental after experiments with tiered
//! allocation (v4-v5) caused quality regressions. See POSTMORTEM.md and
//! beads issue dirpack-ct4 for the full evolution timeline.
//!
//! The tiered allocation concept (Structure/Snippet/Full tiers) has merit
//! but the implementation was flawed. Future work should:
//! - Keep full file listings (don't cap per-directory)
//! - Prioritize docstrings/intent over truncated signatures
//! - Validate with dogfood evals before merging

pub mod content;
pub mod signatures;
pub mod spine;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;
use std::sync::{Condvar, Mutex, OnceLock};

use crate::budget::{Budget, BudgetTarget};
use crate::config::Config;
use crate::error::{DirpackError, Result};
use crate::priority;
use crate::scanner;
use crate::scanner::entry::FileEntry;

use signatures::SignatureExtractor;

// Tree budget ratio (30% to account for header overhead and ensure ≤40% in output)
const TREE_BUDGET_RATIO: f64 = 0.30;
const PACK_CONCURRENCY_ENV: &str = "DIRPACK_PACK_CONCURRENCY_LIMIT";
const PACK_RETRY_AFTER_ENV: &str = "DIRPACK_PACK_RETRY_AFTER_SECS";
const DEFAULT_RETRY_AFTER_SECS: u64 = 1;

static PACK_SEMAPHORE: OnceLock<PackSemaphore> = OnceLock::new();

struct PackSemaphore {
    limit: usize,
    retry_after_secs: u64,
    in_flight: Mutex<usize>,
    cvar: Condvar,
}

impl PackSemaphore {
    fn new() -> Self {
        let limit = read_env_usize(PACK_CONCURRENCY_ENV)
            .filter(|value| *value > 0)
            .unwrap_or_else(default_pack_limit);
        let retry_after_secs = read_env_u64(PACK_RETRY_AFTER_ENV)
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_RETRY_AFTER_SECS);

        Self {
            limit,
            retry_after_secs,
            in_flight: Mutex::new(0),
            cvar: Condvar::new(),
        }
    }

    fn try_acquire(&'static self) -> Option<PackPermit> {
        let mut in_flight = self.in_flight.lock().expect("pack semaphore poisoned");
        if *in_flight >= self.limit {
            return None;
        }
        *in_flight += 1;
        Some(PackPermit { semaphore: self })
    }

    fn acquire(&'static self) -> PackPermit {
        let mut in_flight = self.in_flight.lock().expect("pack semaphore poisoned");
        while *in_flight >= self.limit {
            in_flight = self.cvar.wait(in_flight).expect("pack semaphore poisoned");
        }
        *in_flight += 1;
        PackPermit { semaphore: self }
    }
}

struct PackPermit {
    semaphore: &'static PackSemaphore,
}

impl Drop for PackPermit {
    fn drop(&mut self) {
        let mut in_flight = self
            .semaphore
            .in_flight
            .lock()
            .expect("pack semaphore poisoned");
        *in_flight = in_flight.saturating_sub(1);
        self.semaphore.cvar.notify_one();
    }
}

fn pack_semaphore() -> &'static PackSemaphore {
    PACK_SEMAPHORE.get_or_init(PackSemaphore::new)
}

fn read_env_usize(name: &str) -> Option<usize> {
    std::env::var(name).ok().and_then(|value| value.parse().ok())
}

fn read_env_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|value| value.parse().ok())
}

fn default_pack_limit() -> usize {
    std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
        .max(1)
}

/// Truncation information showing what was cut.
#[derive(Debug, Default)]
pub struct TruncationInfo {
    /// Total files scanned
    pub files_scanned: usize,
    /// Files listed in tree segments
    pub files_in_tree: usize,
    /// Files with signatures added
    pub files_with_signatures: usize,
    /// Directories that didn't fit in tree budget
    pub dirs_truncated: usize,
}

impl TruncationInfo {
    /// Returns true if anything was truncated.
    pub fn has_truncation(&self) -> bool {
        self.files_in_tree < self.files_scanned || self.dirs_truncated > 0
    }

    /// Format as a compact indicator for the output.
    pub fn format_indicator(&self) -> Option<String> {
        if !self.has_truncation() {
            return None;
        }

        let files_hidden = self.files_scanned.saturating_sub(self.files_in_tree);
        if files_hidden > 0 {
            let label = if files_hidden == 1 { "file" } else { "files" };
            Some(format!("[+{} more {} truncated]", files_hidden, label))
        } else if self.dirs_truncated > 0 {
            let label = if self.dirs_truncated == 1 { "dir" } else { "dirs" };
            Some(format!("[+{} more {} truncated]", self.dirs_truncated, label))
        } else {
            None
        }
    }
}

/// Result of packing a directory.
pub struct PackResult {
    pub output: String,
    pub budget_used: usize,
    pub budget_limit: usize,
    pub files_included: usize,
    pub truncation: TruncationInfo,
}

// Tokens reserved for truncation indicator (only for larger budgets)
const TRUNCATION_INDICATOR_RESERVE: usize = 15;
const MIN_BUDGET_FOR_RESERVE: usize = 200;

/// Pack a directory into a budgeted index (blocking if pack slots are full).
pub fn pack(
    root: &Path,
    config: &Config,
    budget_target: BudgetTarget,
    use_git: bool,
    include_signatures: bool,
    root_label: Option<&str>,
) -> PackResult {
    let _permit = pack_semaphore().acquire();
    pack_impl(
        root,
        config,
        budget_target,
        use_git,
        include_signatures,
        root_label,
    )
}

/// Pack a directory into a budgeted index (non-blocking when saturated).
pub fn try_pack(
    root: &Path,
    config: &Config,
    budget_target: BudgetTarget,
    use_git: bool,
    include_signatures: bool,
    root_label: Option<&str>,
) -> Result<PackResult> {
    let semaphore = pack_semaphore();
    let _permit = semaphore.try_acquire().ok_or(DirpackError::PackBusy {
        retry_after_secs: semaphore.retry_after_secs,
    })?;
    Ok(pack_impl(
        root,
        config,
        budget_target,
        use_git,
        include_signatures,
        root_label,
    ))
}

fn pack_impl(
    root: &Path,
    config: &Config,
    budget_target: BudgetTarget,
    use_git: bool,
    include_signatures: bool,
    root_label: Option<&str>,
) -> PackResult {
    // Reserve space for truncation indicator (only for larger budgets)
    let effective_target = match budget_target {
        BudgetTarget::Tokens(t) if t >= MIN_BUDGET_FOR_RESERVE => {
            BudgetTarget::Tokens(t.saturating_sub(TRUNCATION_INDICATOR_RESERVE))
        }
        BudgetTarget::Bytes(b) if b >= MIN_BUDGET_FOR_RESERVE => {
            BudgetTarget::Bytes(b.saturating_sub(TRUNCATION_INDICATOR_RESERVE * 4))
        }
        other => other, // No reserve for small budgets
    };
    let mut budget = Budget::new(effective_target);

    // Get project title from directory name
    let title = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");

    let root_str = root_label
        .map(|label| label.to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());

    // Phase 1: Scan directory
    let entries = scanner::scan(root, config, use_git);
    let files: Vec<_> = entries.iter().filter(|e| !e.is_dir).cloned().collect();
    let mut files_by_priority = files.clone();

    // Sort by priority
    priority::sort_by_priority(
        &mut files_by_priority,
        &config.priority_rules,
        &config.categories,
        &config.priority,
    );
    let entry_point_names = collect_entry_point_names(&files_by_priority);

    // Check for README to extract important note
    let mut important_note = None;
    for file in &files_by_priority {
        let name = file.file_name().to_uppercase();
        if name.starts_with("README") {
            if let Some(content) = content::read_entry_content(file) {
                let summary = content::extract_summary(&content, 3);
                let first_line = summary.lines().next().unwrap_or("").trim();
                if !first_line.is_empty() && first_line.len() < 100 {
                    important_note = Some(first_line.to_string());
                }
                break;
            }
        }
    }

    let mut segments: Vec<String> = Vec::new();

    let mut push_segment = |segments: &mut Vec<String>, budget: &mut Budget, segment: String| -> bool {
        let candidate = if segments.is_empty() {
            segment.clone()
        } else {
            format!("|{}", segment)
        };
        if budget.try_add(&candidate) {
            segments.push(segment);
            true
        } else {
            false
        }
    };

    // Header segments
    let _ = push_segment(&mut segments, &mut budget, format!("[{}]", title));
    let _ = push_segment(&mut segments, &mut budget, format!("root: {}", root_str));
    if let Some(note) = &important_note {
        let _ = push_segment(
            &mut segments,
            &mut budget,
            format!("IMPORTANT: {}", note),
        );
    }

    // Tree segments (Phase 1: SPINE) with budget ratio cap
    let tree_limit = match budget.target {
        BudgetTarget::Tokens(_) => (budget.limit() as f64 * TREE_BUDGET_RATIO).floor() as usize,
        BudgetTarget::Bytes(_) => budget.limit(),
    };
    let mut tree_budget = match budget.target {
        BudgetTarget::Tokens(_) => Budget::tokens(tree_limit),
        BudgetTarget::Bytes(_) => Budget::bytes(tree_limit),
    };

    // Track truncation info
    let mut truncation = TruncationInfo {
        files_scanned: files.len(),
        ..Default::default()
    };

    let tree_stats = add_tree_segments(
        &entries,
        &entry_point_names,
        &mut segments,
        &mut budget,
        &mut tree_budget,
        &mut push_segment,
    );
    truncation.files_in_tree = tree_stats.files_shown;
    truncation.dirs_truncated = tree_stats.dirs_skipped;

    // Phase 2: SIGNATURES
    // Add signatures incrementally - fit as many as possible per file
    if include_signatures && config.signatures.enabled {
        if let Ok(mut extractor) = SignatureExtractor::new() {
            extractor.set_max_signature_length(config.signatures.max_signature_length);

            let signature_budget_limit = budget.remaining();
            if signature_budget_limit > 0 {
                let budget_target = budget.target;
                let make_budget = |limit| match budget_target {
                    BudgetTarget::Tokens(_) => Budget::tokens(limit),
                    BudgetTarget::Bytes(_) => Budget::bytes(limit),
                };

                let mut files_by_top_dir: BTreeMap<String, VecDeque<FileEntry>> = BTreeMap::new();
                let mut top_dir_order: Vec<String> = Vec::new();
                let mut seen_top_dirs: BTreeSet<String> = BTreeSet::new();

                for file in &files_by_priority {
                    let top_dir = top_level_dir(&file.relative_path);
                    if seen_top_dirs.insert(top_dir.clone()) {
                        top_dir_order.push(top_dir.clone());
                    }
                    files_by_top_dir
                        .entry(top_dir)
                        .or_default()
                        .push_back(file.clone());
                }

                let num_top_dirs = top_dir_order.len();
                if num_top_dirs > 0 {
                    let mut per_dir_budget_limit = signature_budget_limit / num_top_dirs;
                    if per_dir_budget_limit == 0 {
                        // Budget too small to split fairly; fall back to round-robin without caps.
                        per_dir_budget_limit = signature_budget_limit;
                    }
                    let mut per_dir_budget: BTreeMap<String, Budget> = BTreeMap::new();
                    for dir in &top_dir_order {
                        per_dir_budget.insert(dir.clone(), make_budget(per_dir_budget_limit));
                    }

                    let mut dir_queue: VecDeque<String> = top_dir_order.into();
                    while let Some(dir) = dir_queue.pop_front() {
                        if budget.is_exhausted() {
                            break;
                        }

                        let dir_budget_exhausted = per_dir_budget
                            .get(&dir)
                            .map(|b| b.is_exhausted())
                            .unwrap_or(true);
                        if dir_budget_exhausted {
                            continue;
                        }

                        if let Some(queue) = files_by_top_dir.get_mut(&dir) {
                            while let Some(file) = queue.pop_front() {
                                if !extractor.supports_extension(&file.extension) {
                                    continue;
                                }

                                let sigs = match extractor.extract_from_file(&file.path) {
                                    Ok(sigs) => sigs,
                                    Err(_) => continue,
                                };
                                if sigs.is_empty() {
                                    continue;
                                }

                                let rel_path = file.relative_path.to_string_lossy().to_string();

                                // Try to fit signatures incrementally
                                let mut sig_texts: Vec<String> = Vec::new();
                                for sig in &sigs {
                                    let compact = sig.compact();
                                    let test_segment = if sig_texts.is_empty() {
                                        format!("{}:{}", rel_path, compact)
                                    } else {
                                        format!("{}:{},{}", rel_path, sig_texts.join(","), compact)
                                    };

                                    // Check if this segment would fit
                                    let candidate = if segments.is_empty() {
                                        test_segment.clone()
                                    } else {
                                        format!("|{}", test_segment)
                                    };
                                    let can_fit = budget.would_fit(&candidate)
                                        && per_dir_budget
                                            .get(&dir)
                                            .map(|b| b.would_fit(&candidate))
                                            .unwrap_or(false);
                                    if can_fit {
                                        sig_texts.push(compact);
                                    } else {
                                        break; // Can't fit more signatures from this file
                                    }
                                }

                                // Add segment if we got any signatures
                                if sig_texts.is_empty() {
                                    continue;
                                }

                                let segment = format!("{}:{}", rel_path, sig_texts.join(","));
                                let candidate = if segments.is_empty() {
                                    segment.clone()
                                } else {
                                    format!("|{}", segment)
                                };
                                if push_segment(&mut segments, &mut budget, segment) {
                                    if let Some(b) = per_dir_budget.get_mut(&dir) {
                                        b.try_add(&candidate);
                                    }
                                    truncation.files_with_signatures += 1;
                                }
                                break; // One file per round for this dir
                            }
                        }

                        // Re-queue dir for round-robin if budget and files remain
                        let dir_budget_exhausted = per_dir_budget
                            .get(&dir)
                            .map(|b| b.is_exhausted())
                            .unwrap_or(true);
                        let has_more_files = files_by_top_dir
                            .get(&dir)
                            .map(|q| !q.is_empty())
                            .unwrap_or(false);
                        if !budget.is_exhausted() && !dir_budget_exhausted && has_more_files {
                            dir_queue.push_back(dir);
                        }
                    }
                }
            }
        }
    }

    // Phase 3: CONTENT
    if !budget.is_exhausted() {
        for file in &files_by_priority {
            if budget.is_exhausted() {
                break;
            }

            let Some(content_text) = content::read_entry_content(file) else {
                continue;
            };

            let rel_path = file.relative_path.to_string_lossy().to_string();
            let safe_content = sanitize_content_for_pipe(&content_text);
            let segment = format!("CONTENT:{}\n```\n{}\n```", rel_path, safe_content);

            if !push_segment(&mut segments, &mut budget, segment) {
                // Try the next file in case a smaller one fits.
                continue;
            }
        }
    }

    // Add truncation indicator if anything was cut
    // Space was reserved upfront via TRUNCATION_INDICATOR_RESERVE
    if let Some(indicator) = truncation.format_indicator() {
        segments.push(indicator);
    }

    let output = segments.join("|");

    // Report actual usage against original limit (not the reduced one)
    let original_limit = match budget_target {
        BudgetTarget::Tokens(t) => t,
        BudgetTarget::Bytes(b) => b,
    };

    PackResult {
        output,
        budget_used: budget.used,
        budget_limit: original_limit,
        files_included: files.len(),
        truncation,
    }
}

/// Pack with default configuration.
pub fn pack_default(root: &Path, target_tokens: usize) -> PackResult {
    let config = Config::default();
    pack(
        root,
        &config,
        BudgetTarget::Tokens(target_tokens),
        true,
        true,
        None,
    )
}

/// Statistics from tree segment generation.
struct TreeStats {
    files_shown: usize,
    dirs_skipped: usize,
}

fn add_tree_segments(
    entries: &[FileEntry],
    entry_point_names: &std::collections::BTreeSet<String>,
    segments: &mut Vec<String>,
    budget: &mut Budget,
    tree_budget: &mut Budget,
    push_segment: &mut dyn FnMut(&mut Vec<String>, &mut Budget, String) -> bool,
) -> TreeStats {
    use std::collections::{BTreeMap, BTreeSet};

    let mut stats = TreeStats {
        files_shown: 0,
        dirs_skipped: 0,
    };

    if tree_budget.limit() == 0 || tree_budget.remaining() == 0 {
        // Count all non-dir entries as not shown
        stats.files_shown = 0;
        return stats;
    }

    // Group files by parent directory, tracking depth for priority
    let mut dirs_by_depth: BTreeMap<usize, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    let mut top_level_dirs: BTreeSet<String> = BTreeSet::new();
    let mut entry_point_dirs: BTreeSet<String> = BTreeSet::new();

    for entry in entries {
        let parent = entry
            .relative_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let file_name = entry.file_name().to_string();
        let depth = if parent.is_empty() {
            0
        } else {
            parent.matches('/').count() + 1
        };

        if entry.is_dir {
            if entry.depth == 0 {
                top_level_dirs.insert(file_name);
            }
        } else {
            // Only add to entry_point_dirs if NOT a test/fixture directory
            if entry_point_names.contains(&file_name) && !is_test_or_fixture_path(&parent) {
                entry_point_dirs.insert(parent.clone());
            }
            dirs_by_depth
                .entry(depth)
                .or_default()
                .entry(parent)
                .or_default()
                .insert(file_name);
        }
    }

    // Add top-level dirs listing first
    if !top_level_dirs.is_empty() {
        let segment = format!(
            "dirs:{{{}}}",
            top_level_dirs.into_iter().collect::<Vec<_>>().join(",")
        );
        if tree_budget.would_fit(&segment) {
            tree_budget.add(&segment);
            let _ = push_segment(segments, budget, segment);
        }
    }

    // Add directories level by level (shallow first) until tree budget exhausted
    let mut processed_dirs: BTreeSet<String> = BTreeSet::new();

    // Ensure entry-point directories are emitted first (but deprioritize test/fixture dirs)
    let mut sorted_entry_dirs: Vec<&String> = entry_point_dirs.iter().collect();
    sorted_entry_dirs.sort_by(|a, b| {
        let a_fixture = is_test_or_fixture_path(a);
        let b_fixture = is_test_or_fixture_path(b);
        let a_core = is_core_dir(a);
        let b_core = is_core_dir(b);
        // Deprioritize test/fixture, prioritize core dirs
        a_fixture
            .cmp(&b_fixture)
            .then_with(|| b_core.cmp(&a_core))
            .then_with(|| a.cmp(b))
    });
    for entry_dir in sorted_entry_dirs {
        let mut files_for_dir: Option<BTreeSet<String>> = None;
        for (_depth, dirs_at_level) in &dirs_by_depth {
            if let Some(files) = dirs_at_level.get(entry_dir) {
                files_for_dir = Some(files.clone());
                break;
            }
        }
        if let Some(files) = files_for_dir {
            if tree_budget.is_exhausted() {
                stats.dirs_skipped += 1;
                break;
            }
            let dir_name = if entry_dir.is_empty() { "." } else { entry_dir };
            let mut file_list = files.into_iter().collect::<Vec<_>>();
            let file_count = file_list.len();
            file_list.sort_by(|a, b| {
                let a_pri = entry_point_names.contains(a);
                let b_pri = entry_point_names.contains(b);
                b_pri.cmp(&a_pri).then_with(|| a.cmp(b))
            });
            let segment = format!("{}:{{{}}}", dir_name, file_list.join(","));
            if tree_budget.would_fit(&segment) {
                tree_budget.add(&segment);
                let _ = push_segment(segments, budget, segment);
                processed_dirs.insert(entry_dir.clone());
                stats.files_shown += file_count;
            } else {
                stats.dirs_skipped += 1;
            }
        }
    }

    for (_depth, dirs_at_level) in dirs_by_depth {
        if tree_budget.is_exhausted() {
            // Count remaining dirs as skipped
            stats.dirs_skipped += dirs_at_level.len();
            continue;
        }
        let mut dirs_vec: Vec<(String, BTreeSet<String>)> = dirs_at_level.into_iter().collect();
        dirs_vec.sort_by(|(a_dir, _), (b_dir, _)| {
            // Priority order: entry point dirs > core dirs > other dirs > test/fixture dirs
            let a_entry = entry_point_dirs.contains(a_dir);
            let b_entry = entry_point_dirs.contains(b_dir);
            let a_fixture = is_test_or_fixture_path(a_dir);
            let b_fixture = is_test_or_fixture_path(b_dir);
            let a_core = is_core_dir(a_dir);
            let b_core = is_core_dir(b_dir);

            // Entry point dirs first
            b_entry
                .cmp(&a_entry)
                // Then deprioritize test/fixture dirs
                .then_with(|| a_fixture.cmp(&b_fixture))
                // Then prioritize core dirs (src/, lib/, etc.)
                .then_with(|| b_core.cmp(&a_core))
                // Finally alphabetically
                .then_with(|| a_dir.cmp(b_dir))
        });

        for (dir, files) in dirs_vec {
            if tree_budget.is_exhausted() {
                stats.dirs_skipped += 1;
                continue;
            }
            if processed_dirs.contains(&dir) {
                continue;
            }

            let dir_name = if dir.is_empty() { "." } else { &dir };
            if !files.is_empty() {
                let file_count = files.len();
                let mut file_list = files.into_iter().collect::<Vec<_>>();
                file_list.sort_by(|a, b| {
                    let a_pri = entry_point_names.contains(a);
                    let b_pri = entry_point_names.contains(b);
                    b_pri.cmp(&a_pri).then_with(|| a.cmp(b))
                });
                let segment = format!(
                    "{}:{{{}}}",
                    dir_name,
                    file_list.join(",")
                );
                if tree_budget.would_fit(&segment) {
                    tree_budget.add(&segment);
                    if push_segment(segments, budget, segment) {
                        stats.files_shown += file_count;
                    } else {
                        stats.dirs_skipped += 1;
                        break;
                    }
                } else {
                    stats.dirs_skipped += 1;
                }
            }
        }
    }

    stats
}

fn collect_entry_point_names(files: &[FileEntry]) -> std::collections::BTreeSet<String> {
    let candidates = [
        "Cargo.toml",
        "pyproject.toml",
        "package.json",
        "main.rs",
        "lib.rs",
        "main.go",
        "app.go",
        "server.go",
        "index.ts",
        "index.tsx",
        "main.ts",
        "main.js",
        "index.js",
        "cli.js",
        "main.py",
        "app.py",
        "__init__.py",
    ];
    let mut names = std::collections::BTreeSet::new();
    for file in files {
        let name = file.file_name();
        if candidates.contains(&name) {
            names.insert(name.to_string());
        }
    }
    names
}

/// Check if a directory path is test or fixture related.
fn is_test_or_fixture_path(dir: &str) -> bool {
    let lower = dir.to_ascii_lowercase();
    lower.contains("test")
        || lower.contains("fixture")
        || lower.contains("mock")
        || lower.contains("spec")
        || lower.contains("e2e")
        || lower.contains("testdata")
}

/// Check if a directory path is a core source directory (not inside tests/fixtures).
fn is_core_dir(dir: &str) -> bool {
    // Exclude test/fixture paths from being considered "core"
    if is_test_or_fixture_path(dir) {
        return false;
    }
    let lower = dir.to_ascii_lowercase();
    // Check if path starts with or contains these core dirs
    lower.starts_with("src")
        || lower.starts_with("lib")
        || lower.starts_with("cmd")
        || lower.starts_with("pkg")
        || lower.starts_with("internal")
        || lower.contains("/src")
        || lower.contains("/lib")
        || lower.contains("/cmd")
        || lower.contains("/pkg")
        || lower.contains("/internal")
}

fn top_level_dir(path: &Path) -> String {
    let mut components = path.components();
    let first = components.next();
    let has_second = components.next().is_some();
    match first {
        None => ".".to_string(),
        Some(component) => {
            if has_second {
                component.as_os_str().to_string_lossy().to_string()
            } else {
                ".".to_string()
            }
        }
    }
}

fn sanitize_content_for_pipe(content: &str) -> String {
    content.replace('|', "<PIPE>")
}
