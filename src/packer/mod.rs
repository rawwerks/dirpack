//! Packer module for progressive disclosure algorithm.
//!
//! Phases:
//! 1. SPINE - Directory tree structure (always included)
//! 2. SIGNATURES - Code signatures for high-priority files
//! 3. SUMMARIES - Doc excerpts for README, etc.
//! 4. CONTENT - Full file contents (budget permitting)

pub mod content;
pub mod signatures;
pub mod spine;

use std::collections::HashMap;
use std::path::Path;

use crate::budget::{Budget, BudgetTarget};
use crate::config::Config;
use crate::priority;
use crate::scanner;
use crate::scanner::entry::{FileEntry, Representation};

use signatures::{extract_markdown_headings, SignatureExtractor};

// Tree budget ratio (30% to account for header overhead and ensure ≤40% in output)
const TREE_BUDGET_RATIO: f64 = 0.30;

// Tiered allocation budgets (applied to remaining budget after tree/header)
const STRUCTURE_BUDGET_RATIO: f64 = 0.40;
const SNIPPET_BUDGET_RATIO: f64 = 0.30;
const FULL_BUDGET_RATIO: f64 = 0.30;

// Max files per directory to reduce lopsidedness (spread coverage evenly)
const MAX_FILES_PER_DIR: usize = 5;

// Max lines for snippet representation
const SNIPPET_MAX_LINES: usize = 6;

/// Result of packing a directory.
pub struct PackResult {
    pub output: String,
    pub budget_used: usize,
    pub budget_limit: usize,
    pub files_included: usize,
}

/// Pack a directory into a budgeted index.
pub fn pack(
    root: &Path,
    config: &Config,
    budget_target: BudgetTarget,
    use_git: bool,
    include_signatures: bool,
) -> PackResult {
    let mut budget = Budget::new(budget_target);

    // Get project title from directory name
    let title = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");

    let root_str = root.to_string_lossy();

    // Phase 1: Scan directory
    let entries = scanner::scan(root, &config.scanning, use_git);
    let files: Vec<_> = entries.iter().filter(|e| !e.is_dir).cloned().collect();
    let mut files_by_priority = files.clone();

    // Sort by priority
    priority::sort_by_priority(
        &mut files_by_priority,
        &config.priority_rules,
        &config.categories,
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
    let tree_limit = (budget.limit() as f64 * TREE_BUDGET_RATIO).floor() as usize;
    let mut tree_budget = match budget.target {
        BudgetTarget::Tokens(_) => Budget::tokens(tree_limit),
        BudgetTarget::Bytes(_) => Budget::bytes(tree_limit),
    };
    add_tree_segments(
        &entries,
        &entry_point_names,
        &mut segments,
        &mut budget,
        &mut tree_budget,
        &mut push_segment,
    );

    // Phase 2-4: Tiered allocation (Structure -> Snippet -> Full)
    let remaining_limit = budget.remaining();
    if remaining_limit > 0 {
        let mut structure_limit =
            (remaining_limit as f64 * STRUCTURE_BUDGET_RATIO).floor() as usize;
        let mut snippet_limit =
            (remaining_limit as f64 * SNIPPET_BUDGET_RATIO).floor() as usize;
        let mut full_limit =
            (remaining_limit as f64 * FULL_BUDGET_RATIO).floor() as usize;
        let allocated = structure_limit + snippet_limit + full_limit;
        if allocated < remaining_limit {
            full_limit = full_limit.saturating_add(remaining_limit - allocated);
        } else if allocated > remaining_limit {
            let overflow = allocated - remaining_limit;
            if full_limit >= overflow {
                full_limit -= overflow;
            } else if snippet_limit >= overflow {
                snippet_limit -= overflow;
            } else if structure_limit >= overflow {
                structure_limit -= overflow;
            }
        }

        let mut structure_budget = new_sub_budget(&budget, structure_limit);
        let mut snippet_budget = new_sub_budget(&budget, snippet_limit);
        let mut full_budget = new_sub_budget(&budget, full_limit);

        let tiers = select_tiered_files(&files_by_priority);

        let push_with_sub_budget = |segments: &mut Vec<String>,
                                    budget: &mut Budget,
                                    sub_budget: &mut Budget,
                                    segment: String|
         -> bool {
            let candidate = if segments.is_empty() {
                segment.clone()
            } else {
                format!("|{}", segment)
            };
            if !sub_budget.would_fit(&candidate) {
                return false;
            }
            if budget.try_add(&candidate) {
                segments.push(segment);
                sub_budget.add(&candidate);
                true
            } else {
                false
            }
        };

        // Structure: signatures for top 20% priority files
        if include_signatures && config.signatures.enabled {
            if let Ok(mut extractor) = SignatureExtractor::new() {
                extractor.set_max_signature_length(config.signatures.max_signature_length);

                for file in &tiers.structure {
                    if budget.is_exhausted() || structure_budget.is_exhausted() {
                        break;
                    }

                    // Get signatures: markdown headings or code signatures
                    let sigs = if file.extension.eq_ignore_ascii_case("md") {
                        // Extract markdown headings
                        if let Some(content) = content::read_entry_content(file) {
                            extract_markdown_headings(&content)
                        } else {
                            continue;
                        }
                    } else if extractor.supports_extension(&file.extension) {
                        // Extract code signatures via tree-sitter
                        match extractor.extract_from_file(&file.path) {
                            Ok(s) => s,
                            Err(_) => continue,
                        }
                    } else {
                        continue;
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

                        let candidate = if segments.is_empty() {
                            test_segment.clone()
                        } else {
                            format!("|{}", test_segment)
                        };
                        if budget.would_fit(&candidate) && structure_budget.would_fit(&candidate) {
                            sig_texts.push(compact);
                        } else {
                            break;
                        }
                    }

                    if !sig_texts.is_empty() {
                        let segment = format!("{}:{}", rel_path, sig_texts.join(","));
                        let _ = push_with_sub_budget(
                            &mut segments,
                            &mut budget,
                            &mut structure_budget,
                            segment,
                        );
                    }
                }
            }
        }

        // Snippet: first N lines for top 10% priority files
        for file in &tiers.snippet {
            if budget.is_exhausted() || snippet_budget.is_exhausted() {
                break;
            }
            if let Some(content) = content::read_entry_content(file) {
                let summary = content::extract_summary(&content, SNIPPET_MAX_LINES);
                let snippet = sanitize_inline(&summary);
                if snippet.is_empty() {
                    continue;
                }
                let rel_path = file.relative_path.to_string_lossy().to_string();
                let segment = format!("{}:{}", rel_path, snippet);
                let _ = push_with_sub_budget(&mut segments, &mut budget, &mut snippet_budget, segment);
            }
        }

        // Full: complete content for top 5% priority files
        for file in &tiers.full {
            if budget.is_exhausted() || full_budget.is_exhausted() {
                break;
            }
            if let Some(content) = content::read_entry_content(file) {
                let full_text = sanitize_inline(&content);
                if full_text.is_empty() {
                    continue;
                }
                let rel_path = file.relative_path.to_string_lossy().to_string();
                let segment = format!("{}:{}", rel_path, full_text);
                let _ = push_with_sub_budget(&mut segments, &mut budget, &mut full_budget, segment);
            }
        }
    }

    let output = segments.join("|");
    let budget_used = budget.used;
    let budget_limit = budget.limit();

    PackResult {
        output,
        budget_used,
        budget_limit,
        files_included: files.len(),
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
    )
}

fn add_tree_segments(
    entries: &[FileEntry],
    entry_point_names: &std::collections::BTreeSet<String>,
    segments: &mut Vec<String>,
    budget: &mut Budget,
    tree_budget: &mut Budget,
    push_segment: &mut dyn FnMut(&mut Vec<String>, &mut Budget, String) -> bool,
) {
    use std::collections::{BTreeMap, BTreeSet};

    if tree_budget.limit() == 0 || tree_budget.remaining() == 0 {
        return;
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
            if entry_point_names.contains(&file_name) {
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

    // Ensure entry-point directories are emitted first
    for entry_dir in entry_point_dirs.iter() {
        let mut files_for_dir: Option<BTreeSet<String>> = None;
        for (_depth, dirs_at_level) in &dirs_by_depth {
            if let Some(files) = dirs_at_level.get(entry_dir) {
                files_for_dir = Some(files.clone());
                break;
            }
        }
        if let Some(files) = files_for_dir {
            if tree_budget.is_exhausted() {
                break;
            }
            let dir_name = if entry_dir.is_empty() { "." } else { entry_dir };
            let mut file_list = files.into_iter().collect::<Vec<_>>();
            file_list.sort_by(|a, b| {
                let a_pri = entry_point_names.contains(a);
                let b_pri = entry_point_names.contains(b);
                b_pri.cmp(&a_pri).then_with(|| a.cmp(b))
            });
            // Cap files per directory to reduce lopsidedness
            file_list.truncate(MAX_FILES_PER_DIR);
            let segment = format!("{}:{{{}}}", dir_name, file_list.join(","));
            if tree_budget.would_fit(&segment) {
                tree_budget.add(&segment);
                let _ = push_segment(segments, budget, segment);
                processed_dirs.insert(entry_dir.clone());
            }
        }
    }

    for (_depth, dirs_at_level) in dirs_by_depth {
        if tree_budget.is_exhausted() {
            break;
        }
        let mut dirs_vec: Vec<(String, BTreeSet<String>)> = dirs_at_level.into_iter().collect();
        dirs_vec.sort_by(|(a_dir, _), (b_dir, _)| {
            let a_pri = entry_point_dirs.contains(a_dir);
            let b_pri = entry_point_dirs.contains(b_dir);
            b_pri.cmp(&a_pri).then_with(|| a_dir.cmp(b_dir))
        });

        for (dir, files) in dirs_vec {
            if tree_budget.is_exhausted() {
                break;
            }
            if processed_dirs.contains(&dir) {
                continue;
            }

            let dir_name = if dir.is_empty() { "." } else { &dir };
            if !files.is_empty() {
                let mut file_list = files.into_iter().collect::<Vec<_>>();
                file_list.sort_by(|a, b| {
                    let a_pri = entry_point_names.contains(a);
                    let b_pri = entry_point_names.contains(b);
                    b_pri.cmp(&a_pri).then_with(|| a.cmp(b))
                });
                // Cap files per directory to reduce lopsidedness
                file_list.truncate(MAX_FILES_PER_DIR);
                let segment = format!(
                    "{}:{{{}}}",
                    dir_name,
                    file_list.join(",")
                );
                if tree_budget.would_fit(&segment) {
                    tree_budget.add(&segment);
                    if !push_segment(segments, budget, segment) {
                        break;
                    }
                }
            }
        }
    }
}

fn collect_entry_point_names(files: &[FileEntry]) -> std::collections::BTreeSet<String> {
    let candidates = [
        "Cargo.toml",
        "pyproject.toml",
        "package.json",
        "main.rs",
        "lib.rs",
        "index.ts",
        "index.tsx",
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

struct TieredFiles {
    structure: Vec<FileEntry>,
    snippet: Vec<FileEntry>,
    full: Vec<FileEntry>,
}

fn select_tiered_files(files: &[FileEntry]) -> TieredFiles {
    let total = files.len();
    if total == 0 {
        return TieredFiles {
            structure: Vec::new(),
            snippet: Vec::new(),
            full: Vec::new(),
        };
    }

    let mut count_full = ((total as f64) * 0.05).ceil() as usize;
    let mut count_snippet = ((total as f64) * 0.10).ceil() as usize;
    let mut count_structure = ((total as f64) * 0.20).ceil() as usize;

    count_full = count_full.max(1).min(total);
    count_snippet = count_snippet.max(count_full).min(total);
    count_structure = count_structure.max(count_snippet).min(total);

    let mut full_needed = count_full;
    let mut snippet_needed = count_snippet.saturating_sub(count_full);
    let mut structure_needed = count_structure.saturating_sub(count_snippet);

    let mut dir_counts: HashMap<String, usize> = HashMap::new();
    let mut full = Vec::new();
    let mut snippet = Vec::new();
    let mut structure = Vec::new();

    for entry in files {
        if full_needed == 0 && snippet_needed == 0 && structure_needed == 0 {
            break;
        }

        let tier = if full_needed > 0 {
            Representation::Full
        } else if snippet_needed > 0 {
            Representation::Snippet
        } else if structure_needed > 0 {
            Representation::Structure
        } else {
            Representation::NameOnly
        };

        if tier == Representation::NameOnly {
            continue;
        }

        let dir = parent_dir(entry);
        let count = dir_counts.get(&dir).copied().unwrap_or(0);
        if count >= MAX_FILES_PER_DIR {
            continue;
        }

        dir_counts.insert(dir, count + 1);
        let mut cloned = entry.clone();
        cloned.representation = tier;

        match tier {
            Representation::Full => {
                full.push(cloned);
                full_needed = full_needed.saturating_sub(1);
            }
            Representation::Snippet => {
                snippet.push(cloned);
                snippet_needed = snippet_needed.saturating_sub(1);
            }
            Representation::Structure => {
                structure.push(cloned);
                structure_needed = structure_needed.saturating_sub(1);
            }
            Representation::NameOnly => {}
        }
    }

    TieredFiles {
        structure,
        snippet,
        full,
    }
}

fn parent_dir(entry: &FileEntry) -> String {
    entry
        .relative_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn sanitize_inline(input: &str) -> String {
    input
        .replace('|', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn new_sub_budget(budget: &Budget, limit: usize) -> Budget {
    match budget.target {
        BudgetTarget::Tokens(_) => Budget::tokens(limit),
        BudgetTarget::Bytes(_) => Budget::bytes(limit),
    }
}
