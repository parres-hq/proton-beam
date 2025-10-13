#!/bin/bash
# Daily Backup Example
#
# This script creates a daily backup of events from specified relays.
# Designed to be run as a cron job or systemd timer.

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
BACKUP_ROOT="$HOME/nostr_backups"
DATE=$(date +%Y-%m-%d)
OUTPUT_DIR="$BACKUP_ROOT/$DATE"
RELAYS=(
  "wss://relay.damus.io"
  "wss://nos.lol"
  "wss://relay.primal.net"
)

# Ensure backup directory exists
mkdir -p "$OUTPUT_DIR"

echo "📦 Daily Nostr Backup"
echo "Date: $DATE"
echo "Output: $OUTPUT_DIR"
echo "Relays: ${#RELAYS[@]}"
echo ""

# Check dependencies
if ! command -v nak &> /dev/null; then
  echo "❌ Error: 'nak' is required but not installed"
  exit 1
fi

# Fetch events from last 24 hours
echo "Fetching events from last 24 hours..."
TEMP_FILE="$OUTPUT_DIR/raw_events.jsonl"

# Combine all relays into one request
nak req --since "24 hours ago" "${RELAYS[@]}" > "$TEMP_FILE"

# Count fetched events
EVENT_COUNT=$(wc -l < "$TEMP_FILE" | xargs)
echo "Fetched: $EVENT_COUNT events"
echo ""

# Convert to protobuf
echo "Converting to protobuf..."
"$PROTON_BEAM" convert "$TEMP_FILE" \
  --output-dir "$OUTPUT_DIR" \
  --batch-size 1000

# Remove temporary file
rm "$TEMP_FILE"

echo ""
echo "Compressing backup..."
cd "$BACKUP_ROOT"
tar -czf "$DATE.tar.gz" "$DATE"

# Get compressed size
COMPRESSED_SIZE=$(du -h "$DATE.tar.gz" | cut -f1)
echo "Compressed size: $COMPRESSED_SIZE"

# Optional: Remove uncompressed directory to save space
rm -rf "$DATE"

echo ""
echo "✅ Backup complete: $BACKUP_ROOT/$DATE.tar.gz"

# Optional: Clean up old backups (keep last 30 days)
echo ""
echo "Cleaning old backups..."
find "$BACKUP_ROOT" -name "*.tar.gz" -mtime +30 -delete
echo "Kept backups from last 30 days"

# Optional: Log to syslog
logger -t proton-beam-backup "Daily backup complete: $EVENT_COUNT events archived"

