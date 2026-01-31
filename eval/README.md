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
