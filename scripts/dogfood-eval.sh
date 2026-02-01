#!/bin/bash
# Dogfood evaluation - agent reads actual output
# Usage: ./scripts/dogfood-eval.sh [budget]
#
# Generates dirpack output for diverse directories.
# Agent MUST read every token and rate usefulness.

set -e

DIRPACK="${DIRPACK:-./target/release/dirpack}"
BUDGET="${1:-2000}"
REPOS_FILE="$(dirname "$0")/dogfood-repos.txt"
OUTPUT_DIR="/tmp/dirpack-dogfood-eval"

mkdir -p "$OUTPUT_DIR"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🐕 DOGFOOD EVAL - Budget: ${BUDGET}t"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

while IFS='|' read -r path type desc; do
  # Skip comments and empty lines
  [[ "$path" =~ ^#.*$ ]] && continue
  [[ -z "$path" ]] && continue
  
  name=$(basename "$path")
  outfile="$OUTPUT_DIR/${name}-${BUDGET}t.txt"
  
  echo "📂 $name ($type)"
  echo "   $desc"
  
  if [ -d "$path" ]; then
    $DIRPACK pack "$path" -t "$BUDGET" > "$outfile" 2>/dev/null
    chars=$(wc -c < "$outfile")
    lines=$(wc -l < "$outfile")
    echo "   → $chars chars, $lines lines"
    echo "   → $outfile"
  else
    echo "   ⚠ Directory not found: $path"
  fi
  echo ""
done < "$REPOS_FILE"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 REVIEW INSTRUCTIONS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "For EACH output file, the reviewing agent must:"
echo ""
echo "1. READ EVERY TOKEN - no skimming"
echo "2. Answer: Could I start working in this codebase?"
echo "3. Answer: What's the main entry point?"
echo "4. Answer: What would I change to make this more useful?"
echo "5. Rate 1-10 for 'usefulness as onboarding aid'"
echo ""
echo "Files to review:"
ls -la "$OUTPUT_DIR"/*.txt 2>/dev/null | awk '{print "  " $NF}'
echo ""
echo "Target: Average rating ≥ 7/10 across all directories"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
