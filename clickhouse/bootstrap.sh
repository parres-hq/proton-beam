#!/usr/bin/env bash

# Proton Beam Clickhouse Bootstrap Script
# Version: 1.0
# Description: Sets up a fresh Clickhouse installation with Nostr event schema
#
# Usage:
#   ./bootstrap.sh [OPTIONS]
#
# Options:
#   --host HOST        Clickhouse host (default: localhost)
#   --port PORT        Clickhouse port (default: 9000)
#   --user USER        Clickhouse user (default: default)
#   --password PASS    Clickhouse password (default: empty)
#   --database DB      Database name (default: nostr)
#   --drop-existing    Drop existing tables before creating (CAUTION: deletes data!)
#   --skip-verify      Skip verification queries
#   --help             Show this help message

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
CLICKHOUSE_HOST="${CLICKHOUSE_HOST:-localhost}"
CLICKHOUSE_PORT="${CLICKHOUSE_PORT:-9000}"
CLICKHOUSE_USER="${CLICKHOUSE_USER:-default}"
CLICKHOUSE_PASSWORD="${CLICKHOUSE_PASSWORD:-}"
DATABASE="nostr"
DROP_EXISTING=false
SKIP_VERIFY=false

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCHEMA_FILE="${SCRIPT_DIR}/schema.sql"

# =============================================================================
# Helper Functions
# =============================================================================

print_header() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

show_help() {
    cat << EOF
Proton Beam Clickhouse Bootstrap Script

Sets up a fresh Clickhouse installation with the Nostr event schema.

USAGE:
    ./bootstrap.sh [OPTIONS]

OPTIONS:
    --host HOST          Clickhouse host (default: localhost)
    --port PORT          Clickhouse native port (default: 9000)
    --user USER          Clickhouse user (default: default)
    --password PASS      Clickhouse password (default: empty)
    --database DB        Database name (default: nostr)
    --drop-existing      Drop existing tables before creating (CAUTION: deletes data!)
    --skip-verify        Skip verification queries
    --help               Show this help message

EXAMPLES:
    # Local installation with defaults
    ./bootstrap.sh

    # Remote Clickhouse server
    ./bootstrap.sh --host clickhouse.example.com --user admin --password secret

    # Rebuild schema (drops existing tables)
    ./bootstrap.sh --drop-existing

ENVIRONMENT VARIABLES:
    CLICKHOUSE_HOST      Clickhouse host
    CLICKHOUSE_PORT      Clickhouse port
    CLICKHOUSE_USER      Clickhouse user
    CLICKHOUSE_PASSWORD  Clickhouse password

REQUIREMENTS:
    - Clickhouse server running
    - clickhouse-client installed locally
    - Network access to Clickhouse server

NOTES:
    This script will create:
    - Database: $DATABASE
    - Table: events_local (main events table)
    - Materialized View: event_tags_flat (for tag queries)
    - Projections: events_by_kind, tags_by_value
    - Helper Views: event_stats, relay_stats, tag_stats

EOF
    exit 0
}

# =============================================================================
# Command Line Parsing
# =============================================================================

while [[ $# -gt 0 ]]; do
    case $1 in
        --host)
            CLICKHOUSE_HOST="$2"
            shift 2
            ;;
        --port)
            CLICKHOUSE_PORT="$2"
            shift 2
            ;;
        --user)
            CLICKHOUSE_USER="$2"
            shift 2
            ;;
        --password)
            CLICKHOUSE_PASSWORD="$2"
            shift 2
            ;;
        --database)
            DATABASE="$2"
            shift 2
            ;;
        --drop-existing)
            DROP_EXISTING=true
            shift
            ;;
        --skip-verify)
            SKIP_VERIFY=true
            shift
            ;;
        --help)
            show_help
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Run './bootstrap.sh --help' for usage information."
            exit 1
            ;;
    esac
done

# =============================================================================
# Pre-flight Checks
# =============================================================================

print_header "Proton Beam Clickhouse Bootstrap"

print_info "Configuration:"
echo "  Host:     ${CLICKHOUSE_HOST}"
echo "  Port:     ${CLICKHOUSE_PORT}"
echo "  User:     ${CLICKHOUSE_USER}"
echo "  Database: ${DATABASE}"
echo ""

# Check if clickhouse-client is installed
if ! command -v clickhouse-client &> /dev/null; then
    print_error "clickhouse-client not found in PATH"
    echo ""
    echo "Please install Clickhouse client:"
    echo "  macOS:   brew install clickhouse"
    echo "  Ubuntu:  sudo apt-get install clickhouse-client"
    echo "  Other:   https://clickhouse.com/docs/en/install"
    exit 1
fi

print_success "clickhouse-client found"

# Check if schema file exists
if [[ ! -f "$SCHEMA_FILE" ]]; then
    print_error "Schema file not found: $SCHEMA_FILE"
    exit 1
fi

print_success "Schema file found: $SCHEMA_FILE"

# Build clickhouse-client command
CLICKHOUSE_CMD="clickhouse-client --host=${CLICKHOUSE_HOST} --port=${CLICKHOUSE_PORT} --user=${CLICKHOUSE_USER}"
if [[ -n "$CLICKHOUSE_PASSWORD" ]]; then
    CLICKHOUSE_CMD="${CLICKHOUSE_CMD} --password=${CLICKHOUSE_PASSWORD}"
fi

# Test connection
print_info "Testing connection to Clickhouse..."
if $CLICKHOUSE_CMD --query="SELECT version()" &> /dev/null; then
    VERSION=$($CLICKHOUSE_CMD --query="SELECT version()")
    print_success "Connected to Clickhouse v${VERSION}"
else
    print_error "Failed to connect to Clickhouse at ${CLICKHOUSE_HOST}:${CLICKHOUSE_PORT}"
    echo ""
    echo "Please ensure:"
    echo "  1. Clickhouse server is running"
    echo "  2. Host and port are correct"
    echo "  3. Credentials are valid"
    echo "  4. Firewall allows connection"
    exit 1
fi

# =============================================================================
# Drop Existing Tables (if requested)
# =============================================================================

if [[ "$DROP_EXISTING" == true ]]; then
    print_warning "Drop existing tables requested"
    echo ""
    read -p "Are you sure you want to drop existing tables? This will DELETE ALL DATA! (yes/no): " -r
    echo ""

    if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
        print_info "Aborted by user"
        exit 0
    fi

    print_info "Dropping existing tables..."

    $CLICKHOUSE_CMD --query="DROP TABLE IF EXISTS ${DATABASE}.events" 2>/dev/null || true
    $CLICKHOUSE_CMD --query="DROP TABLE IF EXISTS ${DATABASE}.events_local" 2>/dev/null || true
    $CLICKHOUSE_CMD --query="DROP TABLE IF EXISTS ${DATABASE}.event_tags" 2>/dev/null || true
    $CLICKHOUSE_CMD --query="DROP TABLE IF EXISTS ${DATABASE}.event_tags_flat" 2>/dev/null || true
    $CLICKHOUSE_CMD --query="DROP VIEW IF EXISTS ${DATABASE}.event_stats" 2>/dev/null || true
    $CLICKHOUSE_CMD --query="DROP VIEW IF EXISTS ${DATABASE}.relay_stats" 2>/dev/null || true
    $CLICKHOUSE_CMD --query="DROP VIEW IF EXISTS ${DATABASE}.tag_stats" 2>/dev/null || true

    print_success "Existing tables dropped"
fi

# =============================================================================
# Create Schema
# =============================================================================

print_header "Creating Schema"

print_info "Executing schema.sql..."

# Replace database name in schema if different from default
TEMP_SCHEMA=$(mktemp)
sed "s/CREATE DATABASE IF NOT EXISTS nostr/CREATE DATABASE IF NOT EXISTS ${DATABASE}/g" "$SCHEMA_FILE" > "$TEMP_SCHEMA"
sed -i.bak "s/USE nostr/USE ${DATABASE}/g" "$TEMP_SCHEMA"
sed -i.bak "s/WHERE database = 'nostr'/WHERE database = '${DATABASE}'/g" "$TEMP_SCHEMA"
rm -f "${TEMP_SCHEMA}.bak"

# Execute schema
if $CLICKHOUSE_CMD --multiquery < "$TEMP_SCHEMA"; then
    print_success "Schema created successfully"
else
    print_error "Failed to create schema"
    rm -f "$TEMP_SCHEMA"
    exit 1
fi

rm -f "$TEMP_SCHEMA"

# =============================================================================
# Verification
# =============================================================================

if [[ "$SKIP_VERIFY" == false ]]; then
    print_header "Verification"

    # Check database exists
    print_info "Checking database..."
    if $CLICKHOUSE_CMD --query="SELECT name FROM system.databases WHERE name = '${DATABASE}'" | grep -q "${DATABASE}"; then
        print_success "Database '${DATABASE}' exists"
    else
        print_error "Database '${DATABASE}' not found"
        exit 1
    fi

    # Check tables exist
    print_info "Checking tables..."

    EXPECTED_TABLES=("events_local" "event_tags_flat")
    for table in "${EXPECTED_TABLES[@]}"; do
        if $CLICKHOUSE_CMD --query="SELECT name FROM system.tables WHERE database = '${DATABASE}' AND name = '${table}'" | grep -q "${table}"; then
            print_success "Table '${table}' exists"
        else
            print_error "Table '${table}' not found"
            exit 1
        fi
    done

    # Check views exist
    print_info "Checking views..."

    EXPECTED_VIEWS=("event_stats" "relay_stats" "tag_stats")
    for view in "${EXPECTED_VIEWS[@]}"; do
        if $CLICKHOUSE_CMD --query="SELECT name FROM system.tables WHERE database = '${DATABASE}' AND name = '${view}'" | grep -q "${view}"; then
            print_success "View '${view}' exists"
        else
            print_warning "View '${view}' not found (non-critical)"
        fi
    done

    # Show table statistics
    echo ""
    print_info "Table statistics:"
    $CLICKHOUSE_CMD --query="
        SELECT
            name as table_name,
            engine,
            formatReadableSize(total_bytes) as size,
            total_rows as rows
        FROM system.tables
        WHERE database = '${DATABASE}' AND engine NOT LIKE '%View%'
        ORDER BY name
        FORMAT PrettyCompact
    "
fi

# =============================================================================
# Success Summary
# =============================================================================

print_header "Setup Complete!"

echo ""
print_success "Clickhouse schema successfully created"
echo ""
print_info "Database: ${DATABASE}"
print_info "Tables created:"
echo "  - events_local (main events table)"
echo "  - event_tags_flat (materialized view for tag queries)"
echo ""
print_info "Views created:"
echo "  - event_stats (event statistics by date and kind)"
echo "  - relay_stats (relay source statistics)"
echo "  - tag_stats (tag usage statistics)"
echo ""

print_info "Next steps:"
echo ""
echo "  1. Test with sample data:"
echo "     ${CLICKHOUSE_CMD} --query=\""
echo "       USE ${DATABASE};"
echo "       INSERT INTO events_local (id, pubkey, created_at, kind, content, sig, tags)"
echo "       VALUES ('test123', 'pubkey456', now(), 1, 'Hello Nostr!', 'sig789', [['t', 'test']]);"
echo "     \""
echo ""
echo "  2. Query your data:"
echo "     ${CLICKHOUSE_CMD} --query=\"SELECT * FROM ${DATABASE}.events_local LIMIT 10\""
echo ""
echo "  3. After bulk import, materialize projections:"
echo "     ${CLICKHOUSE_CMD} --query=\""
echo "       ALTER TABLE ${DATABASE}.events_local MATERIALIZE PROJECTION events_by_kind;"
echo "       ALTER TABLE ${DATABASE}.event_tags_flat MATERIALIZE PROJECTION tags_by_value;"
echo "     \""
echo ""
echo "  4. Build the bulk import tool:"
echo "     cd ../proton-beam-cli && cargo build --release"
echo ""

print_info "Documentation:"
echo "  - Schema details: ../docs/CLICKHOUSE_SCHEMA.md"
echo "  - Query examples: See schema.sql comments"
echo ""

print_success "Bootstrap complete! 🚀"
echo ""

