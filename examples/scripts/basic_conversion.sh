#!/bin/bash
# Basic Conversion Example
#
# This script demonstrates the simplest use case:
# converting a JSONL file to protobuf format

set -e  # Exit on error

# Find proton-beam binary
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

if command -v proton-beam &> /dev/null; then
  PROTON_BEAM="proton-beam"
elif [ -f "$PROJECT_ROOT/target/release/proton-beam" ]; then
  PROTON_BEAM="$PROJECT_ROOT/target/release/proton-beam"
elif [ -f "$PROJECT_ROOT/target/debug/proton-beam" ]; then
  PROTON_BEAM="$PROJECT_ROOT/target/debug/proton-beam"
else
  echo "❌ Error: proton-beam not found"
  echo ""
  echo "Please build the project first:"
  echo "  cargo build --release -p proton-beam-cli"
  echo ""
  echo "Or install it:"
  echo "  cargo install --path proton-beam-cli"
  exit 1
fi

# Configuration
INPUT_FILE="examples/sample_events.jsonl"
OUTPUT_DIR="./pb_data"

echo "🚀 Starting basic conversion..."
echo "Input:  $INPUT_FILE"
echo "Output: $OUTPUT_DIR"
echo ""

# Run conversion
"$PROTON_BEAM" convert "$INPUT_FILE" --output-dir "$OUTPUT_DIR"

echo ""
echo "✅ Conversion complete!"
echo ""
echo "📁 Output files:"
ls -lh "$OUTPUT_DIR"

# Check for errors in log
if [ -f "$OUTPUT_DIR/proton-beam.log" ]; then
  ERROR_COUNT=$(grep -c "ERROR" "$OUTPUT_DIR/proton-beam.log" || echo "0")
  if [ "$ERROR_COUNT" -gt 0 ]; then
    echo ""
    echo "⚠️  Some events failed conversion"
    echo "Error count: $ERROR_COUNT"
    echo "View errors: tail -n 50 $OUTPUT_DIR/proton-beam.log"
  fi
fi

