# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to
Semantic Versioning.

## [Unreleased]

### Added
- On-disk pack cache that short-circuits repeated packs when inputs are unchanged. Cache key is a SHA-256 of the dirpack version, canonical root, budget target, config digest, and a sorted manifest of every scanned file's `(path, size, mtime)`. Opt out with `--no-cache`, `DIRPACK_NO_CACHE=1`, or `[cache] enabled = false` in dirpack.toml.
- `max_file_size_bytes` scanning config (default: 2 MiB). Files larger than this limit still appear in the directory spine but are skipped for signature extraction and content reads, preventing multi-second stalls on repos with large binary/data files.
- CLI `--exclude` patterns are now merged into config exclude patterns and applied in all scan modes (previously `--exclude` was parsed but not wired into `run_pack`).

### Fixed
- User-configured `[exclude] patterns` now apply in `--no-git` mode. Previously they were silently dropped when `use_git` was false, which meant `--no-git` packs could scan directories that should have been excluded (e.g., `target/`, `node_modules/`).

## [0.3.3] - 2026-02-20

### Added
- CLI alias `--budget` for `--target-tokens`.

### Changed
- Raised max token budget from 8,000 to 200,000 (and max byte budget from 32KB to 800KB).

## [0.3.2] - 2026-02-03

### Added
- CLI alias `--token-budget` for `--target-tokens`.

## [0.3.1] - 2026-02-03

### Fixed
- Use unused full-content budget for snippets so content fills available budget.
- Apply per-file snippet cap only when the snippet pool is tight.

### Added
- Eval metrics now report budget utilization ratio and include a utilization guard on a content-heavy fixture.

### Changed
- Utilization guard allows modest variance and skips enforcement when content is insufficient.

## [0.3.0] - 2026-02-03

### Added
- Hybrid content budgeting for progressive disclosure (full content for small/high-priority files, then per-file snippets from remaining budget).
- Configurable content controls: `full_budget_ratio`, `max_full_tokens`, `max_snippet_tokens`, `exclude_patterns`.

## [0.2.0] - 2026-02-02

### Added
- `--root-label` flag to override displayed root path in output
- One-liner install script: `curl -fsSL https://raw.githubusercontent.com/rawwerks/dirpack/master/install.sh | bash`
- GitHub release workflow for pre-built binaries (Linux/macOS, x86_64/aarch64)
- Truncation indicator `[+N files truncated]` shows what was cut from output
- Pack concurrency limiter via `DIRPACK_PACK_CONCURRENCY_LIMIT` env var
- Budget caps and safe config overrides to prevent runaway scanning
- Configurable priority weights in `[priority]` config section
- 44 edge case tests covering all fixture scenarios
- Visual inspection output in eval harness

### Changed
- Round-robin signature budget distribution across top-level directories
- Source code (`src/`) now prioritized over test fixtures in output
- Tree budget ratio reduced to 30% for better signature coverage

### Fixed
- Single directory no longer dominates signature budget
- Test fixtures no longer appear before core code at low budgets

## [0.1.0] - 2026-02-01

### Added
- Initial release of the dirpack CLI with budgeted directory packing.
- Tree-sitter signature extraction and multiple output formats.
