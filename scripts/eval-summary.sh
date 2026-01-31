#!/bin/bash
# Generate summary of eval baselines
# Usage: ./scripts/eval-summary.sh baselines/*.json

echo "# Dirpack Eval Summary"
echo "Generated: $(date)"
echo ""

for f in "$@"; do
    name=$(basename "$f" .json)
    echo "## $name"
    echo ""
    echo "| Budget | Est Tok | Overshoot% | Sigs | Dirs | Lopsided | Tree% | Status |"
    echo "|--------|---------|------------|------|------|----------|-------|--------|"

    # Parse JSON with awk
    awk '
        /"budget_tokens":/ { budget = $2; gsub(",", "", budget) }
        /"est_tokens":/ { tokens = $2; gsub(",", "", tokens) }
        /"overshoot_pct":/ { overshoot = $2; gsub(",", "", overshoot) }
        /"signature_count":/ { sigs = $2; gsub(",", "", sigs) }
        /"directory_count":/ { dirs = $2; gsub(",", "", dirs) }
        /"lopsidedness":/ { lopsided = $2; gsub(",", "", lopsided) }
        /"tree_ratio":/ { tree = $2; gsub(",", "", tree); tree_pct = tree * 100 }
        /"status":/ {
            status = $2; gsub("[,\"]", "", status)
            printf "| %d | %d | %.1f | %d | %d | %.2f | %.0f | %s |\n",
                   budget, tokens, overshoot, sigs, dirs, lopsided, tree_pct, status
        }
    ' "$f"
    echo ""
done

echo "## Thresholds"
echo "- **PASS**: overshoot ≤2%, tree_ratio ≤40%"
echo "- **WARN_TREE**: tree_ratio >40%"
echo "- **WARN_UNDERSHOOT**: undershoot >50%"
echo "- **FAIL**: overshoot >2%"
echo ""
echo "## Lopsidedness"
echo "- 1.0 = perfectly even distribution"
echo "- >3.0 = some directories have many more files than average"
