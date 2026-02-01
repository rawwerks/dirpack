# Eval Harness

This folder captures baseline metrics and the agreed thresholds for dirpack's allocation quality.

## Metrics (Tier 1)

- **Budget overshoot**: `(actual_tokens - target)/target` if actual > target, else 0.
  - **Threshold**: must be **<= 2%** for all evaluated budgets.
- **Entry point coverage**: fraction of expected entry points found in output.
  - Expected entry points are inferred per repo from the files that exist:
    `Cargo.toml`, `pyproject.toml`, `package.json`, `main.rs`, `lib.rs`, `index.ts`, `index.tsx`,
    `main.py`, `app.py`, `__init__.py`.
  - **Threshold**: **100%** at budgets **>= 500 tokens**.
- **Tree ratio**: `tree_tokens / target_tokens` (tree-only segments).
  - **Threshold**: **<= 40%** of budget.
- **Elapsed time**: wall clock time for `pack` (ms).
- **Tokens/sec**: `actual_tokens / elapsed_seconds`.

## Metrics (Tier 2/Informational)

- **Coverage spread**: fraction of top-level dirs that have at least one detailed file.
- **Lopsidedness**: max detailed-files-per-top-dir divided by mean.
- **Signature files**: count of files with signatures.
- **Path diversity**: count of unique path prefixes (depth=2) among detailed files.

## Baseline

`eval/baseline.json` captures a snapshot for:
- `dirpack` (this repo)
- `dspy`
- `build123d`

Budgets: 500, 1000, 2000, 4000 tokens.

Generate a fresh baseline with:

```bash
cargo run -- eval <path> --budgets 500,1000,2000,4000 --pretty
```

## Qualitative Review (MANDATORY)

**After quantitative tests PASS**, you MUST run a dogfood check:

```bash
./target/release/dirpack pack . -b 2000
```

Review the output and answer these questions:

### Checklist

1. **README.md content visible?**
   - The main README.md should have content or headings in output
   - Test fixture READMEs should NOT appear before main README

2. **No duplicate signatures?**
   - Scan for repeated function/struct names
   - Each signature should appear exactly once

3. **Priority makes sense?**
   - Core code (src/) should have more coverage than tests/fixtures/
   - Entry points (main.rs, lib.rs, mod.rs) should appear early
   - Documentation (README, DESIGN) should be prioritized over config files

4. **Architecture understandable?**
   - Could a new developer figure out where to start?
   - Are the main modules and their relationships clear?
   - Is there enough context to understand the codebase purpose?

### Failure Criteria

If ANY of these are true, the PR is NOT ready:
- Main README.md has no content in output
- Duplicate signatures waste budget
- Test fixtures prioritized over core code
- Output is unreadable wall of text with no structure

### Rating

Rate the output 1-10 for "usefulness as codebase onboarding aid".
Target: **7+** before merge to main.
