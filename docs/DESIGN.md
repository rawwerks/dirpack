# dirpack Design Document

A Rust CLI that creates budgeted directory indexes for AI coding agents.

## Overview

dirpack generates compressed directory representations that fit within token/byte budgets. It uses progressive disclosure—spine first, then signatures, then content—stopping when the budget is exhausted.

---

## 1. Config Schema (dirpack.toml)

```toml
# dirpack.toml - Project configuration

[output]
format = "pipe"           # "pipe" | "full" | "json"
default_budget_tokens = 4000
default_budget_bytes = 16000

[scanning]
use_gitignore = true      # Respect .gitignore
include_hidden = false    # Include .dotfiles/dirs
max_depth = 20            # Max recursion depth
follow_symlinks = false

# File categories - used for prioritization and signature extraction
[categories.code]
extensions = ["rs", "go", "py", "ts", "tsx", "js", "jsx", "c", "cpp", "h", "hpp", "java", "rb", "ex", "exs"]
priority = 100

[categories.docs]
extensions = ["md", "mdx", "txt", "rst", "adoc"]
priority = 90

[categories.config]
extensions = ["toml", "yaml", "yml", "json", "ini", "cfg"]
priority = 80

[categories.build]
extensions = ["lock", "sum"]
priority = 20

[categories.data]
extensions = ["csv", "sql"]
priority = 30

# Priority rules - higher = more important, included first in budget
[[priority_rules]]
pattern = "README*"
priority = 200

[[priority_rules]]
pattern = "AGENTS.md"
priority = 200

[[priority_rules]]
pattern = "CLAUDE.md"
priority = 200

[[priority_rules]]
pattern = "Cargo.toml"
priority = 150

[[priority_rules]]
pattern = "package.json"
priority = 150

[[priority_rules]]
pattern = "go.mod"
priority = 150

[[priority_rules]]
pattern = "src/main.*"
priority = 140

[[priority_rules]]
pattern = "src/lib.*"
priority = 140

[[priority_rules]]
pattern = "**/mod.rs"
priority = 130

[[priority_rules]]
pattern = "**/*_test.*"
priority = 50

[[priority_rules]]
pattern = "**/test_*"
priority = 50

[[priority_rules]]
pattern = "**/*.lock"
priority = 10

# Exclude patterns (gitignore syntax)
[exclude]
patterns = [
    "target/",
    "node_modules/",
    "dist/",
    "build/",
    ".git/",
    "__pycache__/",
    "*.pyc",
    ".DS_Store",
    "*.min.js",
    "*.min.css",
    "vendor/",
    ".venv/",
    "venv/",
]

# Tree-sitter signature extraction
[signatures]
enabled = true
# Languages to extract signatures from (must have tree-sitter grammar)
languages = ["rust", "go", "python", "typescript", "javascript", "c", "cpp"]
# What to extract
include_functions = true
include_structs = true
include_traits = true
include_interfaces = true
include_classes = true
include_types = true
include_constants = true
# Max signature length before truncation
max_signature_length = 200
```

---

## 2. CLI Interface

```
dirpack - Create budgeted directory indexes for AI coding agents

USAGE:
    dirpack [OPTIONS] [COMMAND]

COMMANDS:
    pack      Create a directory index (default if no command given)
    init      Create a default dirpack.toml config file
    tree      Display directory structure (debug mode)
    help      Print help information

OPTIONS:
    -h, --help       Print help
    -V, --version    Print version
```

### `dirpack pack` (default command)

```
Create a directory index within a budget

USAGE:
    dirpack pack [OPTIONS] [PATH]

ARGS:
    [PATH]  Directory to pack [default: .]

OPTIONS:
    -t, --target-tokens <N>     Token budget (mutually exclusive with --target-bytes)
    -b, --target-bytes <N>      Byte budget (mutually exclusive with --target-tokens)
    -o, --output <FILE>         Output file [default: stdout]
    -f, --format <FORMAT>       Output format: pipe, full, json [default: from config]
    -c, --config <FILE>         Config file path [default: ./dirpack.toml or ~/.config/dirpack/config.toml]
    -d, --depth <N>             Max recursion depth
    -e, --exclude <PATTERN>     Additional exclude patterns (can be repeated)
    -i, --include <PATTERN>     Force include patterns (overrides excludes)
        --no-git                Don't use git ls-files even if available
        --no-signatures         Skip tree-sitter signature extraction
        --title <TITLE>         Title for the index [default: directory name]
    -v, --verbose               Show what's being included/excluded
    -q, --quiet                 Suppress warnings
```

### `dirpack init`

```
Create a default dirpack.toml configuration file

USAGE:
    dirpack init [OPTIONS]

OPTIONS:
    -o, --output <FILE>    Output path [default: ./dirpack.toml]
        --global           Create in ~/.config/dirpack/config.toml
        --force            Overwrite existing config
```

### `dirpack tree`

```
Display directory structure (for debugging)

USAGE:
    dirpack tree [OPTIONS] [PATH]

ARGS:
    [PATH]  Directory to scan [default: .]

OPTIONS:
    -d, --depth <N>         Max depth to display
        --show-priority     Show computed priority for each file
        --show-category     Show detected category for each file
```

---

## 3. Module Structure

```
src/
├── main.rs              # Entry point, CLI dispatch
├── lib.rs               # Public library API
├── cli.rs               # clap command definitions
├── config.rs            # TOML config loading and defaults
├── error.rs             # Error types
│
├── scanner/
│   ├── mod.rs           # Scanner trait and entry point
│   ├── git.rs           # Git-aware scanning (git ls-files)
│   ├── walk.rs          # Fallback directory walking
│   └── entry.rs         # FileEntry struct with metadata
│
├── priority.rs          # Priority calculation from rules
├── budget.rs            # Budget tracking (bytes/tokens)
├── tokenizer.rs         # Token counting (tiktoken or simple)
│
├── packer/
│   ├── mod.rs           # Progressive disclosure algorithm
│   ├── spine.rs         # Directory tree structure
│   ├── signatures.rs    # Tree-sitter extraction
│   └── content.rs       # Full file content inclusion
│
└── format/
    ├── mod.rs           # Formatter trait
    ├── pipe.rs          # Pipe-delimited output (Vercel-style)
    ├── full.rs          # Full pack with sections
    └── json.rs          # Structured JSON output
```

---

## 4. Output Formats

### Pipe-Delimited (Vercel-style)

Compact, single-line format optimized for token efficiency:

```
[Project Name]|root: ./path|IMPORTANT: notes|dirs:{subdir1,subdir2}|subdir1:{file1.rs,file2.rs}|file1.rs:fn main(),fn helper()|...
```

Structure:
- `[Title]` - Project/directory name
- `root: path` - Root path
- `IMPORTANT: ...` - Optional notes (from config or auto-detected README)
- `dirs:{...}` - Top-level directories
- `dirname:{...}` - Files in each directory
- `filename:signature1,signature2` - Code signatures (if enabled)

### Full Pack

Structured multi-section format for maximum context:

```markdown
# [Project Name]

Root: `./path`

## Structure

```
src/
├── main.rs
├── lib.rs
└── utils/
    └── helpers.rs
```

## Key Files

### src/main.rs (priority: 140)
```rust
fn main() { ... }
fn setup() -> Config { ... }
```

### src/lib.rs (priority: 140)
```rust
pub mod utils;
pub struct App { ... }
impl App { ... }
```

## Signatures

### src/utils/helpers.rs
- `fn format_output(data: &Data) -> String`
- `fn parse_input(raw: &str) -> Result<Input>`

## Content (budget remaining: 2000 tokens)

### README.md
[Full content here...]
```

### JSON

Machine-readable structured output:

```json
{
  "title": "Project Name",
  "root": "./path",
  "budget": {
    "target_tokens": 4000,
    "used_tokens": 3847,
    "target_bytes": null,
    "used_bytes": 15234
  },
  "tree": {
    "dirs": ["src", "tests"],
    "files": ["Cargo.toml", "README.md"]
  },
  "files": [
    {
      "path": "src/main.rs",
      "category": "code",
      "priority": 140,
      "signatures": ["fn main()", "fn setup() -> Config"],
      "content": null
    }
  ]
}
```

---

## 5. Progressive Disclosure Algorithm

The packer fills the budget in phases, stopping when exhausted:

```
Phase 1: SPINE (required baseline)
├── Compute directory tree structure
├── Calculate priority for each file
├── Emit tree skeleton
└── Cost: ~10-20% of typical budget

Phase 2: SIGNATURES (code understanding)
├── Sort files by priority (descending)
├── For each code file:
│   ├── Extract signatures via tree-sitter
│   ├── Add to output if within budget
│   └── Skip if would exceed budget
└── Cost: ~20-40% of typical budget

Phase 3: SUMMARIES (context)
├── For high-priority non-code files (README, docs):
│   ├── Extract first N lines or doc section
│   ├── Add to output if within budget
└── Cost: ~10-20% of typical budget

Phase 4: CONTENT (full text)
├── For remaining budget:
│   ├── Include full content of highest-priority files
│   ├── Stop when budget exhausted
└── Cost: remainder of budget
```

### Budget Tracking

```rust
pub struct Budget {
    target: BudgetTarget,  // Tokens or Bytes
    used: usize,
}

pub enum BudgetTarget {
    Tokens(usize),
    Bytes(usize),
}

impl Budget {
    fn remaining(&self) -> usize;
    fn try_add(&mut self, content: &str) -> bool;  // Returns false if would exceed
    fn would_fit(&self, content: &str) -> bool;
}
```

### Token Counting

For `--target-tokens`, use a simple approximation:
- Words (whitespace-separated) × 1.3 for English text
- Characters / 4 for code (more conservative)

Or optionally integrate `tiktoken-rs` for accurate GPT-family counts.

---

## 6. Key Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
ignore = "0.4"                    # gitignore handling (ripgrep's crate)
walkdir = "2"                     # fallback directory walking
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tree-sitter-go = "0.23"
tree-sitter-python = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-c = "0.23"
tree-sitter-cpp = "0.23"
thiserror = "2"
anyhow = "1"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

---

## 7. Example Usage

```bash
# Basic usage - pack current directory, 4000 token budget
dirpack -t 4000

# Pack specific directory with byte budget
dirpack pack ./my-project -b 16000

# Pipe-delimited output for AGENTS.md
dirpack -t 2000 -f pipe > index.txt

# Full pack to file
dirpack -t 8000 -f full -o CONTEXT.md

# JSON for programmatic use
dirpack -f json | jq '.files | map(select(.priority > 100))'

# Verbose mode to see what's included
dirpack -t 4000 -v

# Use custom config
dirpack -c ./my-dirpack.toml -t 4000

# Initialize config in project
dirpack init

# Debug: see tree with priorities
dirpack tree --show-priority
```

---

## 8. Future Considerations

- **Watch mode**: Re-generate on file changes
- **Diff mode**: Show what changed since last pack
- **Remote**: Pack from git URL without clone
- **Incremental**: Cache tree-sitter parses
- **LLM integration**: Use LLM to generate summaries (opt-in)

---

DESIGN COMPLETE
