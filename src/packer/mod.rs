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

use std::path::Path;

use crate::budget::{Budget, BudgetTarget};
use crate::config::Config;
use crate::priority;
use crate::scanner;
use crate::scanner::entry::FileEntry;

use signatures::SignatureExtractor;

// Tree budget ratio (30% to account for header overhead and ensure ≤40% in output)
const TREE_BUDGET_RATIO: f64 = 0.30;

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
        &mut segments,
        &mut budget,
        &mut tree_budget,
        &mut push_segment,
    );

    // Phase 2: SIGNATURES
    // Add signatures incrementally - fit as many as possible per file
    if include_signatures && config.signatures.enabled {
        if let Ok(mut extractor) = SignatureExtractor::new() {
            extractor.set_max_signature_length(config.signatures.max_signature_length);

            for file in &files_by_priority {
                if budget.is_exhausted() {
                    break;
                }

                if extractor.supports_extension(&file.extension) {
                    if let Ok(sigs) = extractor.extract_from_file(&file.path) {
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
                            let candidate = format!("|{}", test_segment);
                            if budget.would_fit(&candidate) {
                                sig_texts.push(compact);
                            } else {
                                break; // Can't fit more signatures from this file
                            }
                        }

                        // Add segment if we got any signatures
                        if !sig_texts.is_empty() {
                            let segment = format!("{}:{}", rel_path, sig_texts.join(","));
                            push_segment(&mut segments, &mut budget, segment);
                        }
                        // Continue to next file even if nothing fit
                    }
                }
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
    for (_depth, dirs_at_level) in dirs_by_depth {
        if tree_budget.is_exhausted() {
            break;
        }

        for (dir, files) in dirs_at_level {
            if tree_budget.is_exhausted() {
                break;
            }

            let dir_name = if dir.is_empty() { "." } else { &dir };
            if !files.is_empty() {
                let segment = format!(
                    "{}:{{{}}}",
                    dir_name,
                    files.into_iter().collect::<Vec<_>>().join(",")
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
