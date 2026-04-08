use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::config::OutputFormat;

#[derive(Debug, Parser)]
#[command(
    name = "dirpack",
    version,
    about = "Create budgeted directory indexes for AI coding agents",
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create a directory index within a budget
    Pack(PackArgs),
    /// Create a default dirpack.toml configuration file
    Init(InitArgs),
    /// Display directory structure (debug mode)
    Tree(TreeArgs),
    /// Evaluate allocation metrics for a repository
    Eval(EvalArgs),
}

#[derive(Debug, Args, Clone)]
pub struct PackArgs {
    /// Directory to pack
    #[arg(value_name = "PATH", default_value = ".")]
    pub path: PathBuf,

    /// Token budget (mutually exclusive with --target-bytes)
    #[arg(
        short = 't',
        long = "target-tokens",
        aliases = ["token-budget", "budget"],
        value_name = "N",
        conflicts_with = "target_bytes"
    )]
    pub target_tokens: Option<usize>,

    /// Byte budget (mutually exclusive with --target-tokens)
    #[arg(short = 'b', long = "target-bytes", value_name = "N", conflicts_with = "target_tokens")]
    pub target_bytes: Option<usize>,

    /// Output file [default: stdout]
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Override the root path label shown in output
    #[arg(long = "root-label", value_name = "LABEL")]
    pub root_label: Option<String>,

    /// Output format: pipe, full, json [default: from config]
    #[arg(short = 'f', long = "format", value_name = "FORMAT")]
    pub format: Option<OutputFormat>,

    /// Config file path
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Max recursion depth
    #[arg(short = 'd', long = "depth", value_name = "N")]
    pub depth: Option<usize>,

    /// Additional exclude patterns (can be repeated)
    #[arg(short = 'e', long = "exclude", value_name = "PATTERN", action = clap::ArgAction::Append)]
    pub exclude: Vec<String>,

    /// Force include patterns (overrides excludes)
    #[arg(short = 'i', long = "include", value_name = "PATTERN", action = clap::ArgAction::Append)]
    pub include: Vec<String>,

    /// Don't use git ls-files even if available
    #[arg(long = "no-git")]
    pub no_git: bool,

    /// Skip tree-sitter signature extraction
    #[arg(long = "no-signatures")]
    pub no_signatures: bool,

    /// Disable the on-disk pack cache for this invocation
    #[arg(long = "no-cache")]
    pub no_cache: bool,

    /// Title for the index [default: directory name]
    #[arg(long = "title", value_name = "TITLE")]
    pub title: Option<String>,

    /// Show what's being included/excluded
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Print timing and throughput stats
    #[arg(long = "timing")]
    pub timing: bool,

    /// Suppress warnings
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,
}

impl Default for PackArgs {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            target_tokens: None,
            target_bytes: None,
            output: None,
            root_label: None,
            format: None,
            config: None,
            depth: None,
            exclude: Vec::new(),
            include: Vec::new(),
            no_git: false,
            no_signatures: false,
            no_cache: false,
            title: None,
            verbose: false,
            timing: false,
            quiet: false,
        }
    }
}

#[derive(Debug, Args, Clone)]
pub struct InitArgs {
    /// Output path
    #[arg(short = 'o', long = "output", value_name = "FILE", default_value = "dirpack.toml")]
    pub output: PathBuf,

    /// Create in ~/.config/dirpack/config.toml
    #[arg(long = "global")]
    pub global: bool,

    /// Overwrite existing config
    #[arg(long = "force")]
    pub force: bool,
}

#[derive(Debug, Args, Clone)]
pub struct TreeArgs {
    /// Directory to scan
    #[arg(value_name = "PATH", default_value = ".")]
    pub path: PathBuf,

    /// Max depth to display
    #[arg(short = 'd', long = "depth", value_name = "N")]
    pub depth: Option<usize>,

    /// Show computed priority for each file
    #[arg(long = "show-priority")]
    pub show_priority: bool,

    /// Show detected category for each file
    #[arg(long = "show-category")]
    pub show_category: bool,
}

#[derive(Debug, Args, Clone)]
pub struct EvalArgs {
    /// Directory to evaluate
    #[arg(value_name = "PATH", default_value = ".")]
    pub path: PathBuf,

    /// Budgets to evaluate (comma-separated)
    #[arg(long = "budgets", value_name = "N", value_delimiter = ',', num_args = 1..)]
    pub budgets: Vec<usize>,

    /// Pretty-print JSON output
    #[arg(long = "pretty")]
    pub pretty: bool,
}
