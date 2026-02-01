//! Evaluation harness for allocation quality metrics.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::time::Instant;

use serde::Serialize;

use crate::budget::BudgetTarget;
use crate::config::Config;
use crate::packer;
use crate::scanner;
use crate::scanner::entry::FileEntry;
use crate::tokenizer;

#[derive(Debug, Serialize)]
pub struct EvalReport {
    pub repo: String,
    pub budgets: Vec<EvalMetrics>,
}

#[derive(Debug, Serialize)]
pub struct EvalMetrics {
    pub target_tokens: usize,
    pub actual_tokens: usize,
    pub elapsed_ms: u64,
    pub tokens_per_sec: f64,
    pub overshoot_ratio: f64,
    pub tree_tokens: usize,
    pub tree_ratio: f64,
    pub entry_points_expected: Vec<String>,
    pub entry_points_missing: Vec<String>,
    pub entry_point_coverage: f64,
    pub coverage_spread: Option<f64>,
    pub lopsidedness: Option<f64>,
    pub signature_files: usize,
    pub path_diversity: usize,
}

pub fn evaluate(path: &Path, budgets: &[usize]) -> EvalReport {
    let repo = path.to_string_lossy().to_string();
    let config = Config::default();
    let entries = scanner::scan(path, &config, true);
    let entry_points = find_entry_points(&entries);

    let mut results = Vec::new();
    for target in budgets {
        let start = Instant::now();
        let result = packer::pack(path, &config, BudgetTarget::Tokens(*target), true, true);
        let elapsed = start.elapsed();
        let metrics = compute_metrics(&result.output, *target, &entry_points, elapsed);
        results.push(metrics);
    }

    EvalReport {
        repo,
        budgets: results,
    }
}

fn compute_metrics(
    output: &str,
    target: usize,
    entry_points: &[String],
    elapsed: std::time::Duration,
) -> EvalMetrics {
    let actual_tokens = tokenizer::count_tokens(output);
    let elapsed_ms = elapsed.as_millis() as u64;
    let seconds = elapsed.as_secs_f64();
    let tokens_per_sec = if seconds > 0.0 {
        actual_tokens as f64 / seconds
    } else {
        0.0
    };
    let overshoot_ratio = if actual_tokens > target {
        (actual_tokens - target) as f64 / target as f64
    } else {
        0.0
    };

    let parsed = parse_pipe(output);
    let tree_tokens = tokenizer::count_tokens(&parsed.tree_segments.join("|"));
    let tree_ratio = tree_tokens as f64 / target as f64;

    let (entry_points_expected, entry_points_missing, entry_point_coverage) =
        entry_point_coverage(output, entry_points);

    let (coverage_spread, lopsidedness, signature_files, path_diversity) =
        detail_distribution(&parsed.tree_segments, &parsed.signature_segments);

    EvalMetrics {
        target_tokens: target,
        actual_tokens,
        elapsed_ms,
        tokens_per_sec,
        overshoot_ratio,
        tree_tokens,
        tree_ratio,
        entry_points_expected,
        entry_points_missing,
        entry_point_coverage,
        coverage_spread,
        lopsidedness,
        signature_files,
        path_diversity,
    }
}

struct ParsedPipe {
    tree_segments: Vec<String>,
    signature_segments: Vec<String>,
}

fn parse_pipe(output: &str) -> ParsedPipe {
    let mut tree_segments = Vec::new();
    let mut signature_segments = Vec::new();

    for part in output.split('|') {
        if part.starts_with('[')
            || part.starts_with("root:")
            || part.starts_with("IMPORTANT:")
        {
            continue;
        }

        if part.contains(":{") {
            tree_segments.push(part.to_string());
        } else if part.contains(':') {
            signature_segments.push(part.to_string());
        }
    }

    ParsedPipe {
        tree_segments,
        signature_segments,
    }
}

fn entry_point_coverage(output: &str, entry_points: &[String]) -> (Vec<String>, Vec<String>, f64) {
    let expected = entry_points.to_vec();
    if expected.is_empty() {
        return (expected, Vec::new(), 1.0);
    }

    let mut missing = Vec::new();
    for entry in &expected {
        if !output.contains(entry) {
            missing.push(entry.clone());
        }
    }

    let coverage = (expected.len() - missing.len()) as f64 / expected.len() as f64;

    (expected, missing, coverage)
}

fn detail_distribution(
    tree_segments: &[String],
    signature_segments: &[String],
) -> (Option<f64>, Option<f64>, usize, usize) {
    let top_dirs = extract_top_dirs(tree_segments);

    let mut detail_counts: HashMap<String, usize> = HashMap::new();
    let mut signature_files = 0;
    let mut prefixes: BTreeSet<String> = BTreeSet::new();

    for seg in signature_segments {
        let path = seg.splitn(2, ':').next().unwrap_or("");
        if path.is_empty() {
            continue;
        }
        signature_files += 1;
        let top = top_dir(path);
        *detail_counts.entry(top).or_insert(0) += 1;

        let prefix = path_prefix(path, 2);
        prefixes.insert(prefix);
    }

    let coverage_spread = if top_dirs.is_empty() {
        None
    } else {
        let covered = top_dirs
            .iter()
            .filter(|d| detail_counts.contains_key(*d))
            .count();
        Some(covered as f64 / top_dirs.len() as f64)
    };

    let lopsidedness = if detail_counts.is_empty() {
        None
    } else {
        let counts: Vec<usize> = detail_counts.values().copied().collect();
        let mean = counts.iter().sum::<usize>() as f64 / counts.len() as f64;
        if mean == 0.0 {
            None
        } else {
            Some(*counts.iter().max().unwrap_or(&0) as f64 / mean)
        }
    };

    (coverage_spread, lopsidedness, signature_files, prefixes.len())
}

fn extract_top_dirs(tree_segments: &[String]) -> BTreeSet<String> {
    let mut dirs = BTreeSet::new();

    for seg in tree_segments {
        if let Some((label, rest)) = seg.split_once(":{") {
            if label == "dirs" {
                let items = rest.trim_end_matches('}');
                for item in items.split(',').filter(|s| !s.is_empty()) {
                    dirs.insert(item.to_string());
                }
            } else {
                let top = top_dir(label);
                if !top.is_empty() {
                    dirs.insert(top);
                }
            }
        }
    }

    dirs
}

fn top_dir(path: &str) -> String {
    path.split('/').next().unwrap_or(path).to_string()
}

fn path_prefix(path: &str, depth: usize) -> String {
    let mut parts = path.split('/');
    let mut prefix = Vec::new();
    for _ in 0..depth {
        if let Some(part) = parts.next() {
            prefix.push(part);
        } else {
            break;
        }
    }
    if prefix.is_empty() {
        path.to_string()
    } else {
        prefix.join("/")
    }
}

fn find_entry_points(entries: &[FileEntry]) -> Vec<String> {
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

    let mut found = BTreeSet::new();
    for entry in entries {
        if entry.is_dir {
            continue;
        }
        let name = entry.file_name();
        if candidates.contains(&name) {
            found.insert(name.to_string());
        }
    }

    found.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_top_dir() {
        assert_eq!(top_dir("src/main.rs"), "src");
        assert_eq!(top_dir("Cargo.toml"), "Cargo.toml");
    }

    #[test]
    fn test_path_prefix() {
        assert_eq!(path_prefix("src/format/mod.rs", 2), "src/format");
        assert_eq!(path_prefix("main.rs", 2), "main.rs");
    }
}
