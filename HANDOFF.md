# dirpack Session Handoff

## What Was Built
- **dirpack**: Rust CLI for budgeted directory indexes with tree-sitter signatures
- Repo: https://github.com/rawwerks/dirpack (private)
- 20 source files, ~2300 lines, compiles and runs

## Current State
Working MVP with: `dirpack pack . -t 4000` produces pipe-delimited output with signatures.

## Beads Plan (Approved)
```
bd list --pretty
```

| Priority | Issue | Description |
|----------|-------|-------------|
| P1 | dirpack-78h | Representation enum (NameOnly/Structure/Snippet/Full) |
| P1 | dirpack-hnk | Tiered allocation + per-directory cap |
| P2 | dirpack-49r | Representation-aware formatters |
| P3 | dirpack-cl6 | Markdown heading extraction |
| P3 | dirpack-qhg | Hidden dirs as NameOnly |

## Agent Debate Conclusions (Space-Filling)
- ✅ ADD: Per-directory cap (max 5 files detailed per dir) - prevents lopsided output
- ❌ SKIP: Hilbert curves, stratified budgets, Poisson-disc - over-engineering
- ✅ KEEP: Tiered allocation as the 80/20 solution

## Key Files
- DESIGN.md: Full architecture spec
- FEEDBACK.md: Mesh decimation insights
- SPACE_FILLING.md: Coverage algorithm ideas
- README.md: Usage docs

## NTM Session
```
ntm status dirpack  # Check agents
ntm kill dirpack -f # Kill when done
```

## Next Steps
1. Implement dirpack-78h (Representation enum)
2. Implement dirpack-hnk (Tiered allocation + per-dir cap)
3. Test on DSPy/build123d
4. Push to remote

## Commands to Resume
```bash
cd /home/raw/Documents/GitHub/dirpack
bd prime
bd list --pretty
bd show dirpack-78h
```
