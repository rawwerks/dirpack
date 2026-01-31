# Dirpack Eval Summary
Generated: Sat Jan 31 03:02:27 PM EST 2026

## dspy-stable-v1

| Budget | Est Tok | Overshoot% | Sigs | Dirs | Lopsided | Tree% | Status |
|--------|---------|------------|------|------|----------|-------|--------|
| 500 | 505 | 1.0 | 10 | 14 | 3.26 | 49 | WARN_TREE |
| 1000 | 1009 | 0.9 | 20 | 21 | 3.25 | 45 | WARN_TREE |
| 2000 | 2013 | 0.7 | 32 | 37 | 4.02 | 43 | WARN_TREE |
| 4000 | 4026 | 0.7 | 65 | 74 | 4.40 | 42 | WARN_TREE |
| 8000 | 8055 | 0.7 | 157 | 135 | 5.82 | 35 | PASS |
| 16000 | 16054 | 0.3 | 469 | 135 | 5.82 | 17 | PASS |

## build123d-stable-v1

| Budget | Est Tok | Overshoot% | Sigs | Dirs | Lopsided | Tree% | Status |
|--------|---------|------------|------|------|----------|-------|--------|
| 500 | 503 | 0.6 | 10 | 6 | 3.00 | 46 | WARN_TREE |
| 1000 | 1008 | 0.8 | 21 | 6 | 4.40 | 42 | WARN_TREE |
| 2000 | 2016 | 0.8 | 45 | 10 | 4.43 | 41 | WARN_TREE |
| 4000 | 4018 | 0.5 | 81 | 19 | 4.46 | 40 | WARN_TREE |
| 8000 | 8024 | 0.3 | 187 | 23 | 7.00 | 39 | PASS |
| 16000 | 16025 | 0.2 | 600 | 23 | 7.00 | 20 | PASS |

## Thresholds
- **PASS**: overshoot ≤2%, tree_ratio ≤40%
- **WARN_TREE**: tree_ratio >40%
- **WARN_UNDERSHOOT**: undershoot >50%
- **FAIL**: overshoot >2%

## Lopsidedness
- 1.0 = perfectly even distribution
- >3.0 = some directories have many more files than average
