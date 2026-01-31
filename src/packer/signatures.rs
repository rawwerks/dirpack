//! Tree-sitter based signature extraction for code files.
//!
//! Extracts function signatures, struct definitions, traits, interfaces,
//! and other important code constructs from source files.

use std::collections::HashMap;
use std::path::Path;
use streaming_iterator::StreamingIterator;
use thiserror::Error;
use tree_sitter::{Language, Parser, Query, QueryCursor};

/// Errors that can occur during signature extraction.
#[derive(Error, Debug)]
pub enum SignatureError {
    #[error("unsupported language for extension: {0}")]
    UnsupportedLanguage(String),

    #[error("failed to set parser language: {0}")]
    ParserLanguage(String),

    #[error("failed to parse file: {0}")]
    ParseFailed(String),

    #[error("failed to compile query: {0}")]
    QueryCompile(String),

    #[error("failed to read file: {0}")]
    FileRead(#[from] std::io::Error),
}

/// The kind of code construct represented by a signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureKind {
    Function,
    Method,
    Struct,
    Trait,
    Interface,
    Class,
    TypeAlias,
    Constant,
    Enum,
    Module,
    /// Markdown heading (H1/H2/H3)
    Heading,
}

impl std::fmt::Display for SignatureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignatureKind::Function => write!(f, "fn"),
            SignatureKind::Method => write!(f, "method"),
            SignatureKind::Struct => write!(f, "struct"),
            SignatureKind::Trait => write!(f, "trait"),
            SignatureKind::Interface => write!(f, "interface"),
            SignatureKind::Class => write!(f, "class"),
            SignatureKind::TypeAlias => write!(f, "type"),
            SignatureKind::Constant => write!(f, "const"),
            SignatureKind::Enum => write!(f, "enum"),
            SignatureKind::Module => write!(f, "mod"),
            SignatureKind::Heading => write!(f, "#"),
        }
    }
}

/// A signature extracted from source code.
#[derive(Debug, Clone)]
pub struct Signature {
    /// The kind of construct (function, struct, etc.)
    pub kind: SignatureKind,
    /// The name of the construct
    pub name: String,
    /// The full signature text (e.g., "fn foo(x: i32) -> bool")
    pub text: String,
    /// Line number where this signature starts (1-indexed)
    pub line: usize,
    /// Optional visibility modifier (pub, private, etc.)
    pub visibility: Option<String>,
}

impl Signature {
    /// Create a compact representation for pipe-delimited output.
    pub fn compact(&self) -> String {
        self.text.clone()
    }

    /// Truncate the signature text to a maximum length.
    pub fn truncated(&self, max_len: usize) -> String {
        if self.text.len() <= max_len {
            self.text.clone()
        } else {
            format!("{}...", &self.text[..max_len.saturating_sub(3)])
        }
    }
}

/// Configuration for a supported language.
#[derive(Clone)]
pub struct LanguageConfig {
    /// The tree-sitter Language
    pub language: Language,
    /// File extensions that map to this language
    pub extensions: Vec<&'static str>,
    /// Query string for extracting signatures
    pub query: &'static str,
}

/// Main signature extractor using tree-sitter.
pub struct SignatureExtractor {
    parser: Parser,
    languages: HashMap<String, LanguageConfig>,
    max_signature_length: usize,
}

impl SignatureExtractor {
    /// Create a new signature extractor with default language configurations.
    pub fn new() -> Result<Self, SignatureError> {
        let mut extractor = Self {
            parser: Parser::new(),
            languages: HashMap::new(),
            max_signature_length: 200,
        };

        extractor.register_rust()?;
        extractor.register_go()?;
        extractor.register_python()?;
        extractor.register_javascript()?;
        extractor.register_typescript()?;
        extractor.register_c()?;
        extractor.register_cpp()?;

        Ok(extractor)
    }

    /// Set the maximum signature length before truncation.
    pub fn set_max_signature_length(&mut self, len: usize) {
        self.max_signature_length = len;
    }

    /// Check if a file extension is supported.
    pub fn supports_extension(&self, ext: &str) -> bool {
        self.languages.contains_key(ext)
    }

    /// Get list of supported extensions.
    pub fn supported_extensions(&self) -> Vec<&str> {
        self.languages.keys().map(|s| s.as_str()).collect()
    }

    /// Extract signatures from a file.
    pub fn extract_from_file(&mut self, path: &Path) -> Result<Vec<Signature>, SignatureError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let content = std::fs::read_to_string(path)?;
        self.extract(ext, &content)
    }

    /// Extract signatures from source code with the given extension.
    pub fn extract(&mut self, ext: &str, source: &str) -> Result<Vec<Signature>, SignatureError> {
        let config = self
            .languages
            .get(ext)
            .ok_or_else(|| SignatureError::UnsupportedLanguage(ext.to_string()))?
            .clone();

        self.parser
            .set_language(&config.language)
            .map_err(|e| SignatureError::ParserLanguage(e.to_string()))?;

        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| SignatureError::ParseFailed(ext.to_string()))?;

        let query = Query::new(&config.language, config.query)
            .map_err(|e| SignatureError::QueryCompile(e.to_string()))?;

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        let mut signatures = Vec::new();
        let capture_names = query.capture_names();

        while let Some(m) = matches.next() {
            if let Some(sig) = self.process_match(m, capture_names, source, ext) {
                signatures.push(sig);
            }
        }

        Ok(signatures)
    }

    fn process_match(
        &self,
        m: &tree_sitter::QueryMatch,
        capture_names: &[&str],
        source: &str,
        ext: &str,
    ) -> Option<Signature> {
        let mut name = None;
        let mut kind = None;
        let mut text = None;
        let mut line = 0;
        let mut visibility = None;

        for capture in m.captures {
            let capture_name = *capture_names.get(capture.index as usize)?;
            let node_text = capture.node.utf8_text(source.as_bytes()).ok()?;

            match capture_name {
                "name" => {
                    name = Some(node_text.to_string());
                    line = capture.node.start_position().row + 1;
                }
                "kind" => {
                    kind = Some(self.parse_kind(node_text, ext));
                }
                "signature" => {
                    text = Some(self.clean_signature(node_text));
                }
                "visibility" => {
                    visibility = Some(node_text.to_string());
                }
                _ => {}
            }
        }

        // If we have a name but no explicit signature, construct one
        let final_text = text.or_else(|| {
            let k = kind.as_ref()?;
            let n = name.as_ref()?;
            Some(format!("{} {}", k, n))
        })?;

        Some(Signature {
            kind: kind?,
            name: name?,
            text: if final_text.len() > self.max_signature_length {
                format!("{}...", &final_text[..self.max_signature_length - 3])
            } else {
                final_text
            },
            line,
            visibility,
        })
    }

    fn parse_kind(&self, node_text: &str, ext: &str) -> SignatureKind {
        match node_text {
            "fn" | "func" | "def" | "function" => SignatureKind::Function,
            "struct" => SignatureKind::Struct,
            "trait" => SignatureKind::Trait,
            "interface" => SignatureKind::Interface,
            "class" => SignatureKind::Class,
            "type" => SignatureKind::TypeAlias,
            "const" | "let" => SignatureKind::Constant,
            "enum" => SignatureKind::Enum,
            "mod" | "module" => SignatureKind::Module,
            "impl" => SignatureKind::Method,
            _ => {
                // Infer from extension for ambiguous cases
                match ext {
                    "py" if node_text == "def" => SignatureKind::Function,
                    "go" if node_text == "func" => SignatureKind::Function,
                    _ => SignatureKind::Function,
                }
            }
        }
    }

    fn clean_signature(&self, text: &str) -> String {
        // Remove body, keep just the signature
        let text = text.trim();

        // Find where the body starts (opening brace or colon for Python)
        if let Some(pos) = text.find('{') {
            text[..pos].trim().to_string()
        } else if let Some(pos) = text.find(":\n") {
            // Python function with body
            text[..pos].trim().to_string()
        } else {
            // Single line or no body
            text.lines().next().unwrap_or(text).trim().to_string()
        }
    }

    // Language registration methods

    fn register_rust(&mut self) -> Result<(), SignatureError> {
        let language = tree_sitter_rust::LANGUAGE.into();
        let query = r#"
            ; Functions
            (function_item
                (visibility_modifier)? @visibility
                "fn" @kind
                name: (identifier) @name
            ) @signature

            ; Methods in impl blocks
            (impl_item
                (declaration_list
                    (function_item
                        (visibility_modifier)? @visibility
                        "fn" @kind
                        name: (identifier) @name
                    ) @signature
                )
            )

            ; Structs
            (struct_item
                (visibility_modifier)? @visibility
                "struct" @kind
                name: (type_identifier) @name
            ) @signature

            ; Enums
            (enum_item
                (visibility_modifier)? @visibility
                "enum" @kind
                name: (type_identifier) @name
            ) @signature

            ; Traits
            (trait_item
                (visibility_modifier)? @visibility
                "trait" @kind
                name: (type_identifier) @name
            ) @signature

            ; Type aliases
            (type_item
                (visibility_modifier)? @visibility
                "type" @kind
                name: (type_identifier) @name
            ) @signature

            ; Constants
            (const_item
                (visibility_modifier)? @visibility
                "const" @kind
                name: (identifier) @name
            ) @signature

            ; Modules
            (mod_item
                (visibility_modifier)? @visibility
                "mod" @kind
                name: (identifier) @name
            )
        "#;

        let config = LanguageConfig {
            language,
            extensions: vec!["rs"],
            query,
        };

        self.languages.insert("rs".to_string(), config);
        Ok(())
    }

    fn register_go(&mut self) -> Result<(), SignatureError> {
        let language = tree_sitter_go::LANGUAGE.into();
        let query = r#"
            ; Functions
            (function_declaration
                "func" @kind
                name: (identifier) @name
            ) @signature

            ; Methods
            (method_declaration
                "func" @kind
                name: (field_identifier) @name
            ) @signature

            ; Struct types
            (type_declaration
                (type_spec
                    name: (type_identifier) @name
                    type: (struct_type) @kind
                )
            ) @signature

            ; Interface types
            (type_declaration
                (type_spec
                    name: (type_identifier) @name
                    type: (interface_type) @kind
                )
            ) @signature

            ; Type aliases
            (type_declaration
                (type_spec
                    name: (type_identifier) @name
                )
            ) @signature

            ; Constants
            (const_declaration
                (const_spec
                    name: (identifier) @name
                )
            ) @signature
        "#;

        let config = LanguageConfig {
            language,
            extensions: vec!["go"],
            query,
        };

        self.languages.insert("go".to_string(), config);
        Ok(())
    }

    fn register_python(&mut self) -> Result<(), SignatureError> {
        let language = tree_sitter_python::LANGUAGE.into();
        let query = r#"
            ; Functions
            (function_definition
                "def" @kind
                name: (identifier) @name
            ) @signature

            ; Classes
            (class_definition
                "class" @kind
                name: (identifier) @name
            ) @signature

            ; Async functions
            (function_definition
                "async"
                "def" @kind
                name: (identifier) @name
            ) @signature
        "#;

        let config = LanguageConfig {
            language,
            extensions: vec!["py"],
            query,
        };

        self.languages.insert("py".to_string(), config);
        Ok(())
    }

    fn register_javascript(&mut self) -> Result<(), SignatureError> {
        let language = tree_sitter_javascript::LANGUAGE.into();
        let query = r#"
            ; Function declarations
            (function_declaration
                "function" @kind
                name: (identifier) @name
            ) @signature

            ; Arrow functions assigned to variables
            (variable_declarator
                name: (identifier) @name
                value: (arrow_function) @signature
            )

            ; Class declarations
            (class_declaration
                "class" @kind
                name: (identifier) @name
            ) @signature

            ; Method definitions in classes
            (method_definition
                name: (property_identifier) @name
            ) @signature

            ; Export function
            (export_statement
                (function_declaration
                    "function" @kind
                    name: (identifier) @name
                ) @signature
            )

            ; Export class
            (export_statement
                (class_declaration
                    "class" @kind
                    name: (identifier) @name
                ) @signature
            )
        "#;

        let config = LanguageConfig {
            language,
            extensions: vec!["js", "jsx", "mjs", "cjs"],
            query,
        };

        for ext in &["js", "jsx", "mjs", "cjs"] {
            self.languages.insert(ext.to_string(), config.clone());
        }
        Ok(())
    }

    fn register_typescript(&mut self) -> Result<(), SignatureError> {
        let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        let query = r#"
            ; Function declarations
            (function_declaration
                "function" @kind
                name: (identifier) @name
            ) @signature

            ; Arrow functions assigned to variables
            (variable_declarator
                name: (identifier) @name
                value: (arrow_function) @signature
            )

            ; Class declarations
            (class_declaration
                "class" @kind
                name: (type_identifier) @name
            ) @signature

            ; Interface declarations
            (interface_declaration
                "interface" @kind
                name: (type_identifier) @name
            ) @signature

            ; Type aliases
            (type_alias_declaration
                "type" @kind
                name: (type_identifier) @name
            ) @signature

            ; Method definitions in classes
            (method_definition
                name: (property_identifier) @name
            ) @signature

            ; Enum declarations
            (enum_declaration
                "enum" @kind
                name: (identifier) @name
            ) @signature

            ; Export function
            (export_statement
                (function_declaration
                    "function" @kind
                    name: (identifier) @name
                ) @signature
            )

            ; Export class
            (export_statement
                (class_declaration
                    "class" @kind
                    name: (type_identifier) @name
                ) @signature
            )

            ; Export interface
            (export_statement
                (interface_declaration
                    "interface" @kind
                    name: (type_identifier) @name
                ) @signature
            )
        "#;

        let config = LanguageConfig {
            language,
            extensions: vec!["ts"],
            query,
        };

        self.languages.insert("ts".to_string(), config.clone());

        // TSX needs separate language
        let tsx_language = tree_sitter_typescript::LANGUAGE_TSX.into();
        let tsx_config = LanguageConfig {
            language: tsx_language,
            extensions: vec!["tsx"],
            query,
        };
        self.languages.insert("tsx".to_string(), tsx_config);

        Ok(())
    }

    fn register_c(&mut self) -> Result<(), SignatureError> {
        let language = tree_sitter_c::LANGUAGE.into();
        let query = r#"
            ; Function definitions
            (function_definition
                declarator: (function_declarator
                    declarator: (identifier) @name
                )
            ) @signature

            ; Function declarations (prototypes)
            (declaration
                declarator: (function_declarator
                    declarator: (identifier) @name
                )
            ) @signature

            ; Struct definitions
            (struct_specifier
                "struct" @kind
                name: (type_identifier) @name
            ) @signature

            ; Enum definitions
            (enum_specifier
                "enum" @kind
                name: (type_identifier) @name
            ) @signature

            ; Typedef
            (type_definition
                declarator: (type_identifier) @name
            ) @signature
        "#;

        let config = LanguageConfig {
            language,
            extensions: vec!["c", "h"],
            query,
        };

        for ext in &["c", "h"] {
            self.languages.insert(ext.to_string(), config.clone());
        }
        Ok(())
    }

    fn register_cpp(&mut self) -> Result<(), SignatureError> {
        let language = tree_sitter_cpp::LANGUAGE.into();
        let query = r#"
            ; Function definitions
            (function_definition
                declarator: (function_declarator
                    declarator: (identifier) @name
                )
            ) @signature

            ; Function definitions with qualified name
            (function_definition
                declarator: (function_declarator
                    declarator: (qualified_identifier
                        name: (identifier) @name
                    )
                )
            ) @signature

            ; Class definitions
            (class_specifier
                "class" @kind
                name: (type_identifier) @name
            ) @signature

            ; Struct definitions
            (struct_specifier
                "struct" @kind
                name: (type_identifier) @name
            ) @signature

            ; Enum definitions
            (enum_specifier
                "enum" @kind
                name: (type_identifier) @name
            ) @signature

            ; Namespace
            (namespace_definition
                "namespace" @kind
                name: (identifier) @name
            )

            ; Template class
            (template_declaration
                (class_specifier
                    "class" @kind
                    name: (type_identifier) @name
                )
            ) @signature

            ; Template function
            (template_declaration
                (function_definition
                    declarator: (function_declarator
                        declarator: (identifier) @name
                    )
                )
            ) @signature
        "#;

        let config = LanguageConfig {
            language,
            extensions: vec!["cpp", "cc", "cxx", "hpp", "hxx", "hh"],
            query,
        };

        for ext in &["cpp", "cc", "cxx", "hpp", "hxx", "hh"] {
            self.languages.insert(ext.to_string(), config.clone());
        }
        Ok(())
    }
}

impl Default for SignatureExtractor {
    fn default() -> Self {
        Self::new().expect("failed to initialize signature extractor")
    }
}

/// Extract headings from markdown content as signatures.
/// Returns H1, H2, and H3 headings.
pub fn extract_markdown_headings(content: &str) -> Vec<Signature> {
    let mut signatures = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();

        // Match ATX-style headings (# ## ###)
        if let Some(heading) = parse_atx_heading(trimmed) {
            if heading.level <= 3 {
                signatures.push(Signature {
                    kind: SignatureKind::Heading,
                    name: heading.text.clone(),
                    text: heading.text,
                    line: line_num + 1,
                    visibility: None,
                });
            }
        }
    }

    signatures
}

struct MarkdownHeading {
    level: usize,
    text: String,
}

fn parse_atx_heading(line: &str) -> Option<MarkdownHeading> {
    // Count leading # characters
    let hash_count = line.chars().take_while(|&c| c == '#').count();

    if hash_count == 0 || hash_count > 6 {
        return None;
    }

    // Must have space after # (or be empty heading)
    let rest = &line[hash_count..];
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }

    // Extract heading text, removing trailing # and whitespace
    let text = rest
        .trim()
        .trim_end_matches('#')
        .trim()
        .to_string();

    if text.is_empty() {
        return None;
    }

    Some(MarkdownHeading {
        level: hash_count,
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_signatures() {
        let mut extractor = SignatureExtractor::new().unwrap();
        let source = r#"
pub fn hello(name: &str) -> String {
    format!("Hello, {}", name)
}

struct Point {
    x: i32,
    y: i32,
}

pub trait Drawable {
    fn draw(&self);
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
}

enum Color {
    Red,
    Green,
    Blue,
}

type Result<T> = std::result::Result<T, Error>;

const MAX_SIZE: usize = 100;
"#;

        let sigs = extractor.extract("rs", source).unwrap();

        let names: Vec<_> = sigs.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"), "should find hello function");
        assert!(names.contains(&"Point"), "should find Point struct");
        assert!(names.contains(&"Drawable"), "should find Drawable trait");
        assert!(names.contains(&"new"), "should find new method");
        assert!(names.contains(&"Color"), "should find Color enum");
    }

    #[test]
    fn test_go_signatures() {
        let mut extractor = SignatureExtractor::new().unwrap();
        let source = r#"
package main

func Hello(name string) string {
    return "Hello, " + name
}

type Point struct {
    X int
    Y int
}

func (p *Point) Move(dx, dy int) {
    p.X += dx
    p.Y += dy
}

type Reader interface {
    Read(p []byte) (n int, err error)
}

const MaxSize = 100
"#;

        let sigs = extractor.extract("go", source).unwrap();

        let names: Vec<_> = sigs.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Hello"), "should find Hello function");
        assert!(names.contains(&"Move"), "should find Move method");
    }

    #[test]
    fn test_python_signatures() {
        let mut extractor = SignatureExtractor::new().unwrap();
        let source = r#"
def hello(name: str) -> str:
    return f"Hello, {name}"

class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

    def move(self, dx: int, dy: int):
        self.x += dx
        self.y += dy

async def fetch_data(url: str) -> dict:
    pass
"#;

        let sigs = extractor.extract("py", source).unwrap();

        let names: Vec<_> = sigs.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"), "should find hello function");
        assert!(names.contains(&"Point"), "should find Point class");
        assert!(names.contains(&"fetch_data"), "should find async function");
    }

    #[test]
    fn test_typescript_signatures() {
        let mut extractor = SignatureExtractor::new().unwrap();
        let source = r#"
function hello(name: string): string {
    return `Hello, ${name}`;
}

class Point {
    constructor(public x: number, public y: number) {}

    move(dx: number, dy: number): void {
        this.x += dx;
        this.y += dy;
    }
}

interface Reader {
    read(buffer: Uint8Array): number;
}

type Result<T> = { ok: true; value: T } | { ok: false; error: Error };

enum Color {
    Red,
    Green,
    Blue,
}

const greet = (name: string) => `Hi, ${name}`;
"#;

        let sigs = extractor.extract("ts", source).unwrap();

        let names: Vec<_> = sigs.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"), "should find hello function");
        assert!(names.contains(&"Point"), "should find Point class");
        assert!(names.contains(&"Reader"), "should find Reader interface");
        assert!(names.contains(&"Color"), "should find Color enum");
    }

    #[test]
    fn test_unsupported_extension() {
        let mut extractor = SignatureExtractor::new().unwrap();
        let result = extractor.extract("xyz", "some content");
        assert!(result.is_err());
    }

    #[test]
    fn test_signature_truncation() {
        let mut extractor = SignatureExtractor::new().unwrap();
        extractor.set_max_signature_length(50);

        let source = r#"
fn very_long_function_name_that_exceeds_the_limit(param1: VeryLongTypeName, param2: AnotherLongTypeName) -> ResultType {
}
"#;
        let sigs = extractor.extract("rs", source).unwrap();

        for sig in &sigs {
            assert!(sig.text.len() <= 50, "signature should be truncated");
        }
    }

    #[test]
    fn test_markdown_heading_extraction() {
        let content = r#"# My Project

Some intro text here.

## Installation

Install with pip.

### Requirements

- Python 3.8+
- Some library

## Usage

Use like this.

#### Deep Heading

This is H4, should be skipped.

# Another H1
"#;
        let sigs = super::extract_markdown_headings(content);

        let names: Vec<_> = sigs.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names.len(), 5, "should find 5 headings (H1-H3 only)");
        assert!(names.contains(&"My Project"), "should find H1");
        assert!(names.contains(&"Installation"), "should find H2");
        assert!(names.contains(&"Requirements"), "should find H3");
        assert!(names.contains(&"Usage"), "should find H2");
        assert!(names.contains(&"Another H1"), "should find second H1");
        assert!(!names.contains(&"Deep Heading"), "should skip H4");

        // Check line numbers
        let h1 = sigs.iter().find(|s| s.name == "My Project").unwrap();
        assert_eq!(h1.line, 1);
    }

    #[test]
    fn test_markdown_heading_edge_cases() {
        // Test various edge cases
        let content = "# Simple\n##NoSpace\n### With Trailing ### \n###### H6 ignored";
        let sigs = super::extract_markdown_headings(content);

        let names: Vec<_> = sigs.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Simple"), "basic H1");
        assert!(names.contains(&"With Trailing"), "trailing hashes removed");
        assert!(!names.contains(&"NoSpace"), "no space after # means not a heading");
    }
}
