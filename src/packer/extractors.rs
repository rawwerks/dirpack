//! Content extractors for the content phase.
//!
//! Instead of including a file's full text, an extractor transforms it into
//! a shorter, higher-signal version that fits the token budget better.
//! For example, `json_keys:dependencies,scripts` extracts only those
//! top-level keys from a package.json, replacing a 200-line file with a
//! 20-line focused summary.
//!
//! Extractors are specified via the `extract` field on `PriorityRule`
//! using a simple DSL: `"extractor_name:arg1,arg2"`. Four built-in
//! extractors ship by default; adding a new one is a function plus a
//! match arm in `run_extractor`.

use crate::config::PriorityRule;
use crate::priority;
use crate::scanner::entry::FileEntry;

/// Try to extract content for a file using the first matching priority
/// rule that has an `extract` spec. Returns `Some(extracted)` if
/// extraction produced output, `None` to use the original content.
pub fn maybe_extract(
    entry: &FileEntry,
    content: &str,
    rules: &[PriorityRule],
) -> Option<String> {
    for rule in rules {
        let Some(spec) = rule.extract.as_deref() else {
            continue;
        };
        if !priority::matches_pattern(&entry.relative_path, &rule.pattern) {
            continue;
        }
        let (name, args) = parse_extract_spec(spec);
        return run_extractor(name, content, &args);
    }
    None
}

/// Parse `"extractor_name:arg1,arg2"` into `("extractor_name", ["arg1", "arg2"])`.
/// For extractors with no args (like `api_surface`), returns an empty vec.
fn parse_extract_spec(spec: &str) -> (&str, Vec<&str>) {
    if let Some((name, args_str)) = spec.split_once(':') {
        let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).collect();
        (name, args)
    } else {
        (spec, vec![])
    }
}

/// Dispatch to the appropriate extractor.
fn run_extractor(name: &str, content: &str, args: &[&str]) -> Option<String> {
    match name {
        "json_keys" => extract_json_keys(content, args),
        "toml_sections" => extract_toml_sections(content, args),
        "lines_matching" => extract_lines_matching(content, args),
        "api_surface" => extract_api_surface(content),
        _ => {
            eprintln!("dirpack: unknown extractor: {}", name);
            None
        }
    }
}

/// Extract specific top-level keys from a JSON object.
///
/// Example: `json_keys:dependencies,devDependencies,scripts` on a
/// package.json returns only those three keys with their values.
fn extract_json_keys(content: &str, keys: &[&str]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    let obj = value.as_object()?;

    let mut lines = Vec::new();

    // Always include name+version as a one-line header if present
    let name = obj.get("name").and_then(|v| v.as_str());
    let version = obj.get("version").and_then(|v| v.as_str());
    match (name, version) {
        (Some(n), Some(v)) => lines.push(format!("// {} v{}", n, v)),
        (Some(n), None) => lines.push(format!("// {}", n)),
        _ => {}
    }

    for key in keys {
        if let Some(val) = obj.get(*key) {
            // For objects (like dependencies), format as key-value pairs
            if let Some(inner) = val.as_object() {
                lines.push(format!("\"{}\":", key));
                for (k, v) in inner {
                    let v_str = match v.as_str() {
                        Some(s) => format!("\"{}\"", s),
                        None => v.to_string(),
                    };
                    lines.push(format!("  \"{}\": {}", k, v_str));
                }
            } else if let Some(arr) = val.as_array() {
                lines.push(format!("\"{}\": [", key));
                for item in arr {
                    lines.push(format!("  {}", item));
                }
                lines.push("]".to_string());
            } else {
                lines.push(format!("\"{}\": {}", key, val));
            }
        }
    }

    if lines.is_empty() {
        return None;
    }

    Some(lines.join("\n"))
}

/// Extract specific sections from a TOML document.
///
/// Example: `toml_sections:dependencies,dev-dependencies` on a
/// Cargo.toml returns the [package] name/version plus those sections.
fn extract_toml_sections(content: &str, sections: &[&str]) -> Option<String> {
    let value: toml::Value = toml::from_str(content).ok()?;
    let table = value.as_table()?;

    let mut lines = Vec::new();

    // Always include [package] name+version as a header (Cargo.toml convention)
    if let Some(pkg) = table.get("package").and_then(|v| v.as_table()) {
        let name = pkg.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("?");
        lines.push(format!("[package] {} v{}", name, version));
    }
    // Also check top-level name/version (pyproject.toml style)
    else if let Some(name) = table.get("name").and_then(|v| v.as_str()) {
        let version = table
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        lines.push(format!("{} v{}", name, version));
    }

    for section in sections {
        if let Some(val) = table.get(*section) {
            if let Some(inner) = val.as_table() {
                lines.push(format!("\n[{}]", section));
                for (k, v) in inner {
                    let v_display = format_toml_value(v);
                    lines.push(format!("{} = {}", k, v_display));
                }
            }
        }
    }

    if lines.is_empty() {
        return None;
    }

    Some(lines.join("\n"))
}

/// Format a TOML value compactly for display.
fn format_toml_value(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => format!("\"{}\"", s),
        toml::Value::Table(t) => {
            // Inline table: { version = "1.0", features = ["derive"] }
            let pairs: Vec<String> = t
                .iter()
                .map(|(k, v)| format!("{} = {}", k, format_toml_value(v)))
                .collect();
            format!("{{ {} }}", pairs.join(", "))
        }
        toml::Value::Array(a) => {
            let items: Vec<String> = a.iter().map(format_toml_value).collect();
            format!("[{}]", items.join(", "))
        }
        other => other.to_string(),
    }
}

/// Extract lines that start with any of the given prefixes.
///
/// Args are `|`-separated prefixes (joined into a single arg string by
/// the config DSL). Example: `lines_matching:require |module ` extracts
/// all lines from go.mod that start with `require ` or `module `.
fn extract_lines_matching(content: &str, args: &[&str]) -> Option<String> {
    // Args may be a single string with `|` separators, or multiple args.
    // Handle both: "require |module " as one arg, or "require ","module " as two.
    let prefixes: Vec<&str> = args
        .iter()
        .flat_map(|a| a.split('|'))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if prefixes.is_empty() {
        return None;
    }

    let matched: Vec<&str> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            prefixes.iter().any(|p| trimmed.starts_with(p))
        })
        .collect();

    if matched.is_empty() {
        return None;
    }

    Some(matched.join("\n"))
}

/// Extract the public API surface from an entry-point file.
///
/// Returns the first 5 non-empty lines (typically module docstring or
/// package declaration) plus all lines that declare public API:
/// `pub mod`, `pub use`, `pub fn`, `pub struct`, `pub enum`, `pub trait`,
/// `pub type`, `pub const`, `export`, `module.exports`, `__all__`.
///
/// No args needed. Returns `None` for files under 10 lines (small files
/// don't need extraction — they'll be included in full).
fn extract_api_surface(content: &str) -> Option<String> {
    let all_lines: Vec<&str> = content.lines().collect();
    if all_lines.len() < 10 {
        return None; // Small file, include as-is
    }

    let mut result = Vec::new();

    // First 5 non-empty lines (docstring/header)
    let mut header_count = 0;
    for line in &all_lines {
        if header_count >= 5 {
            break;
        }
        if !line.trim().is_empty() {
            result.push(*line);
            header_count += 1;
        }
    }

    // Add separator if we got headers
    if !result.is_empty() {
        result.push("// ...");
    }

    // All API declaration lines
    let api_prefixes = [
        "pub mod ",
        "pub use ",
        "pub fn ",
        "pub struct ",
        "pub enum ",
        "pub trait ",
        "pub type ",
        "pub const ",
        "pub static ",
        "pub async fn ",
        "export ",
        "export{",
        "module.exports",
        "__all__",
    ];

    for line in &all_lines {
        let trimmed = line.trim();
        if api_prefixes.iter().any(|p| trimmed.starts_with(p)) {
            // Avoid duplicating lines already in the header
            if !result.contains(line) {
                result.push(line);
            }
        }
    }

    // If we only got the header and no API lines, fall back
    if result.len() <= 6 {
        // 5 header + separator
        return None;
    }

    Some(result.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extract_spec_with_args() {
        let (name, args) = parse_extract_spec("json_keys:dependencies,devDependencies,scripts");
        assert_eq!(name, "json_keys");
        assert_eq!(args, vec!["dependencies", "devDependencies", "scripts"]);
    }

    #[test]
    fn test_parse_extract_spec_no_args() {
        let (name, args) = parse_extract_spec("api_surface");
        assert_eq!(name, "api_surface");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_extract_spec_pipe_args() {
        let (name, args) = parse_extract_spec("lines_matching:require |module ");
        assert_eq!(name, "lines_matching");
        // The whole arg string after the colon is one element (no comma split)
        // but the arg gets trimmed
        assert_eq!(args, vec!["require |module"]);
    }

    #[test]
    fn test_json_keys_package_json() {
        let content = r#"{
  "name": "my-app",
  "version": "1.0.0",
  "description": "A test app",
  "main": "index.js",
  "scripts": {
    "test": "jest",
    "build": "tsc"
  },
  "dependencies": {
    "express": "^4.18.0",
    "lodash": "^4.17.21"
  },
  "devDependencies": {
    "jest": "^29.0.0",
    "typescript": "^5.0.0"
  },
  "eslintConfig": {
    "extends": "eslint:recommended"
  }
}"#;
        let result = extract_json_keys(content, &["dependencies", "scripts"]).unwrap();
        assert!(result.contains("my-app v1.0.0"));
        assert!(result.contains("express"));
        assert!(result.contains("lodash"));
        assert!(result.contains("jest"));
        assert!(result.contains("build"));
        assert!(!result.contains("eslintConfig"));
        assert!(!result.contains("eslint:recommended"));
    }

    #[test]
    fn test_json_keys_missing_keys() {
        let content = r#"{"name": "test"}"#;
        let result = extract_json_keys(content, &["nonexistent"]);
        // Should still return Some because name header is included
        assert!(result.is_some());
        assert!(result.unwrap().contains("test"));
    }

    #[test]
    fn test_json_keys_invalid_json() {
        let result = extract_json_keys("not json at all", &["dependencies"]);
        assert!(result.is_none());
    }

    #[test]
    fn test_toml_sections_cargo_toml() {
        let content = r#"
[package]
name = "dirpack"
version = "0.3.3"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"

[dev-dependencies]
tempfile = "3"

[profile.release]
opt-level = 3
"#;
        let result =
            extract_toml_sections(content, &["dependencies", "dev-dependencies"]).unwrap();
        assert!(result.contains("[package] dirpack v0.3.3"));
        assert!(result.contains("[dependencies]"));
        assert!(result.contains("clap"));
        assert!(result.contains("serde"));
        assert!(result.contains("[dev-dependencies]"));
        assert!(result.contains("tempfile"));
        assert!(!result.contains("profile"));
        assert!(!result.contains("opt-level"));
    }

    #[test]
    fn test_toml_sections_missing() {
        let content = r#"
[package]
name = "test"
version = "0.1.0"
"#;
        let result = extract_toml_sections(content, &["nonexistent"]);
        // Should return Some with just the header
        assert!(result.is_some());
        assert!(result.unwrap().contains("test v0.1.0"));
    }

    #[test]
    fn test_lines_matching_go_mod() {
        let content = r#"module github.com/example/project

go 1.21

require (
	github.com/gorilla/mux v1.8.0
	github.com/lib/pq v1.10.9
)

require (
	github.com/davecgh/go-spew v1.1.1 // indirect
)
"#;
        let result = extract_lines_matching(content, &["require |module "]).unwrap();
        assert!(result.contains("module github.com/example/project"));
        assert!(result.contains("require ("));
        assert!(!result.contains("go 1.21"));
    }

    #[test]
    fn test_lines_matching_no_matches() {
        let result = extract_lines_matching("hello\nworld\n", &["nonexistent"]);
        assert!(result.is_none());
    }

    #[test]
    fn test_api_surface_rust_lib() {
        let content = r#"//! My library crate.
//!
//! This provides useful utilities.

use std::path::Path;
use std::io;

pub mod config;
pub mod error;
pub mod packer;
pub mod scanner;
pub mod security;
pub mod tokenizer;

pub use crate::config::{Config, OutputFormat};
pub use crate::error::{DirpackError, Result};

fn internal_helper() {}
fn another_private() {}
"#;
        let result = extract_api_surface(content).unwrap();
        assert!(result.contains("//! My library crate."));
        assert!(result.contains("pub mod config;"));
        assert!(result.contains("pub mod scanner;"));
        assert!(result.contains("pub use crate::config"));
        assert!(!result.contains("internal_helper"));
        assert!(!result.contains("another_private"));
    }

    #[test]
    fn test_api_surface_small_file_returns_none() {
        let content = "pub fn foo() {}\npub fn bar() {}\n";
        let result = extract_api_surface(content);
        assert!(result.is_none()); // < 10 lines, include as-is
    }

    #[test]
    fn test_api_surface_typescript_exports() {
        let mut lines = vec!["// index.ts", "import { foo } from './foo';", ""];
        // Pad to >10 lines
        for i in 0..10 {
            lines.push("// padding");
        }
        lines.push("export function createStore() {}");
        lines.push("export type Config = {};");
        lines.push("export { foo };");
        let content = lines.join("\n");

        let result = extract_api_surface(&content).unwrap();
        assert!(result.contains("export function createStore"));
        assert!(result.contains("export type Config"));
        assert!(result.contains("export { foo }"));
    }

    #[test]
    fn test_unknown_extractor() {
        let result = run_extractor("nonexistent", "content", &[]);
        assert!(result.is_none());
    }
}
