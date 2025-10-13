#!/bin/bash
# Basic Conversion Example
#
# This script demonstrates the simplest use case:
# converting a JSONL file to protobuf format

set -e  # Exit on error

# Configuration
INPUT_FILE="examples/sample_events.jsonl"
OUTPUT_DIR="./pb_data"

echo "🚀 Starting basic conversion..."
echo "Input:  $INPUT_FILE"
echo "Output: $OUTPUT_DIR"
echo ""

# Run conversion
proton-beam convert "$INPUT_FILE" --output-dir "$OUTPUT_DIR"

echo ""
echo "✅ Conversion complete!"
echo ""
echo "📁 Output files:"
ls -lh "$OUTPUT_DIR"

# Check for errors
if [ -f "$OUTPUT_DIR/errors.jsonl" ]; then
  echo ""
  echo "⚠️  Some events failed conversion"
  echo "Error count: $(wc -l < "$OUTPUT_DIR/errors.jsonl")"
  echo "See: $OUTPUT_DIR/errors.jsonl"
fi

