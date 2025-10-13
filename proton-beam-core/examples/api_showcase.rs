//! Showcase of all the new API features in Proton Beam Core
//!
//! Run with: cargo run --example api_showcase

use proton_beam_core::{EventBatch, ProtoEvent, ProtoEventBuilder, Result};

fn main() -> Result<()> {
    println!("🚀 Proton Beam Core API Showcase\n");

    // Feature 1: Builder Pattern
    println!("1️⃣  Builder Pattern");
    println!("   Building events with fluent API...\n");

    let event1 = ProtoEventBuilder::new()
        .id("event_001")
        .pubkey("pubkey_alice")
        .created_at(1234567890)
        .kind(1)
        .content("Hello from the builder!")
        .add_tag(vec!["t", "showcase"])
        .add_tag(vec!["e", "referenced_event"])
        .sig("signature_001")
        .build();

    println!("   ✅ Built event: {}", event1.id);

    // Feature 2: Display Trait
    println!("\n2️⃣  Display Trait");
    println!("   Pretty-printed JSON for debugging:\n");

    println!("{}", event1);

    // Feature 3: Serde Serialization
    println!("\n3️⃣  Serde Support");
    println!("   Direct JSON serialization (no validation overhead)...\n");

    let json = serde_json::to_string_pretty(&event1)?;
    println!("   Serialized length: {} bytes", json.len());

    let deserialized: ProtoEvent = serde_json::from_str(&json)?;
    println!("   ✅ Deserialized successfully");

    // Feature 4: PartialEq
    println!("\n4️⃣  PartialEq Trait");
    println!("   Comparing events...\n");

    if event1 == deserialized {
        println!("   ✅ Events are equal (round-trip successful!)");
    }

    // Feature 5: FromIterator & Extend
    println!("\n5️⃣  FromIterator & Extend");
    println!("   Collecting events into batches...\n");

    // Create multiple events using builder
    let events: Vec<ProtoEvent> = (1..=5)
        .map(|i| {
            ProtoEventBuilder::new()
                .id(format!("event_{:03}", i))
                .pubkey("pubkey_bob")
                .created_at(1234567890 + i)
                .kind(1)
                .content(format!("Event number {}", i))
                .add_tag(vec!["t", "batch"])
                .sig(format!("sig_{:03}", i))
                .build()
        })
        .collect();

    // Collect into EventBatch using FromIterator
    let batch: EventBatch = events.into_iter().collect();
    println!("   ✅ Collected {} events into batch", batch.events.len());

    // Extend the batch
    let more_events = vec![
        ProtoEventBuilder::new().id("extra_1").kind(1).build(),
        ProtoEventBuilder::new().id("extra_2").kind(1).build(),
    ];

    let mut mutable_batch = batch;
    mutable_batch.extend(more_events);
    println!(
        "   ✅ Extended batch to {} events",
        mutable_batch.events.len()
    );

    // Bonus: Filter and collect
    println!("\n6️⃣  Bonus: Filter & Collect");
    println!("   Filtering events by kind...\n");

    let mixed_events = vec![
        ProtoEventBuilder::new().id("text1").kind(1).build(),
        ProtoEventBuilder::new().id("contacts").kind(3).build(),
        ProtoEventBuilder::new().id("text2").kind(1).build(),
        ProtoEventBuilder::new().id("reaction").kind(7).build(),
    ];

    let text_notes: EventBatch = mixed_events.into_iter().filter(|e| e.kind == 1).collect();

    println!(
        "   ✅ Filtered to {} text notes (kind 1)",
        text_notes.events.len()
    );

    // Summary
    println!("\n✨ Summary");
    println!("   All new features demonstrated successfully!");
    println!("   • Builder Pattern: Fluent event construction");
    println!("   • Display Trait: Pretty-printed debugging");
    println!("   • Serde Support: Fast serialization");
    println!("   • PartialEq: Easy comparisons");
    println!("   • FromIterator: Ergonomic batch creation");
    println!("   • Extend: Add to existing batches");
    println!("   • Filter & Collect: Powerful event processing");

    Ok(())
}
