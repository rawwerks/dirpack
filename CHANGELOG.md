# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to
Semantic Versioning.

## [Unreleased]

### Fixed
- Use unused full-content budget for snippets so content fills available budget.
- Apply per-file snippet cap only when the snippet pool is tight.

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
