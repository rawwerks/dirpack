use std::fs;
use std::path::Path;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::budget::BudgetTarget;
use crate::error::Result;
use crate::limits;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub output: OutputConfig,
    pub scanning: ScanningConfig,
    pub categories: CategoryConfig,
    pub priority: PriorityWeights,
    pub priority_rules: Vec<PriorityRule>,
    pub exclude: ExcludeConfig,
    pub signatures: SignatureConfig,
    pub content: ContentConfig,
}

pub use crate::limits::{
    MAX_BUDGET_BYTES as SAFE_MAX_BUDGET_BYTES, MAX_BUDGET_TOKENS as SAFE_MAX_BUDGET_TOKENS,
    MAX_SCAN_DEPTH as SAFE_MAX_SCAN_DEPTH,
};

impl Config {
    pub fn from_str(contents: &str) -> Result<Self> {
        Ok(toml::from_str(contents)?)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        Self::from_str(&contents)
    }
}

pub fn clamp_budget_target(target: BudgetTarget) -> BudgetTarget {
    match target {
        BudgetTarget::Tokens(tokens) => {
            let clamped = limits::clamp_budget_tokens(tokens);
            if clamped != tokens {
                eprintln!(
                    "SECURITY: token budget clamped to {}",
                    SAFE_MAX_BUDGET_TOKENS
                );
            }
            BudgetTarget::Tokens(clamped)
        }
        BudgetTarget::Bytes(bytes) => {
            let clamped = limits::clamp_budget_bytes(bytes);
            if clamped != bytes {
                eprintln!(
                    "SECURITY: byte budget clamped to {}",
                    SAFE_MAX_BUDGET_BYTES
                );
            }
            BudgetTarget::Bytes(clamped)
        }
    }
}

pub fn apply_security_overrides(config: &mut Config) {
    if config.scanning.follow_symlinks {
        eprintln!("SECURITY: follow_symlinks forced off");
        config.scanning.follow_symlinks = false;
    }

    if config.scanning.include_hidden {
        eprintln!("SECURITY: include_hidden forced off");
        config.scanning.include_hidden = false;
    }

    if config.scanning.max_depth == 0 || config.scanning.max_depth > SAFE_MAX_SCAN_DEPTH {
        eprintln!(
            "SECURITY: max_depth clamped to {}",
            SAFE_MAX_SCAN_DEPTH
        );
        config.scanning.max_depth = SAFE_MAX_SCAN_DEPTH;
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output: OutputConfig::default(),
            scanning: ScanningConfig::default(),
            categories: CategoryConfig::default(),
            priority: PriorityWeights::default(),
            priority_rules: default_priority_rules(),
            exclude: ExcludeConfig::default(),
            signatures: SignatureConfig::default(),
            content: ContentConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Pipe,
    Full,
    Json,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Pipe
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    pub format: OutputFormat,
    pub default_budget_tokens: usize,
    pub default_budget_bytes: usize,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: OutputFormat::Pipe,
            default_budget_tokens: 4000,
            default_budget_bytes: 16_000,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ScanningConfig {
    pub use_gitignore: bool,
    pub include_hidden: bool,
    pub max_depth: usize,
    pub follow_symlinks: bool,
    pub no_git_safety: bool,
}

impl Default for ScanningConfig {
    fn default() -> Self {
        Self {
            use_gitignore: true,
            include_hidden: false,
            max_depth: 20,
            follow_symlinks: false,
            no_git_safety: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CategoryConfig {
    pub code: Category,
    pub docs: Category,
    pub config: Category,
    pub build: Category,
    pub data: Category,
}

impl Default for CategoryConfig {
    fn default() -> Self {
        Self {
            code: Category::new(
                &[
                    "rs", "go", "py", "ts", "tsx", "js", "jsx", "c", "cpp", "h", "hpp", "java",
                    "rb", "ex", "exs",
                ],
                100,
            ),
            docs: Category::new(&["md", "mdx", "txt", "rst", "adoc"], 90),
            config: Category::new(&["toml", "yaml", "yml", "json", "ini", "cfg"], 80),
            build: Category::new(&["lock", "sum"], 20),
            data: Category::new(&["csv", "sql"], 30),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Category {
    pub extensions: Vec<String>,
    pub priority: i32,
}

impl Category {
    fn new(extensions: &[&str], priority: i32) -> Self {
        Self {
            extensions: extensions.iter().map(|ext| ext.to_string()).collect(),
            priority,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PriorityRule {
    pub pattern: String,
    pub priority: i32,
}

/// Configurable priority weight adjustments.
/// These modify the base priority score for files based on their characteristics.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PriorityWeights {
    /// Default priority for files that don't match any rule (default: 50)
    pub default_priority: i32,
    /// Boost for entry point files like main.rs, lib.rs, index.ts (default: 40)
    pub entrypoint_boost: i32,
    /// Boost for code files at repository root (default: 20)
    pub root_code_boost: i32,
    /// Boost for files in focus directories like src/, lib/, cmd/ (default: 15)
    pub focus_dir_boost: i32,
    /// Penalty for test files and directories (default: -40)
    pub test_penalty: i32,
    /// Penalty for fixture/mock files and directories (default: -25)
    pub fixture_penalty: i32,
    /// Penalty per depth level beyond 2 (default: -5)
    pub depth_penalty_step: i32,
    /// Maximum depth penalty (default: -30)
    pub max_depth_penalty: i32,
}

impl Default for PriorityWeights {
    fn default() -> Self {
        Self {
            default_priority: 50,
            entrypoint_boost: 40,
            root_code_boost: 20,
            focus_dir_boost: 15,
            test_penalty: -40,
            fixture_penalty: -25,
            depth_penalty_step: -5,
            max_depth_penalty: -30,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ExcludeConfig {
    pub patterns: Vec<String>,
}

impl Default for ExcludeConfig {
    fn default() -> Self {
        Self {
            patterns: vec![
                "target/".to_string(),
                "node_modules/".to_string(),
                "dist/".to_string(),
                "build/".to_string(),
                ".git/".to_string(),
                "__pycache__/".to_string(),
                "*.pyc".to_string(),
                ".DS_Store".to_string(),
                "*.min.js".to_string(),
                "*.min.css".to_string(),
                "vendor/".to_string(),
                ".venv/".to_string(),
                "venv/".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SignatureConfig {
    pub enabled: bool,
    pub languages: Vec<String>,
    pub include_functions: bool,
    pub include_structs: bool,
    pub include_traits: bool,
    pub include_interfaces: bool,
    pub include_classes: bool,
    pub include_types: bool,
    pub include_constants: bool,
    pub max_signature_length: usize,
}

impl Default for SignatureConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            languages: vec![
                "rust".to_string(),
                "go".to_string(),
                "python".to_string(),
                "typescript".to_string(),
                "javascript".to_string(),
                "c".to_string(),
                "cpp".to_string(),
            ],
            include_functions: true,
            include_structs: true,
            include_traits: true,
            include_interfaces: true,
            include_classes: true,
            include_types: true,
            include_constants: true,
            max_signature_length: 200,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ContentConfig {
    pub enabled: bool,
    pub full_budget_ratio: f64,
    pub max_full_tokens: usize,
    pub max_snippet_tokens: usize,
    pub exclude_patterns: Vec<String>,
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            full_budget_ratio: 0.6,
            max_full_tokens: 500,
            max_snippet_tokens: 200,
            exclude_patterns: vec![
                "*.lock".to_string(),
                "*.min.js".to_string(),
                "*.min.css".to_string(),
            ],
        }
    }
}

fn default_priority_rules() -> Vec<PriorityRule> {
    vec![
        PriorityRule {
            pattern: "README*".to_string(),
            priority: 200,
        },
        PriorityRule {
            pattern: "AGENTS.md".to_string(),
            priority: 200,
        },
        PriorityRule {
            pattern: "CLAUDE.md".to_string(),
            priority: 200,
        },
        PriorityRule {
            pattern: "Cargo.toml".to_string(),
            priority: 150,
        },
        PriorityRule {
            pattern: "package.json".to_string(),
            priority: 150,
        },
        PriorityRule {
            pattern: "go.mod".to_string(),
            priority: 150,
        },
        PriorityRule {
            pattern: "src/main.*".to_string(),
            priority: 140,
        },
        PriorityRule {
            pattern: "src/lib.*".to_string(),
            priority: 140,
        },
        PriorityRule {
            pattern: "**/mod.rs".to_string(),
            priority: 130,
        },
        PriorityRule {
            pattern: "**/*_test.*".to_string(),
            priority: 50,
        },
        PriorityRule {
            pattern: "**/test_*".to_string(),
            priority: 50,
        },
        PriorityRule {
            pattern: "**/*.lock".to_string(),
            priority: 10,
        },
    ]
}

const SECURITY_EXCLUDE_PATTERNS: &[&str] = &[
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    "credentials*",
];

pub fn security_exclude_patterns() -> Vec<String> {
    SECURITY_EXCLUDE_PATTERNS
        .iter()
        .map(|pattern| pattern.to_string())
        .collect()
}
