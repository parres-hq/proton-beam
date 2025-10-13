//! Proton Beam Core Library
//!
//! This library provides the core functionality for converting Nostr events
//! between JSON and Protocol Buffer formats, with full validation support.
//!
//! # Features
//!
//! - JSON ↔ Protobuf conversion for Nostr events (using idiomatic `TryFrom`/`From` traits)
//! - Event ID validation (SHA-256 verification)
//! - Schnorr signature verification
//! - Length-delimited protobuf I/O for streaming
//! - SQLite index for event deduplication and fast lookups
//! - Fluent builder pattern for constructing events
//! - Serde support for direct JSON serialization
//! - `Display` trait for human-readable output
//! - `FromIterator` for collecting events into batches
//! - `PartialEq`/`Eq` for easy comparisons
//!
//! # Examples
//!
//! ## Using idiomatic trait-based conversions
//!
//! ```no_run
//! use proton_beam_core::{ProtoEvent, validate_event};
//! use std::convert::TryFrom;
//!
//! // JSON string -> ProtoEvent
//! let json = r#"{"id":"...", "pubkey":"...", "created_at":1234567890, "kind":1, "tags":[], "content":"Hello", "sig":"..."}"#;
//! let event = ProtoEvent::try_from(json)?;
//! validate_event(&event)?;
//!
//! // ProtoEvent -> JSON string
//! let json_output = String::try_from(&event)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Using the builder pattern
//!
//! ```no_run
//! use proton_beam_core::ProtoEventBuilder;
//!
//! let event = ProtoEventBuilder::new()
//!     .id("abc123")
//!     .pubkey("def456")
//!     .created_at(1234567890)
//!     .kind(1)
//!     .content("Hello, Nostr!")
//!     .add_tag(vec!["e", "event_id"])
//!     .sig("sig789")
//!     .build();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Collecting events into batches
//!
//! ```no_run
//! use proton_beam_core::{EventBatch, ProtoEvent, ProtoEventBuilder};
//!
//! let events: Vec<ProtoEvent> = vec![
//!     ProtoEventBuilder::new().id("1").build(),
//!     ProtoEventBuilder::new().id("2").build(),
//! ];
//!
//! let batch: EventBatch = events.into_iter().collect();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

// Include the generated protobuf code
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/nostr.rs"));
}

// Re-export main types
pub use proto::{EventBatch, ProtoEvent, Tag};

// Public modules
pub mod builder;
pub mod conversion;
pub mod display;
pub mod error;
pub mod index;
pub mod iter;
pub mod serde_support;
pub mod storage;
pub mod validation;

// Re-export commonly used types and functions
pub use builder::ProtoEventBuilder;
pub use conversion::{json_to_proto, proto_to_json};
pub use error::{Error, Result};
pub use index::{EventIndex, EventRecord, IndexStats};
pub use storage::{read_events_delimited, write_event_delimited, write_events_delimited};
pub use validation::validate_event;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protobuf_generation() {
        // Verify that protobuf code was generated
        let event = ProtoEvent {
            id: String::from("test"),
            pubkey: String::from("test"),
            created_at: 0,
            kind: 1,
            tags: vec![],
            content: String::from("test"),
            sig: String::from("test"),
        };
        assert_eq!(event.id, "test");
    }
}
