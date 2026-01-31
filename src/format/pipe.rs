//! Pipe-delimited output format (Vercel-style).
//!
//! Format: [Title]|root: path|IMPORTANT: notes|dirs:{...}|dir:{files}|file:signatures

use crate::format::Formatter;
use crate::packer::signatures::Signature;
use crate::packer::spine::format_tree_compact;
use crate::scanner::entry::FileEntry;

/// Builder for pipe-delimited output.
pub struct PipeFormatter {
    title: String,
    root: String,
    important: Option<String>,
    entries: Vec<FileEntry>,
    signatures: Vec<(String, Vec<Signature>)>, // (path, signatures)
}

impl PipeFormatter {
    pub fn new(title: &str, root: &str) -> Self {
        Self {
            title: title.to_string(),
            root: root.to_string(),
            important: None,
            entries: Vec::new(),
            signatures: Vec::new(),
        }
    }

    pub fn set_important(&mut self, note: &str) -> &mut Self {
        self.important = Some(note.to_string());
        self
    }

    pub fn set_entries(&mut self, entries: Vec<FileEntry>) -> &mut Self {
        self.entries = entries;
        self
    }

    pub fn add_signatures(&mut self, path: &str, sigs: Vec<Signature>) -> &mut Self {
        if !sigs.is_empty() {
            self.signatures.push((path.to_string(), sigs));
        }
        self
    }

    pub fn format(&self) -> String {
        let mut parts = Vec::new();

        // Title
        parts.push(format!("[{}]", self.title));

        // Root
        parts.push(format!("root: {}", self.root));

        // Important note
        if let Some(ref note) = self.important {
            parts.push(format!("IMPORTANT: {}", note));
        }

        // Tree structure
        let tree = format_tree_compact(&self.entries);
        if !tree.is_empty() {
            parts.push(tree);
        }

        // Signatures
        for (path, sigs) in &self.signatures {
            let sig_strs: Vec<_> = sigs
                .iter()
                .map(|s| s.compact())
                .collect();
            if !sig_strs.is_empty() {
                parts.push(format!("{}:{}", path, sig_strs.join(",")));
            }
        }

        parts.join("|")
    }
}

impl Formatter for PipeFormatter {
    fn format(&self) -> String {
        PipeFormatter::format(self)
    }
}

/// Format a single line, collapsing whitespace.
pub fn collapse_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_format() {
        let formatter = PipeFormatter::new("myproject", ".");
        let output = formatter.format();
        assert!(output.contains("[myproject]"));
        assert!(output.contains("root: ."));
    }

    #[test]
    fn test_with_important() {
        let mut formatter = PipeFormatter::new("myproject", ".");
        formatter.set_important("Use retrieval-led reasoning");
        let output = formatter.format();
        assert!(output.contains("IMPORTANT: Use retrieval-led reasoning"));
    }
}
