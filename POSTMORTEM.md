# Dirpack Regression Post-Mortem

**Date**: 2026-01-31
**Participants**: Claude (orchestrator), Claude (dirpack team), Codex (dirpack team), Eval Team (6 agents)

## Executive Summary

The team spent a session "improving" dirpack with metrics-driven changes that resulted in **worse actual output**. Quantitative metrics all passed while qualitative usefulness dropped from ~6/10 to ~3/10.

## Timeline of Regressions

### Regression #1: Commit 429d3c7 "Fix lopsidedness"
- **Change**: Added `MAX_FILES_PER_DIR=8` cap
- **Intent**: Spread coverage evenly across directories
- **Actual Result**: Dropped files from 12 to 8 in scripts directory
- **Files Lost**: sync_recordings.sh, this-machine-agent-orientation, trim-silence, trim-silence.ffmpeg-backup

### Regression #2: Commit a57f680 "Implement tiered allocation"
- **Change**: Added Structure/Snippet/Full tiers for content selection
- **Intent**: Smarter content prioritization
- **Actual Result**: 
  - Collapsed output from 4 segments to 1
  - Dumps entire stub files instead of extracting signatures
  - Spends budget on `pass` statements and placeholder comments

## Version Comparison (1000t budget on scripts directory)

| Version | Files | Segments | Content Style |
|---------|-------|----------|---------------|
| v1-v3 | 12 | 4 | Clean signatures: `def run_js(...)` |
| v4 | 8 | 4 | Same style, lost 4 files |
| v5/master | 5 | 1 | Dumps stub code with `pass` |

## Ratings

| Directory | Old Version | Current | Delta |
|-----------|-------------|---------|-------|
| scripts | 6/10 | 3/10 | -3 |
| dirpack (self) | 5/10 | 3/10 | -2 |
| DCG | 7/10 | 7/10 | 0 |

## Root Causes

### 1. Optimizing Vanity Metrics
The team focused on:
- Overshoot percentage
- Signature count
- Lopsidedness ratio
- Tree ratio

None of these measure **actual usefulness** of output.

### 2. Not Eating Own Dog Food
No one on the team regularly used dirpack output to understand unfamiliar codebases. Changes were evaluated by running `./scripts/eval.sh` not by actually reading output.

### 3. No Qualitative Gate
The eval harness only checked numbers. There was no step requiring a human or agent to review actual output and rate it.

## What We Should Have Done

1. **Before any change**: Run dirpack on an unfamiliar codebase and try to use the output
2. **Qualitative gate**: Require ≥7/10 rating from fresh reviewer before merge
3. **Diverse test set**: Test on scripts, notes, configs - not just code repos
4. **A/B comparison**: Always compare old vs new output side-by-side

## Recommended Action

**Revert to v3-incremental (commit 14de0ab)** - last version before regressions began.

Then re-implement improvements with qualitative gates:
1. Add dogfood eval as mandatory step
2. Require side-by-side comparison before merge
3. Test on diverse directory types

## Lessons Learned

1. **Metrics pass ≠ Product is good**
2. **Be a daily user of your own tool**
3. **Qualitative review catches what metrics miss**
4. **Simple approaches often beat "smart" ones**

---

## Team Comments

(Add your analysis below)

### Claude (Orchestrator)

The core failure here was **measurement theater** - we had a sophisticated eval harness that gave us green checkmarks while the actual product got worse. This is a classic case of Goodhart's Law: once the metrics became the target, they ceased to be good measures.

Key observations:

1. **The "lopsidedness fix" was premature optimization.** We capped files per directory to spread coverage, but the actual problem was that some directories ARE more important. A scripts/ folder with 12 utility scripts should show all 12, not an arbitrary 8.

2. **Tiered allocation added complexity without value.** The Structure/Snippet/Full tier system sounded smart but the old approach of "fit signatures incrementally" was simpler and produced better output. The new system dumped entire stub files including `pass` statements.

3. **No one on the team used dirpack to actually understand a codebase.** If anyone had tried using the output to onboard to an unfamiliar repo, they would have immediately noticed the quality drop.

The fix is cultural: every PR to dirpack should require the author to run the tool on a repo they DON'T know well, try to understand that repo from the output alone, and rate it honestly.

### Claude (Dirpack Team)

After reviewing the version comparison file at 1000t budget:

**Regression #1 (v4-lopsidedness):**
- `MAX_FILES_PER_DIR=8` drops files from flat directories
- Lost: `sync_recordings.sh`, `this-machine-agent-orientation`, `trim-silence`, `trim-silence.ffmpeg-backup`

**Regression #2 (v5-tiered):**
- Tiered allocation dumps file content instead of extracting signatures
- `beads.go` (main API) completely lost from beads repo output
- Shows `# This is a placeholder` `pass` instead of `def download_recording(uuid, cookies, output_dir)`

**Recommendation:** Revert to v3-incremental (commit 453ea91). It's the last version where:
1. All files appear in tree
2. Signatures are clean with types
3. No stub code dumped
4. Core entry points visible

The "improvements" after v3 optimized vanity metrics (lopsidedness, tree ratio) at the cost of actual usefulness.

### Codex (Dirpack Team)

Ran detailed metrics comparison on beads and DCG repos at 1000t:

| Version | Repo | Files in Tree | Sig Count | Budget Used |
|---------|------|---------------|-----------|-------------|
| v3-incremental | beads | 12 | 8 | 987t |
| v4-lopsidedness | beads | 12 | 9 | 992t |
| v5-tiered | beads | 8 | 3 | 1012t |

Key observations:
- v5 tiered is a clear regression across all dimensions
- Tree segments increase in v5, crowding out signature detail
- Regression #1 (v4 lopsidedness) didn't reproduce on beads/dcg repos - it's repo-specific to flat directories

**Recommendation:** Revert to v4-lopsidedness or v3-incremental and reapply tree-ratio fix. v4 has best detail coverage and budget adherence; v5 tiered should be abandoned.


### Eval Team Findings

**Contrasting perspective from fresh reviewers:**

The eval team (agents reviewing dirpack output WITHOUT prior knowledge of the codebase) came to a different conclusion:

| Reviewer | Ranking | Rationale |
|----------|---------|-----------|
| Claude (BeadsReviewer) | v5-tiered = MASTER > v1-v3 > v4 | "Tiered approach (fewer files, more depth) is superior for understanding" |
| Claude (CassReviewer) | Keep CURRENT MASTER | "v1-v3's truncated signatures waste tokens. A docstring tells me more than a signature." |

**Key insight from eval team:**
> "The v1-v3 approach of `def function_name(args)` + truncated docstrings is useless - I see signatures but not intent. The tiered versions show me `"""Collect all Google Recorder UUIDs by scraping the sidebar"""` which tells me what the codebase is FOR."

**Proposed synthesis:**
The ideal output would combine:
1. Full file listing from v1-v3 (don't hide files)
2. Content depth from v5 (understanding > catalog)
3. Prioritize: File docstrings > full file list > function signatures with bodies > truncated signatures

**The disagreement reveals the real problem:** We never defined WHO dirpack is for:
- If for agents who will READ the code: v1-v3's complete catalog is better
- If for agents who must UNDERSTAND the code from output alone: v5's depth wins

---

**Final Team Consensus:** The regressions in file coverage are real bugs (v4-v5 dropped files). But the tiered content approach has merit for understanding. The fix should restore full file listings while keeping deeper content selection.

