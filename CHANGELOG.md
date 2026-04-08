# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to
Semantic Versioning.

## [Unreleased]

### Added
- Pi launch-context extension at `.pi/extensions/dirpack-launch-context.ts` for automatic hidden `dirpack` injection on session start, enabled by default with a 2000-token budget and `/dirpack on|off|budget|status|refresh` controls.
- `/dirpack create <tokens>` one-shot command for explicit hidden context injection without persisting config changes.
- Claude Code plugin at `integrations/claude-code/` (marketplace `rawwerks-dirpack`). Injects a fresh token-budgeted dirpack index into every Claude Code session via a `SessionStart` hook, and ships five slash commands: `/dirpack:status`, `/dirpack:on`, `/dirpack:off`, `/dirpack:budget <N>`, and `/dirpack:create <N>` (the last being a one-shot pack of the current working directory that doesn't mutate persisted config). Default budget is 2000 tokens; state persists to `${XDG_CONFIG_HOME:-~/.config}/dirpack/cc-plugin.json`.

### Fixed
- Submodule-like edge-case tests now stage their nested `.git` file in a temp copy of the fixture so colocated `jj` repos do not report phantom deletions under `tests/fixtures/submodule_like/`.

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
