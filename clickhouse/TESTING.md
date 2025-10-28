# Testing the ClickHouse Bulk Importer

This guide walks you through testing the ClickHouse importer locally with a small dataset.

## Step 1: Install ClickHouse

### macOS
```bash
brew install clickhouse
brew services start clickhouse
```

### Ubuntu/Debian
```bash
sudo apt-get install clickhouse-server clickhouse-client
sudo service clickhouse-server start
```

### Verify Installation
```bash
clickhouse-client --query "SELECT version()"
```

You should see a version number (e.g., `24.10.1.2812`).

## Step 2: Initialize the Schema

Run the bootstrap script to create the database and tables:

```bash
cd clickhouse
./bootstrap.sh
```

This creates:
- Database: `nostr`
- Table: `events_local` (main events table)
- Materialized view: `event_tags_flat` (for tag queries)
- Helper views: `event_stats`, `relay_stats`, `tag_stats`

## Step 3: Build the Importer

```bash
cd ..
cargo build --release --features clickhouse
```

The binary will be at: `target/release/proton-beam-clickhouse-import`

## Step 4: Test with Sample Data

The repository includes a sample file: `pb_data/2025_09_27.pb.gz`

### Dry Run (No Insert)

First, test that the file can be read without inserting:

```bash
./target/release/proton-beam-clickhouse-import \
  --input pb_data/2025_09_27.pb.gz \
  --dry-run
```

Expected output:
```
Starting ClickHouse bulk import
Configuration:
  Host: localhost:8123
  Database: nostr
  Table: events_local
  Batch size: 5000
  Input files: 1

Dry run mode - skipping ClickHouse connection
Processing file: pb_data/2025_09_27.pb.gz
✓ Processed XXX events from pb_data/2025_09_27.pb.gz

Import complete!
  Total events: XXX
  Total time: X.XXs
  Speed: XXXX events/sec
```

### Actual Import

Now import the data into ClickHouse:

```bash
./target/release/proton-beam-clickhouse-import \
  --input pb_data/2025_09_27.pb.gz
```

Expected output:
```
Starting ClickHouse bulk import
Configuration:
  Host: localhost:8123
  Database: nostr
  Table: events_local
  Batch size: 5000
  Input files: 1

Testing ClickHouse connection...
✓ Connection successful
Verifying database schema...
✓ Schema verified
Current event count: 0
Processing file: pb_data/2025_09_27.pb.gz
✓ Processed XXX events from pb_data/2025_09_27.pb.gz

Import complete!
  Total events: XXX
  Total time: X.XXs
  Speed: XXXX events/sec
  Final event count: XXX
```

## Step 5: Verify the Import

### Check Event Count

```bash
clickhouse-client --query="SELECT count() FROM nostr.events_local"
```

### View Sample Events

```bash
clickhouse-client --query="
  SELECT id, kind, pubkey, created_at, content
  FROM nostr.events_local
  LIMIT 5
  FORMAT Vertical
"
```

### Check Events by Kind

```bash
clickhouse-client --query="
  SELECT kind, count() as count
  FROM nostr.events_local
  GROUP BY kind
  ORDER BY count DESC
"
```

### Check Tag Statistics

```bash
clickhouse-client --query="
  SELECT tag_name, count() as count
  FROM nostr.event_tags_flat
  GROUP BY tag_name
  ORDER BY count DESC
  LIMIT 10
"
```

## Step 6: Test Queries

Try some example queries from the schema documentation:

### Time-Range Query
```bash
clickhouse-client --query="
  SELECT id, kind, pubkey, toDateTime(created_at) as time
  FROM nostr.events_local
  WHERE created_at >= now() - INTERVAL 7 DAY
  ORDER BY created_at DESC
  LIMIT 10
"
```

### Kind-Specific Query
```bash
clickhouse-client --query="
  SELECT count() as total
  FROM nostr.events_local
  WHERE kind = 1  -- Text notes
"
```

### Tag Query
```bash
clickhouse-client --query="
  SELECT event_id, kind, created_at
  FROM nostr.event_tags_flat
  WHERE tag_name = 'p'  -- Pubkey mentions
  LIMIT 10
"
```

## Benchmarking

To test performance with the sample file:

### Import Speed
```bash
time ./target/release/proton-beam-clickhouse-import \
  --input pb_data/2025_09_27.pb.gz
```

### Query Performance

Enable query timing in clickhouse-client:
```bash
clickhouse-client --time --query="
  SELECT count() FROM nostr.events_local WHERE kind = 1
"
```

The `--time` flag shows query execution time.

## Troubleshooting

### "Connection refused"

**Problem:** Can't connect to ClickHouse

**Solution:**
```bash
# Check if ClickHouse is running
brew services list | grep clickhouse  # macOS
sudo service clickhouse-server status  # Linux

# Try to connect
clickhouse-client --query "SELECT 1"
```

### "Database 'nostr' does not exist"

**Problem:** Schema not initialized

**Solution:**
```bash
cd clickhouse && ./bootstrap.sh
```

### "Permission denied"

**Problem:** Binary not executable or file permissions

**Solution:**
```bash
chmod +x target/release/proton-beam-clickhouse-import
```

## Next Steps

Once local testing is successful:

1. **Test with multiple files**: Import all `.pb.gz` files in `pb_data/`
2. **Monitor performance**: Watch import speed and query performance
3. **Test on server**: Deploy to a production server (see IMPORT_README.md)
4. **Set up monitoring**: Use ClickHouse system tables to monitor performance

## Performance Expectations

For reference, on a typical development machine:

- **Import speed**: 10,000-50,000 events/sec (depends on event size)
- **Query speed**: Sub-second for most queries on millions of events
- **Disk usage**: ~30-50% of original `.pb.gz` size (ClickHouse compression)

## Clean Up

To reset and start over:

```bash
# Drop all data
clickhouse-client --query="DROP DATABASE IF EXISTS nostr"

# Reinitialize
cd clickhouse && ./bootstrap.sh
```

## Getting Help

If you encounter issues:

1. Check the logs: `pb_data/proton-beam.log`
2. Enable verbose mode: `--verbose`
3. Try dry-run first: `--dry-run`
4. Check ClickHouse logs: `/var/log/clickhouse-server/clickhouse-server.log`



