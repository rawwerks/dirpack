# Dirpack Improvement Changelog

## Stable v3 (2026-01-31) 🎉
**Lopsidedness fix merged**
- Budget accuracy: all ≤2% overshoot ✅
- Tree ratio: 31-39% (all PASS!) ✅
- Lopsidedness: 1.3-2.8 (all under 3!) ✅
- Signatures: 11-476 at all budgets ✅

| Budget | Tree% | Sigs | Lopsidedness | Status |
|--------|-------|------|--------------|--------|
| 500t | 39% | 11 | 1.33 | PASS |
| 1000t | 35% | 22 | 1.66 | PASS |
| 2000t | 33% | 38 | 1.69 | PASS |
| 4000t | 32% | 76 | 2.32 | PASS |
| 8000t | 31% | 165 | 2.75 | PASS |
| 16000t | 16% | 476 | 2.76 | PASS |

**Key change**: `MAX_FILES_PER_DIR = 8` caps files per directory for even coverage.

## Stable v2 (2026-01-31)
**Tree ratio fix merged**
- Budget accuracy: all ≤2% overshoot ✅
- Tree ratio: 31-38% (all PASS!) ✅
- Lopsidedness: 1.5-5.8 (next target: <3)
- Signatures: 12-469 at all budgets ✅

| Budget | Tree% | Sigs | Status |
|--------|-------|------|--------|
| 500t | 38% | 12 | PASS |
| 1000t | 35% | 22 | PASS |
| 2000t | 33% | 38 | PASS |
| 4000t | 32% | 76 | PASS |
| 8000t | 31% | 164 | PASS |
| 16000t | 17% | 469 | PASS |

## Stable v1 (2026-01-31)
**Baseline established**
- Budget accuracy: all ≤2% overshoot ✅
- Tree ratio: 40-49% (WARN_TREE at low budgets)
- Lopsidedness: 3.0-4.5
- Signatures: present at all budgets

---
*Updated automatically by exploration process*
