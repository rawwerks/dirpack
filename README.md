# dirpack

**Budgeted directory indexes for AI coding agents**

dirpack creates compressed directory representations that fit within token/byte budgets. Unlike other tools, it uses **progressive disclosure**—including structure first, then signatures, then content—stopping exactly when the budget is exhausted.

dirpack was inspired by [recent research from Jude Gao at Vercel](https://vercel.com/blog/agents-md-outperforms-skills-in-our-agent-evals), which suggests that putting a "compressed" index of the directory directly inside `AGENTS.md` outperforms the "progressive disclosure" strategy recommended by [Agent Skills](https://github.com/agentskills/agentskills).

## Features

- **Budget-aware**: Set token or byte limits; output stops at the boundary
- **Tree-sitter signatures**: Extract function/struct/trait signatures for Rust, Go, Python, TypeScript, JavaScript, C, C++
- **Progressive disclosure**: Spine → signatures → summaries → content
- **Git-aware**: Uses `git ls-files` when available, respects `.gitignore`
- **Configurable**: Priority rules, file categories, exclude patterns via TOML

## Installation

### Quick Install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/rawwerks/dirpack/master/install.sh | bash
```

This downloads a pre-built binary for your platform and installs it to `~/.local/bin`.

### Using Cargo

```bash
cargo install dirpack

# or from source
cargo install --git https://github.com/rawwerks/dirpack
```

## Quick Start

```bash
# Pack current directory with 4K token budget
dirpack pack . -t 4000

# Pack with 16KB byte budget, markdown output
dirpack pack . -b 16000 -f full

# View directory tree with priorities
dirpack tree . --show-priority

# Create default config
dirpack init
```

## Output Formats

### Pipe (default)

Compact, single-line format optimized for AGENTS.md:

```
[myproject]|root: ./path|dirs:{src,tests}|src:{main.rs,lib.rs}|main.rs:fn main(),fn setup()
```

### Full

Markdown with sections for structure, signatures, and content.

### JSON

Machine-readable output with budget stats.

## Configuration

Create `dirpack.toml` in your project:

```toml
[output]
format = "pipe"
default_budget_tokens = 4000

[scanning]
use_gitignore = true
include_hidden = false
max_depth = 20

[signatures]
enabled = true
max_signature_length = 200

# High-priority files get included first
[[priority_rules]]
pattern = "README*"
priority = 200

[[priority_rules]]
pattern = "src/main.*"
priority = 140
```

Run `dirpack init` to generate a full default config.

## Pack Cache

By default, `dirpack pack` caches its output on disk. If nothing has changed
(same budget, config, dirpack version, and no files modified) the cache returns
the previous result in milliseconds instead of re-scanning and re-packing.

Cache entries live under `${XDG_CACHE_HOME:-~/.cache}/dirpack/packs/`. Invalidation
is implicit: any change to a file's mtime/size, the config, the dirpack version,
or the budget/format produces a new cache key.

```bash
# Disable cache for one run
dirpack pack . -t 2000 --no-cache

# Or via environment variable
DIRPACK_NO_CACHE=1 dirpack pack . -t 2000

# Or disable permanently in dirpack.toml
# [cache]
# enabled = false
```

To clear the cache: `rm ~/.cache/dirpack/packs/*.json`

## Large File Handling

Files larger than `max_file_size_bytes` (default: 2 MiB) still appear in the
directory spine but are skipped for signature extraction and content inclusion.
This prevents multi-second stalls on repos containing large binary or data files.

```toml
[scanning]
max_file_size_bytes = 2097152  # 2 MiB (default)
# max_file_size_bytes = 0      # disable limit (read everything)
```

## Runtime Limits

`dirpack` enforces a per-process concurrency cap for pack jobs to protect CPU/IO.

- `DIRPACK_PACK_CONCURRENCY_LIMIT`: max concurrent pack jobs (default: available CPU parallelism)
- `DIRPACK_PACK_RETRY_AFTER_SECS`: suggested retry delay for saturated servers (default: 1)

## Pi Extension: Launch Context

This repo ships a pi extension at `.pi/extensions/dirpack-launch-context.ts`.

Symlink it into `~/.pi/agent/extensions/` so pi auto-loads it globally:

```bash
ln -sfn /path/to/dirpack/.pi/extensions/dirpack-launch-context.ts \
  ~/.pi/agent/extensions/dirpack-launch-context.ts
```

Behavior:

- enabled by default
- default budget is `3000` tokens
- on pi session start, it generates a fresh `dirpack pack . -t <budget> -f pipe --root-label .`
- injects that pack as a hidden custom message in the pi session log/context

Slash commands:

- `/dirpack on`
- `/dirpack off`
- `/dirpack budget 4000`
- `/dirpack create 1500`
- `/dirpack status`
- `/dirpack refresh`

`/dirpack create <tokens>` is a one-shot injection for the current cwd. It bypasses the
current on/off setting, does not persist config changes, and records the usual hidden
`dirpack-launch-event` + `dirpack-launch-context` entries in the session log.

Binary resolution order:

1. `PI_DIRPACK_LAUNCH_CONTEXT_COMMAND`
2. `target/debug/dirpack` from this repo
3. `target/release/dirpack` from this repo
4. `dirpack` on `PATH`

## Claude Code Plugin

This repo ships a Claude Code plugin at `integrations/claude-code/` that injects a fresh token-budgeted dirpack index of the current working directory into every new Claude Code session, via a `SessionStart` hook.

Install via Claude Code's plugin marketplace (uses your local repo as the source):

```bash
claude plugin marketplace add /path/to/dirpack/integrations/claude-code
claude plugin install dirpack@rawwerks-dirpack
```

Behavior:

- enabled by default
- default budget is `3000` tokens
- on every new/resumed/cleared session, it runs `dirpack pack . -t <budget> -f pipe` against `$CWD` and emits `hookSpecificOutput.additionalContext`
- state persists to `${XDG_CONFIG_HOME:-~/.config}/dirpack/cc-plugin.json`

Slash commands (Claude Code namespaces plugin commands as `/<plugin>:<command>`):

- `/dirpack:status` — show current config
- `/dirpack:on` — enable SessionStart injection
- `/dirpack:off` — disable SessionStart injection
- `/dirpack:budget 4000` — set persistent token budget
- `/dirpack:create 1500` — one-shot pack of the current working directory at the given token budget (does not persist)

`/dirpack:create <tokens>` is a shortcut for `dirpack pack . -t <tokens> -f pipe --root-label .` with no config mutation — useful when you want a fresh snapshot without changing the automatic injection settings.

Binary resolution order:

1. `dirpack` on `PATH`
2. `~/.local/bin/dirpack`
3. `~/.cargo/bin/dirpack`
4. `/usr/local/bin/dirpack`
5. `/usr/bin/dirpack`

See `integrations/claude-code/README.md` for the full plugin layout and internals.

## How It Works

1. **Scan** directory (git-aware or walkdir fallback)
2. **Prioritize** files by pattern rules and category
3. **Pack** progressively:
   - Phase 1: Directory spine (always included)
   - Phase 2: Code signatures (high-priority files first)
   - Phase 3: Doc summaries (README excerpts)
   - Phase 4: Full content (budget permitting)
4. **Stop** when budget exhausted

## Comparison

| Tool | Budgeting | Signatures | Progressive |
|------|-----------|------------|-------------|
| **dirpack** | ✅ tokens/bytes | ✅ tree-sitter | ✅ 4-phase |
| yek | ✅ tokens | ❌ | ❌ |
| code2prompt | ❌ | ❌ | ❌ |
| repomix | ❌ | ❌ | ❌ |

dirpack's unique value: **progressive disclosure with real signatures**, not just file listings.

## CLI Reference

```
dirpack pack [PATH] [OPTIONS]
  -t, --target-tokens <N>   Token budget
  -b, --target-bytes <N>    Byte budget
  -f, --format <FORMAT>     Output: pipe, full, json
  -o, --output <FILE>       Write to file instead of stdout
  -c, --config <FILE>       Custom config path
  --root-label <LABEL>      Override root path in output (e.g., '.')
  --no-git                  Don't use git ls-files
  --no-signatures           Skip tree-sitter extraction
  --no-cache                Disable the on-disk pack cache for this run
  -v, --verbose             Show stats

dirpack tree [PATH]
  --show-priority           Display computed priorities

dirpack init
  --global                  Create in ~/.config/dirpack/
  --force                   Overwrite existing
```

## License

MIT
