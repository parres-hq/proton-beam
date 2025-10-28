# Clickhouse Setup for Proton Beam

This directory contains the Clickhouse schema and setup tools for storing Nostr events.

## Quick Start

### Prerequisites

- Clickhouse server installed and running
- `clickhouse-client` CLI tool installed

**Install Clickhouse:**
```bash
# macOS
brew install clickhouse
# Note: Homebrew doesn't include a service, see MACOS_SETUP.md for startup instructions

# Ubuntu/Debian
sudo apt-get install clickhouse-server clickhouse-client

# Start server (macOS)
# Use the startup script: ./start-clickhouse-macos.sh
# See MACOS_SETUP.md for details

# Start server (Linux)
sudo service clickhouse-server start
```

### Setup Schema

Run the bootstrap script to create the database and tables:

```bash
./bootstrap.sh
```

This will:
1. Test connection to Clickhouse
2. Create the `nostr` database
3. Create all tables, views, and projections
4. Verify the setup

### Custom Configuration

```bash
# Remote Clickhouse server
./bootstrap.sh --host clickhouse.example.com --user admin --password secret

# Custom database name
./bootstrap.sh --database my_nostr_db

# Rebuild schema (WARNING: deletes all data!)
./bootstrap.sh --drop-existing

# See all options
./bootstrap.sh --help
```

### Environment Variables

You can also configure via environment variables:

```bash
export CLICKHOUSE_HOST=localhost
export CLICKHOUSE_PORT=9000
export CLICKHOUSE_USER=default
export CLICKHOUSE_PASSWORD=your_password

./bootstrap.sh
```

## Schema Overview

### Tables

**`events_local`** - Main events table
- Stores all Nostr events (NIP-01 format)
- Primary sort: `(created_at, kind, pubkey)` - optimized for time-range queries
- Projection: `(kind, created_at, pubkey)` - optimized for kind-specific queries
- Partitioned by month (`YYYYMM`)
- Deduplication via `ReplacingMergeTree` on event `id`

**`event_tags_flat`** - Materialized view for tag queries
- Flattens nested tag arrays for efficient searching
- Primary sort: `(tag_name, tag_value_1, created_at, event_id)`
- Projection: `(tag_value_1, tag_name, created_at, event_id)`
- Automatically populated from `events_local`

### Helper Views

- **`event_stats`** - Event statistics by date and kind
- **`relay_stats`** - Statistics by relay source
- **`tag_stats`** - Tag usage statistics

## Manual Setup (Alternative)

If you prefer to run the SQL directly:

```bash
# Connect to Clickhouse
clickhouse-client

# Run schema
clickhouse-client --multiquery < schema.sql
```

## Verification

Check that everything was created successfully:

```sql
-- Show all tables
SELECT name, engine, total_rows
FROM system.tables
WHERE database = 'nostr';

-- Show partitions
SELECT table, partition, rows, formatReadableSize(bytes_on_disk) as size
FROM system.parts
WHERE database = 'nostr' AND active = 1
ORDER BY table, partition DESC;
```

## Testing with Sample Data

Insert a test event:

```sql
USE nostr;

INSERT INTO events_local (id, pubkey, created_at, kind, content, sig, tags)
VALUES (
    'abc123def456',
    '82341f882b6ea6431e4d92d626a170a16b44a9b4f0cda53eb5236230e1ba3cd3',
    now(),
    1,
    'Hello Nostr from Clickhouse!',
    'sig123456789',
    [['t', 'test'], ['p', '82341f882b6ea6431e4d92d626a170a16b44a9b4']]
);

-- Query it back
SELECT * FROM events_local LIMIT 10;

-- Check tags were flattened
SELECT * FROM event_tags_flat WHERE event_id = 'abc123def456';
```

## Common Queries

### Time-Range Queries

```sql
-- Events from last 24 hours
SELECT id, kind, pubkey, content, created_at
FROM events_local
WHERE created_at >= now() - INTERVAL 1 DAY
ORDER BY created_at DESC
LIMIT 100;

-- Events from October 2025
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
-- Find all events tagged with 'bitcoin'
SELECT event_id, kind, created_at
FROM event_tags_flat
WHERE tag_name = 't' AND tag_value_1 = 'bitcoin'
ORDER BY created_at DESC;

-- Find all replies (e tag with 'reply' marker)
SELECT event_id, tag_value_1 as replied_to_event
FROM event_tags_flat
WHERE tag_name = 'e' AND tag_value_2 = 'reply'
ORDER BY created_at DESC;

-- Find all mentions of a pubkey
SELECT event_id, kind, created_at
FROM event_tags_flat
WHERE tag_name = 'p' AND tag_value_1 = '82341f882b6ea6431e4d92d626a170a16b44a9b4'
ORDER BY created_at DESC;
```

### Author Queries

```sql
-- All events by specific author
SELECT id, kind, content, created_at
FROM events_local
WHERE pubkey = '82341f882b6ea6431e4d92d626a170a16b44a9b4f0cda53eb5236230e1ba3cd3'
ORDER BY created_at DESC;
```

### Statistics

```sql
-- Use helper views for quick stats
SELECT * FROM event_stats ORDER BY date DESC LIMIT 100;
SELECT * FROM relay_stats ORDER BY event_count DESC;
SELECT * FROM tag_stats ORDER BY occurrence_count DESC LIMIT 20;
```

## Maintenance

### Materialize Projections

After bulk data import, materialize projections for optimal performance:

```sql
ALTER TABLE events_local MATERIALIZE PROJECTION events_by_kind;
ALTER TABLE event_tags_flat MATERIALIZE PROJECTION tags_by_value;
```

### Force Deduplication

ReplacingMergeTree deduplicates during background merges. To force immediate deduplication:

```sql
-- Merge specific partition
OPTIMIZE TABLE events_local PARTITION 202510 FINAL;

-- Merge all partitions (expensive!)
OPTIMIZE TABLE events_local FINAL;
```

### Drop Old Partitions

For data lifecycle management:

```sql
-- List all partitions
SELECT partition, rows, formatReadableSize(bytes_on_disk) as size
FROM system.parts
WHERE table = 'events_local' AND database = 'nostr' AND active = 1
ORDER BY partition DESC;

-- Drop partition (e.g., January 2023)
ALTER TABLE events_local DROP PARTITION 202301;
```

### Monitor Query Performance

```sql
-- Recent queries
SELECT
    query,
    query_duration_ms,
    read_rows,
    formatReadableSize(read_bytes) as read_size
FROM system.query_log
WHERE type = 'QueryFinish' AND query LIKE '%events%'
ORDER BY event_time DESC
LIMIT 20;

-- Check which projection was used
SELECT
    query,
    ProjectionNames
FROM system.query_log
WHERE type = 'QueryFinish'
  AND has(ProjectionNames, 'events_by_kind')
ORDER BY event_time DESC
LIMIT 10;
```

## Troubleshooting

### Connection Issues

```bash
# Test connection
clickhouse-client --host localhost --port 9000 --query "SELECT 1"

# Check server status
clickhouse-client --query "SELECT version()"

# Check if server is listening
netstat -an | grep 9000  # Native protocol port
netstat -an | grep 8123  # HTTP interface port
```

### Permission Issues

```sql
-- Check current user
SELECT currentUser();

-- Grant permissions (run as admin)
GRANT ALL ON nostr.* TO default;
```

### Storage Issues

```sql
-- Check disk usage
SELECT
    database,
    table,
    formatReadableSize(sum(bytes_on_disk)) as size
FROM system.parts
WHERE active = 1
GROUP BY database, table
ORDER BY sum(bytes_on_disk) DESC;

-- Check partition sizes
SELECT
    table,
    partition,
    formatReadableSize(sum(bytes_on_disk)) as size,
    sum(rows) as rows
FROM system.parts
WHERE database = 'nostr' AND active = 1
GROUP BY table, partition
ORDER BY table, partition DESC;
```

## Bulk Import Tool

✅ **Available!** Import existing `.pb.gz` files into ClickHouse.

See [IMPORT_README.md](./IMPORT_README.md) for full documentation.

### Quick Start

```bash
# Build the importer
cargo build --release --features clickhouse

# Import data
./target/release/proton-beam-clickhouse-import --input pb_data/*.pb.gz
```

## Next Steps

1. ✅ **Bulk Import Tool** - Available (see IMPORT_README.md)
2. **Daemon Integration** - Modify daemon to write to both `.pb` files and ClickHouse
3. **Query API** - Build REST API for querying events from ClickHouse
4. **Monitoring** - Set up Grafana dashboards for event metrics

## Documentation

- **Schema Details**: `../docs/CLICKHOUSE_SCHEMA.md`
- **Architecture**: `../docs/ARCHITECTURE.md`
- **Project Status**: `../docs/PROJECT_STATUS.md`

## Files

- **`schema.sql`** - Complete Clickhouse schema definition
- **`bootstrap.sh`** - Automated setup script
- **`README.md`** - This file

