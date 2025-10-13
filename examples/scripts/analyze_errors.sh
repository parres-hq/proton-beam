#!/bin/bash
# Error Analysis Example
#
# This script analyzes conversion errors and provides detailed reports

set -e

# Configuration
ERRORS_FILE="${1:-./pb_data/errors.jsonl}"

echo "🔍 Analyzing conversion errors..."
echo "File: $ERRORS_FILE"
echo ""

# Check if errors file exists
if [ ! -f "$ERRORS_FILE" ]; then
  echo "✅ No errors file found - all conversions successful!"
  exit 0
fi

# Check if file is empty
if [ ! -s "$ERRORS_FILE" ]; then
  echo "✅ Errors file is empty - all conversions successful!"
  exit 0
fi

# Check if jq is available
if ! command -v jq &> /dev/null; then
  echo "⚠️  'jq' not found - showing basic statistics only"
  echo ""
  ERROR_COUNT=$(wc -l < "$ERRORS_FILE" | xargs)
  echo "Total errors: $ERROR_COUNT"
  echo ""
  echo "Install jq for detailed analysis:"
  echo "  brew install jq    # macOS"
  echo "  apt install jq     # Debian/Ubuntu"
  exit 0
fi

# Total error count
ERROR_COUNT=$(wc -l < "$ERRORS_FILE" | xargs)
echo "📊 Error Statistics"
echo "═══════════════════════════════════════"
echo "Total errors: $ERROR_COUNT"
echo ""

# Error types breakdown
echo "Error types:"
echo "─────────────────────────────────────"
jq -r '.error' "$ERRORS_FILE" | \
  sed 's/:.*$//' | \
  sort | uniq -c | sort -rn | \
  awk '{printf "  %-30s %s\n", $2, $1}'

echo ""

# Most common error messages (top 5)
echo "Most common error messages:"
echo "─────────────────────────────────────"
jq -r '.error' "$ERRORS_FILE" | \
  sort | uniq -c | sort -rn | head -5 | \
  awk '{$1=$1; printf "  [%d] %s\n", $1, substr($0, length($1)+2)}'

echo ""

# Show first error example
echo "Example error:"
echo "─────────────────────────────────────"
head -n 1 "$ERRORS_FILE" | jq '.'

echo ""
echo "═══════════════════════════════════════"

# Suggest fixes
echo ""
echo "💡 Suggestions:"
echo ""

# Check for parse errors
PARSE_ERRORS=$(grep -c "parse_error" "$ERRORS_FILE" || true)
if [ "$PARSE_ERRORS" -gt 0 ]; then
  echo "  • $PARSE_ERRORS parse errors found"
  echo "    - Check if JSON is properly formatted"
  echo "    - Ensure one event per line (.jsonl format)"
  echo ""
fi

# Check for validation errors
VALIDATION_ERRORS=$(grep -c "validation_error" "$ERRORS_FILE" || true)
if [ "$VALIDATION_ERRORS" -gt 0 ]; then
  echo "  • $VALIDATION_ERRORS validation errors found"
  echo "    - Events may have invalid signatures or IDs"
  echo "    - Consider using --no-validate if data is trusted"
  echo ""
fi

# Offer to extract failed events
echo "To retry failed events without validation:"
echo "  jq -r '.original_json' $ERRORS_FILE > failed_events.jsonl"
echo "  proton-beam convert failed_events.jsonl --no-validate"
echo ""
echo "Note: This script is for analysis only and doesn't require proton-beam to be in PATH"

