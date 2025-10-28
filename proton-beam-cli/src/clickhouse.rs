//! ClickHouse integration for bulk importing Nostr events
//!
//! This module provides functionality to insert ProtoEvent data into ClickHouse.

use anyhow::{Context, Result};
use proton_beam_core::ProtoEvent;

#[cfg(feature = "clickhouse")]
use clickhouse::{Client, Row};

#[cfg(feature = "clickhouse")]
use serde::Serialize;

/// Configuration for ClickHouse connection
#[derive(Debug, Clone)]
pub struct ClickHouseConfig {
    /// ClickHouse host (e.g., "localhost")
    pub host: String,

    /// ClickHouse HTTP port (default: 8123)
    pub port: u16,

    /// ClickHouse user
    pub user: String,

    /// ClickHouse password
    pub password: String,

    /// Database name
    pub database: String,

    /// Table name (default: "events_local")
    pub table: String,
}

impl Default for ClickHouseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8123,
            user: "default".to_string(),
            password: String::new(),
            database: "nostr".to_string(),
            table: "events_local".to_string(),
        }
    }
}

/// ClickHouse event row for insertion
/// This matches the schema defined in clickhouse/schema.sql
#[cfg(feature = "clickhouse")]
#[derive(Debug, Clone, Row, Serialize)]
pub struct EventRow {
    pub id: String,
    pub pubkey: String,
    pub created_at: u32,
    pub kind: u16,
    pub content: String,
    pub sig: String,
    pub tags: Vec<Vec<String>>,
    pub relay_source: String,
}

#[cfg(feature = "clickhouse")]
impl From<ProtoEvent> for EventRow {
    fn from(event: ProtoEvent) -> Self {
        // Convert tags from ProtoEvent format
        let tags: Vec<Vec<String>> = event
            .tags
            .into_iter()
            .map(|tag| tag.values)
            .collect();

        Self {
            id: event.id,
            pubkey: event.pubkey,
            created_at: event.created_at as u32,
            kind: event.kind as u16,
            content: event.content,
            sig: event.sig,
            tags,
            relay_source: String::new(), // Can be populated if we track relay sources
        }
    }
}

/// ClickHouse client wrapper for event insertion
#[cfg(feature = "clickhouse")]
pub struct ClickHouseClient {
    client: Client,
    config: ClickHouseConfig,
}

#[cfg(feature = "clickhouse")]
impl ClickHouseClient {
    /// Create a new ClickHouse client
    pub fn new(config: ClickHouseConfig) -> Result<Self> {
        let url = format!("http://{}:{}", config.host, config.port);

        let client = Client::default()
            .with_url(&url)
            .with_user(&config.user)
            .with_password(&config.password)
            .with_database(&config.database)
            .with_option("async_insert", "1")
            .with_option("wait_for_async_insert", "0")
            .with_option("async_insert_max_data_size", "10000000") // 10MB
            .with_option("async_insert_busy_timeout_ms", "5000"); // 5s

        Ok(Self { client, config })
    }

    /// Test the connection to ClickHouse
    pub async fn test_connection(&self) -> Result<()> {
        let version: String = self
            .client
            .query("SELECT version()")
            .fetch_one()
            .await
            .context("Failed to connect to ClickHouse")?;

        tracing::info!("Connected to ClickHouse version: {}", version);
        Ok(())
    }

    /// Insert a batch of events into ClickHouse
    pub async fn insert_events(&self, events: Vec<EventRow>) -> Result<usize> {
        let count = events.len();

        if count == 0 {
            return Ok(0);
        }

        let mut insert = self
            .client
            .insert(&self.config.table)
            .context("Failed to create insert statement")?;

        for event in events {
            insert.write(&event).await.context("Failed to write event")?;
        }

        insert.end().await.context("Failed to finalize insert")?;

        Ok(count)
    }

    /// Insert events in batches with progress reporting
    pub async fn insert_events_batched<F>(
        &self,
        events: Vec<EventRow>,
        batch_size: usize,
        mut progress_callback: F,
    ) -> Result<usize>
    where
        F: FnMut(usize, usize),
    {
        let total = events.len();
        let mut inserted = 0;

        for chunk in events.chunks(batch_size) {
            let count = self.insert_events(chunk.to_vec()).await?;
            inserted += count;
            progress_callback(inserted, total);
        }

        Ok(inserted)
    }

    /// Get the count of events in the table
    pub async fn get_event_count(&self) -> Result<u64> {
        let count: u64 = self
            .client
            .query(&format!("SELECT count() FROM {}", self.config.table))
            .fetch_one()
            .await
            .context("Failed to get event count")?;

        Ok(count)
    }

    /// Check if the database and table exist
    pub async fn verify_schema(&self) -> Result<()> {
        // Check database exists
        let _db_exists: u8 = self
            .client
            .query("SELECT 1 FROM system.databases WHERE name = ?")
            .bind(&self.config.database)
            .fetch_one()
            .await
            .context(format!(
                "Database '{}' does not exist. Run clickhouse/bootstrap.sh first",
                self.config.database
            ))?;

        // Check table exists
        let _table_exists: u8 = self
            .client
            .query("SELECT 1 FROM system.tables WHERE database = ? AND name = ?")
            .bind(&self.config.database)
            .bind(&self.config.table)
            .fetch_one()
            .await
            .context(format!(
                "Table '{}.{}' does not exist. Run clickhouse/bootstrap.sh first",
                self.config.database, self.config.table
            ))?;

        Ok(())
    }
}

#[cfg(not(feature = "clickhouse"))]
pub struct ClickHouseClient;

#[cfg(not(feature = "clickhouse"))]
impl ClickHouseClient {
    pub fn new(_config: ClickHouseConfig) -> Result<Self> {
        anyhow::bail!("ClickHouse support not enabled. Compile with --features clickhouse")
    }
}

#[cfg(test)]
#[cfg(feature = "clickhouse")]
mod tests {
    use super::*;

    #[test]
    fn test_event_row_conversion() {
        let proto_event = ProtoEvent {
            id: "test123".to_string(),
            pubkey: "pubkey456".to_string(),
            created_at: 1234567890,
            kind: 1,
            tags: vec![
                proton_beam_core::Tag {
                    values: vec!["e".to_string(), "event_id".to_string()],
                },
                proton_beam_core::Tag {
                    values: vec!["p".to_string(), "pubkey".to_string()],
                },
            ],
            content: "Hello Nostr!".to_string(),
            sig: "signature789".to_string(),
        };

        let event_row: EventRow = proto_event.into();

        assert_eq!(event_row.id, "test123");
        assert_eq!(event_row.pubkey, "pubkey456");
        assert_eq!(event_row.created_at, 1234567890);
        assert_eq!(event_row.kind, 1);
        assert_eq!(event_row.content, "Hello Nostr!");
        assert_eq!(event_row.sig, "signature789");
        assert_eq!(event_row.tags.len(), 2);
        assert_eq!(event_row.tags[0], vec!["e", "event_id"]);
        assert_eq!(event_row.tags[1], vec!["p", "pubkey"]);
    }

    #[test]
    fn test_default_config() {
        let config = ClickHouseConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 8123);
        assert_eq!(config.user, "default");
        assert_eq!(config.password, "");
        assert_eq!(config.database, "nostr");
        assert_eq!(config.table, "events_local");
    }
}

