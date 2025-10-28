# ClickHouse Bulk Import Tool

This tool allows you to import existing `.pb.gz` (protobuf + gzip compressed) files into ClickHouse for efficient querying.

## Prerequisites

1. **ClickHouse installed and running**
   ```bash
   # macOS
   brew install clickhouse
   brew services start clickhouse

   # Ubuntu/Debian
   sudo apt-get install clickhouse-server clickhouse-client
   sudo service clickhouse-server start
   ```

2. **Schema initialized**
   ```bash
   cd clickhouse
   ./bootstrap.sh
   ```

3. **Build the importer** (with clickhouse feature enabled)
   ```bash
   cd ..
   cargo build --release --features clickhouse
   ```

## Quick Start

### Local Testing with Sample Data

Import a single `.pb.gz` file to your local ClickHouse:

```bash
./target/release/proton-beam-clickhouse-import \
  --input pb_data/2025_09_27.pb.gz
```

This will:
- Connect to `localhost:8123` (default ClickHouse HTTP port)
- Import all events from the file into the `nostr.events_local` table
- Show progress and statistics

### Import Multiple Files

```bash
./target/release/proton-beam-clickhouse-import \
  --input pb_data/*.pb.gz
```

### Custom ClickHouse Connection

For remote ClickHouse instances or custom configurations:

```bash
./target/release/proton-beam-clickhouse-import \
  --input pb_data/*.pb.gz \
  --host clickhouse.example.com \
  --port 8123 \
  --user admin \
  --password secret \
  --database nostr
```

### Dry Run (Test Without Inserting)

To validate your files without actually inserting into ClickHouse:

```bash
./target/release/proton-beam-clickhouse-import \
  --input pb_data/*.pb.gz \
  --dry-run
```

## Command-Line Options

| Option | Default | Description |
|--------|---------|-------------|
| `--input` | *required* | Input `.pb.gz` file(s) to import |
| `--host` | `localhost` | ClickHouse host |
| `--port` | `8123` | ClickHouse HTTP port |
| `--user` | `default` | ClickHouse user |
| `--password` | *(empty)* | ClickHouse password |
| `--database` | `nostr` | ClickHouse database name |
| `--table` | `events_local` | ClickHouse table name |
| `--batch-size` | `5000` | Events per batch insert |
| `--skip-test` | - | Skip connection test at startup |
| `--dry-run` | - | Parse files but don't insert |
| `--verbose` | - | Enable verbose logging |

## Performance Tuning

### Batch Size

The `--batch-size` parameter controls how many events are inserted at once:

- **Smaller batches (1000-2000)**: Lower memory usage, more network overhead
- **Larger batches (10000-20000)**: Higher throughput but more memory usage

Default is 5000 which provides a good balance for most use cases.

```bash
# For memory-constrained environments
./target/release/proton-beam-clickhouse-import \
  --input large_file.pb.gz \
  --batch-size 2000

# For high-performance imports
./target/release/proton-beam-clickhouse-import \
  --input large_file.pb.gz \
  --batch-size 10000
```

### Async Inserts

The importer uses ClickHouse async inserts by default, which:
- Buffers data before writing to disk
- Reduces write amplification
- Improves throughput

This is configured automatically in the client.

### Post-Import Optimization

After bulk importing, optimize your ClickHouse tables:

```bash
clickhouse-client --query="
  -- Materialize projections for better query performance
  ALTER TABLE nostr.events_local MATERIALIZE PROJECTION events_by_kind;
  ALTER TABLE nostr.event_tags_flat MATERIALIZE PROJECTION tags_by_value;

  -- Force deduplication (optional, merges happen automatically)
  OPTIMIZE TABLE nostr.events_local FINAL;
"
```

## Server Deployment

For production deployments on a server:

### 1. Copy Files to Server

```bash
# Copy the binary
scp target/release/proton-beam-clickhouse-import user@server:/usr/local/bin/

# Copy data files (or use rsync for large datasets)
rsync -avz --progress pb_data/ user@server:/data/nostr/pb_data/
```

### 2. Run Import on Server

```bash
ssh user@server

# Run import (consider using screen or tmux for long-running imports)
screen -S clickhouse-import

/usr/local/bin/proton-beam-clickhouse-import \
  --input /data/nostr/pb_data/*.pb.gz \
  --batch-size 10000 \
  --verbose
```

### 3. Monitor Progress

The importer shows:
- Number of events processed
- Processing speed (events/sec)
- Current file being processed

Progress is logged to stdout. For long-running imports, consider redirecting to a log file:

```bash
/usr/local/bin/proton-beam-clickhouse-import \
  --input /data/nostr/pb_data/*.pb.gz \
  2>&1 | tee import.log
```

## Verification

After import, verify your data:

```bash
clickhouse-client --query="
  -- Check event count
  SELECT count() as total_events
  FROM nostr.events_local;

  -- Check events by kind
  SELECT kind, count() as count
  FROM nostr.events_local
  GROUP BY kind
  ORDER BY count DESC
  LIMIT 10;

  -- Check date range
  SELECT
    toDate(min(created_at)) as earliest,
    toDate(max(created_at)) as latest
  FROM nostr.events_local;
"
```

## Troubleshooting

### Connection Refused

```
Error: Failed to connect to ClickHouse
```

**Solution:**
1. Check ClickHouse is running: `clickhouse-client --query "SELECT 1"`
2. Verify port (HTTP: 8123, Native: 9000)
3. Check firewall rules

### Database/Table Not Found

```
Error: Database 'nostr' does not exist
```

**Solution:** Run the bootstrap script first:
```bash
cd clickhouse && ./bootstrap.sh
```

### Out of Memory

```
Error: Cannot allocate memory
```

**Solution:** Reduce batch size:
```bash
--batch-size 1000
```

### Duplicate Events

The importer does NOT handle deduplication. The ClickHouse table uses `ReplacingMergeTree` which automatically deduplicates events with the same ID during background merges.

To force immediate deduplication:
```bash
clickhouse-client --query="OPTIMIZE TABLE nostr.events_local FINAL"
```

## Architecture Notes

- **Async Inserts**: Enabled by default for better throughput
- **Compression**: Data is automatically compressed by ClickHouse (LZ4 by default)
- **Partitioning**: Events are partitioned by month (`YYYYMM`)
- **Deduplication**: Handled by `ReplacingMergeTree` engine

## Next Steps

After importing your data:

1. **Query your data** - See `../docs/CLICKHOUSE_SCHEMA.md` for query examples
2. **Set up monitoring** - Track query performance and table sizes
3. **Schedule regular imports** - Set up a cron job or daemon for continuous imports
4. **Optimize queries** - Use projections and materialized views for common query patterns

## See Also

- [ClickHouse Schema Documentation](./README.md)
- [Bootstrap Script](./bootstrap.sh)
- [SQL Schema](./schema.sql)


