#!/bin/bash
# Fast Conversion Example
#
# This script demonstrates maximum performance settings:
# - No validation
# - Large batch size
# - No progress bar
#
# ⚠️  Use only for trusted, pre-validated data!

set -e

# Configuration
INPUT_FILE="${1:-examples/sample_events.jsonl}"
OUTPUT_DIR="./pb_data_fast"

echo "⚡ Fast conversion mode"
echo "⚠️  Validation disabled - use only for trusted data!"
echo ""
echo "Input:  $INPUT_FILE"
echo "Output: $OUTPUT_DIR"
echo ""

# Check if input file exists
if [ ! -f "$INPUT_FILE" ]; then
  echo "❌ Error: Input file not found: $INPUT_FILE"
  exit 1
fi

# Time the conversion
START_TIME=$(date +%s)

# Run conversion with performance settings
proton-beam convert "$INPUT_FILE" \
  --output-dir "$OUTPUT_DIR" \
  --no-validate \
  --batch-size 2000 \
  --no-progress

END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))

echo ""
echo "✅ Fast conversion complete!"
echo "Time elapsed: ${ELAPSED}s"
echo ""

# Calculate statistics
EVENT_COUNT=$(wc -l < "$INPUT_FILE" | xargs)
if [ "$EVENT_COUNT" -gt 0 ] && [ "$ELAPSED" -gt 0 ]; then
  RATE=$((EVENT_COUNT / ELAPSED))
  echo "Processing rate: ~${RATE} events/second"
fi

echo ""
echo "📁 Output files:"
ls -lh "$OUTPUT_DIR"

