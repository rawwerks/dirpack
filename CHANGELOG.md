# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to
Semantic Versioning.

## [Unreleased]

## [0.2.0] - 2026-02-02

### Added
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
