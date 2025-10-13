#!/bin/bash
# Test Script for Examples
#
# This script validates that all example scripts are properly formatted
# and have the correct permissions

# Note: Not using 'set -e' to allow test to continue after failures

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🧪 Testing Example Scripts"
echo "═══════════════════════════════════════"
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

TESTS_PASSED=0
TESTS_FAILED=0

# Test function
test_script() {
  local script=$1
  local name=$(basename "$script")

  # Check if file exists
  if [ ! -f "$script" ]; then
    echo -e "${RED}✗${NC} $name - File not found"
    ((TESTS_FAILED++))
    return 1
  fi

  # Check if executable
  if [ ! -x "$script" ]; then
    echo -e "${YELLOW}!${NC} $name - Not executable (fixing...)"
    chmod +x "$script"
  fi

  # Check shebang
  if ! head -n 1 "$script" | grep -q '^#!/'; then
    echo -e "${RED}✗${NC} $name - Missing shebang"
    ((TESTS_FAILED++))
    return 1
  fi

  # Check for 'set -e'
  if ! grep -q '^set -e' "$script"; then
    echo -e "${YELLOW}!${NC} $name - Missing 'set -e' (not critical)"
  fi

  # Check syntax (basic)
  if ! bash -n "$script" 2>/dev/null; then
    echo -e "${RED}✗${NC} $name - Syntax error"
    ((TESTS_FAILED++))
    return 1
  fi

  echo -e "${GREEN}✓${NC} $name - OK"
  ((TESTS_PASSED++))
}

# Test all shell scripts
echo "Testing shell scripts:"
echo "───────────────────────────────────────"

for script in *.sh; do
  # Skip this test script
  if [ "$script" == "test_examples.sh" ]; then
    continue
  fi

  test_script "$script"
done

echo ""
echo "═══════════════════════════════════════"
echo "Tests passed: $TESTS_PASSED"
echo "Tests failed: $TESTS_FAILED"
echo ""

# Check if proton-beam is installed
echo "Checking dependencies:"
echo "───────────────────────────────────────"

check_command() {
  local cmd=$1
  local name=$2
  local required=$3

  if command -v "$cmd" &> /dev/null; then
    echo -e "${GREEN}✓${NC} $name installed"
  else
    if [ "$required" == "required" ]; then
      echo -e "${RED}✗${NC} $name - NOT INSTALLED (required)"
    else
      echo -e "${YELLOW}!${NC} $name - not installed (optional)"
    fi
  fi
}

# Check for proton-beam (check PATH, release, or debug)
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
if command -v proton-beam &> /dev/null; then
  echo -e "${GREEN}✓${NC} proton-beam installed (in PATH)"
elif [ -f "$PROJECT_ROOT/target/release/proton-beam" ]; then
  echo -e "${GREEN}✓${NC} proton-beam built (release mode)"
elif [ -f "$PROJECT_ROOT/target/debug/proton-beam" ]; then
  echo -e "${GREEN}✓${NC} proton-beam built (debug mode)"
else
  echo -e "${RED}✗${NC} proton-beam - NOT FOUND"
  echo "  Build it with: cargo build --release -p proton-beam-cli"
fi

check_command "nak" "nak" "optional"
check_command "jq" "jq" "optional"
check_command "bc" "bc" "optional"

echo ""
echo "═══════════════════════════════════════"

if [ $TESTS_FAILED -eq 0 ]; then
  echo -e "${GREEN}✅ All tests passed!${NC}"
  exit 0
else
  echo -e "${RED}❌ Some tests failed${NC}"
  exit 1
fi

