//! Full markdown output format with sections.

use crate::format::Formatter;
use crate::packer::signatures::Signature;
use crate::packer::spine::format_tree_ascii;
use crate::scanner::entry::FileEntry;

/// Builder for full markdown output.
pub struct FullFormatter {
    title: String,
    root: String,
    important: Option<String>,
    entries: Vec<FileEntry>,
    signatures: Vec<(String, Vec<Signature>)>, // (path, signatures)
}

impl FullFormatter {
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
        let mut output = String::new();

        output.push_str("# ");
        output.push_str(&self.title);
        output.push_str("\n\n");

        output.push_str("Root: `");
        output.push_str(&self.root);
        output.push_str("`\n\n");

        if let Some(ref note) = self.important {
            output.push_str("> IMPORTANT: ");
            output.push_str(note);
            output.push_str("\n\n");
        }

        output.push_str("## Structure\n\n");
        let tree = format_tree_ascii(&self.entries);
        if tree.trim().is_empty() {
            output.push_str("_No files found._\n\n");
        } else {
            output.push_str("```txt\n");
            output.push_str(&tree);
            if !tree.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("```\n\n");
        }

        output.push_str("## Signatures\n\n");
        if self.signatures.is_empty() {
            output.push_str("_No signatures available._\n");
        } else {
            for (path, sigs) in &self.signatures {
                output.push_str("### ");
                output.push_str(path);
                output.push('\n');
                for sig in sigs {
                    output.push_str("- ");
                    output.push_str(&sig.compact());
                    output.push('\n');
                }
                output.push('\n');
            }
        }

        output
    }
}

impl Formatter for FullFormatter {
    fn format(&self) -> String {
        FullFormatter::format(self)
    }
}
