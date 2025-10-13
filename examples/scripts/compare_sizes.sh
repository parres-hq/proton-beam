#!/bin/bash
# Size Comparison Example
#
# This script compares JSON vs Protobuf storage sizes
# to demonstrate compression efficiency

set -e

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
INPUT_FILE="${1:-examples/sample_events.jsonl}"
OUTPUT_DIR="./pb_data_comparison"

echo "📊 JSON vs Protobuf Size Comparison"
echo "════════════════════════════════════════════════"
echo ""

# Check if input file exists
if [ ! -f "$INPUT_FILE" ]; then
  echo "❌ Error: Input file not found: $INPUT_FILE"
  exit 1
fi

# Get JSON size
JSON_SIZE=$(wc -c < "$INPUT_FILE")
EVENT_COUNT=$(wc -l < "$INPUT_FILE" | xargs)

echo "Input file: $INPUT_FILE"
echo "Events: $EVENT_COUNT"
echo ""

# Convert to protobuf
echo "Converting to protobuf..."
rm -rf "$OUTPUT_DIR"
"$PROTON_BEAM" convert "$INPUT_FILE" \
  --output-dir "$OUTPUT_DIR" \
  --no-progress > /dev/null 2>&1

# Calculate protobuf size
PB_SIZE=$(find "$OUTPUT_DIR" -name "*.pb" -type f -exec wc -c {} + | \
  awk '{sum+=$1} END {print sum}')

# Ensure PB_SIZE is not zero
if [ -z "$PB_SIZE" ] || [ "$PB_SIZE" -eq 0 ]; then
  echo "❌ Error: No protobuf files generated"
  exit 1
fi

# Format sizes for display
format_bytes() {
  local bytes=$1
  if command -v numfmt &> /dev/null; then
    numfmt --to=iec-i --suffix=B "$bytes"
  else
    # Fallback for systems without numfmt
    if [ "$bytes" -lt 1024 ]; then
      echo "${bytes}B"
    elif [ "$bytes" -lt 1048576 ]; then
      echo "$(echo "scale=1; $bytes/1024" | bc)KB"
    else
      echo "$(echo "scale=1; $bytes/1048576" | bc)MB"
    fi
  fi
}

JSON_SIZE_FMT=$(format_bytes "$JSON_SIZE")
PB_SIZE_FMT=$(format_bytes "$PB_SIZE")

# Calculate savings
SAVED_BYTES=$((JSON_SIZE - PB_SIZE))
SAVED_BYTES_FMT=$(format_bytes "$SAVED_BYTES")

# Calculate percentage
if [ "$JSON_SIZE" -gt 0 ]; then
  SAVED_PERCENT=$(echo "scale=1; ($SAVED_BYTES * 100) / $JSON_SIZE" | bc)
else
  SAVED_PERCENT="0"
fi

# Calculate compression ratio
if [ "$PB_SIZE" -gt 0 ]; then
  COMPRESSION_RATIO=$(echo "scale=2; $JSON_SIZE / $PB_SIZE" | bc)
else
  COMPRESSION_RATIO="N/A"
fi

# Display results
echo "✅ Conversion complete!"
echo ""
echo "Results:"
echo "────────────────────────────────────────────────"
printf "%-25s %15s\n" "JSON size:" "$JSON_SIZE_FMT"
printf "%-25s %15s\n" "Protobuf size:" "$PB_SIZE_FMT"
printf "%-25s %15s\n" "Space saved:" "$SAVED_BYTES_FMT"
echo "────────────────────────────────────────────────"
printf "%-25s %14s%%\n" "Compression:" "$SAVED_PERCENT"
printf "%-25s %14sx\n" "Ratio:" "$COMPRESSION_RATIO"
echo "────────────────────────────────────────────────"

# Per-event statistics
if [ "$EVENT_COUNT" -gt 0 ]; then
  JSON_PER_EVENT=$((JSON_SIZE / EVENT_COUNT))
  PB_PER_EVENT=$((PB_SIZE / EVENT_COUNT))

  JSON_PER_EVENT_FMT=$(format_bytes "$JSON_PER_EVENT")
  PB_PER_EVENT_FMT=$(format_bytes "$PB_PER_EVENT")

  echo ""
  echo "Per-event average:"
  printf "  JSON:     %s/event\n" "$JSON_PER_EVENT_FMT"
  printf "  Protobuf: %s/event\n" "$PB_PER_EVENT_FMT"
fi

echo ""
echo "════════════════════════════════════════════════"

# Visualization
if [ "$SAVED_PERCENT" != "0" ]; then
  echo ""
  echo "Visual comparison:"

  # Create simple bar chart
  JSON_BAR_LEN=40
  PB_BAR_LEN=$(echo "scale=0; ($PB_SIZE * $JSON_BAR_LEN) / $JSON_SIZE" | bc)

  # Ensure at least 1 character
  if [ "$PB_BAR_LEN" -lt 1 ]; then
    PB_BAR_LEN=1
  fi

  printf "  JSON:     "
  printf '█%.0s' $(seq 1 "$JSON_BAR_LEN")
  echo ""

  printf "  Protobuf: "
  printf '█%.0s' $(seq 1 "$PB_BAR_LEN")
  echo ""
fi

echo ""
echo "💡 Protobuf is ~${SAVED_PERCENT}% smaller than JSON"

# Cleanup
rm -rf "$OUTPUT_DIR"

