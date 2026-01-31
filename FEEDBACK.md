# Feedback: Mesh Decimation Architecture

## Core Insight

What we're building is **not a summarizer**. It's a:

> **Topology-preserving, budget-constrained repository LOD generator for agents**

## The Missing 15-30%

Current dirpack does ~70-85% of what's needed. The gaps:

1. **Hidden dirs as "names-only"** - show `.claude/` but don't descend
2. **Variable compression per file** - headings only, signatures only, snippets only
3. **Budget-driven representation choice** - not just "include or exclude" but *how much* of each file

This is **LOD selection**, not filtering.

## The Right Architecture

Don't think "file walker". Think **allocator**.

### Core Pipeline

```
scan → score → allocate → render
```

### Key Change: Representation Levels

Instead of binary include/exclude, assign **representation level**:

```rust
enum Representation {
    NameOnly,        // just in tree
    Metadata,        // size, type
    Structure,       // headings / signatures
    Snippet,         // partial content
    Full,            // entire file
}
```

### Allocation Algorithm (Greedy Mesh Simplification)

```rust
global_budget = 8KB / 64KB / 128KB
while budget > 0:
  allocate next highest marginal utility upgrade
```

Each file starts at `NameOnly` and gets upgraded based on:
- Available budget
- Saliency score
- Marginal cost of next level

### Saliency Scoring

```rust
score =
  entrypoint_bonus +      // main.rs, index.ts
  config_bonus +          // Cargo.toml, package.json
  exported_symbol_bonus + // pub fn, export
  git_activity_bonus -    // recently changed
  generated_penalty -     // lock files
  test_penalty;           // test files
```

## Render Order (Stable + Agent-Friendly)

Output must be **deterministic and index-first**:

1. Header + instructions
2. Directory tree (always)
3. Compressed index
4. Structural summaries
5. Snippets (only if budget remains)

This matches why `AGENTS.md` works so well.

## Mesh Decimation Analogy

| Mesh Decimation       | Repo Compression                  |
| --------------------- | --------------------------------- |
| preserve silhouette   | preserve directory tree           |
| collapse flat regions | drop boilerplate/tests            |
| keep sharp edges      | keep APIs / configs               |
| progressive LODs      | signatures → snippets → full text |

## Implementation Tasks

1. Add `Representation` enum to `FileEntry`
2. Modify `Budget` to track representation upgrades, not just inclusions
3. Implement greedy allocation: start all files at `NameOnly`, upgrade highest-value first
4. Add representation-aware rendering to each formatter
5. Handle hidden directories specially (always `NameOnly`)

## Example Output at Different Budgets

### 1K tokens (NameOnly + Structure for top files)
```
[project]|root:.|dirs:{src,tests}|src:{main.rs,lib.rs}|main.rs:fn main(),fn setup()
```

### 4K tokens (Structure for most, Snippets for top)
```
[project]|root:.|IMPORTANT:...|dirs:{src,tests}|src:{main.rs[fn main()->setup()->run()],lib.rs[pub mod...]}|README.md:[first 3 lines]
```

### 16K tokens (Snippets for most, Full for top)
```
[Full structured output with actual code for high-priority files]
```

## Priority

1. **Representation enum + allocation** - core architectural change
2. **Hidden dir handling** - `.git/`, `.claude/` as NameOnly
3. **Greedy upgrader** - the actual "mesh decimation" algorithm
4. **Test on real repos** - DSPy, build123d at multiple budget levels
