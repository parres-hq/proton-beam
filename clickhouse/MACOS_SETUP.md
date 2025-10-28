# ClickHouse Setup on macOS

If you installed ClickHouse via Homebrew (`brew install clickhouse`), the package includes the binaries but doesn't set up the server as a service. This guide helps you run ClickHouse manually.

## Quick Start

### 1. Start the Server

```bash
cd clickhouse
./start-clickhouse-macos.sh
```

This script will:
- Create necessary directories in `~/.clickhouse/`
- Generate a minimal config file
- Start the server in the background
- Verify the server is running

### 2. Connect to Server

```bash
clickhouse client
```

You should see:
```
ClickHouse client version 25.9.3.48 (official build).
Connecting to localhost:9000 as user default.
Connected to ClickHouse server version 25.9.3.
```

### 3. Initialize Schema

```bash
./bootstrap.sh
```

### 4. Import Data

```bash
cd ..
./target/release/proton-beam-clickhouse-import --input pb_data/*.pb.gz
```

## Manual Server Management

### Start Server (Manual)

```bash
# Create data directory
mkdir -p ~/.clickhouse/{data,metadata,tmp,log}

# Start server with minimal config
clickhouse server --config-file=~/.clickhouse/config.xml &
```

### Stop Server

```bash
# Find and kill the server process
pkill -f 'clickhouse.*server'

# Or if you know the PID
kill <PID>
```

### Check if Server is Running

```bash
# Check port 9000 (native protocol)
lsof -i :9000

# Check port 8123 (HTTP interface)
lsof -i :8123

# Or try to connect
clickhouse client --query "SELECT 1"
```

### View Logs

```bash
tail -f ~/.clickhouse/log/clickhouse-server.log
tail -f ~/.clickhouse/log/clickhouse-server.err.log
```

## Troubleshooting

### Problem: "Connection refused (localhost:9000)"

**Cause:** Server is not running.

**Solution:** Start the server with `./start-clickhouse-macos.sh`

### Problem: "Address already in use"

**Cause:** Another process is using port 9000 or 8123.

**Solution:**
```bash
# Find what's using the port
lsof -i :9000
lsof -i :8123

# Kill the process if needed
kill <PID>
```

### Problem: Server starts but immediately stops

**Cause:** Configuration or permission issues.

**Solution:** Check error logs:
```bash
tail -f ~/.clickhouse/log/clickhouse-server.err.log
```

Common issues:
- Disk space full
- Insufficient permissions on data directory
- Port already in use

### Problem: "No available formula with the name 'clickhouse'"

This happens when trying `brew services start clickhouse` because Homebrew doesn't include a service definition for ClickHouse.

**Solution:** Use the manual startup script instead:
```bash
./start-clickhouse-macos.sh
```

## Alternative: Docker

If you prefer using Docker instead:

```bash
# Start ClickHouse in Docker
docker run -d \
  --name clickhouse-server \
  -p 8123:8123 \
  -p 9000:9000 \
  --ulimit nofile=262144:262144 \
  clickhouse/clickhouse-server

# Connect
clickhouse client --host localhost

# Or use docker exec
docker exec -it clickhouse-server clickhouse-client
```

## Server Locations

When running manually (via the startup script):

- **Config**: `~/.clickhouse/config.xml`
- **Data**: `~/.clickhouse/data/`
- **Logs**: `~/.clickhouse/log/`
- **Temp**: `~/.clickhouse/tmp/`

## Testing the Setup

### 1. Test Connection
```bash
clickhouse client --query "SELECT version()"
```

### 2. Create Test Database
```bash
clickhouse client --query "CREATE DATABASE IF NOT EXISTS test"
```

### 3. Create Test Table
```bash
clickhouse client --query "
CREATE TABLE test.events (
    id String,
    created_at DateTime
) ENGINE = MergeTree()
ORDER BY created_at
"
```

### 4. Insert Test Data
```bash
clickhouse client --query "
INSERT INTO test.events VALUES
    ('test1', now()),
    ('test2', now())
"
```

### 5. Query Test Data
```bash
clickhouse client --query "SELECT * FROM test.events"
```

### 6. Clean Up
```bash
clickhouse client --query "DROP DATABASE test"
```

## Next Steps

Once the server is running:

1. ✅ Server is running → Initialize schema with `./bootstrap.sh`
2. ✅ Schema is initialized → Import data with the import tool
3. ✅ Data is imported → Run queries and analyze events

See [TESTING.md](./TESTING.md) for the complete testing workflow.

## Performance Notes

The manual server setup uses default settings optimized for single-machine use. For production:

- Adjust memory limits in `config.xml`
- Configure data retention policies
- Set up proper backups
- Monitor disk space
- Consider using Docker or a dedicated VM

## References

- [ClickHouse Documentation](https://clickhouse.com/docs)
- [Installation Guide](https://clickhouse.com/docs/en/install)
- [Configuration Reference](https://clickhouse.com/docs/en/operations/configuration-files)



