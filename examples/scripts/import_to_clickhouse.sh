#!/usr/bin/env bash
#
# Example script for importing .pb.gz files into ClickHouse
#
# This script demonstrates how to use the clickhouse-import tool
# for various common scenarios.

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the project root (two directories up from this script)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

IMPORTER="${PROJECT_ROOT}/target/release/proton-beam-clickhouse-import"
PB_DATA_DIR="${PROJECT_ROOT}/pb_data"

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}ClickHouse Import Examples${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check if importer is built
if [[ ! -f "$IMPORTER" ]]; then
    echo -e "${YELLOW}⚠️  Importer not built. Building now...${NC}"
    cd "$PROJECT_ROOT"
    cargo build --release --features clickhouse
    echo ""
fi

# Check if ClickHouse is running
if ! command -v clickhouse-client &> /dev/null; then
    echo -e "${YELLOW}⚠️  clickhouse-client not found. Please install ClickHouse:${NC}"
    echo ""
    echo "  macOS:   brew install clickhouse"
    echo "  Ubuntu:  sudo apt-get install clickhouse-client"
    echo ""
    exit 1
fi

if ! clickhouse-client --query "SELECT 1" &> /dev/null; then
    echo -e "${YELLOW}⚠️  Cannot connect to ClickHouse. Is the server running?${NC}"
    echo ""
    echo "  macOS:   brew services start clickhouse"
    echo "  Linux:   sudo service clickhouse-server start"
    echo ""
    exit 1
fi

echo -e "${GREEN}✓${NC} ClickHouse is running"

# Check if schema is initialized
if ! clickhouse-client --query "SELECT 1 FROM system.tables WHERE database = 'nostr' AND name = 'events_local'" | grep -q "1"; then
    echo -e "${YELLOW}⚠️  Schema not initialized. Run:${NC}"
    echo ""
    echo "  cd ${PROJECT_ROOT}/clickhouse"
    echo "  ./bootstrap.sh"
    echo ""
    exit 1
fi

echo -e "${GREEN}✓${NC} Schema initialized"
echo ""

# Function to run an example
run_example() {
    local title="$1"
    local description="$2"
    shift 2

    echo -e "${BLUE}Example: ${title}${NC}"
    echo "  ${description}"
    echo ""
    echo "  Command:"
    echo "    $*"
    echo ""

    if [[ "${DRY_RUN:-no}" == "yes" ]]; then
        echo "  (Skipping - dry run mode)"
        echo ""
        return
    fi

    # Ask user if they want to run this example
    read -p "  Run this example? (y/N) " -r
    echo ""

    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "  Skipped."
        echo ""
        return
    fi

    "$@"
    echo ""
}

# Example 1: Import a single file
run_example \
    "Import Single File" \
    "Import one .pb.gz file from pb_data/" \
    "$IMPORTER" --input "${PB_DATA_DIR}/"*.pb.gz

# Example 2: Import with custom batch size
run_example \
    "Custom Batch Size" \
    "Import with smaller batch size (useful for limited memory)" \
    "$IMPORTER" --input "${PB_DATA_DIR}/"*.pb.gz --batch-size 2000

# Example 3: Verbose mode
run_example \
    "Verbose Import" \
    "Import with detailed logging" \
    "$IMPORTER" --input "${PB_DATA_DIR}/"*.pb.gz --verbose

# Example 4: Dry run
echo -e "${BLUE}Example: Dry Run${NC}"
echo "  Parse files without actually inserting into ClickHouse"
echo ""
echo "  Command:"
echo "    $IMPORTER --input ${PB_DATA_DIR}/*.pb.gz --dry-run"
echo ""
read -p "  Run this example? (y/N) " -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    "$IMPORTER" --input "${PB_DATA_DIR}/"*.pb.gz --dry-run
    echo ""
fi

# Show final statistics
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Current Database Statistics${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

clickhouse-client --query="
SELECT
    count() as total_events,
    uniq(pubkey) as unique_authors,
    uniq(kind) as unique_kinds,
    formatReadableSize(sum(length(content))) as total_content_size,
    toDate(min(created_at)) as earliest_event,
    toDate(max(created_at)) as latest_event
FROM nostr.events_local
FORMAT Vertical
"

echo ""
echo -e "${GREEN}✓${NC} Done! Check out IMPORT_README.md for more options."
echo ""



