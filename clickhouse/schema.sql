-- Proton Beam Clickhouse Schema
-- Version: 1.0
-- Description: Complete schema for storing Nostr events in Clickhouse
--
-- Usage:
--   clickhouse-client --multiquery < schema.sql
--   OR
--   ./bootstrap.sh

-- =============================================================================
-- DATABASE SETUP
-- =============================================================================

-- Create database if it doesn't exist
CREATE DATABASE IF NOT EXISTS nostr;

-- Use the nostr database
USE nostr;

-- =============================================================================
-- MAIN EVENTS TABLE
-- =============================================================================

-- Drop existing tables if rebuilding (CAUTION: This deletes all data!)
-- Uncomment these lines only if you want to start fresh:
-- DROP TABLE IF EXISTS events;
-- DROP TABLE IF EXISTS events_local;
-- DROP TABLE IF EXISTS event_tags;
-- DROP TABLE IF EXISTS event_tags_flat;

-- Main events table (local storage)
-- This stores all Nostr events with optimized indexing for time-range queries
CREATE TABLE IF NOT EXISTS events_local (
    -- Event fields (NIP-01)
    id String COMMENT '32-byte hex event ID (SHA-256 hash)',
    pubkey String COMMENT '32-byte hex public key of event creator',
    created_at DateTime COMMENT 'Unix timestamp when event was created',
    kind UInt16 COMMENT 'Event kind (0-65535, see NIP-01)',
    content String COMMENT 'Event content (arbitrary string, format depends on kind)',
    sig String COMMENT '64-byte hex Schnorr signature',
    tags Array(Array(String)) COMMENT 'Nested array of tags',
    
    -- Metadata fields
    indexed_at DateTime DEFAULT now() COMMENT 'When this event was indexed into Clickhouse',
    relay_source String DEFAULT '' COMMENT 'Source relay URL (e.g., wss://relay.damus.io)',
    
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
    allow_nullable_key = 0
COMMENT 'Main Nostr events table with time-first sort order';

-- Add projection with alternate sort order for kind-first queries
-- This allows Clickhouse to automatically optimize queries that filter by kind
ALTER TABLE events_local ADD PROJECTION IF NOT EXISTS events_by_kind (
    SELECT *
    ORDER BY (kind, created_at, pubkey)
);

-- Note: Materialize projection after data is loaded
-- Uncomment this line after bulk import:
-- ALTER TABLE events_local MATERIALIZE PROJECTION events_by_kind;

-- =============================================================================
-- TAG MATERIALIZED VIEW
-- =============================================================================

-- Flattened tag view for efficient tag queries
-- This view automatically updates when new events are inserted into events_local
CREATE MATERIALIZED VIEW IF NOT EXISTS event_tags_flat
ENGINE = MergeTree()
ORDER BY (tag_name, tag_value_1, created_at, event_id)
PARTITION BY toYYYYMM(created_at)
COMMENT 'Flattened tag view for efficient tag searching'
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

-- Add projection for value-first tag queries
ALTER TABLE event_tags_flat ADD PROJECTION IF NOT EXISTS tags_by_value (
    SELECT *
    ORDER BY (tag_value_1, tag_name, created_at, event_id)
);

-- Note: Materialize projection after data is loaded
-- Uncomment this line after bulk import:
-- ALTER TABLE event_tags_flat MATERIALIZE PROJECTION tags_by_value;

-- =============================================================================
-- DISTRIBUTED TABLES (Optional - for multi-node setups)
-- =============================================================================

-- Uncomment these sections if you're setting up a cluster
-- For single-node setups, you can query events_local directly

-- CREATE TABLE IF NOT EXISTS events AS events_local
-- ENGINE = Distributed(default, nostr, events_local, rand())
-- COMMENT 'Distributed view of events table for multi-node setup';

-- CREATE TABLE IF NOT EXISTS event_tags AS event_tags_flat
-- ENGINE = Distributed(default, nostr, event_tags_flat, rand())
-- COMMENT 'Distributed view of event_tags table for multi-node setup';

-- =============================================================================
-- HELPER VIEWS
-- =============================================================================

-- View for common event statistics
CREATE VIEW IF NOT EXISTS event_stats AS
SELECT
    toStartOfDay(created_at) as date,
    kind,
    count() as event_count,
    uniq(pubkey) as unique_authors,
    avg(length(content)) as avg_content_length,
    sum(length(tags)) as total_tags
FROM events_local
GROUP BY date, kind
ORDER BY date DESC, event_count DESC;

-- View for relay statistics
CREATE VIEW IF NOT EXISTS relay_stats AS
SELECT
    relay_source,
    count() as event_count,
    uniq(id) as unique_events,
    min(created_at) as earliest_event,
    max(created_at) as latest_event,
    uniq(pubkey) as unique_authors
FROM events_local
WHERE relay_source != ''
GROUP BY relay_source
ORDER BY event_count DESC;

-- View for tag statistics
CREATE VIEW IF NOT EXISTS tag_stats AS
SELECT
    tag_name,
    count() as occurrence_count,
    uniq(event_id) as unique_events,
    avg(tag_length) as avg_tag_length
FROM event_tags_flat
GROUP BY tag_name
ORDER BY occurrence_count DESC;

-- =============================================================================
-- VERIFICATION QUERIES
-- =============================================================================

-- Check that tables were created successfully
SELECT 
    database,
    name as table_name,
    engine,
    total_rows,
    total_bytes,
    formatReadableSize(total_bytes) as readable_size
FROM system.tables
WHERE database = 'nostr'
ORDER BY name;

-- Show partition information
SELECT 
    table,
    partition,
    name,
    rows,
    bytes_on_disk,
    formatReadableSize(bytes_on_disk) as readable_size,
    modification_time
FROM system.parts
WHERE database = 'nostr' AND active = 1
ORDER BY table, partition DESC;

-- =============================================================================
-- NOTES
-- =============================================================================

/*
NEXT STEPS:

1. Verify schema creation:
   SELECT * FROM system.tables WHERE database = 'nostr';

2. Test with sample data:
   INSERT INTO events_local (id, pubkey, created_at, kind, content, sig, tags)
   VALUES ('abc123...', 'def456...', now(), 1, 'Hello Nostr!', 'sig123...', [['t', 'test']]);

3. Check data:
   SELECT * FROM events_local LIMIT 10;

4. After bulk import, materialize projections:
   ALTER TABLE events_local MATERIALIZE PROJECTION events_by_kind;
   ALTER TABLE event_tags_flat MATERIALIZE PROJECTION tags_by_value;

5. Monitor query performance:
   SELECT query, query_duration_ms FROM system.query_log 
   WHERE type = 'QueryFinish' ORDER BY event_time DESC LIMIT 20;

MAINTENANCE COMMANDS:

- Force deduplication merge:
  OPTIMIZE TABLE events_local FINAL;

- Drop old partitions (e.g., before 2024):
  ALTER TABLE events_local DROP PARTITION 202312;

- Check projection usage:
  SHOW CREATE TABLE events_local;

- Rebuild projections:
  ALTER TABLE events_local DROP PROJECTION events_by_kind;
  ALTER TABLE events_local ADD PROJECTION events_by_kind (SELECT * ORDER BY (kind, created_at, pubkey));
  ALTER TABLE events_local MATERIALIZE PROJECTION events_by_kind;
*/

