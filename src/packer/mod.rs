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
use crate::format::pipe::PipeFormatter;
use crate::priority;
use crate::scanner;

use signatures::SignatureExtractor;

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

    // Create formatter
    let mut formatter = PipeFormatter::new(title, &root_str);
    formatter.set_entries(entries.clone());

    // Add the tree structure (Phase 1: SPINE)
    let tree_output = spine::format_tree_compact(&entries);
    budget.add(&tree_output);

    // Phase 2: SIGNATURES
    if include_signatures && config.signatures.enabled {
        if let Ok(mut extractor) = SignatureExtractor::new() {
            extractor.set_max_signature_length(config.signatures.max_signature_length);

            for file in &files_by_priority {
                if budget.is_exhausted() {
                    break;
                }

                if extractor.supports_extension(&file.extension) {
                    if let Ok(sigs) = extractor.extract_from_file(&file.path) {
                        // Estimate signature cost
                        let sig_text: String = sigs.iter().map(|s| s.compact()).collect();
                        if budget.would_fit(&sig_text) {
                            budget.add(&sig_text);
                            let rel_path = file.relative_path.to_string_lossy().to_string();
                            formatter.add_signatures(&rel_path, sigs);
                        }
                    }
                }
            }
        }
    }

    // Check for README to extract important note
    for file in &files_by_priority {
        let name = file.file_name().to_uppercase();
        if name.starts_with("README") {
            if let Some(content) = content::read_entry_content(file) {
                let summary = content::extract_summary(&content, 3);
                let first_line = summary.lines().next().unwrap_or("");
                if !first_line.is_empty() && first_line.len() < 100 {
                    formatter.set_important(first_line);
                }
                break;
            }
        }
    }

    let output = formatter.format();
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
