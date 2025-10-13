//! Builder pattern for ProtoEvent construction

use crate::{ProtoEvent, Tag};

/// Fluent builder for constructing ProtoEvent instances
///
/// # Example
///
/// ```
/// use proton_beam_core::ProtoEventBuilder;
///
/// let event = ProtoEventBuilder::new()
///     .id("abc123")
///     .pubkey("def456")
///     .created_at(1234567890)
///     .kind(1)
///     .content("Hello, Nostr!")
///     .add_tag(vec!["e", "event_id"])
///     .add_tag(vec!["p", "pubkey_id"])
///     .sig("sig789")
///     .build();
///
/// assert_eq!(event.id, "abc123");
/// assert_eq!(event.tags.len(), 2);
/// ```
pub struct ProtoEventBuilder {
    id: String,
    pubkey: String,
    created_at: i64,
    kind: i32,
    tags: Vec<Tag>,
    content: String,
    sig: String,
}

impl ProtoEventBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            id: String::new(),
            pubkey: String::new(),
            created_at: 0,
            kind: 0,
            tags: Vec::new(),
            content: String::new(),
            sig: String::new(),
        }
    }

    /// Set the event ID
    pub fn id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = id.into();
        self
    }

    /// Set the public key
    pub fn pubkey<S: Into<String>>(mut self, pubkey: S) -> Self {
        self.pubkey = pubkey.into();
        self
    }

    /// Set the creation timestamp
    pub fn created_at(mut self, timestamp: i64) -> Self {
        self.created_at = timestamp;
        self
    }

    /// Set the event kind
    pub fn kind(mut self, kind: i32) -> Self {
        self.kind = kind;
        self
    }

    /// Set the content
    pub fn content<S: Into<String>>(mut self, content: S) -> Self {
        self.content = content.into();
        self
    }

    /// Set the signature
    pub fn sig<S: Into<String>>(mut self, sig: S) -> Self {
        self.sig = sig.into();
        self
    }

    /// Add a single tag
    ///
    /// Accepts any iterator of string-like values
    pub fn add_tag<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.push(Tag {
            values: values.into_iter().map(|s| s.into()).collect(),
        });
        self
    }

    /// Add multiple tags at once
    pub fn tags(mut self, tags: Vec<Tag>) -> Self {
        self.tags = tags;
        self
    }

    /// Add a tag from a Tag instance
    pub fn add_tag_instance(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self
    }

    /// Build the ProtoEvent
    pub fn build(self) -> ProtoEvent {
        ProtoEvent {
            id: self.id,
            pubkey: self.pubkey,
            created_at: self.created_at,
            kind: self.kind,
            tags: self.tags,
            content: self.content,
            sig: self.sig,
        }
    }
}

impl Default for ProtoEventBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let event = ProtoEventBuilder::new()
            .id("test_id")
            .pubkey("test_pubkey")
            .created_at(1234567890)
            .kind(1)
            .content("Hello!")
            .sig("test_sig")
            .build();

        assert_eq!(event.id, "test_id");
        assert_eq!(event.pubkey, "test_pubkey");
        assert_eq!(event.created_at, 1234567890);
        assert_eq!(event.kind, 1);
        assert_eq!(event.content, "Hello!");
        assert_eq!(event.sig, "test_sig");
        assert_eq!(event.tags.len(), 0);
    }

    #[test]
    fn test_builder_with_tags() {
        let event = ProtoEventBuilder::new()
            .id("test")
            .pubkey("test")
            .add_tag(vec!["e", "event_id"])
            .add_tag(vec!["p", "pubkey_id", "relay_url"])
            .add_tag(vec!["t", "nostr"])
            .created_at(123)
            .kind(1)
            .content("test")
            .sig("test")
            .build();

        assert_eq!(event.tags.len(), 3);
        assert_eq!(event.tags[0].values, vec!["e", "event_id"]);
        assert_eq!(event.tags[1].values, vec!["p", "pubkey_id", "relay_url"]);
        assert_eq!(event.tags[2].values, vec!["t", "nostr"]);
    }

    #[test]
    fn test_builder_default() {
        let event = ProtoEventBuilder::default().build();

        assert_eq!(event.id, "");
        assert_eq!(event.pubkey, "");
        assert_eq!(event.created_at, 0);
        assert_eq!(event.kind, 0);
        assert_eq!(event.content, "");
        assert_eq!(event.sig, "");
        assert_eq!(event.tags.len(), 0);
    }

    #[test]
    fn test_builder_string_conversion() {
        let event = ProtoEventBuilder::new()
            .id(String::from("owned_string"))
            .pubkey("str_slice")
            .content("test".to_string())
            .build();

        assert_eq!(event.id, "owned_string");
        assert_eq!(event.pubkey, "str_slice");
        assert_eq!(event.content, "test");
    }

    #[test]
    fn test_builder_fluent_api() {
        // Test that chaining works smoothly
        let event = ProtoEventBuilder::new()
            .id("1")
            .pubkey("2")
            .created_at(3)
            .kind(4)
            .content("5")
            .sig("6")
            .add_tag(vec!["a"])
            .add_tag(vec!["b"])
            .build();

        assert_eq!(event.id, "1");
        assert_eq!(event.tags.len(), 2);
    }

    #[test]
    fn test_builder_add_tag_instance() {
        let tag = Tag {
            values: vec!["custom".to_string(), "tag".to_string()],
        };

        let event = ProtoEventBuilder::new()
            .id("test")
            .pubkey("test")
            .add_tag_instance(tag)
            .build();

        assert_eq!(event.tags.len(), 1);
        assert_eq!(event.tags[0].values, vec!["custom", "tag"]);
    }
}
