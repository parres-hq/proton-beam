# Clickhouse Schema Design for Proton Beam

**Version:** 1.0
**Last Updated:** 2025-10-16

## Overview

This document describes the Clickhouse schema for storing Nostr events. The schema is optimized for:
- Time-range queries (most common)
- Event kind filtering
- Author (pubkey) queries
- Tag searching with flexible patterns
- Deduplication by event ID
- Scalability to billions of events

## Schema Design Philosophy

### Multi-Sort Strategy (Projections)

We use **projections** to maintain multiple sort orders of the same data, allowing Clickhouse to automatically choose the optimal index at query time:

1. **Primary sort:** `(created_at, kind, pubkey)` - Optimized for time-range queries
2. **Projection sort:** `(kind, created_at, pubkey)` - Optimized for kind-specific queries

This approach provides:
- ✅ Fast time-range queries: `WHERE created_at >= X`
- ✅ Fast kind queries: `WHERE kind = X`
- ✅ Fast combined queries: `WHERE created_at >= X AND kind = Y`
- ✅ No query rewriting needed - Clickhouse chooses automatically

### Partitioning Strategy

**Monthly partitioning** by `created_at`:
```sql
PARTITION BY toYYYYMM(created_at)
```

**Benefits:**
- Aligns with date-based `.pb` file organization
- Easy to drop old partitions for data lifecycle management
- Transparent querying across partitions
- Optimal for time-range queries (partition pruning)

**Partition lifecycle:**
```
202501 (Jan 2025)
202502 (Feb 2025)
...
202510 (Oct 2025) ← Current
```

### Deduplication Strategy

**ReplacingMergeTree** with `indexed_at` as version column:
- Deduplicates based on `PRIMARY KEY (id)`
- Keeps the latest version (highest `indexed_at`)
- Merges happen automatically in background
- For real-time queries, use `FINAL` keyword (slight performance penalty)

**Hybrid approach:**
- Pre-insert deduplication via SQLite index (fast, immediate)
- Post-insert deduplication via ReplacingMergeTree (eventual consistency)

---

## Main Events Table

```sql
-- Main events table with primary sort order optimized for time-range queries
CREATE TABLE events_local (
    -- Event fields (from NIP-01)
    id String,                              -- 32-byte hex event ID (SHA-256)
    pubkey String,                          -- 32-byte hex public key
    created_at DateTime,                    -- Unix timestamp (converted to DateTime)
    kind UInt16,                            -- Event kind (0-65535)
    content String,                         -- Event content (arbitrary string)
    sig String,                             -- 64-byte hex Schnorr signature
    tags Array(Array(String)),              -- Nested array of tags

    -- Metadata fields
    indexed_at DateTime DEFAULT now(),     -- When event was indexed (for deduplication)
    relay_source String,                    -- Source relay (e.g., "wss://relay.damus.io")

    -- Primary key for deduplication
    PRIMARY KEY (id),

    -- Secondary indexes for non-sorted columns
    INDEX idx_kind kind TYPE minmax GRANULARITY 4,
    INDEX idx_pubkey pubkey TYPE bloom_filter(0.01) GRANULARITY 4

) ENGINE = ReplacingMergeTree(indexed_at)
ORDER BY (created_at, kind, pubkey)
PARTITION BY toYYYYMM(created_at)
SETTINGS
    index_granularity = 8192,
    allow_nullable_key = 0;

-- Add projection with alternate sort order for kind-first queries
ALTER TABLE events_local ADD PROJECTION events_by_kind (
    SELECT *
    ORDER BY (kind, created_at, pubkey)
);

-- Materialize the projection
ALTER TABLE events_local MATERIALIZE PROJECTION events_by_kind;
```

### Field Descriptions

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `id` | String | Event ID (32-byte hex) | `"a1b2c3d4..."` |
| `pubkey` | String | Author public key (32-byte hex) | `"82341f882..."` |
| `created_at` | DateTime | Event creation time | `2025-10-16 14:30:00` |
| `kind` | UInt16 | Event kind (0-65535) | `1` (text note) |
| `content` | String | Event content | `"Hello Nostr!"` |
| `sig` | String | Schnorr signature (64-byte hex) | `"3045022100..."` |
| `tags` | Array(Array(String)) | Tags (nested arrays) | `[["e", "abc..."], ["p", "def..."]]` |
| `indexed_at` | DateTime | Indexing timestamp | `2025-10-16 14:35:22` |
| `relay_source` | String | Source relay URL | `"wss://relay.damus.io"` |

### Index Explanations

**Primary Key: `id`**
- Used for deduplication by ReplacingMergeTree
- Enables fast point lookups: `WHERE id = 'abc123...'`

**Secondary Index: `idx_kind`**
- Type: `minmax` (stores min/max values per granule)
- Helps queries like `WHERE kind = 1` when not using primary sort order
- Lightweight (minimal storage overhead)

**Secondary Index: `idx_pubkey`**
- Type: `bloom_filter` (probabilistic filter for string matching)
- Helps queries like `WHERE pubkey = 'abc...'`
- 0.01 false positive rate, 4-granule blocks

---

## Tag Materialized View

Tags are the most complex part of Nostr events. This materialized view flattens tags for efficient querying:

```sql
-- Flattened tag view for flexible tag queries
CREATE MATERIALIZED VIEW event_tags_flat
ENGINE = MergeTree()
ORDER BY (tag_name, tag_value_1, created_at, event_id)
PARTITION BY toYYYYMM(created_at)
POPULATE  -- Backfill existing events
AS SELECT
    id as event_id,
    pubkey,
    created_at,
    kind,
    arrayJoin(tags) as tag_array,                                           -- Flatten tags array
    tag_array[1] as tag_name,                                              -- Tag type: 'e', 'p', 'a', 't', etc.
    if(length(tag_array) >= 2, tag_array[2], '') as tag_value_1,          -- First value
    if(length(tag_array) >= 3, tag_array[3], '') as tag_value_2,          -- Second value (relay hints, markers)
    if(length(tag_array) >= 4, tag_array[4], '') as tag_value_3,          -- Third value
    if(length(tag_array) >= 5, tag_array[5], '') as tag_value_4,          -- Fourth value
    length(tag_array) as tag_length,                                       -- Number of elements in tag
    tag_array as tag_full                                                  -- Keep full tag for reference
FROM events_local;

-- Add projection for tag value queries
ALTER TABLE event_tags_flat ADD PROJECTION tags_by_value (
    SELECT *
    ORDER BY (tag_value_1, tag_name, created_at, event_id)
);

-- Materialize the projection
ALTER TABLE event_tags_flat MATERIALIZE PROJECTION tags_by_value;
```

### Tag Query Examples

**Find all references to a specific event:**
```sql
SELECT event_id, kind, created_at
FROM event_tags_flat
WHERE tag_name = 'e' AND tag_value_1 = 'abc123...'
ORDER BY created_at DESC
LIMIT 100;
```

**Find all replies (e tag with 'reply' marker):**
```sql
SELECT event_id, pubkey, created_at
FROM event_tags_flat
WHERE tag_name = 'e' AND tag_value_2 = 'reply'
ORDER BY created_at DESC;
```

**Find all posts tagged with #bitcoin:**
```sql
SELECT event_id, content, created_at
FROM event_tags_flat t
JOIN events_local e ON t.event_id = e.id
WHERE tag_name = 't' AND tag_value_1 = 'bitcoin'
ORDER BY created_at DESC;
```

**Find all mentions of a pubkey:**
```sql
SELECT event_id, kind, created_at
FROM event_tags_flat
WHERE tag_name = 'p' AND tag_value_1 = '82341f882...'
ORDER BY created_at DESC;
```

**Find tags with specific relay hints:**
```sql
SELECT event_id, tag_name, tag_value_1
FROM event_tags_flat
WHERE tag_value_2 = 'wss://relay.damus.io'
ORDER BY created_at DESC;
```

---

## Distributed Table Setup (Future-Proofing)

Even with a single node, we set up distributed tables for easy horizontal scaling:

```sql
-- Distributed table (query interface)
-- Note: 'cluster' should be defined in Clickhouse config.xml
-- For single node, use 'default' cluster or skip this step

CREATE TABLE events AS events_local
ENGINE = Distributed(default, default, events_local, rand());

CREATE TABLE event_tags AS event_tags_flat
ENGINE = Distributed(default, default, event_tags_flat, rand());
```

**Benefits:**
- Query `events` instead of `events_local` in application code
- Add shards later without code changes
- Automatic query distribution across nodes
- `rand()` sharding key for balanced distribution

**Scaling path:**
1. **Single node:** Use `events_local` directly or distributed table with 1 shard
2. **Horizontal scaling:** Add nodes to cluster config, Clickhouse handles routing
3. **Rebalancing:** Use `clickhouse-copier` to redistribute data

---

## Query Performance Optimization

### Partition Pruning

Clickhouse automatically skips partitions based on `WHERE` clauses:

```sql
-- Only scans Oct 2025 partition
SELECT * FROM events
WHERE created_at >= '2025-10-01' AND created_at < '2025-11-01';

-- Scans all partitions (no time filter)
SELECT * FROM events WHERE kind = 1;
```

**Best practice:** Always include time bounds when possible.

### Projection Selection

Clickhouse automatically chooses the best projection:

```sql
-- Uses primary sort: (created_at, kind, pubkey)
SELECT * FROM events
WHERE created_at >= '2025-10-01';

-- Uses projection: (kind, created_at, pubkey)
SELECT * FROM events
WHERE kind = 1;

-- Uses projection: (kind, created_at, pubkey)
SELECT * FROM events
WHERE kind = 1 AND created_at >= '2025-10-01';
```

**No query changes needed** - optimization is automatic!

### Deduplication Queries

For real-time queries requiring immediate deduplication:

```sql
-- With deduplication (slight performance penalty)
SELECT * FROM events FINAL
WHERE created_at >= '2025-10-16';

-- Without deduplication (faster, may include duplicates)
SELECT * FROM events
WHERE created_at >= '2025-10-16';
```

**Recommendation:** Use `FINAL` only when necessary (analytics, exports). For real-time feeds, duplicates are merged eventually.

---

## Common Query Patterns

### Time-Range Queries

```sql
-- Last 24 hours
SELECT id, kind, pubkey, created_at, content
FROM events
WHERE created_at >= now() - INTERVAL 1 DAY
ORDER BY created_at DESC
LIMIT 100;

-- Specific date range
SELECT kind, count() as event_count
FROM events
WHERE created_at BETWEEN '2025-10-01' AND '2025-10-31'
GROUP BY kind
ORDER BY event_count DESC;
```

### Kind-Specific Queries

```sql
-- All text notes (kind 1) from last week
SELECT id, pubkey, content, created_at
FROM events
WHERE kind = 1
  AND created_at >= now() - INTERVAL 7 DAY
ORDER BY created_at DESC;

-- Event kind distribution
SELECT kind, count() as total
FROM events
GROUP BY kind
ORDER BY total DESC
LIMIT 20;
```

### Author Queries

```sql
-- All events by specific author
SELECT id, kind, created_at, content
FROM events
WHERE pubkey = '82341f882b6ea6431e4d92d626a170a16b44a9b4f0cda53eb5236230e1ba3cd3'
ORDER BY created_at DESC;

-- Author activity by kind
SELECT kind, count() as post_count
FROM events
WHERE pubkey = '82341f882b6ea6431e4d92d626a170a16b44a9b4f0cda53eb5236230e1ba3cd3'
GROUP BY kind
ORDER BY post_count DESC;
```

### Combined Filters

```sql
-- Text notes by author in last 30 days
SELECT id, content, created_at
FROM events
WHERE pubkey = '82341f882b6ea6431e4d92d626a170a16b44a9b4f0cda53eb5236230e1ba3cd3'
  AND kind = 1
  AND created_at >= now() - INTERVAL 30 DAY
ORDER BY created_at DESC;
```

---

## Storage Estimates

### Events Table

**Per event storage (compressed):**
- Event fields: ~100-150 bytes (with Clickhouse LZ4 compression)
- Tags: ~50-100 bytes (varies by tag count)
- Indexes: ~1-2 bytes per event
- Projection: 2x storage (duplicate with different sort order)

**Total:** ~300-500 bytes per event (including projection)

**Scaling estimates:**
- 1 million events: ~300-500 MB
- 10 million events: ~3-5 GB
- 100 million events: ~30-50 GB
- 1 billion events: ~300-500 GB
- 10 billion events: ~3-5 TB

### Tag Materialized View

**Per tag storage:**
- Flattened tag row: ~50-100 bytes
- Average 3-5 tags per event
- Projection: 2x storage

**Total:** ~300-1000 bytes per event (for all tags)

**Combined storage (events + tags):**
- 1 billion events: ~800GB - 1.5TB
- 10 billion events: ~8-15TB

### Hardware Recommendations

**For 1 billion events (single node):**
- **CPU:** 16+ cores
- **RAM:** 128GB+ (indexes mostly fit in memory)
- **Disk:** 2TB NVMe SSD (with compression)
- **Network:** 10Gbps (for replication/backups)

**For 10 billion events:**
- Consider 2-4 node cluster
- Each node: 256GB RAM, 10TB SSD
- Replicate across nodes for redundancy

---

## Maintenance Operations

### Manual Merge (Force Deduplication)

```sql
-- Force merge of specific partition
OPTIMIZE TABLE events_local PARTITION 202510 FINAL;

-- Force merge of all partitions (expensive!)
OPTIMIZE TABLE events_local FINAL;
```

### Partition Management

```sql
-- List all partitions
SELECT
    partition,
    name,
    rows,
    bytes_on_disk,
    formatReadableSize(bytes_on_disk) as readable_size
FROM system.parts
WHERE table = 'events_local' AND active
ORDER BY partition DESC;

-- Drop old partition (e.g., older than 2 years)
ALTER TABLE events_local DROP PARTITION 202301;
```

### Index Statistics

```sql
-- Check projection usage
SELECT
    query,
    query_duration_ms,
    read_rows,
    read_bytes
FROM system.query_log
WHERE type = 'QueryFinish'
  AND query LIKE '%events%'
ORDER BY event_time DESC
LIMIT 100;
```

---

## Migration and Setup Script

```sql
-- Run this script to set up the complete schema

-- 1. Create main events table
CREATE TABLE IF NOT EXISTS events_local (
    id String,
    pubkey String,
    created_at DateTime,
    kind UInt16,
    content String,
    sig String,
    tags Array(Array(String)),
    indexed_at DateTime DEFAULT now(),
    relay_source String,
    PRIMARY KEY (id),
    INDEX idx_kind kind TYPE minmax GRANULARITY 4,
    INDEX idx_pubkey pubkey TYPE bloom_filter(0.01) GRANULARITY 4
) ENGINE = ReplacingMergeTree(indexed_at)
ORDER BY (created_at, kind, pubkey)
PARTITION BY toYYYYMM(created_at)
SETTINGS index_granularity = 8192;

-- 2. Add kind-first projection
ALTER TABLE events_local ADD PROJECTION IF NOT EXISTS events_by_kind (
    SELECT *
    ORDER BY (kind, created_at, pubkey)
);

-- 3. Materialize projection (if data already exists)
-- ALTER TABLE events_local MATERIALIZE PROJECTION events_by_kind;

-- 4. Create tag materialized view
CREATE MATERIALIZED VIEW IF NOT EXISTS event_tags_flat
ENGINE = MergeTree()
ORDER BY (tag_name, tag_value_1, created_at, event_id)
PARTITION BY toYYYYMM(created_at)
AS SELECT
    id as event_id,
    pubkey,
    created_at,
    kind,
    arrayJoin(tags) as tag_array,
    tag_array[1] as tag_name,
    if(length(tag_array) >= 2, tag_array[2], '') as tag_value_1,
    if(length(tag_array) >= 3, tag_array[3], '') as tag_value_2,
    if(length(tag_array) >= 4, tag_array[4], '') as tag_value_3,
    if(length(tag_array) >= 5, tag_array[5], '') as tag_value_4,
    length(tag_array) as tag_length,
    tag_array as tag_full
FROM events_local;

-- 5. Add value-first projection to tag view
ALTER TABLE event_tags_flat ADD PROJECTION IF NOT EXISTS tags_by_value (
    SELECT *
    ORDER BY (tag_value_1, tag_name, created_at, event_id)
);

-- 6. Materialize tag projection (if data already exists)
-- ALTER TABLE event_tags_flat MATERIALIZE PROJECTION tags_by_value;

-- 7. Optional: Create distributed tables (for multi-node setup)
-- CREATE TABLE IF NOT EXISTS events AS events_local
-- ENGINE = Distributed(default, default, events_local, rand());

-- CREATE TABLE IF NOT EXISTS event_tags AS event_tags_flat
-- ENGINE = Distributed(default, default, event_tags_flat, rand());
```

---

## Backup and Recovery

### Backup Strategy

```bash
# Backup specific partition
clickhouse-backup create partition_202510

# Backup full database
clickhouse-backup create full_backup
```

### Point-in-Time Recovery

```sql
-- Export events to JSON (for disaster recovery)
SELECT * FROM events
WHERE created_at >= '2025-10-01'
FORMAT JSONEachRow
INTO OUTFILE '/backups/events_2025_10.jsonl';
```

---

## Configuration Recommendations

### clickhouse-server config.xml

```xml
<clickhouse>
    <!-- Increase max memory for large batch inserts -->
    <max_memory_usage>100000000000</max_memory_usage>

    <!-- Background merge settings -->
    <background_pool_size>16</background_pool_size>
    <background_schedule_pool_size>16</background_schedule_pool_size>

    <!-- Compression -->
    <compression>
        <case>
            <method>lz4</method>
        </case>
    </compression>

    <!-- Keep more parts for better deduplication -->
    <merge_tree>
        <max_parts_in_total>10000</max_parts_in_total>
    </merge_tree>
</clickhouse>
```

---

## Next Steps

1. **Set up Clickhouse server** (single node to start)
2. **Run migration script** to create schema
3. **Test with sample data** (use existing `.pb` files)
4. **Build bulk import tool** (`proton-beam clickhouse import`)
5. **Integrate with daemon** for real-time dual writes
6. **Monitor performance** and adjust settings

---

**Document Status:** Complete
**Last Updated:** 2025-10-16

