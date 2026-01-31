# Space-Filling Algorithm Ideas

## Core Insight

Two complementary lenses:
- **Decimation:** "What can I remove with minimal error?"
- **Space-filling:** "Given a fixed budget, how do I *cover* the most important 'space' as uniformly as possible?"

Current plan (78h + hnk) is decimation-focused. Space-filling adds **coverage guarantees** and **diversity**.

---

## Ideas to Evaluate

### 1) Space-filling curves (Hilbert/Z-order)
Map files to feature space, apply curve for locality-preserving ordering.
- File features: path tokens, extension, size, role flags
- Taking first N items gives balanced slice, not just "hot files in one corner"

### 2) Multi-choice knapsack
Each file offers multiple LOD options with different costs.
- Greedy: start minimal, repeatedly apply best upgrade (Δvalue/Δcost)
- Inverse of decimation: "start with coverage, add detail where it buys most"

### 3) Stratified sampling
Force coverage across strata:
- 40% core code (src/lib/app)
- 15% config/build/CI
- 15% docs
- 20% tests
- 10% long-tail sampler

### 4) Blue-noise / Poisson-disc
Avoid selecting files that are "too similar" (same folder, same role).
- Diversity penalty based on path prefix + extension similarity
- No clumps, more informative spread

### 5) Hybrid algorithm
A. **Coverage skeleton** (space-filling): tree + per-dir summaries + minimal entries
B. **Fill with upgrades** (knapsack): manifests→detail, APIs→signatures
C. **Decimate if tight** (error-guided): collapse noisy clusters

---

## Questions to Debate

1. Does current tiered allocation (hnk) already achieve coverage? Or does it still produce "lopsided" results?

2. Is stratified sampling (budget % per file type) worth the complexity?

3. Should we add a diversity penalty to avoid selecting similar files?

4. Is the hybrid (skeleton → upgrades → decimate) better than pure tiered allocation?

5. What's the simplest change that adds coverage guarantees?
