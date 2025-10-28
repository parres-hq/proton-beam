#!/usr/bin/env bash
#
# Start ClickHouse server on macOS (when installed via Homebrew)
#
# Usage: ./start-clickhouse-macos.sh

set -euo pipefail

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}Starting ClickHouse Server${NC}"
echo ""

# Check if clickhouse is installed
if ! command -v clickhouse &> /dev/null; then
    echo -e "${RED}✗${NC} ClickHouse not found. Install with:"
    echo "  brew install clickhouse"
    exit 1
fi

echo -e "${GREEN}✓${NC} ClickHouse binary found: $(which clickhouse)"

# Create data directory if it doesn't exist
DATA_DIR="$HOME/.clickhouse"
mkdir -p "$DATA_DIR"
mkdir -p "$DATA_DIR/data"
mkdir -p "$DATA_DIR/metadata"
mkdir -p "$DATA_DIR/tmp"
mkdir -p "$DATA_DIR/user_files"
mkdir -p "$DATA_DIR/format_schemas"
mkdir -p "$DATA_DIR/log"

echo -e "${GREEN}✓${NC} Data directory: $DATA_DIR"

# Check if server is already running
if lsof -Pi :9000 -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠${NC}  ClickHouse server already running on port 9000"
    echo ""
    echo "To connect:"
    echo "  clickhouse client"
    echo ""
    echo "To stop:"
    echo "  pkill -f 'clickhouse.*server'"
    exit 0
fi

# Create minimal config
CONFIG_FILE="$DATA_DIR/config.xml"
if [[ ! -f "$CONFIG_FILE" ]]; then
    cat > "$CONFIG_FILE" << 'EOF'
<clickhouse>
    <logger>
        <level>information</level>
        <log>~/.clickhouse/log/clickhouse-server.log</log>
        <errorlog>~/.clickhouse/log/clickhouse-server.err.log</errorlog>
        <size>100M</size>
        <count>3</count>
    </logger>

    <http_port>8123</http_port>
    <tcp_port>9000</tcp_port>

    <listen_host>::1</listen_host>
    <listen_host>127.0.0.1</listen_host>

    <path>~/.clickhouse/data/</path>
    <tmp_path>~/.clickhouse/tmp/</tmp_path>
    <user_files_path>~/.clickhouse/user_files/</user_files_path>
    <format_schema_path>~/.clickhouse/format_schemas/</format_schema_path>

    <users>
        <default>
            <password></password>
            <networks>
                <ip>::/0</ip>
            </networks>
            <profile>default</profile>
            <quota>default</quota>
        </default>
    </users>

    <profiles>
        <default>
            <max_memory_usage>10000000000</max_memory_usage>
            <use_uncompressed_cache>0</use_uncompressed_cache>
            <load_balancing>random</load_balancing>
        </default>
    </profiles>

    <quotas>
        <default>
            <interval>
                <duration>3600</duration>
                <queries>0</queries>
                <errors>0</errors>
                <result_rows>0</result_rows>
                <read_rows>0</read_rows>
                <execution_time>0</execution_time>
            </interval>
        </default>
    </quotas>
</clickhouse>
EOF
    echo -e "${GREEN}✓${NC} Created config file: $CONFIG_FILE"
else
    echo -e "${GREEN}✓${NC} Using existing config: $CONFIG_FILE"
fi

# Start server in background
LOG_FILE="$DATA_DIR/log/clickhouse-server.log"
ERR_FILE="$DATA_DIR/log/clickhouse-server.err.log"

echo ""
echo "Starting server..."
echo "  Config: $CONFIG_FILE"
echo "  Log: $LOG_FILE"
echo "  Error log: $ERR_FILE"
echo ""

nohup clickhouse server --config-file="$CONFIG_FILE" > /dev/null 2>&1 &
SERVER_PID=$!

echo "Waiting for server to start..."
sleep 2

# Check if server is running
if ! lsof -Pi :9000 -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo -e "${RED}✗${NC} Failed to start server. Check logs:"
    echo "  tail -f $ERR_FILE"
    exit 1
fi

echo -e "${GREEN}✓${NC} ClickHouse server started (PID: $SERVER_PID)"
echo ""

# Test connection
if clickhouse client --query "SELECT version()" &> /dev/null; then
    VERSION=$(clickhouse client --query "SELECT version()")
    echo -e "${GREEN}✓${NC} Server is responding (version: $VERSION)"
else
    echo -e "${YELLOW}⚠${NC}  Server started but not responding yet. Wait a few seconds."
fi

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}ClickHouse is ready!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Connect to server:"
echo "  clickhouse client"
echo ""
echo "HTTP interface:"
echo "  curl http://localhost:8123"
echo ""
echo "View logs:"
echo "  tail -f $LOG_FILE"
echo ""
echo "Stop server:"
echo "  pkill -f 'clickhouse.*server'"
echo "  # or: kill $SERVER_PID"
echo ""
echo "Next steps:"
echo "  1. Initialize schema: cd $(pwd) && ./bootstrap.sh"
echo "  2. Import data: ../target/release/proton-beam-clickhouse-import --input ../pb_data/*.pb.gz"
echo ""



