#!/bin/bash
# Analyze benchmark history to detect trends and regressions
# Usage: ./scripts/analyze-benchmark-history.sh [directory]

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

BENCH_DIR="${1:-benchmark-results}"

if [ ! -d "$BENCH_DIR" ]; then
    echo -e "${RED}Error: Directory $BENCH_DIR not found${NC}"
    echo "Usage: $0 [directory]"
    echo "Example: $0 benchmark-results"
    exit 1
fi

echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║          Benchmark History Analysis for Proton Beam           ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Find all benchmark result files
FILES=$(find "$BENCH_DIR" -name "bench-*.txt" -type f | sort)
FILE_COUNT=$(echo "$FILES" | wc -l | tr -d ' ')

if [ "$FILE_COUNT" -eq 0 ]; then
    echo -e "${YELLOW}No benchmark files found in $BENCH_DIR${NC}"
    echo ""
    echo "To create benchmark results:"
    echo "  just bench-save"
    echo ""
    echo "This will create files like:"
    echo "  benchmark-results/bench-20251014-153045.txt"
    exit 0
fi

echo -e "${GREEN}Found $FILE_COUNT benchmark result file(s)${NC}"
echo ""

# Show list of available files
echo -e "${BLUE}Available benchmark results:${NC}"
echo "$FILES" | nl -w2 -s'. '
echo ""

if [ "$FILE_COUNT" -eq 1 ]; then
    echo -e "${YELLOW}Only one benchmark file found. Need at least 2 to compare.${NC}"
    echo ""
    echo "Run another benchmark to track changes:"
    echo "  just bench-save"
    exit 0
fi

# Get the two most recent files for comparison
OLDEST=$(echo "$FILES" | head -1)
LATEST=$(echo "$FILES" | tail -1)

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}Comparing oldest vs latest:${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "Oldest:  ${YELLOW}$(basename "$OLDEST")${NC}"
echo -e "Latest:  ${YELLOW}$(basename "$LATEST")${NC}"
echo ""

# Function to extract key metrics from a benchmark file
extract_metrics() {
    local file=$1
    local name=$2

    echo "Extracting metrics from $name..."

    # Look for common patterns in benchmark output
    # This is a simplified parser - adjust based on your actual output format

    # Example patterns to look for:
    # - "XXX conversions/sec"
    # - "XXX events/sec"
    # - "XXX MB/sec"
    # - "Time taken: X.XXs"

    grep -E "conversions?/sec|events?/sec|MB/sec|validations?/sec" "$file" 2>/dev/null || true
}

# Extract and compare metrics
echo -e "${BLUE}Key Metrics Comparison:${NC}"
echo ""

OLD_METRICS=$(extract_metrics "$OLDEST" "oldest")
NEW_METRICS=$(extract_metrics "$LATEST" "latest")

if [ -z "$OLD_METRICS" ] || [ -z "$NEW_METRICS" ]; then
    echo -e "${YELLOW}Could not extract metrics automatically.${NC}"
    echo ""
    echo "Showing full diff instead:"
    echo ""
    diff -u "$OLDEST" "$LATEST" | head -50 || true
else
    echo -e "${GREEN}Oldest:${NC}"
    echo "$OLD_METRICS" | head -10
    echo ""
    echo -e "${GREEN}Latest:${NC}"
    echo "$NEW_METRICS" | head -10
    echo ""
fi

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}Full Comparison (diff):${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Show diff with context
if command -v colordiff &> /dev/null; then
    diff -u "$OLDEST" "$LATEST" | colordiff | head -100 || echo "Files are identical"
else
    diff -u "$OLDEST" "$LATEST" | head -100 || echo "Files are identical"
fi

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}Summary${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "📊 Total benchmark files: $FILE_COUNT"
echo "📁 Directory: $BENCH_DIR"
echo ""
echo "💡 Tips:"
echo "  • Run 'just bench-save' to add a new data point"
echo "  • Compare specific files: just bench-compare FILE1 FILE2"
echo "  • Look for lines starting with '+' (improvements) or '-' (regressions)"
echo "  • Focus on 'events/sec' and 'MB/sec' metrics for real-world impact"
echo ""
echo "🎯 Performance Guidelines:"
echo "  • >15% regression: Investigate immediately"
echo "  • 10-15% regression: Review and justify"
echo "  • 5-10% variance: Normal noise"
echo "  • >10% improvement: Document what changed!"
echo ""
echo "📖 See docs/BENCHMARK_PRACTICES.md for detailed guidance"
echo ""

