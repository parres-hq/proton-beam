#!/bin/bash
# Example: Using preprocessing to filter invalid kinds
#
# This script demonstrates the --filter-invalid-kinds flag which provides
# ultra-fast filtering of events with invalid kind values before JSON parsing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BINARY="$PROJECT_ROOT/target/release/proton-beam"

# Build if needed
if [ ! -f "$BINARY" ]; then
    echo "Building proton-beam..."
    cd "$PROJECT_ROOT"
    cargo build --release --bin proton-beam
fi

# Check if sample file exists
SAMPLE_FILE="$PROJECT_ROOT/examples/sample_events.jsonl"
if [ ! -f "$SAMPLE_FILE" ]; then
    echo "Error: Sample file not found at $SAMPLE_FILE"
    exit 1
fi

echo "================================================"
echo "Preprocessing Filter Demo"
echo "================================================"
echo ""

# Create output directories
OUTPUT_WITHOUT_FILTER="$PROJECT_ROOT/output_no_filter"
OUTPUT_WITH_FILTER="$PROJECT_ROOT/output_with_filter"

rm -rf "$OUTPUT_WITHOUT_FILTER" "$OUTPUT_WITH_FILTER"
mkdir -p "$OUTPUT_WITHOUT_FILTER" "$OUTPUT_WITH_FILTER"

echo "1️⃣  Converting WITHOUT preprocessing filter..."
echo "   Command: proton-beam convert sample_events.jsonl --validate-signatures=false --validate-event-ids=false -j 1"
echo ""
time "$BINARY" convert "$SAMPLE_FILE" \
    --output-dir "$OUTPUT_WITHOUT_FILTER" \
    --validate-signatures=false \
    --validate-event-ids=false \
    -j 1

echo ""
echo "================================================"
echo ""

echo "2️⃣  Converting WITH preprocessing filter..."
echo "   Command: proton-beam convert sample_events.jsonl --validate-signatures=false --validate-event-ids=false --filter-invalid-kinds -j 1"
echo ""
time "$BINARY" convert "$SAMPLE_FILE" \
    --output-dir "$OUTPUT_WITH_FILTER" \
    --validate-signatures=false \
    --validate-event-ids=false \
    --filter-invalid-kinds \
    -j 1

echo ""
echo "================================================"
echo "Summary"
echo "================================================"
echo ""
echo "The preprocessing filter:"
echo "  ✓ Scans each line with a fast regex BEFORE JSON parsing"
echo "  ✓ Skips lines where 'kind' field > 65535 (u16 max)"
echo "  ✓ Reduces parsing overhead for invalid events"
echo "  ✓ Provides cleaner logs (no parse errors for filtered events)"
echo ""
echo "Best used when:"
echo "  • Processing large datasets with known invalid kinds"
echo "  • You want to avoid error logs for out-of-range kinds"
echo "  • Maximum conversion speed is needed"
echo ""
echo "Combine with disabled validation and --parallel for maximum throughput:"
echo "  proton-beam convert events.jsonl --filter-invalid-kinds --validate-signatures=false --validate-event-ids=false -j 8"
echo ""

# Cleanup
rm -rf "$OUTPUT_WITHOUT_FILTER" "$OUTPUT_WITH_FILTER"

echo "✅ Demo complete!"

