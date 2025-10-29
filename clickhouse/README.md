# ClickHouse Setup for Proton Beam

This directory contains the ClickHouse schema and bulk import tool for storing and querying Nostr events efficiently.

## Quick Start

### 1. Install ClickHouse

**macOS:**
```bash
brew install clickhouse

# Homebrew doesn't include a service, so start manually:
./start-clickhouse-macos.sh
```

**Ubuntu/Debian:**
```bash
sudo apt-get install clickhouse-server clickhouse-client
sudo service clickhouse-server start
```

**Verify installation:**
```bash
clickhouse-client --query "SELECT version()"
```

### 2. Initialize Schema

```bash
cd clickhouse
./bootstrap.sh
```

This creates:
- Database: `nostr`
- Table: `events_local` (main events table)
- Materialized view: `event_tags_flat` (for tag queries)
- Helper views: `event_stats`, `relay_stats`, `tag_stats`, and more

### 3. Build the Import Tool

```bash
cd ..
cargo build --release --features clickhouse
```

### 4. Import Data

```bash
# Import a single file
./target/release/proton-beam-clickhouse-import --input pb_data/2025_09_27.pb.gz

# Import multiple files
./target/release/proton-beam-clickhouse-import --input pb_data/*.pb.gz

# Test without inserting (dry run)
./target/release/proton-beam-clickhouse-import --input pb_data/*.pb.gz --dry-run
```

## Schema Overview

### Main Tables

**`events_local`** - Main events table
- Stores all Nostr events (NIP-01 format)
- Engine: `ReplacingMergeTree` - automatic deduplication by event ID
- Sort order: `(created_at, kind, pubkey)` - optimized for time-range queries
- Partitioning: Monthly (`YYYYMM`) for easy data lifecycle management
- Projection: Alternate `(kind, created_at, pubkey)` order for kind-specific queries

**`event_tags_flat`** - Materialized view for tag queries
- Automatically flattens nested tag arrays from events
- Sort order: `(tag_name, tag_value_primary, created_at, event_id)`
- Projections for value-first and event-first queries
- Updated automatically when events are inserted

### Analytical Views

Pre-built views for common analytics:
- `event_stats` - Event counts by date and kind
- `relay_stats` - Statistics by relay source
- `tag_stats` - Tag usage statistics
- `daily_active_users`, `weekly_active_users`, `monthly_active_users`
- `users_with_metadata` - Users with kind 0 profile data
- `top_publishers`, `top_verified_publishers`
- `activity_by_kind` - Daily event breakdown by kind

## Import Tool Usage

### Basic Import

```bash
./target/release/proton-beam-clickhouse-import --input events.pb.gz
```

### Remote ClickHouse

```bash
./target/release/proton-beam-clickhouse-import \
  --input pb_data/*.pb.gz \
  --host clickhouse.example.com \
  --port 8123 \
  --user admin \
  --password secret \
  --database nostr
```

### Performance Tuning

```bash
# Larger batches (10k) for high throughput
./target/release/proton-beam-clickhouse-import \
  --input large_file.pb.gz \
  --batch-size 10000

# Smaller batches (2k) for memory-constrained environments
./target/release/proton-beam-clickhouse-import \
  --input large_file.pb.gz \
  --batch-size 2000
```

### Command-Line Options

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

## Common Queries

### Time-Range Queries

```sql
-- Events from last 24 hours
SELECT id, kind, pubkey, content, created_at
FROM events_local
WHERE created_at >= now() - INTERVAL 1 DAY
ORDER BY created_at DESC
LIMIT 100;

-- Events from a specific month
SELECT count() as total, kind
FROM events_local
WHERE toYYYYMM(created_at) = 202510
GROUP BY kind
ORDER BY total DESC;
```

### Kind-Specific Queries

```sql
-- All text notes (kind 1)
SELECT id, pubkey, content, created_at
FROM events_local
WHERE kind = 1
ORDER BY created_at DESC
LIMIT 100;

-- Event distribution by kind
SELECT kind, count() as total
FROM events_local
GROUP BY kind
ORDER BY total DESC;
```

### Tag Queries

```sql
-- Find events tagged with 'bitcoin'
SELECT event_id, kind, created_at
FROM event_tags_flat
WHERE tag_name = 't' AND tag_value_primary = 'bitcoin'
ORDER BY created_at DESC;

-- Find replies (e tag with 'reply' marker)
SELECT event_id, tag_value_primary as replied_to_event
FROM event_tags_flat
WHERE tag_name = 'e' AND tag_value_position_4 = 'reply'
ORDER BY created_at DESC;

-- Find mentions of a specific pubkey
SELECT event_id, kind, created_at
FROM event_tags_flat
WHERE tag_name = 'p' AND tag_value_primary = 'YOUR_PUBKEY_HERE'
ORDER BY created_at DESC;
```

### Author Queries

```sql
-- All events by specific author
SELECT id, kind, content, created_at
FROM events_local
WHERE pubkey = 'YOUR_PUBKEY_HERE'
ORDER BY created_at DESC;
```

### Statistics

```sql
-- Daily active users
SELECT * FROM daily_active_users
WHERE date >= today() - 30
ORDER BY date DESC;

-- Top publishers
SELECT * FROM top_publishers LIMIT 100;

-- Activity by kind
SELECT * FROM activity_by_kind
WHERE date >= today() - 7
ORDER BY date DESC, events DESC;
```

## Verification & Maintenance

### Verify Import

```bash
# Check event count
clickhouse-client --query="SELECT count() FROM nostr.events_local"

# Check events by kind
clickhouse-client --query="
  SELECT kind, count() as count
  FROM nostr.events_local
  GROUP BY kind
  ORDER BY count DESC
  LIMIT 10
"

# Check date range
clickhouse-client --query="
  SELECT
    toDate(min(created_at)) as earliest,
    toDate(max(created_at)) as latest
  FROM nostr.events_local
"
```

### Materialize Projections

After bulk import, materialize projections for optimal query performance:

```sql
ALTER TABLE events_local MATERIALIZE PROJECTION events_by_kind;
ALTER TABLE event_tags_flat MATERIALIZE PROJECTION tags_by_value;
ALTER TABLE event_tags_flat MATERIALIZE PROJECTION tags_by_event;
```

### Force Deduplication

ClickHouse deduplicates automatically during background merges. To force immediate deduplication:

```sql
-- Specific partition
OPTIMIZE TABLE events_local PARTITION 202510 FINAL;

-- All partitions (expensive!)
OPTIMIZE TABLE events_local FINAL;
```

### Drop Old Partitions

```sql
-- List partitions
SELECT partition, rows, formatReadableSize(bytes_on_disk) as size
FROM system.parts
WHERE table = 'events_local' AND database = 'nostr' AND active = 1
ORDER BY partition DESC;

-- Drop old partition
ALTER TABLE events_local DROP PARTITION 202301;
```

### Monitor Performance

```sql
-- Recent query performance
SELECT
    query,
    query_duration_ms,
    read_rows,
    formatReadableSize(read_bytes) as read_size
FROM system.query_log
WHERE type = 'QueryFinish' AND query LIKE '%events%'
ORDER BY event_time DESC
LIMIT 20;

-- Check projection usage
SELECT query, ProjectionNames
FROM system.query_log
WHERE type = 'QueryFinish'
  AND has(ProjectionNames, 'events_by_kind')
ORDER BY event_time DESC
LIMIT 10;
```

## Troubleshooting

### Connection Refused

**Problem:** Can't connect to ClickHouse

**Solution:**
```bash
# Check if server is running
clickhouse-client --query "SELECT 1"

# macOS: Start server
cd clickhouse && ./start-clickhouse-macos.sh

# Linux: Check service status
sudo service clickhouse-server status
```

### Database Not Found

**Problem:** `Database 'nostr' does not exist`

**Solution:**
```bash
cd clickhouse && ./bootstrap.sh
```

### Out of Memory

**Problem:** Import fails with memory errors

**Solution:** Reduce batch size:
```bash
./target/release/proton-beam-clickhouse-import \
  --input large_file.pb.gz \
  --batch-size 1000
```

### Server Won't Start (macOS)

**Problem:** "Address already in use" or server stops immediately

**Solution:**
```bash
# Check what's using the ports
lsof -i :9000
lsof -i :8123

# Kill existing process if needed
pkill -f 'clickhouse.*server'

# Check error logs
tail -f ~/.clickhouse/log/clickhouse-server.err.log
```

## Advanced Configuration

### Custom Bootstrap Options

```bash
# Remote server
./bootstrap.sh --host clickhouse.example.com --user admin --password secret

# Custom database name
./bootstrap.sh --database my_nostr_db

# Rebuild schema (WARNING: deletes all data!)
./bootstrap.sh --drop-existing

# See all options
./bootstrap.sh --help
```

### Environment Variables

```bash
export CLICKHOUSE_HOST=localhost
export CLICKHOUSE_PORT=9000
export CLICKHOUSE_USER=default
export CLICKHOUSE_PASSWORD=your_password

./bootstrap.sh
```

## Architecture

The ClickHouse integration uses:

- **Async inserts**: Batched writes with configurable batch size (default: 5000 events)
- **Compression**: ZSTD level 3 for content, automatic LZ4 for other columns
- **Monthly partitioning**: Easy data lifecycle management and query optimization
- **Projections**: Multiple sort orders without data duplication
- **Materialized views**: Automatic tag flattening for efficient queries
- **ReplacingMergeTree**: Automatic deduplication by event ID

## Files

- `schema.sql` - Complete ClickHouse schema definition
- `bootstrap.sh` - Automated setup script with options
- `start-clickhouse-macos.sh` - macOS server startup helper
- `clickhouse-import.toml.example` - Configuration template
- `README.md` - This file

## Performance Notes

Typical performance on standard hardware:
- **Import speed**: 10,000-50,000 events/sec (depends on event size and batch size)
- **Query speed**: Sub-second for most queries on millions of events
- **Disk usage**: ~30-50% of original `.pb.gz` size after ClickHouse compression
- **Memory usage**: ~100-500MB during import (depends on batch size)

## Next Steps

Future enhancements:
1. **Daemon Integration** - Real-time streaming from relays directly to ClickHouse
2. **Query API** - REST API for querying events
3. **Monitoring** - Grafana dashboards for event metrics
4. **Config file support** - Load settings from TOML file
5. **Resume capability** - Skip already-imported files
6. **Parallel imports** - Multiple files in parallel

## License

MIT License - See LICENSE file for details
