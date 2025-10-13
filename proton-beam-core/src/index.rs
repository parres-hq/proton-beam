//! SQLite index for event deduplication and fast lookups
//!
//! This module provides a SQLite-based index for Nostr events, enabling:
//! - Fast event deduplication by ID
//! - Queries by kind, pubkey, or date range
//! - Event ID to file path mapping
//!
//! # Examples
//!
//! ```no_run
//! use proton_beam_core::{EventIndex, ProtoEvent};
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create or open an index
//! let mut index = EventIndex::new(Path::new("./pb_data/.index.db"))?;
//!
//! // Check if an event already exists
//! if index.contains("event_id_123")? {
//!     println!("Event already indexed");
//! }
//!
//! // Insert an event
//! let event = ProtoEvent {
//!     id: "event_id_123".to_string(),
//!     kind: 1,
//!     pubkey: "pubkey_abc".to_string(),
//!     created_at: 1234567890,
//!     content: "Hello".to_string(),
//!     tags: vec![],
//!     sig: "sig_xyz".to_string(),
//! };
//! index.insert(&event, "2025_10_13.pb")?;
//!
//! // Get statistics
//! let stats = index.stats()?;
//! println!("Total events: {}", stats.total_events);
//! # Ok(())
//! # }
//! ```

use crate::{Error, ProtoEvent, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

/// SQLite-based event index for deduplication and queries
pub struct EventIndex {
    conn: Connection,
}

/// Record returned from index queries
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRecord {
    /// Event ID (hex-encoded)
    pub id: String,
    /// Event kind
    pub kind: i32,
    /// Public key (hex-encoded)
    pub pubkey: String,
    /// Unix timestamp
    pub created_at: i64,
    /// Path to the .pb file containing this event
    pub file_path: String,
    /// Unix timestamp when the event was indexed
    pub indexed_at: i64,
}

/// Statistics about the event index
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexStats {
    /// Total number of indexed events
    pub total_events: u64,
    /// Number of unique file paths
    pub unique_files: u64,
    /// Number of unique pubkeys
    pub unique_pubkeys: u64,
    /// Earliest event timestamp
    pub earliest_event: Option<i64>,
    /// Latest event timestamp
    pub latest_event: Option<i64>,
}

impl EventIndex {
    /// Create or open an event index at the specified path
    ///
    /// If the database doesn't exist, it will be created with the proper schema.
    /// If it exists, it will be opened and the schema will be verified.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the SQLite database file
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use proton_beam_core::EventIndex;
    /// use std::path::Path;
    ///
    /// let index = EventIndex::new(Path::new("./pb_data/.index.db"))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path).map_err(|e| {
            Error::InvalidEvent(format!("Failed to open database at {:?}: {}", db_path, e))
        })?;

        // Create schema if needed
        Self::create_schema(&conn)?;

        Ok(Self { conn })
    }

    /// Create the database schema
    fn create_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                kind INTEGER NOT NULL,
                pubkey TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                indexed_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_kind ON events(kind);
            CREATE INDEX IF NOT EXISTS idx_pubkey ON events(pubkey);
            CREATE INDEX IF NOT EXISTS idx_created_at ON events(created_at);
            CREATE INDEX IF NOT EXISTS idx_file_path ON events(file_path);
            "#,
        )
        .map_err(|e| Error::InvalidEvent(format!("Failed to create schema: {}", e)))?;

        Ok(())
    }

    /// Check if an event ID exists in the index
    ///
    /// # Arguments
    ///
    /// * `event_id` - Event ID to check (hex-encoded)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use proton_beam_core::EventIndex;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let index = EventIndex::new(Path::new("./pb_data/.index.db"))?;
    /// if index.contains("event_id_123")? {
    ///     println!("Event already exists");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn contains(&self, event_id: &str) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT 1 FROM events WHERE id = ?")
            .map_err(|e| Error::InvalidEvent(format!("Failed to prepare query: {}", e)))?;

        let exists = stmt
            .query_row(params![event_id], |_| Ok(()))
            .optional()
            .map_err(|e| Error::InvalidEvent(format!("Failed to check existence: {}", e)))?
            .is_some();

        Ok(exists)
    }

    /// Insert a single event into the index
    ///
    /// # Arguments
    ///
    /// * `event` - Event to index
    /// * `file_path` - Path to the .pb file containing this event (relative or absolute)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use proton_beam_core::{EventIndex, ProtoEvent};
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut index = EventIndex::new(Path::new("./pb_data/.index.db"))?;
    /// # let event = ProtoEvent {
    /// #     id: "event_id_123".to_string(),
    /// #     kind: 1,
    /// #     pubkey: "pubkey_abc".to_string(),
    /// #     created_at: 1234567890,
    /// #     content: "Hello".to_string(),
    /// #     tags: vec![],
    /// #     sig: "sig_xyz".to_string(),
    /// # };
    /// index.insert(&event, "2025_10_13.pb")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn insert(&mut self, event: &ProtoEvent, file_path: &str) -> Result<()> {
        let indexed_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.conn
            .execute(
                "INSERT OR IGNORE INTO events (id, kind, pubkey, created_at, file_path, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    &event.id,
                    event.kind,
                    &event.pubkey,
                    event.created_at,
                    file_path,
                    indexed_at
                ],
            )
            .map_err(|e| Error::InvalidEvent(format!("Failed to insert event: {}", e)))?;

        Ok(())
    }

    /// Insert multiple events into the index in a single transaction
    ///
    /// This is significantly faster than inserting events one at a time.
    ///
    /// # Arguments
    ///
    /// * `events` - Slice of (event, file_path) tuples to index
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use proton_beam_core::{EventIndex, ProtoEvent};
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut index = EventIndex::new(Path::new("./pb_data/.index.db"))?;
    /// # let event1 = ProtoEvent {
    /// #     id: "event1".to_string(),
    /// #     kind: 1,
    /// #     pubkey: "pubkey1".to_string(),
    /// #     created_at: 1234567890,
    /// #     content: "Hello".to_string(),
    /// #     tags: vec![],
    /// #     sig: "sig1".to_string(),
    /// # };
    /// # let event2 = ProtoEvent {
    /// #     id: "event2".to_string(),
    /// #     kind: 1,
    /// #     pubkey: "pubkey2".to_string(),
    /// #     created_at: 1234567891,
    /// #     content: "World".to_string(),
    /// #     tags: vec![],
    /// #     sig: "sig2".to_string(),
    /// # };
    /// let events = vec![
    ///     (&event1, "2025_10_13.pb"),
    ///     (&event2, "2025_10_13.pb"),
    /// ];
    /// index.insert_batch(&events)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn insert_batch(&mut self, events: &[(&ProtoEvent, &str)]) -> Result<()> {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| Error::InvalidEvent(format!("Failed to start transaction: {}", e)))?;

        let indexed_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR IGNORE INTO events (id, kind, pubkey, created_at, file_path, indexed_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                )
                .map_err(|e| Error::InvalidEvent(format!("Failed to prepare insert: {}", e)))?;

            for (event, file_path) in events {
                stmt.execute(params![
                    &event.id,
                    event.kind,
                    &event.pubkey,
                    event.created_at,
                    file_path,
                    indexed_at
                ])
                .map_err(|e| {
                    Error::InvalidEvent(format!("Failed to insert event in batch: {}", e))
                })?;
            }
        }

        tx.commit()
            .map_err(|e| Error::InvalidEvent(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Get statistics about the index
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use proton_beam_core::EventIndex;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let index = EventIndex::new(Path::new("./pb_data/.index.db"))?;
    /// let stats = index.stats()?;
    /// println!("Total events: {}", stats.total_events);
    /// println!("Unique files: {}", stats.unique_files);
    /// println!("Unique pubkeys: {}", stats.unique_pubkeys);
    /// # Ok(())
    /// # }
    /// ```
    pub fn stats(&self) -> Result<IndexStats> {
        let total_events: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .map_err(|e| Error::InvalidEvent(format!("Failed to query total events: {}", e)))?;

        let unique_files: u64 = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT file_path) FROM events",
                [],
                |row| row.get(0),
            )
            .map_err(|e| Error::InvalidEvent(format!("Failed to query unique files: {}", e)))?;

        let unique_pubkeys: u64 = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT pubkey) FROM events",
                [],
                |row| row.get(0),
            )
            .map_err(|e| Error::InvalidEvent(format!("Failed to query unique pubkeys: {}", e)))?;

        let earliest_event: Option<i64> = self
            .conn
            .query_row("SELECT MIN(created_at) FROM events", [], |row| row.get(0))
            .optional()
            .map_err(|e| Error::InvalidEvent(format!("Failed to query earliest event: {}", e)))?
            .flatten();

        let latest_event: Option<i64> = self
            .conn
            .query_row("SELECT MAX(created_at) FROM events", [], |row| row.get(0))
            .optional()
            .map_err(|e| Error::InvalidEvent(format!("Failed to query latest event: {}", e)))?
            .flatten();

        Ok(IndexStats {
            total_events,
            unique_files,
            unique_pubkeys,
            earliest_event,
            latest_event,
        })
    }

    /// Query events by kind
    ///
    /// # Arguments
    ///
    /// * `kind` - Event kind to query
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use proton_beam_core::EventIndex;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let index = EventIndex::new(Path::new("./pb_data/.index.db"))?;
    /// let text_notes = index.query_by_kind(1)?;
    /// println!("Found {} text notes", text_notes.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_by_kind(&self, kind: i32) -> Result<Vec<EventRecord>> {
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT id, kind, pubkey, created_at, file_path, indexed_at
                 FROM events WHERE kind = ? ORDER BY created_at DESC",
            )
            .map_err(|e| Error::InvalidEvent(format!("Failed to prepare query: {}", e)))?;

        let records = stmt
            .query_map(params![kind], |row| {
                Ok(EventRecord {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    pubkey: row.get(2)?,
                    created_at: row.get(3)?,
                    file_path: row.get(4)?,
                    indexed_at: row.get(5)?,
                })
            })
            .map_err(|e| Error::InvalidEvent(format!("Failed to query by kind: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::InvalidEvent(format!("Failed to collect results: {}", e)))?;

        Ok(records)
    }

    /// Query events by pubkey
    ///
    /// # Arguments
    ///
    /// * `pubkey` - Public key to query (hex-encoded)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use proton_beam_core::EventIndex;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let index = EventIndex::new(Path::new("./pb_data/.index.db"))?;
    /// let events = index.query_by_pubkey("pubkey_abc")?;
    /// println!("Found {} events from this pubkey", events.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_by_pubkey(&self, pubkey: &str) -> Result<Vec<EventRecord>> {
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT id, kind, pubkey, created_at, file_path, indexed_at
                 FROM events WHERE pubkey = ? ORDER BY created_at DESC",
            )
            .map_err(|e| Error::InvalidEvent(format!("Failed to prepare query: {}", e)))?;

        let records = stmt
            .query_map(params![pubkey], |row| {
                Ok(EventRecord {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    pubkey: row.get(2)?,
                    created_at: row.get(3)?,
                    file_path: row.get(4)?,
                    indexed_at: row.get(5)?,
                })
            })
            .map_err(|e| Error::InvalidEvent(format!("Failed to query by pubkey: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::InvalidEvent(format!("Failed to collect results: {}", e)))?;

        Ok(records)
    }

    /// Query events by date range
    ///
    /// # Arguments
    ///
    /// * `start` - Start timestamp (inclusive)
    /// * `end` - End timestamp (inclusive)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use proton_beam_core::EventIndex;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let index = EventIndex::new(Path::new("./pb_data/.index.db"))?;
    /// let events = index.query_by_date_range(1697000000, 1697086400)?;
    /// println!("Found {} events in range", events.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_by_date_range(&self, start: i64, end: i64) -> Result<Vec<EventRecord>> {
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT id, kind, pubkey, created_at, file_path, indexed_at
                 FROM events WHERE created_at >= ? AND created_at <= ? ORDER BY created_at DESC",
            )
            .map_err(|e| Error::InvalidEvent(format!("Failed to prepare query: {}", e)))?;

        let records = stmt
            .query_map(params![start, end], |row| {
                Ok(EventRecord {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    pubkey: row.get(2)?,
                    created_at: row.get(3)?,
                    file_path: row.get(4)?,
                    indexed_at: row.get(5)?,
                })
            })
            .map_err(|e| Error::InvalidEvent(format!("Failed to query by date range: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::InvalidEvent(format!("Failed to collect results: {}", e)))?;

        Ok(records)
    }

    /// Get an event record by ID
    ///
    /// Returns `None` if the event is not in the index.
    ///
    /// # Arguments
    ///
    /// * `event_id` - Event ID to look up (hex-encoded)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use proton_beam_core::EventIndex;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let index = EventIndex::new(Path::new("./pb_data/.index.db"))?;
    /// if let Some(record) = index.get("event_id_123")? {
    ///     println!("Event found in file: {}", record.file_path);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get(&self, event_id: &str) -> Result<Option<EventRecord>> {
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT id, kind, pubkey, created_at, file_path, indexed_at
                 FROM events WHERE id = ?",
            )
            .map_err(|e| Error::InvalidEvent(format!("Failed to prepare query: {}", e)))?;

        let record = stmt
            .query_row(params![event_id], |row| {
                Ok(EventRecord {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    pubkey: row.get(2)?,
                    created_at: row.get(3)?,
                    file_path: row.get(4)?,
                    indexed_at: row.get(5)?,
                })
            })
            .optional()
            .map_err(|e| Error::InvalidEvent(format!("Failed to get event: {}", e)))?;

        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProtoEventBuilder;
    use tempfile::TempDir;

    fn create_test_index() -> (EventIndex, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let index = EventIndex::new(&db_path).unwrap();
        (index, temp_dir)
    }

    fn create_test_event(id: &str, kind: i32, pubkey: &str, created_at: i64) -> ProtoEvent {
        ProtoEventBuilder::new()
            .id(id)
            .kind(kind)
            .pubkey(pubkey)
            .created_at(created_at)
            .content("test content")
            .sig("test_sig")
            .build()
    }

    #[test]
    fn test_create_index() {
        let (index, _temp_dir) = create_test_index();
        let stats = index.stats().unwrap();
        assert_eq!(stats.total_events, 0);
    }

    #[test]
    fn test_insert_and_contains() {
        let (mut index, _temp_dir) = create_test_index();

        let event = create_test_event("event_1", 1, "pubkey_1", 1234567890);

        assert!(!index.contains("event_1").unwrap());
        index.insert(&event, "2025_10_13.pb").unwrap();
        assert!(index.contains("event_1").unwrap());
    }

    #[test]
    fn test_insert_duplicate() {
        let (mut index, _temp_dir) = create_test_index();

        let event = create_test_event("event_1", 1, "pubkey_1", 1234567890);

        index.insert(&event, "2025_10_13.pb").unwrap();
        index.insert(&event, "2025_10_13.pb").unwrap(); // Should not error

        let stats = index.stats().unwrap();
        assert_eq!(stats.total_events, 1); // Only one event should be stored
    }

    #[test]
    fn test_insert_batch() {
        let (mut index, _temp_dir) = create_test_index();

        let event1 = create_test_event("event_1", 1, "pubkey_1", 1234567890);
        let event2 = create_test_event("event_2", 1, "pubkey_2", 1234567891);
        let event3 = create_test_event("event_3", 3, "pubkey_3", 1234567892);

        let events = vec![
            (&event1, "2025_10_13.pb"),
            (&event2, "2025_10_13.pb"),
            (&event3, "2025_10_14.pb"),
        ];

        index.insert_batch(&events).unwrap();

        let stats = index.stats().unwrap();
        assert_eq!(stats.total_events, 3);
        assert_eq!(stats.unique_files, 2);
        assert_eq!(stats.unique_pubkeys, 3);
    }

    #[test]
    fn test_stats() {
        let (mut index, _temp_dir) = create_test_index();

        let event1 = create_test_event("event_1", 1, "pubkey_1", 1234567890);
        let event2 = create_test_event("event_2", 1, "pubkey_1", 1234567891);
        let event3 = create_test_event("event_3", 3, "pubkey_2", 1234567892);

        index.insert(&event1, "file1.pb").unwrap();
        index.insert(&event2, "file1.pb").unwrap();
        index.insert(&event3, "file2.pb").unwrap();

        let stats = index.stats().unwrap();
        assert_eq!(stats.total_events, 3);
        assert_eq!(stats.unique_files, 2);
        assert_eq!(stats.unique_pubkeys, 2);
        assert_eq!(stats.earliest_event, Some(1234567890));
        assert_eq!(stats.latest_event, Some(1234567892));
    }

    #[test]
    fn test_query_by_kind() {
        let (mut index, _temp_dir) = create_test_index();

        let event1 = create_test_event("event_1", 1, "pubkey_1", 1234567890);
        let event2 = create_test_event("event_2", 1, "pubkey_2", 1234567891);
        let event3 = create_test_event("event_3", 3, "pubkey_3", 1234567892);

        index.insert(&event1, "file1.pb").unwrap();
        index.insert(&event2, "file1.pb").unwrap();
        index.insert(&event3, "file2.pb").unwrap();

        let kind_1_events = index.query_by_kind(1).unwrap();
        assert_eq!(kind_1_events.len(), 2);
        assert_eq!(kind_1_events[0].id, "event_2"); // Should be ordered by created_at DESC
        assert_eq!(kind_1_events[1].id, "event_1");

        let kind_3_events = index.query_by_kind(3).unwrap();
        assert_eq!(kind_3_events.len(), 1);
        assert_eq!(kind_3_events[0].id, "event_3");
    }

    #[test]
    fn test_query_by_pubkey() {
        let (mut index, _temp_dir) = create_test_index();

        let event1 = create_test_event("event_1", 1, "pubkey_1", 1234567890);
        let event2 = create_test_event("event_2", 3, "pubkey_1", 1234567891);
        let event3 = create_test_event("event_3", 1, "pubkey_2", 1234567892);

        index.insert(&event1, "file1.pb").unwrap();
        index.insert(&event2, "file1.pb").unwrap();
        index.insert(&event3, "file2.pb").unwrap();

        let pubkey_1_events = index.query_by_pubkey("pubkey_1").unwrap();
        assert_eq!(pubkey_1_events.len(), 2);
        assert_eq!(pubkey_1_events[0].id, "event_2");
        assert_eq!(pubkey_1_events[1].id, "event_1");

        let pubkey_2_events = index.query_by_pubkey("pubkey_2").unwrap();
        assert_eq!(pubkey_2_events.len(), 1);
        assert_eq!(pubkey_2_events[0].id, "event_3");
    }

    #[test]
    fn test_query_by_date_range() {
        let (mut index, _temp_dir) = create_test_index();

        let event1 = create_test_event("event_1", 1, "pubkey_1", 1000);
        let event2 = create_test_event("event_2", 1, "pubkey_2", 2000);
        let event3 = create_test_event("event_3", 1, "pubkey_3", 3000);

        index.insert(&event1, "file1.pb").unwrap();
        index.insert(&event2, "file1.pb").unwrap();
        index.insert(&event3, "file2.pb").unwrap();

        let range_events = index.query_by_date_range(1500, 2500).unwrap();
        assert_eq!(range_events.len(), 1);
        assert_eq!(range_events[0].id, "event_2");

        let all_events = index.query_by_date_range(0, 10000).unwrap();
        assert_eq!(all_events.len(), 3);
    }

    #[test]
    fn test_get() {
        let (mut index, _temp_dir) = create_test_index();

        let event = create_test_event("event_1", 1, "pubkey_1", 1234567890);
        index.insert(&event, "2025_10_13.pb").unwrap();

        let record = index.get("event_1").unwrap();
        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.id, "event_1");
        assert_eq!(record.kind, 1);
        assert_eq!(record.pubkey, "pubkey_1");
        assert_eq!(record.created_at, 1234567890);
        assert_eq!(record.file_path, "2025_10_13.pb");

        let missing = index.get("nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_reopen_index() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create index and insert data
        {
            let mut index = EventIndex::new(&db_path).unwrap();
            let event = create_test_event("event_1", 1, "pubkey_1", 1234567890);
            index.insert(&event, "2025_10_13.pb").unwrap();
        }

        // Reopen and verify data persisted
        {
            let index = EventIndex::new(&db_path).unwrap();
            assert!(index.contains("event_1").unwrap());
            let stats = index.stats().unwrap();
            assert_eq!(stats.total_events, 1);
        }
    }
}

