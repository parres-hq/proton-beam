//! Integration tests showcasing all the new API features

use proton_beam_core::{EventBatch, ProtoEvent, ProtoEventBuilder, Result};

#[test]
fn test_builder_pattern() -> Result<()> {
    // Create event using builder
    let event = ProtoEventBuilder::new()
        .id("test_id_123")
        .pubkey("test_pubkey_456")
        .created_at(1234567890)
        .kind(1)
        .content("Built with builder pattern!")
        .add_tag(vec!["e", "referenced_event"])
        .add_tag(vec!["p", "mentioned_user"])
        .sig("test_signature")
        .build();

    assert_eq!(event.id, "test_id_123");
    assert_eq!(event.kind, 1);
    assert_eq!(event.tags.len(), 2);
    assert_eq!(event.content, "Built with builder pattern!");

    Ok(())
}

#[test]
fn test_display_trait() -> Result<()> {
    let event = ProtoEventBuilder::new()
        .id("display_test")
        .pubkey("pubkey123")
        .created_at(1234567890)
        .kind(1)
        .content("Testing Display trait")
        .add_tag(vec!["t", "test"])
        .sig("sig123")
        .build();

    // Use Display trait
    let displayed = format!("{}", event);

    // Should be valid JSON
    assert!(displayed.contains("display_test"));
    assert!(displayed.contains("pubkey123"));
    assert!(displayed.contains("Testing Display trait"));

    // Should be pretty-printed
    assert!(displayed.contains('\n'));

    Ok(())
}

#[test]
fn test_from_iterator() -> Result<()> {
    // Create multiple events
    let events: Vec<ProtoEvent> = (1..=5)
        .map(|i| {
            ProtoEventBuilder::new()
                .id(format!("event_{}", i))
                .pubkey("test_pubkey")
                .created_at(1000 + i)
                .kind(1)
                .content(format!("Event number {}", i))
                .sig("test_sig")
                .build()
        })
        .collect();

    // Collect into EventBatch using FromIterator
    let batch: EventBatch = events.into_iter().collect();

    assert_eq!(batch.events.len(), 5);
    assert_eq!(batch.events[0].id, "event_1");
    assert_eq!(batch.events[4].id, "event_5");

    Ok(())
}

#[test]
fn test_extend_event_batch() -> Result<()> {
    // Start with one event
    let mut batch = EventBatch {
        events: vec![ProtoEventBuilder::new().id("first").build()],
    };

    // Extend with more events
    let new_events: Vec<ProtoEvent> = vec![
        ProtoEventBuilder::new().id("second").build(),
        ProtoEventBuilder::new().id("third").build(),
    ];

    batch.extend(new_events);

    assert_eq!(batch.events.len(), 3);
    assert_eq!(batch.events[0].id, "first");
    assert_eq!(batch.events[1].id, "second");
    assert_eq!(batch.events[2].id, "third");

    Ok(())
}

#[test]
fn test_serde_serialize() -> Result<()> {
    let event = ProtoEventBuilder::new()
        .id("serde_test")
        .pubkey("pubkey_abc")
        .created_at(9999999)
        .kind(42)
        .content("Testing serde")
        .add_tag(vec!["custom", "tag"])
        .sig("sig_xyz")
        .build();

    // Serialize using serde
    let json = serde_json::to_string(&event)?;

    // Verify JSON contains expected data
    assert!(json.contains("serde_test"));
    assert!(json.contains("pubkey_abc"));
    assert!(json.contains("9999999"));
    assert!(json.contains("Testing serde"));

    Ok(())
}

#[test]
fn test_serde_deserialize() -> Result<()> {
    let json = r#"{
        "id": "deser_test",
        "pubkey": "deser_pubkey",
        "created_at": 7777777,
        "kind": 10,
        "tags": [
            ["e", "event_ref"],
            ["p", "person_ref", "relay"]
        ],
        "content": "Deserialized content",
        "sig": "deser_sig"
    }"#;

    // Deserialize using serde
    let event: ProtoEvent = serde_json::from_str(json)?;

    assert_eq!(event.id, "deser_test");
    assert_eq!(event.pubkey, "deser_pubkey");
    assert_eq!(event.created_at, 7777777);
    assert_eq!(event.kind, 10);
    assert_eq!(event.tags.len(), 2);
    assert_eq!(event.tags[0].values, vec!["e", "event_ref"]);
    assert_eq!(event.tags[1].values, vec!["p", "person_ref", "relay"]);
    assert_eq!(event.content, "Deserialized content");
    assert_eq!(event.sig, "deser_sig");

    Ok(())
}

#[test]
fn test_serde_round_trip() -> Result<()> {
    let original = ProtoEventBuilder::new()
        .id("round_trip")
        .pubkey("pubkey_round_trip")
        .created_at(5555555)
        .kind(7)
        .content("Round trip test")
        .add_tag(vec!["a", "b", "c"])
        .add_tag(vec!["x", "y"])
        .sig("round_trip_sig")
        .build();

    // Serialize
    let json = serde_json::to_string(&original)?;

    // Deserialize
    let deserialized: ProtoEvent = serde_json::from_str(&json)?;

    // Should be equal (using PartialEq)
    assert_eq!(original, deserialized);

    Ok(())
}

#[test]
fn test_partial_eq() -> Result<()> {
    let event1 = ProtoEventBuilder::new()
        .id("same_id")
        .pubkey("same_pubkey")
        .created_at(123)
        .kind(1)
        .content("same content")
        .sig("same_sig")
        .build();

    let event2 = ProtoEventBuilder::new()
        .id("same_id")
        .pubkey("same_pubkey")
        .created_at(123)
        .kind(1)
        .content("same content")
        .sig("same_sig")
        .build();

    let event3 = ProtoEventBuilder::new()
        .id("different_id")
        .pubkey("same_pubkey")
        .created_at(123)
        .kind(1)
        .content("same content")
        .sig("same_sig")
        .build();

    // Events 1 and 2 should be equal
    assert_eq!(event1, event2);

    // Events 1 and 3 should not be equal
    assert_ne!(event1, event3);

    Ok(())
}

#[test]
fn test_combined_workflow() -> Result<()> {
    // 1. Build events using builder
    let events: Vec<ProtoEvent> = (1..=3)
        .map(|i| {
            ProtoEventBuilder::new()
                .id(format!("workflow_{}", i))
                .pubkey("workflow_pubkey")
                .created_at(1000000 + i)
                .kind(1)
                .content(format!("Workflow event {}", i))
                .add_tag(vec!["workflow", "test"])
                .sig("workflow_sig")
                .build()
        })
        .collect();

    // 2. Collect into batch using FromIterator
    let batch: EventBatch = events.into_iter().collect();
    assert_eq!(batch.events.len(), 3);

    // 3. Serialize batch events using Display
    for event in &batch.events {
        let displayed = format!("{}", event);
        assert!(displayed.contains("workflow"));
    }

    // 4. Serialize using serde
    let json = serde_json::to_string(&batch.events[0])?;
    assert!(json.contains("workflow_1"));

    // 5. Deserialize using serde
    let deserialized: ProtoEvent = serde_json::from_str(&json)?;

    // 6. Compare using PartialEq
    assert_eq!(batch.events[0], deserialized);

    Ok(())
}

#[test]
fn test_filter_and_collect() -> Result<()> {
    // Create events with different kinds
    let all_events: Vec<ProtoEvent> = vec![
        ProtoEventBuilder::new()
            .id("1")
            .kind(1)
            .content("text note")
            .build(),
        ProtoEventBuilder::new()
            .id("2")
            .kind(3)
            .content("contacts")
            .build(),
        ProtoEventBuilder::new()
            .id("3")
            .kind(1)
            .content("another text note")
            .build(),
        ProtoEventBuilder::new()
            .id("4")
            .kind(7)
            .content("reaction")
            .build(),
        ProtoEventBuilder::new()
            .id("5")
            .kind(1)
            .content("third text note")
            .build(),
    ];

    // Filter to only kind 1 events and collect into batch
    let text_notes_batch: EventBatch = all_events.into_iter().filter(|e| e.kind == 1).collect();

    assert_eq!(text_notes_batch.events.len(), 3);
    assert!(text_notes_batch.events.iter().all(|e| e.kind == 1));
    assert_eq!(text_notes_batch.events[0].content, "text note");
    assert_eq!(text_notes_batch.events[1].content, "another text note");
    assert_eq!(text_notes_batch.events[2].content, "third text note");

    Ok(())
}

#[test]
fn test_pretty_print_json() -> Result<()> {
    let event = ProtoEventBuilder::new()
        .id("pretty")
        .pubkey("test")
        .created_at(123)
        .kind(1)
        .content("Pretty printed")
        .add_tag(vec!["t", "beautiful"])
        .sig("sig")
        .build();

    // Pretty print using serde
    let pretty_json = serde_json::to_string_pretty(&event)?;

    // Should have indentation
    assert!(pretty_json.contains("  \"id\""));
    assert!(pretty_json.contains("  \"pubkey\""));

    // Display trait also pretty prints
    let display_output = format!("{}", event);
    assert!(display_output.contains("  \"id\""));

    Ok(())
}

#[test]
fn test_compatibility_with_existing_api() -> Result<()> {
    // Test that both old and new APIs work
    // Using builder to create events (not testing full validation here)
    let event1 = ProtoEventBuilder::new()
        .id("compat_test_1")
        .pubkey("test_pubkey")
        .created_at(1234567890)
        .kind(1)
        .content("API compatibility test")
        .add_tag(vec!["t", "test"])
        .sig("test_sig")
        .build();

    let event2 = ProtoEventBuilder::new()
        .id("compat_test_1")
        .pubkey("test_pubkey")
        .created_at(1234567890)
        .kind(1)
        .content("API compatibility test")
        .add_tag(vec!["t", "test"])
        .sig("test_sig")
        .build();

    // Test PartialEq works
    assert_eq!(event1, event2);

    // Test serde serialization works
    let json1 = serde_json::to_string(&event1)?;
    let json2 = serde_json::to_string(&event2)?;
    assert_eq!(json1, json2);

    // Test serde deserialization works
    let deserialized: ProtoEvent = serde_json::from_str(&json1)?;
    assert_eq!(event1, deserialized);

    // Test Display trait works
    let display_output = format!("{}", event1);
    assert!(display_output.contains("compat_test_1"));

    // Test FromIterator works
    let batch: EventBatch = vec![event1.clone(), event2.clone()].into_iter().collect();
    assert_eq!(batch.events.len(), 2);

    Ok(())
}
