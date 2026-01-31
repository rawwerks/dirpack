//! Directory tree structure generation.

use std::collections::{BTreeMap, BTreeSet};

use crate::budget::Budget;
use crate::scanner::entry::FileEntry;

/// Maximum fraction of budget that spine can use (40%).
pub const SPINE_BUDGET_FRACTION: f64 = 0.40;

/// A node in the directory tree.
#[derive(Debug, Default)]
pub struct TreeNode {
    pub name: String,
    pub is_dir: bool,
    pub children: BTreeMap<String, TreeNode>,
}

impl TreeNode {
    pub fn new(name: &str, is_dir: bool) -> Self {
        Self {
            name: name.to_string(),
            is_dir,
            children: BTreeMap::new(),
        }
    }
}

/// Build a tree structure from file entries.
pub fn build_tree(entries: &[FileEntry]) -> TreeNode {
    let mut root = TreeNode::new(".", true);

    for entry in entries {
        let components: Vec<_> = entry.relative_path.components().collect();
        let mut current = &mut root;

        for (i, component) in components.iter().enumerate() {
            let name = component.as_os_str().to_string_lossy().to_string();
            let is_last = i == components.len() - 1;
            let is_dir = if is_last { entry.is_dir } else { true };

            current = current
                .children
                .entry(name.clone())
                .or_insert_with(|| TreeNode::new(&name, is_dir));
        }
    }

    root
}

/// Generate a compact tree representation for pipe-delimited output.
/// Format: dirs:{dir1,dir2}|dir1:{file1,file2}|...
pub fn format_tree_compact(entries: &[FileEntry]) -> String {
    let mut parts = Vec::new();

    // Group files by parent directory
    let mut dirs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut top_level_dirs: BTreeSet<String> = BTreeSet::new();

    for entry in entries {
        let parent = entry
            .relative_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let file_name = entry.file_name().to_string();

        if entry.is_dir {
            // Track directories
            if entry.depth == 0 {
                top_level_dirs.insert(file_name);
            }
        } else {
            // Track files in their parent directory
            dirs.entry(parent).or_default().insert(file_name);
        }
    }

    // Add top-level dirs
    if !top_level_dirs.is_empty() {
        parts.push(format!("dirs:{{{}}}", top_level_dirs.into_iter().collect::<Vec<_>>().join(",")));
    }

    // Add files grouped by directory
    for (dir, files) in dirs {
        let dir_name = if dir.is_empty() { "." } else { &dir };
        if !files.is_empty() {
            parts.push(format!(
                "{}:{{{}}}",
                dir_name,
                files.into_iter().collect::<Vec<_>>().join(",")
            ));
        }
    }

    parts.join("|")
}

/// Generate a budget-aware compact tree representation.
/// Stops adding directories when spine budget (40% of total) is exhausted.
/// Prioritizes shallow directories over deep ones for better coverage.
pub fn format_tree_compact_budgeted(entries: &[FileEntry], budget: &mut Budget) -> String {
    let spine_budget = (budget.limit() as f64 * SPINE_BUDGET_FRACTION) as usize;
    let mut spine_used = 0;

    // Group files by parent directory, tracking depth
    let mut dirs_by_depth: BTreeMap<usize, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    let mut top_level_dirs: BTreeSet<String> = BTreeSet::new();

    for entry in entries {
        let parent = entry
            .relative_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let file_name = entry.file_name().to_string();
        let depth = if parent.is_empty() { 0 } else { parent.matches('/').count() + 1 };

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

    let mut parts = Vec::new();

    // Always try to add top-level dirs listing first
    if !top_level_dirs.is_empty() {
        let dirs_part = format!("dirs:{{{}}}", top_level_dirs.into_iter().collect::<Vec<_>>().join(","));
        let cost = estimate_spine_cost(&dirs_part, budget);
        if spine_used + cost <= spine_budget {
            spine_used += cost;
            parts.push(dirs_part);
        }
    }

    // Add directories level by level (shallow first) until budget exhausted
    for (_depth, dirs_at_level) in dirs_by_depth {
        if spine_used >= spine_budget {
            break;
        }

        for (dir, files) in dirs_at_level {
            if spine_used >= spine_budget {
                break;
            }

            let dir_name = if dir.is_empty() { "." } else { &dir };
            if !files.is_empty() {
                let part = format!(
                    "{}:{{{}}}",
                    dir_name,
                    files.into_iter().collect::<Vec<_>>().join(",")
                );
                let cost = estimate_spine_cost(&part, budget);
                if spine_used + cost <= spine_budget {
                    spine_used += cost;
                    parts.push(part);
                }
            }
        }
    }

    let result = parts.join("|");
    budget.add(&result);
    result
}

/// Estimate the cost of a spine segment based on budget type.
fn estimate_spine_cost(s: &str, budget: &Budget) -> usize {
    match budget.target {
        crate::budget::BudgetTarget::Tokens(_) => crate::tokenizer::count_tokens(s),
        crate::budget::BudgetTarget::Bytes(_) => s.len(),
    }
}

/// Generate an ASCII tree representation.
pub fn format_tree_ascii(entries: &[FileEntry]) -> String {
    let tree = build_tree(entries);
    let mut output = String::new();
    format_node_ascii(&tree, "", true, &mut output, true);
    output
}

fn format_node_ascii(node: &TreeNode, prefix: &str, is_last: bool, output: &mut String, is_root: bool) {
    if !is_root {
        let connector = if is_last { "└── " } else { "├── " };
        output.push_str(prefix);
        output.push_str(connector);
        output.push_str(&node.name);
        if node.is_dir {
            output.push('/');
        }
        output.push('\n');
    }

    let children: Vec<_> = node.children.values().collect();
    let new_prefix = if is_root {
        String::new()
    } else {
        format!("{}{}", prefix, if is_last { "    " } else { "│   " })
    };

    for (i, child) in children.iter().enumerate() {
        let is_last_child = i == children.len() - 1;
        format_node_ascii(child, &new_prefix, is_last_child, output, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_entry(path: &str, is_dir: bool) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            relative_path: PathBuf::from(path),
            is_dir,
            size: 100,
            extension: path.rsplit('.').next().unwrap_or("").to_string(),
            depth: path.matches('/').count(),
            representation: crate::scanner::entry::Representation::NameOnly,
        }
    }

    #[test]
    fn test_compact_format() {
        let entries = vec![
            make_entry("src", true),
            make_entry("src/main.rs", false),
            make_entry("src/lib.rs", false),
            make_entry("Cargo.toml", false),
        ];

        let compact = format_tree_compact(&entries);
        assert!(compact.contains("src"));
        assert!(compact.contains("main.rs"));
        assert!(compact.contains("lib.rs"));
    }
}
