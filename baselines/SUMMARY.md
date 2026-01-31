# Dirpack Eval Summary
Generated: Sat Jan 31 02:51:19 PM EST 2026

## build123d-baseline-v2

| Budget | Est Tok | Overshoot% | Sigs | Dirs | Lopsided | Tree% | Status |
|--------|---------|------------|------|------|----------|-------|--------|
| 500 | 3133 | 526.6 | 0 | 23 | 7.00 | 100 | FAIL |
| 1000 | 3133 | 213.3 | 0 | 23 | 7.00 | 100 | FAIL |
| 2000 | 3133 | 56.6 | 0 | 23 | 7.00 | 100 | FAIL |
| 4000 | 4057 | 1.4 | 35 | 23 | 7.00 | 77 | WARN_TREE |
| 8000 | 8196 | 2.5 | 191 | 23 | 7.00 | 38 | FAIL |
| 16000 | 16292 | 1.8 | 609 | 23 | 7.00 | 19 | PASS |

## dspy-baseline-v2

| Budget | Est Tok | Overshoot% | Sigs | Dirs | Lopsided | Tree% | Status |
|--------|---------|------------|------|------|----------|-------|--------|
| 500 | 2779 | 455.8 | 0 | 135 | 5.82 | 100 | FAIL |
| 1000 | 2779 | 177.9 | 0 | 135 | 5.82 | 100 | FAIL |
| 2000 | 2779 | 39.0 | 0 | 135 | 5.82 | 100 | FAIL |
| 4000 | 4079 | 2.0 | 38 | 135 | 5.82 | 69 | WARN_TREE |
| 8000 | 8175 | 2.2 | 161 | 135 | 5.82 | 34 | FAIL |
| 16000 | 16400 | 2.5 | 483 | 135 | 5.82 | 17 | FAIL |

## small-project-baseline-v2

| Budget | Est Tok | Overshoot% | Sigs | Dirs | Lopsided | Tree% | Status |
|--------|---------|------------|------|------|----------|-------|--------|
| 500 | 192 | -61.6 | 18 | 4 | 1.78 | 33 | WARN_UNDERSHOOT |
| 1000 | 192 | -80.8 | 18 | 4 | 1.78 | 33 | WARN_UNDERSHOOT |
| 2000 | 192 | -90.4 | 18 | 4 | 1.78 | 33 | WARN_UNDERSHOOT |
| 4000 | 192 | -95.2 | 18 | 4 | 1.78 | 33 | WARN_UNDERSHOOT |
| 8000 | 192 | -97.6 | 18 | 4 | 1.78 | 33 | WARN_UNDERSHOOT |
| 16000 | 192 | -98.8 | 18 | 4 | 1.78 | 33 | WARN_UNDERSHOOT |

## Thresholds
- **PASS**: overshoot ≤2%, tree_ratio ≤40%
- **WARN_TREE**: tree_ratio >40%
- **WARN_UNDERSHOOT**: undershoot >50%
- **FAIL**: overshoot >2%

## Lopsidedness
- 1.0 = perfectly even distribution
- >3.0 = some directories have many more files than average
