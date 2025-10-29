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
    id FixedString(64) COMMENT '32-byte hex event ID (SHA-256 hash)',
    pubkey FixedString(64) COMMENT '32-byte hex public key of event creator',
    created_at DateTime COMMENT 'Unix timestamp when event was created',
    kind UInt16 COMMENT 'Event kind (0-65535, see NIP-01)',
    content String CODEC(ZSTD(3)) COMMENT 'Event content (arbitrary string, format depends on kind)',
    sig FixedString(128) COMMENT '64-byte hex Schnorr signature',
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

-- Optimized tag view based on Nostr usage patterns:
-- - Most queries filter on tag_name (position 1) and primary value (position 2)
-- - Positions 3+ contain relay hints, markers, and metadata (rarely queried)
-- - This creates 1 row per tag (not per value) for storage efficiency
-- - Supports marker-only tags like ["-"] (NIP-70)
CREATE MATERIALIZED VIEW IF NOT EXISTS event_tags_flat
ENGINE = MergeTree()
ORDER BY (tag_name, tag_value_primary, created_at, event_id)
PARTITION BY toYYYYMM(created_at)
SETTINGS index_granularity = 8192
COMMENT 'Flattened tag view optimized for tag_name + primary_value queries (positions 1 & 2)'
AS SELECT
    id as event_id,
    pubkey,
    created_at,
    kind,
    arrayJoin(tags) as tag_array,
    -- Position 1: Tag type/name (e.g., "e", "p", "t", "-")
    tag_array[1] as tag_name,
    -- Position 2: Primary value (event ID, pubkey, hashtag, etc.)
    -- Empty string for marker-only tags like ["-"]
    if(length(tag_array) >= 2, tag_array[2], '') as tag_value_primary,
    -- Position 3: Usually relay hints, dimensions, or other metadata
    if(length(tag_array) >= 3, tag_array[3], '') as tag_value_position_3,
    -- Position 4: Usually markers like "root", "reply", "mention" or additional hints
    if(length(tag_array) >= 4, tag_array[4], '') as tag_value_position_4,
    -- Position 5+: Rare additional metadata
    if(length(tag_array) >= 5, tag_array[5], '') as tag_value_position_5,
    if(length(tag_array) >= 6, tag_array[6], '') as tag_value_position_6,
    -- Store all values as array for rare multi-position queries
    arraySlice(tag_array, 2) as tag_values_all,
    -- Total number of values (excluding tag name)
    length(tag_array) - 1 as tag_value_count,
    -- Full original tag array for reference
    tag_array as tag_full
FROM events_local
WHERE length(tag_array) >= 1;  -- Include all valid tags, even marker-only ones

-- Secondary index for kind-based tag filtering
-- Example query: SELECT * FROM event_tags_flat WHERE kind = 1 AND tag_name = 'p'
-- The minmax index helps skip granules where kind is outside the query range
ALTER TABLE event_tags_flat ADD INDEX IF NOT EXISTS idx_kind kind TYPE minmax GRANULARITY 4;

-- Add projection for value-first queries (less common but useful)
-- Example query: SELECT * FROM event_tags_flat WHERE tag_value_primary = 'some_pubkey' ORDER BY created_at DESC
-- This projection optimizes lookups when you know the tag value but want to find all tag types referencing it
ALTER TABLE event_tags_flat ADD PROJECTION IF NOT EXISTS tags_by_value (
    SELECT *
    ORDER BY (tag_value_primary, tag_name, created_at, event_id)
);

-- Add projection for event-first queries (getting all tags for a specific event)
-- Example query: SELECT * FROM event_tags_flat WHERE event_id = 'some_event_id'
-- This projection optimizes retrieving all tags associated with a specific event
ALTER TABLE event_tags_flat ADD PROJECTION IF NOT EXISTS tags_by_event (
    SELECT *
    ORDER BY (event_id, tag_name, created_at)
);

-- Note: Materialize projections after data is loaded
-- Uncomment these lines after bulk import:
-- ALTER TABLE event_tags_flat MATERIALIZE PROJECTION tags_by_value;
-- ALTER TABLE event_tags_flat MATERIALIZE PROJECTION tags_by_event;

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

-- =============================================================================
-- EXAMPLES OF ANALYTICAL VIEWS
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
    uniq(event_id) as unique_events
FROM event_tags_flat
GROUP BY tag_name
ORDER BY occurrence_count DESC;

-- View: Daily Active Users (users who published events each day)
-- Example query: SELECT * FROM daily_active_users WHERE date >= today() - 30 ORDER BY date DESC
CREATE VIEW IF NOT EXISTS daily_active_users AS
SELECT
    toDate(created_at) AS date,
    uniq(pubkey) AS active_users,
    count() AS total_events
FROM events_local
GROUP BY date
ORDER BY date DESC;

-- View: Weekly Active Users (aggregated by week starting Monday)
-- Example query: SELECT * FROM weekly_active_users WHERE week >= toMonday(today()) - 90 ORDER BY week DESC
CREATE VIEW IF NOT EXISTS weekly_active_users AS
SELECT
    toMonday(created_at) AS week,
    uniq(pubkey) AS active_users,
    count() AS total_events
FROM events_local
GROUP BY week
ORDER BY week DESC;

-- View: Monthly Active Users (aggregated by month)
-- Example query: SELECT * FROM monthly_active_users WHERE month >= toStartOfMonth(today()) - 180 ORDER BY month DESC
CREATE VIEW IF NOT EXISTS monthly_active_users AS
SELECT
    toStartOfMonth(created_at) AS month,
    uniq(pubkey) AS active_users,
    count() AS total_events
FROM events_local
GROUP BY month
ORDER BY month DESC;

-- View: Users with Kind 0 metadata (profile information)
-- Example query: SELECT * FROM users_with_metadata ORDER BY last_updated DESC LIMIT 100
CREATE VIEW IF NOT EXISTS users_with_metadata AS
SELECT
    pubkey,
    argMax(content, created_at) AS latest_metadata,
    max(created_at) AS last_updated,
    count() AS metadata_updates
FROM events_local
WHERE kind = 0
GROUP BY pubkey;

-- View: Event activity by users with metadata
-- Shows publishing activity only for users who have published profile metadata
-- Example query: SELECT * FROM verified_user_activity WHERE date >= today() - 7 ORDER BY total_events DESC
CREATE VIEW IF NOT EXISTS verified_user_activity AS
SELECT
    e.pubkey,
    toDate(e.created_at) AS date,
    count() AS total_events,
    uniqExact(e.kind) AS unique_kinds,
    groupArray(e.kind) AS kinds_used
FROM events_local e
WHERE e.pubkey IN (
    SELECT DISTINCT pubkey
    FROM events_local
    WHERE kind = 0
)
GROUP BY e.pubkey, date
ORDER BY date DESC, total_events DESC;

-- View: Kind 0 metadata summary statistics
-- Example query: SELECT * FROM metadata_stats
CREATE VIEW IF NOT EXISTS metadata_stats AS
SELECT
    count(DISTINCT pubkey) AS total_users_with_metadata,
    count() AS total_metadata_events,
    avg(metadata_updates) AS avg_updates_per_user,
    quantile(0.5)(metadata_updates) AS median_updates_per_user,
    max(metadata_updates) AS max_updates_per_user
FROM (
    SELECT
        pubkey,
        count() AS metadata_updates
    FROM events_local
    WHERE kind = 0
    GROUP BY pubkey
);

-- View: Activity breakdown by kind
-- Shows daily event counts grouped by event kind
-- Example query: SELECT * FROM activity_by_kind WHERE date >= today() - 7 ORDER BY date DESC, events DESC
CREATE VIEW IF NOT EXISTS activity_by_kind AS
SELECT
    toDate(created_at) AS date,
    kind,
    count() AS events,
    uniq(pubkey) AS unique_publishers
FROM events_local
GROUP BY date, kind
ORDER BY date DESC, events DESC;

-- View: Top publishers (overall)
-- Example query: SELECT * FROM top_publishers LIMIT 100
CREATE VIEW IF NOT EXISTS top_publishers AS
SELECT
    pubkey,
    count() AS total_events,
    uniqExact(kind) AS unique_kinds,
    min(created_at) AS first_event,
    max(created_at) AS last_event,
    dateDiff('day', min(created_at), max(created_at)) AS days_active
FROM events_local
GROUP BY pubkey
ORDER BY total_events DESC;

-- View: Top publishers with metadata
-- Only includes users who have published kind 0 metadata
-- Example query: SELECT * FROM top_verified_publishers LIMIT 100
CREATE VIEW IF NOT EXISTS top_verified_publishers AS
SELECT
    e.pubkey,
    count() AS total_events,
    uniqExact(e.kind) AS unique_kinds,
    min(e.created_at) AS first_event,
    max(e.created_at) AS last_event,
    dateDiff('day', min(e.created_at), max(e.created_at)) AS days_active,
    any(m.latest_metadata) AS metadata
FROM events_local e
INNER JOIN users_with_metadata m ON e.pubkey = m.pubkey
GROUP BY e.pubkey
ORDER BY total_events DESC;

-- =============================================================================
-- MAINTENANCE COMMANDS
-- =============================================================================

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
