//! Display implementation for ProtoEvent

use crate::ProtoEvent;
use std::fmt;

/// Display implementation that outputs pretty-printed JSON
impl fmt::Display for ProtoEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Convert tags to JSON-compatible format
        let tags: Vec<Vec<String>> = self.tags.iter().map(|tag| tag.values.clone()).collect();

        // Build JSON object
        let json_obj = serde_json::json!({
            "id": self.id,
            "pubkey": self.pubkey,
            "created_at": self.created_at,
            "kind": self.kind,
            "tags": tags,
            "content": self.content,
            "sig": self.sig,
        });

        // Pretty print the JSON
        match serde_json::to_string_pretty(&json_obj) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "<invalid ProtoEvent>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tag;

    #[test]
    fn test_display_simple_event() {
        let event = ProtoEvent {
            id: "abc123".to_string(),
            pubkey: "def456".to_string(),
            created_at: 1234567890,
            kind: 1,
            tags: vec![],
            content: "Hello, Nostr!".to_string(),
            sig: "sig789".to_string(),
        };

        let output = format!("{}", event);

        // Check that output contains expected fields
        assert!(output.contains("\"id\""));
        assert!(output.contains("\"abc123\""));
        assert!(output.contains("\"content\""));
        assert!(output.contains("\"Hello, Nostr!\""));
        assert!(output.contains("1234567890"));
    }

    #[test]
    fn test_display_with_tags() {
        let event = ProtoEvent {
            id: "test".to_string(),
            pubkey: "test".to_string(),
            created_at: 123,
            kind: 1,
            tags: vec![
                Tag {
                    values: vec!["e".to_string(), "event_id".to_string()],
                },
                Tag {
                    values: vec!["p".to_string(), "pubkey_id".to_string()],
                },
            ],
            content: "test".to_string(),
            sig: "test".to_string(),
        };

        let output = format!("{}", event);

        // Check that tags are formatted as arrays
        assert!(output.contains("\"tags\""));
        assert!(output.contains("\"e\""));
        assert!(output.contains("\"event_id\""));
    }

    #[test]
    fn test_display_is_valid_json() {
        let event = ProtoEvent {
            id: "test".to_string(),
            pubkey: "test".to_string(),
            created_at: 123,
            kind: 1,
            tags: vec![],
            content: "test".to_string(),
            sig: "test".to_string(),
        };

        let output = format!("{}", event);

        // Parse to verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["id"], "test");
        assert_eq!(parsed["kind"], 1);
    }
}
