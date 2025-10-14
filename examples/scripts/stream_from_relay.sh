#!/bin/bash
# Stream from Relay Example
#
# This script fetches events from a Nostr relay using 'nak'
# and converts them on-the-fly using proton-beam
#
# Prerequisites: Install nak from https://github.com/fiatjaf/nak

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
RELAY="wss://relay.damus.io"
OUTPUT_DIR="./relay_data"
EVENT_LIMIT=100

echo "🌐 Streaming from relay..."
echo "Relay:  $RELAY"
echo "Limit:  $EVENT_LIMIT events"
echo "Output: $OUTPUT_DIR"
echo ""

# Check if nak is installed
if ! command -v nak &> /dev/null; then
  echo "❌ Error: 'nak' is not installed"
  echo ""
  echo "Install nak:"
  echo "  go install github.com/fiatjaf/nak@latest"
  echo ""
  exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Fetch and convert events
echo "Fetching events..."
nak req -k 1 --limit "$EVENT_LIMIT" "$RELAY" | \
  "$PROTON_BEAM" convert - --output-dir "$OUTPUT_DIR"

echo ""
echo "✅ Stream processing complete!"
echo ""
echo "📁 Output files:"
ls -lh "$OUTPUT_DIR"

# Show statistics
if [ -f "$OUTPUT_DIR/proton-beam.log" ]; then
  ERROR_COUNT=$(grep -c "ERROR" "$OUTPUT_DIR/proton-beam.log" || echo "0")
  if [ "$ERROR_COUNT" -gt 0 ]; then
    echo ""
    echo "⚠️  Errors: $ERROR_COUNT"
    echo "View errors: tail -n 50 $OUTPUT_DIR/proton-beam.log"
  else
    echo ""
    echo "✨ No errors!"
  fi
else
  echo ""
  echo "✨ No errors!"
fi

