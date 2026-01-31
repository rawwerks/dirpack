//! JSON structured output format.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;

use crate::format::Formatter;
use crate::packer::signatures::Signature;
use crate::scanner::entry::FileEntry;

/// Builder for JSON output.
pub struct JsonFormatter {
    title: String,
    root: String,
    important: Option<String>,
    entries: Vec<FileEntry>,
    signatures: Vec<(String, Vec<Signature>)>, // (path, signatures)
    contents: Vec<(String, String)>,           // (path, content)
}

impl JsonFormatter {
    pub fn new(title: &str, root: &str) -> Self {
        Self {
            title: title.to_string(),
            root: root.to_string(),
            important: None,
            entries: Vec::new(),
            signatures: Vec::new(),
            contents: Vec::new(),
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

    pub fn add_content(&mut self, path: &str, content: String) -> &mut Self {
        self.contents.push((path.to_string(), content));
        self
    }

    pub fn format(&self) -> String {
        let mut top_dirs: BTreeSet<String> = BTreeSet::new();
        let mut top_files: BTreeSet<String> = BTreeSet::new();

        for entry in &self.entries {
            if entry.depth == 0 {
                if entry.is_dir {
                    top_dirs.insert(entry.file_name().to_string());
                } else {
                    top_files.insert(entry.file_name().to_string());
                }
            }
        }

        let mut sig_map: HashMap<String, Vec<String>> = HashMap::new();
        for (path, sigs) in &self.signatures {
            let texts = sigs.iter().map(|s| s.compact()).collect::<Vec<_>>();
            sig_map.insert(path.clone(), texts);
        }

        let mut content_map: HashMap<String, String> = HashMap::new();
        for (path, content) in &self.contents {
            content_map.insert(path.clone(), content.clone());
        }

        let mut files = self
            .entries
            .iter()
            .filter(|e| !e.is_dir)
            .collect::<Vec<_>>();
        files.sort_by(|a, b| {
            a.relative_path
                .to_string_lossy()
                .cmp(&b.relative_path.to_string_lossy())
        });

        let mut output = String::from("{");
        let mut first = true;

        push_field(&mut output, &mut first, "title", &json_string(&self.title));
        push_field(&mut output, &mut first, "root", &json_string(&self.root));

        if let Some(ref note) = self.important {
            push_field(&mut output, &mut first, "important", &json_string(note));
        }

        let tree_json = format!(
            "{{\"dirs\":{},\"files\":{}}}",
            json_array(top_dirs.into_iter().collect::<Vec<_>>().as_slice()),
            json_array(top_files.into_iter().collect::<Vec<_>>().as_slice())
        );
        push_field(&mut output, &mut first, "tree", &tree_json);

        let mut file_objects = Vec::new();
        for entry in files {
            let rel_path = entry.relative_path.to_string_lossy().to_string();
            let sigs = sig_map.get(&rel_path).cloned().unwrap_or_default();
            let content = content_map.get(&rel_path).cloned();
            file_objects.push(json_file(&rel_path, &sigs, content.as_deref()));
        }

        push_field(
            &mut output,
            &mut first,
            "files",
            &format!("[{}]", file_objects.join(",")),
        );

        output.push('}');
        output
    }
}

impl Formatter for JsonFormatter {
    fn format(&self) -> String {
        JsonFormatter::format(self)
    }
}

fn json_file(path: &str, signatures: &[String], content: Option<&str>) -> String {
    let mut output = String::from("{");
    let mut first = true;

    push_field(&mut output, &mut first, "path", &json_string(path));
    push_field(
        &mut output,
        &mut first,
        "signatures",
        &json_array(signatures),
    );

    if let Some(text) = content {
        push_field(&mut output, &mut first, "content", &json_string(text));
    }

    output.push('}');
    output
}

fn json_array(values: &[String]) -> String {
    let mut output = String::from("[");
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            output.push(',');
        }
        output.push_str(&json_string(value));
    }
    output.push(']');
    output
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0C}' => output.push_str("\\f"),
            c if c.is_control() => {
                let _ = write!(output, "\\u{:04x}", c as u32);
            }
            _ => output.push(ch),
        }
    }
    output.push('"');
    output
}

fn push_field(output: &mut String, first: &mut bool, key: &str, value: &str) {
    if !*first {
        output.push(',');
    }
    *first = false;
    output.push_str(&json_string(key));
    output.push(':');
    output.push_str(value);
}
