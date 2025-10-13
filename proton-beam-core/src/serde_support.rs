//! Serde support for ProtoEvent
//!
//! This module provides Serialize and Deserialize implementations for ProtoEvent,
//! allowing direct JSON serialization without going through nostr-sdk.

use crate::{ProtoEvent, Tag};
use serde::{Deserialize, Serialize};

/// Serde-compatible wrapper for ProtoEvent
///
/// This allows serializing/deserializing ProtoEvent directly with serde,
/// which can be more efficient than converting through nostr-sdk Event.
///
/// # Example
///
/// ```
/// use proton_beam_core::{ProtoEvent, ProtoEventBuilder};
/// use serde_json;
///
/// let event = ProtoEventBuilder::new()
///     .id("abc123")
///     .pubkey("def456")
///     .created_at(1234567890)
///     .kind(1)
///     .content("Hello!")
///     .sig("sig789")
///     .build();
///
/// // Serialize to JSON
/// let json = serde_json::to_string(&event).unwrap();
///
/// // Deserialize from JSON
/// let deserialized: ProtoEvent = serde_json::from_str(&json).unwrap();
/// assert_eq!(event, deserialized);
/// ```
impl Serialize for ProtoEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("ProtoEvent", 7)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("pubkey", &self.pubkey)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("kind", &self.kind)?;

        // Convert tags to Vec<Vec<String>> for JSON compatibility
        let tags: Vec<Vec<String>> = self.tags.iter().map(|tag| tag.values.clone()).collect();
        state.serialize_field("tags", &tags)?;

        state.serialize_field("content", &self.content)?;
        state.serialize_field("sig", &self.sig)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ProtoEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ProtoEventHelper {
            id: String,
            pubkey: String,
            created_at: i64,
            kind: i32,
            tags: Vec<Vec<String>>,
            content: String,
            sig: String,
        }

        let helper = ProtoEventHelper::deserialize(deserializer)?;

        Ok(ProtoEvent {
            id: helper.id,
            pubkey: helper.pubkey,
            created_at: helper.created_at,
            kind: helper.kind,
            tags: helper
                .tags
                .into_iter()
                .map(|values| Tag { values })
                .collect(),
            content: helper.content,
            sig: helper.sig,
        })
    }
}

/// Implement Serialize for Tag as well
impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.values.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let values = Vec::<String>::deserialize(deserializer)?;
        Ok(Tag { values })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProtoEventBuilder;

    #[test]
    fn test_serialize_simple_event() {
        let event = ProtoEventBuilder::new()
            .id("test_id")
            .pubkey("test_pubkey")
            .created_at(1234567890)
            .kind(1)
            .content("Hello, world!")
            .sig("test_sig")
            .build();

        let json = serde_json::to_string(&event).unwrap();

        // Verify it contains expected fields
        assert!(json.contains("test_id"));
        assert!(json.contains("test_pubkey"));
        assert!(json.contains("1234567890"));
        assert!(json.contains("Hello, world!"));
    }

    #[test]
    fn test_serialize_with_tags() {
        let event = ProtoEventBuilder::new()
            .id("test")
            .pubkey("test")
            .add_tag(vec!["e", "event_id"])
            .add_tag(vec!["p", "pubkey_id", "relay"])
            .build();

        let json = serde_json::to_string(&event).unwrap();

        // Parse to verify structure
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value["tags"].is_array());
        assert_eq!(value["tags"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_deserialize_simple_event() {
        let json = r#"{
            "id": "test_id",
            "pubkey": "test_pubkey",
            "created_at": 1234567890,
            "kind": 1,
            "tags": [],
            "content": "test content",
            "sig": "test_sig"
        }"#;

        let event: ProtoEvent = serde_json::from_str(json).unwrap();

        assert_eq!(event.id, "test_id");
        assert_eq!(event.pubkey, "test_pubkey");
        assert_eq!(event.created_at, 1234567890);
        assert_eq!(event.kind, 1);
        assert_eq!(event.content, "test content");
        assert_eq!(event.sig, "test_sig");
        assert_eq!(event.tags.len(), 0);
    }

    #[test]
    fn test_deserialize_with_tags() {
        let json = r#"{
            "id": "test",
            "pubkey": "test",
            "created_at": 123,
            "kind": 1,
            "tags": [
                ["e", "event_id", "relay_url"],
                ["p", "pubkey_id"],
                ["t", "nostr"]
            ],
            "content": "test",
            "sig": "test"
        }"#;

        let event: ProtoEvent = serde_json::from_str(json).unwrap();

        assert_eq!(event.tags.len(), 3);
        assert_eq!(event.tags[0].values, vec!["e", "event_id", "relay_url"]);
        assert_eq!(event.tags[1].values, vec!["p", "pubkey_id"]);
        assert_eq!(event.tags[2].values, vec!["t", "nostr"]);
    }

    #[test]
    fn test_round_trip_serde() {
        let original = ProtoEventBuilder::new()
            .id("round_trip_test")
            .pubkey("pubkey123")
            .created_at(9999999)
            .kind(42)
            .add_tag(vec!["custom", "tag", "values"])
            .content("Round trip content")
            .sig("sig123")
            .build();

        // Serialize
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize
        let deserialized: ProtoEvent = serde_json::from_str(&json).unwrap();

        // Compare (uses PartialEq)
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_pretty_print() {
        let event = ProtoEventBuilder::new()
            .id("test")
            .pubkey("test")
            .kind(1)
            .build();

        let json = serde_json::to_string_pretty(&event).unwrap();

        // Check it's formatted nicely
        assert!(json.contains('\n'));
        assert!(json.contains("  \"id\""));
    }

    #[test]
    fn test_serialize_tag() {
        let tag = Tag {
            values: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };

        let json = serde_json::to_string(&tag).unwrap();
        assert_eq!(json, r#"["a","b","c"]"#);
    }

    #[test]
    fn test_deserialize_tag() {
        let json = r#"["x", "y", "z"]"#;
        let tag: Tag = serde_json::from_str(json).unwrap();

        assert_eq!(tag.values, vec!["x", "y", "z"]);
    }
}
