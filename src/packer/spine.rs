//! Directory tree structure generation.

use std::collections::{BTreeMap, BTreeSet};

use crate::scanner::entry::FileEntry;

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
