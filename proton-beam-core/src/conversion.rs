//! Conversion between JSON and Protobuf formats
//!
//! This module provides idiomatic Rust trait implementations for converting
//! between JSON strings, nostr-sdk Events, and ProtoEvents.

use crate::{ProtoEvent, Tag, error::Result};

// ============================================================================
// From/TryFrom Trait Implementations
// ============================================================================

/// Convert from a nostr-sdk Event to a ProtoEvent (infallible)
impl From<nostr_sdk::Event> for ProtoEvent {
    fn from(nostr_event: nostr_sdk::Event) -> Self {
        ProtoEvent {
            id: nostr_event.id.to_hex(),
            pubkey: nostr_event.pubkey.to_string(),
            created_at: nostr_event.created_at.as_u64() as i64,
            kind: nostr_event.kind.as_u16() as i32,
            tags: nostr_event
                .tags
                .iter()
                .map(|tag| Tag {
                    values: tag.as_vec().iter().map(|s| s.to_string()).collect(),
                })
                .collect(),
            content: nostr_event.content.clone(),
            sig: nostr_event.sig.to_string(),
        }
    }
}

/// Convert from a JSON string slice to a ProtoEvent (fallible)
///
/// # Example
///
/// ```no_run
/// use proton_beam_core::ProtoEvent;
/// use std::convert::TryFrom;
///
/// let json = r#"{"id":"abc...","pubkey":"def...","created_at":1234567890,"kind":1,"tags":[],"content":"Hello","sig":"123..."}"#;
/// let event = ProtoEvent::try_from(json)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
impl TryFrom<&str> for ProtoEvent {
    type Error = crate::error::Error;

    fn try_from(json: &str) -> Result<Self> {
        // First, pre-validate the kind field before passing to nostr-sdk
        // This prevents nostr-sdk from silently truncating invalid kind values
        if let Some(kind) = serde_json::from_str::<serde_json::Value>(json)
            .ok()
            .and_then(|v| v.get("kind").and_then(|k| k.as_i64()))
            .filter(|k| !(0..=65535).contains(k))
        {
            return Err(crate::error::Error::Conversion(format!(
                "Event kind {} is out of valid range (0-65535). Nostr event kinds must fit in a u16.",
                kind
            )));
        }

        // Parse JSON using nostr-sdk for proper validation
        let nostr_event: nostr_sdk::Event = serde_json::from_str(json)
            .map_err(|e| {
                // Enhance error message with more context
                let msg = e.to_string();

                // Try to identify which field caused the issue
                let hint = if msg.contains("expected a string") && msg.contains("line") && msg.contains("column") {
                    // Extract position info to give better context
                    msg.find("column")
                        .and_then(|col_idx| {
                            let col_part = &msg[col_idx..];
                            col_part.split_whitespace().nth(1)
                        })
                        .and_then(|num_str| num_str.parse::<usize>().ok())
                        .map(|col| {
                            if col < 100 {
                                " (hint: check that id, pubkey, and sig are hex strings)"
                            } else {
                                " (hint: all tag values must be strings, not numbers)"
                            }
                        })
                        .unwrap_or(" (hint: ensure id, pubkey, sig are hex strings and all tag values are strings)")
                } else if msg.contains("expected a string") && msg.contains("tags") {
                    " (hint: all tag values must be strings - check for numeric values in tag arrays)"
                } else if msg.contains("expected a string") {
                    " (hint: ensure id, pubkey, sig are hex strings and all tag values are strings)"
                } else if msg.contains("missing field") {
                    " (required Nostr event fields: id, pubkey, created_at, kind, tags, content, sig)"
                } else if msg.contains("invalid type") && msg.contains("tags") {
                    " (hint: tags must be an array of string arrays, all values must be strings)"
                } else {
                    ""
                };

                crate::error::Error::Conversion(format!("{}{}", msg, hint))
            })?;
        Ok(ProtoEvent::from(nostr_event))
    }
}

/// Convert from an owned JSON string to a ProtoEvent (fallible)
impl TryFrom<String> for ProtoEvent {
    type Error = crate::error::Error;

    fn try_from(json: String) -> Result<Self> {
        ProtoEvent::try_from(json.as_str())
    }
}

/// Convert from a ProtoEvent reference to a JSON string (fallible)
///
/// # Example
///
/// ```no_run
/// use proton_beam_core::ProtoEvent;
/// use std::convert::TryFrom;
///
/// let event = ProtoEvent {
///     id: "abc...".to_string(),
///     pubkey: "def...".to_string(),
///     created_at: 1234567890,
///     kind: 1,
///     tags: vec![],
///     content: "Hello".to_string(),
///     sig: "123...".to_string(),
/// };
/// let json = String::try_from(&event)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
impl TryFrom<&ProtoEvent> for String {
    type Error = crate::error::Error;

    fn try_from(event: &ProtoEvent) -> Result<Self> {
        // Convert protobuf tags to JSON array format
        let tags: Vec<Vec<String>> = event.tags.iter().map(|tag| tag.values.clone()).collect();

        // Build JSON object manually to match Nostr format exactly
        let json_obj = serde_json::json!({
            "id": event.id,
            "pubkey": event.pubkey,
            "created_at": event.created_at,
            "kind": event.kind,
            "tags": tags,
            "content": event.content,
            "sig": event.sig,
        });

        Ok(serde_json::to_string(&json_obj)?)
    }
}

// ============================================================================
// Convenience Functions (for ergonomics)
// ============================================================================

/// Convert a JSON string to a Protobuf ProtoEvent
///
/// This is a convenience wrapper around `ProtoEvent::try_from()`.
///
/// # Example
///
/// ```no_run
/// use proton_beam_core::json_to_proto;
///
/// let json = r#"{"id":"abc...","pubkey":"def...","created_at":1234567890,"kind":1,"tags":[],"content":"Hello","sig":"123..."}"#;
/// let event = json_to_proto(json)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn json_to_proto(json: &str) -> Result<ProtoEvent> {
    ProtoEvent::try_from(json)
}

/// Convert a Protobuf ProtoEvent to a JSON string
///
/// This is a convenience wrapper around `String::try_from()`.
///
/// # Example
///
/// ```no_run
/// use proton_beam_core::{ProtoEvent, proto_to_json};
///
/// let event = ProtoEvent {
///     id: "abc...".to_string(),
///     pubkey: "def...".to_string(),
///     created_at: 1234567890,
///     kind: 1,
///     tags: vec![],
///     content: "Hello".to_string(),
///     sig: "123...".to_string(),
/// };
/// let json = proto_to_json(&event)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn proto_to_json(event: &ProtoEvent) -> Result<String> {
    String::try_from(event)
}

/// Convert a Protobuf ProtoEvent to a nostr-sdk Event for validation
///
/// This is an internal helper function used by the validation module.
pub(crate) fn proto_to_nostr_event(event: &ProtoEvent) -> Result<nostr_sdk::Event> {
    // First convert to JSON, then parse with nostr-sdk
    // This ensures we use nostr-sdk's proper parsing
    let json = proto_to_json(event)?;
    Ok(serde_json::from_str(&json)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::convert::TryFrom;

    const SAMPLE_EVENT_JSON: &str = r#"{
        "id":"4376c65d2f232afbe9b882a35baa4f6fe8667c4e684749af565f981833ed6a65",
        "pubkey":"79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3",
        "created_at":1671217411,
        "kind":1,
        "tags":[
            ["e","5c83da77af1dec6d7289834998ad7aafbd9e2191396d75ec3cc27f5a77226f36","wss://nostr.example.com"],
            ["p","f7234bd4c1394dda46d09f35bd384dd30cc552ad5541990f98844fb06676e9ca"]
        ],
        "content":"This is a reply to another note!",
        "sig":"908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262"
    }"#;

    // ========================================================================
    // Tests for TryFrom trait implementations
    // ========================================================================

    #[test]
    fn test_try_from_str() {
        let event = ProtoEvent::try_from(SAMPLE_EVENT_JSON).unwrap();

        assert_eq!(
            event.id,
            "4376c65d2f232afbe9b882a35baa4f6fe8667c4e684749af565f981833ed6a65"
        );
        assert_eq!(
            event.pubkey,
            "79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3"
        );
        assert_eq!(event.created_at, 1671217411);
        assert_eq!(event.kind, 1);
        assert_eq!(event.tags.len(), 2);
        assert_eq!(event.content, "This is a reply to another note!");
    }

    #[test]
    fn test_try_from_string() {
        let json = SAMPLE_EVENT_JSON.to_string();
        let event = ProtoEvent::try_from(json).unwrap();

        assert_eq!(
            event.id,
            "4376c65d2f232afbe9b882a35baa4f6fe8667c4e684749af565f981833ed6a65"
        );
    }

    #[test]
    fn test_string_try_from_proto_event() {
        let event = ProtoEvent {
            id: "test_id".to_string(),
            pubkey: "test_pubkey".to_string(),
            created_at: 1234567890,
            kind: 1,
            tags: vec![
                Tag {
                    values: vec!["e".to_string(), "event_id".to_string()],
                },
                Tag {
                    values: vec!["p".to_string(), "pubkey_id".to_string()],
                },
            ],
            content: "Test content".to_string(),
            sig: "test_sig".to_string(),
        };

        let json = String::try_from(&event).unwrap();

        // Parse to verify it's valid JSON
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["id"], "test_id");
        assert_eq!(parsed["pubkey"], "test_pubkey");
        assert_eq!(parsed["created_at"], 1234567890);
        assert_eq!(parsed["kind"], 1);
        assert_eq!(parsed["content"], "Test content");
        assert_eq!(parsed["sig"], "test_sig");

        // Check tags structure
        assert!(parsed["tags"].is_array());
        assert_eq!(parsed["tags"][0][0], "e");
        assert_eq!(parsed["tags"][0][1], "event_id");
    }

    #[test]
    fn test_try_from_invalid_json() {
        let result = ProtoEvent::try_from("not valid json");
        assert!(result.is_err());
    }

    // ========================================================================
    // Tests for convenience functions (backward compatibility)
    // ========================================================================

    #[test]
    fn test_json_to_proto() {
        let event = json_to_proto(SAMPLE_EVENT_JSON).unwrap();

        assert_eq!(
            event.id,
            "4376c65d2f232afbe9b882a35baa4f6fe8667c4e684749af565f981833ed6a65"
        );
        assert_eq!(
            event.pubkey,
            "79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3"
        );
        assert_eq!(event.created_at, 1671217411);
        assert_eq!(event.kind, 1);
        assert_eq!(event.tags.len(), 2);
        assert_eq!(event.tags[0].values.len(), 3);
        assert_eq!(event.tags[0].values[0], "e");
        assert_eq!(event.content, "This is a reply to another note!");
    }

    #[test]
    fn test_proto_to_json() {
        let event = ProtoEvent {
            id: "test_id".to_string(),
            pubkey: "test_pubkey".to_string(),
            created_at: 1234567890,
            kind: 1,
            tags: vec![
                Tag {
                    values: vec!["e".to_string(), "event_id".to_string()],
                },
                Tag {
                    values: vec!["p".to_string(), "pubkey_id".to_string()],
                },
            ],
            content: "Test content".to_string(),
            sig: "test_sig".to_string(),
        };

        let json = proto_to_json(&event).unwrap();

        // Parse to verify it's valid JSON
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["id"], "test_id");
        assert_eq!(parsed["pubkey"], "test_pubkey");
        assert_eq!(parsed["created_at"], 1234567890);
        assert_eq!(parsed["kind"], 1);
        assert_eq!(parsed["content"], "Test content");
        assert_eq!(parsed["sig"], "test_sig");

        // Check tags structure
        assert!(parsed["tags"].is_array());
        assert_eq!(parsed["tags"][0][0], "e");
        assert_eq!(parsed["tags"][0][1], "event_id");
    }

    #[test]
    fn test_round_trip_conversion() {
        // Convert JSON -> Proto -> JSON and verify they match
        let original_event = json_to_proto(SAMPLE_EVENT_JSON).unwrap();
        let json_output = proto_to_json(&original_event).unwrap();
        let round_trip_event = json_to_proto(&json_output).unwrap();

        assert_eq!(original_event.id, round_trip_event.id);
        assert_eq!(original_event.pubkey, round_trip_event.pubkey);
        assert_eq!(original_event.created_at, round_trip_event.created_at);
        assert_eq!(original_event.kind, round_trip_event.kind);
        assert_eq!(original_event.content, round_trip_event.content);
        assert_eq!(original_event.sig, round_trip_event.sig);
        assert_eq!(original_event.tags.len(), round_trip_event.tags.len());
    }

    #[test]
    fn test_invalid_json() {
        let result = json_to_proto("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_kind_out_of_range_too_large() {
        // Test that kind > 65535 is rejected with a clear error message
        let json = r#"{"id": "6c6b55e939006d134889c0caba72d7c5dfd072f3394268ccd3c5eddc38c2f29a", "sig": "a409b1d05384da8478a445ecdd0a88d968c02d289326ac6e57ac60625defe56660308200ea6fb4d5d8860be42b6c4a7a05f3a73f82b0028f78c4f86fc4129173", "kind": 70202, "tags": [], "pubkey": "f79a5103bda9e48ed6aa468210453edce21227ca679fdcd2b33d8fe8adaa9408", "content": "test", "created_at": 1671557217}"#;

        let result = json_to_proto(json);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("70202"));
        assert!(err_msg.contains("out of valid range"));
        assert!(err_msg.contains("0-65535"));
    }

    #[test]
    fn test_kind_out_of_range_negative() {
        // Test that negative kind is rejected
        let json = r#"{"id": "6c6b55e939006d134889c0caba72d7c5dfd072f3394268ccd3c5eddc38c2f29a", "sig": "a409b1d05384da8478a445ecdd0a88d968c02d289326ac6e57ac60625defe56660308200ea6fb4d5d8860be42b6c4a7a05f3a73f82b0028f78c4f86fc4129173", "kind": -1, "tags": [], "pubkey": "f79a5103bda9e48ed6aa468210453edce21227ca679fdcd2b33d8fe8adaa9408", "content": "test", "created_at": 1671557217}"#;

        let result = json_to_proto(json);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("out of valid range"));
    }

    #[test]
    fn test_kind_max_valid() {
        // Test that kind = 65535 is accepted
        let json = format!(
            r#"{{"id": "{}", "sig": "{}", "kind": 65535, "tags": [], "pubkey": "{}", "content": "test", "created_at": 1671557217}}"#,
            "a".repeat(64),
            "b".repeat(128),
            "c".repeat(64)
        );

        // This will fail nostr-sdk validation (invalid signatures) but should pass kind range check
        let result = json_to_proto(&json);
        // It might fail for other reasons (bad signature), but NOT for kind range
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(!err_msg.contains("kind") || !err_msg.contains("out of valid range"));
        }
    }

    #[test]
    fn test_proto_to_json_with_empty_tags() {
        // Test using proto_to_json which doesn't require nostr-sdk validation
        let event = ProtoEvent {
            id: "test".to_string(),
            pubkey: "test".to_string(),
            created_at: 123,
            kind: 0,
            tags: vec![],
            content: "test".to_string(),
            sig: "test".to_string(),
        };

        let json = proto_to_json(&event).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["tags"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_proto_to_json_with_complex_tags() {
        // Test using proto_to_json which doesn't require nostr-sdk validation
        let event = ProtoEvent {
            id: "test".to_string(),
            pubkey: "test".to_string(),
            created_at: 123,
            kind: 1,
            tags: vec![
                Tag {
                    values: vec![
                        "e".to_string(),
                        "id1".to_string(),
                        "relay1".to_string(),
                        "marker1".to_string(),
                    ],
                },
                Tag {
                    values: vec!["p".to_string(), "pubkey1".to_string()],
                },
                Tag {
                    values: vec!["t".to_string(), "hashtag".to_string()],
                },
            ],
            content: "test".to_string(),
            sig: "test".to_string(),
        };

        let json = proto_to_json(&event).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["tags"].as_array().unwrap().len(), 3);
        assert_eq!(parsed["tags"][0].as_array().unwrap().len(), 4);
        assert_eq!(parsed["tags"][1].as_array().unwrap().len(), 2);
        assert_eq!(parsed["tags"][2].as_array().unwrap().len(), 2);
    }
}
