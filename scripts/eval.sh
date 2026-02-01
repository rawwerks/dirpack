#!/bin/bash
# dirpack eval harness - measures quality + speed
# Usage: ./scripts/eval.sh [path] [output.json]
#
# Thresholds (configurable via env vars):
#   EVAL_OVERSHOOT_FAIL=2     # Fail if overshoot > 2%
#   EVAL_TREE_RATIO_WARN=0.40 # Warn if tree ratio > 40%
#   EVAL_UNDERSHOOT_WARN=50   # Warn if undershoot > 50%

set -e

DIRPACK="${DIRPACK:-./target/release/dirpack}"
TARGET="${1:-$(pwd)}"
OUTPUT_FILE="${2:-/dev/stdout}"

# Thresholds
OVERSHOOT_FAIL="${EVAL_OVERSHOOT_FAIL:-2}"
TREE_RATIO_WARN="${EVAL_TREE_RATIO_WARN:-0.40}"
UNDERSHOOT_WARN="${EVAL_UNDERSHOOT_WARN:-50}"

# Budget levels to test
BUDGETS=(500 1000 2000 4000 8000 16000)

# Start JSON output
echo "{" > "$OUTPUT_FILE"
echo "  \"target\": \"$TARGET\"," >> "$OUTPUT_FILE"
echo "  \"timestamp\": \"$(date -Iseconds)\"," >> "$OUTPUT_FILE"
echo "  \"dirpack_version\": \"$(cargo pkgid 2>/dev/null | sed 's/.*#//' || echo 'unknown')\"," >> "$OUTPUT_FILE"
echo "  \"thresholds\": {" >> "$OUTPUT_FILE"
echo "    \"overshoot_fail_pct\": $OVERSHOOT_FAIL," >> "$OUTPUT_FILE"
echo "    \"tree_ratio_warn\": $TREE_RATIO_WARN," >> "$OUTPUT_FILE"
echo "    \"undershoot_warn_pct\": $UNDERSHOOT_WARN" >> "$OUTPUT_FILE"
echo "  }," >> "$OUTPUT_FILE"
echo "  \"results\": [" >> "$OUTPUT_FILE"

first=true
for budget in "${BUDGETS[@]}"; do
    # Measure timing
    start_ns=$(date +%s%N)
    output=$($DIRPACK pack "$TARGET" -t "$budget" 2>&1)
    end_ns=$(date +%s%N)

    # Calculate metrics
    chars=$(echo "$output" | wc -c)

    # Estimate tokens (chars/4 approximation)
    est_tokens=$((chars / 4))

    # Budget accuracy using awk
    accuracy=$(awk "BEGIN {printf \"%.4f\", $est_tokens / $budget}")
    overshoot=$(awk "BEGIN {printf \"%.2f\", ($est_tokens - $budget) / $budget * 100}")

    # Count signatures (rough proxy)
    sig_count=$(echo "$output" | grep -oE '(fn |pub fn |def |class |interface |struct |trait |const |enum |type |impl |mod )' | wc -l)

    # Count directory listings and calculate lopsidedness
    # Extract file counts per directory: dir:{file1,file2,...}
    dir_file_counts=$(echo "$output" | grep -oE '[a-zA-Z0-9_./-]+:\{[^}]+\}' | while read -r dir_block; do
        # Count commas + 1 = number of files
        files_in_dir=$(echo "$dir_block" | sed 's/[^,]//g' | wc -c)
        echo "$files_in_dir"
    done)

    dir_count=$(echo "$dir_file_counts" | wc -l)

    # Calculate lopsidedness (max/mean)
    if [ "$dir_count" -gt 0 ] && [ -n "$dir_file_counts" ]; then
        lopsidedness=$(echo "$dir_file_counts" | awk '
            BEGIN { max=0; sum=0; count=0 }
            { if ($1 > max) max=$1; sum+=$1; count++ }
            END {
                if (count > 0 && sum > 0) {
                    mean = sum / count
                    printf "%.2f", max / mean
                } else {
                    print "1.00"
                }
            }
        ')
    else
        lopsidedness="1.00"
    fi

    # Tree ratio estimate
    tree_ratio="0.0"
    first_sig_pos=$(echo "$output" | grep -ob 'fn \|def \|class ' | head -1 | cut -d: -f1 2>/dev/null || echo "")
    if [ -n "$first_sig_pos" ] && [ "$first_sig_pos" -gt 0 ] 2>/dev/null; then
        tree_ratio=$(awk "BEGIN {printf \"%.4f\", $first_sig_pos / $chars}")
    else
        tree_ratio="1.0"
    fi

    # Timing in milliseconds
    duration_ns=$((end_ns - start_ns))
    duration_ms=$((duration_ns / 1000000))
    duration_sec=$(awk "BEGIN {printf \"%.3f\", $duration_ns / 1000000000}")
    tokens_per_sec=$(awk "BEGIN {printf \"%.0f\", $est_tokens / ($duration_ns / 1000000000)}")

    # Entry point coverage (check for main/lib/index)
    has_main=$(echo "$output" | grep -q 'main\.' && echo "true" || echo "false")
    has_lib=$(echo "$output" | grep -q 'lib\.' && echo "true" || echo "false")
    has_index=$(echo "$output" | grep -q 'index\.' && echo "true" || echo "false")

    # Config coverage
    has_cargo=$(echo "$output" | grep -q 'Cargo.toml' && echo "true" || echo "false")
    has_package=$(echo "$output" | grep -q 'package.json' && echo "true" || echo "false")
    has_pyproject=$(echo "$output" | grep -q 'pyproject.toml' && echo "true" || echo "false")

    # Determine pass/warn/fail status
    status="PASS"
    overshoot_num=$(echo "$overshoot" | sed 's/-//')
    is_undershoot=$(echo "$overshoot" | grep -q '^-' && echo "true" || echo "false")

    if [ "$is_undershoot" = "false" ]; then
        # Check overshoot
        is_fail=$(awk "BEGIN {print ($overshoot > $OVERSHOOT_FAIL) ? 1 : 0}")
        if [ "$is_fail" = "1" ]; then
            status="FAIL"
        fi
    else
        # Check undershoot
        undershoot_pct=$(echo "$overshoot" | sed 's/^-//')
        is_warn=$(awk "BEGIN {print ($undershoot_pct > $UNDERSHOOT_WARN) ? 1 : 0}")
        if [ "$is_warn" = "1" ]; then
            status="WARN_UNDERSHOOT"
        fi
    fi

    # Check tree ratio
    if [ "$status" = "PASS" ]; then
        is_tree_warn=$(awk "BEGIN {print ($tree_ratio > $TREE_RATIO_WARN) ? 1 : 0}")
        if [ "$is_tree_warn" = "1" ]; then
            status="WARN_TREE"
        fi
    fi

    # Output JSON
    if [ "$first" = true ]; then
        first=false
    else
        echo "," >> "$OUTPUT_FILE"
    fi

    cat >> "$OUTPUT_FILE" << EOF
    {
      "budget_tokens": $budget,
      "output_chars": $chars,
      "est_tokens": $est_tokens,
      "budget_accuracy": $accuracy,
      "overshoot_pct": $overshoot,
      "signature_count": $sig_count,
      "directory_count": $dir_count,
      "lopsidedness": $lopsidedness,
      "tree_ratio": $tree_ratio,
      "duration_ms": $duration_ms,
      "duration_sec": $duration_sec,
      "tokens_per_sec": $tokens_per_sec,
      "has_main": $has_main,
      "has_lib": $has_lib,
      "has_index": $has_index,
      "has_cargo_toml": $has_cargo,
      "has_package_json": $has_package,
      "has_pyproject": $has_pyproject,
      "status": "$status"
    }
EOF
done

echo "" >> "$OUTPUT_FILE"
echo "  ]" >> "$OUTPUT_FILE"
echo "}" >> "$OUTPUT_FILE"

echo "Eval complete. Results in: $OUTPUT_FILE" >&2

# Qualitative review reminder
echo "" >&2
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
echo "📋 REQUIRED: Qualitative Review (dogfooding)" >&2
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
echo "Run: $DIRPACK pack . -t 2000" >&2
echo "" >&2
echo "Check:" >&2
echo "  [ ] Is README.md/DESIGN.md content visible?" >&2
echo "  [ ] Are there duplicate signatures?" >&2
echo "  [ ] Priority sensible? (core > tests)" >&2
echo "  [ ] Would a new dev understand the architecture?" >&2
echo "" >&2
echo "Metrics PASS ≠ output is good. Look at it!" >&2
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
