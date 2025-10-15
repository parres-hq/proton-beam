#!/bin/bash
# Error Analysis Example
#
# This script analyzes conversion errors from the log file and provides detailed reports

set -e

# Configuration
LOG_FILE="${1:-./pb_data/proton-beam.log}"

echo "🔍 Analyzing conversion errors..."
echo "Log file: $LOG_FILE"
echo ""

# Check if log file exists
if [ ! -f "$LOG_FILE" ]; then
  echo "✅ No log file found - all conversions successful!"
  exit 0
fi

# Check if file is empty
if [ ! -s "$LOG_FILE" ]; then
  echo "✅ Log file is empty!"
  exit 0
fi

# Total error count
ERROR_COUNT=$(grep -c "ERROR" "$LOG_FILE" || echo "0")
WARN_COUNT=$(grep -c "WARN" "$LOG_FILE" || echo "0")

echo "📊 Error Statistics"
echo "═══════════════════════════════════════"
echo "Total errors:   $ERROR_COUNT"
echo "Total warnings: $WARN_COUNT"
echo ""

if [ "$ERROR_COUNT" -eq 0 ]; then
  echo "✅ No errors found!"
  exit 0
fi

# Error types breakdown
echo "Error types:"
echo "─────────────────────────────────────"
grep "ERROR" "$LOG_FILE" | \
  cut -d' ' -f3 | \
  cut -d':' -f1 | \
  sort | uniq -c | sort -rn | \
  awk '{printf "  %-30s %s\n", $2, $1}'

echo ""

# Show lines with errors
echo "Lines with errors (top 10):"
echo "─────────────────────────────────────"
grep "ERROR" "$LOG_FILE" | grep -o "line=[0-9]*" | \
  cut -d'=' -f2 | sort -n | head -10 | \
  awk '{printf "  Line %s\n", $1}'

echo ""

# Show first few error examples
echo "Recent error examples:"
echo "─────────────────────────────────────"
grep "ERROR" "$LOG_FILE" | tail -5

echo ""
echo "═══════════════════════════════════════"

# Suggest fixes
echo ""
echo "💡 Suggestions:"
echo ""

# Check for parse errors
PARSE_ERRORS=$(grep "parse_error" "$LOG_FILE" 2>/dev/null | wc -l | xargs)
if [ "$PARSE_ERRORS" -gt 0 ]; then
  echo "  • $PARSE_ERRORS parse errors found"
  echo "    - Check if JSON is properly formatted"
  echo "    - Ensure one event per line (.jsonl format)"
  echo ""
fi

# Check for validation errors
VALIDATION_ERRORS=$(grep "validation_error" "$LOG_FILE" 2>/dev/null | wc -l | xargs)
if [ "$VALIDATION_ERRORS" -gt 0 ]; then
  echo "  • $VALIDATION_ERRORS validation errors found"
  echo "    - Events may have invalid signatures or IDs"
  echo "    - Consider using --validate-signatures=false --validate-event-ids=false if data is trusted"
  echo ""
fi

# Check for storage errors
STORAGE_ERRORS=$(grep "storage_error" "$LOG_FILE" 2>/dev/null | wc -l | xargs)
if [ "$STORAGE_ERRORS" -gt 0 ]; then
  echo "  • $STORAGE_ERRORS storage errors found"
  echo "    - Check disk space and write permissions"
  echo "    - Verify output directory is writable"
  echo ""
fi

echo "View full log:"
echo "  tail -n 100 $LOG_FILE"
echo ""
echo "Filter by error type:"
echo "  grep 'parse_error' $LOG_FILE"
echo "  grep 'validation_error' $LOG_FILE"
echo "  grep 'storage_error' $LOG_FILE"
echo ""

