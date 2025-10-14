use proton_beam_core::{
    ProtoEvent, ProtoEventBuilder, validate_event, validation::validate_basic_fields,
};
use std::time::Instant;

fn create_mock_event() -> ProtoEvent {
    ProtoEventBuilder::new()
        .id("4376c65d2f232afbe9b882a35baa4f6fe8667c4e684749af565f981833ed6a65")
        .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
        .created_at(1671217411)
        .kind(1)
        .content("Test event content for validation benchmarking")
        .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
        .build()
}

fn benchmark_basic_validation() {
    println!("\n=== Benchmark: Basic Field Validation ===");

    let event = create_mock_event();
    let num_validations = 1_000_000;
    let start = Instant::now();

    for _ in 0..num_validations {
        let _ = validate_basic_fields(&event);
    }

    let duration = start.elapsed();
    let validations_per_sec = num_validations as f64 / duration.as_secs_f64();

    println!("  Validations: {}", num_validations);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Validations/sec: {:.0}", validations_per_sec);
    println!(
        "  Avg time per validation: {:.2}ns",
        duration.as_nanos() as f64 / num_validations as f64
    );
}

fn benchmark_full_validation() {
    println!("\n=== Benchmark: Full Event Validation (with crypto) ===");

    // Note: This will fail validation because the signature is not real,
    // but we're measuring the performance of the validation attempt
    let event = create_mock_event();
    let num_validations = 10_000;
    let start = Instant::now();

    for _ in 0..num_validations {
        let _ = validate_event(&event);
    }

    let duration = start.elapsed();
    let validations_per_sec = num_validations as f64 / duration.as_secs_f64();

    println!("  Validations: {}", num_validations);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Validations/sec: {:.0}", validations_per_sec);
    println!(
        "  Avg time per validation: {:.2}µs",
        duration.as_micros() as f64 / num_validations as f64
    );
}

fn benchmark_validation_with_tags() {
    println!("\n=== Benchmark: Basic Validation (Event with Multiple Tags) ===");

    let event = ProtoEventBuilder::new()
        .id("4376c65d2f232afbe9b882a35baa4f6fe8667c4e684749af565f981833ed6a65")
        .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
        .created_at(1671217411)
        .kind(1)
        .add_tag(vec!["e", "5c83da77af1dec6d7289834998ad7aafbd9e2191396d75ec3cc27f5a77226f36"])
        .add_tag(vec!["p", "f7234bd4c1394dda46d09f35bd384dd30cc552ad5541990f98844fb06676e9ca"])
        .add_tag(vec!["t", "nostr"])
        .add_tag(vec!["t", "benchmark"])
        .add_tag(vec!["t", "performance"])
        .content("Test event with multiple tags")
        .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
        .build();

    let num_validations = 500_000;
    let start = Instant::now();

    for _ in 0..num_validations {
        let _ = validate_basic_fields(&event);
    }

    let duration = start.elapsed();
    let validations_per_sec = num_validations as f64 / duration.as_secs_f64();

    println!("  Validations: {}", num_validations);
    println!("  Tag count: {}", event.tags.len());
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Validations/sec: {:.0}", validations_per_sec);
    println!(
        "  Avg time per validation: {:.2}ns",
        duration.as_nanos() as f64 / num_validations as f64
    );
}

fn benchmark_invalid_detection() {
    println!("\n=== Benchmark: Invalid Event Detection ===");

    // Create events with various invalid fields
    let invalid_events = vec![
        ProtoEventBuilder::new()
            .id("short") // Invalid: too short
            .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
            .created_at(1671217411)
            .kind(1)
            .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
            .build(),
        ProtoEventBuilder::new()
            .id("4376c65d2f232afbe9b882a35baa4f6fe8667c4e684749af565f981833ed6a65")
            .pubkey("invalid_hex_g".to_string() + &"0".repeat(50)) // Invalid: non-hex characters
            .created_at(1671217411)
            .kind(1)
            .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
            .build(),
        ProtoEventBuilder::new()
            .id("4376c65d2f232afbe9b882a35baa4f6fe8667c4e684749af565f981833ed6a65")
            .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
            .created_at(-1) // Invalid: negative timestamp
            .kind(1)
            .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
            .build(),
        ProtoEventBuilder::new()
            .id("4376c65d2f232afbe9b882a35baa4f6fe8667c4e684749af565f981833ed6a65")
            .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
            .created_at(1671217411)
            .kind(70000) // Invalid: kind out of range
            .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
            .build(),
    ];

    let num_validations = 100_000;
    let start = Instant::now();

    for _ in 0..num_validations {
        for event in &invalid_events {
            let _ = validate_basic_fields(event);
        }
    }

    let duration = start.elapsed();
    let total_checks = num_validations * invalid_events.len() as u64;
    let validations_per_sec = total_checks as f64 / duration.as_secs_f64();

    println!("  Invalid events: {}", invalid_events.len());
    println!("  Total checks: {}", total_checks);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Checks/sec: {:.0}", validations_per_sec);
    println!(
        "  Avg time per check: {:.2}ns",
        duration.as_nanos() as f64 / total_checks as f64
    );
}

fn benchmark_batch_validation() {
    println!("\n=== Benchmark: Batch Validation (10k Events) ===");

    let events: Vec<ProtoEvent> = (0..10_000)
        .map(|i| {
            ProtoEventBuilder::new()
                .id(format!("{:064x}", i))
                .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
                .created_at(1671217411 + i as i64)
                .kind(1)
                .content(format!("Event {}", i))
                .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
                .build()
        })
        .collect();

    let start = Instant::now();

    let valid_count = events
        .iter()
        .filter(|e| validate_basic_fields(e).is_ok())
        .count();

    let duration = start.elapsed();
    let validations_per_sec = events.len() as f64 / duration.as_secs_f64();

    println!("  Events validated: {}", events.len());
    println!("  Valid events: {}", valid_count);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Validations/sec: {:.0}", validations_per_sec);
    println!(
        "  Avg time per event: {:.2}µs",
        duration.as_micros() as f64 / events.len() as f64
    );
}

fn main() {
    println!("╔════════════════════════════════════════════════╗");
    println!("║   Proton Beam Validation Performance Tests    ║");
    println!("╚════════════════════════════════════════════════╝");

    benchmark_basic_validation();
    benchmark_validation_with_tags();
    benchmark_invalid_detection();
    benchmark_batch_validation();
    benchmark_full_validation();

    println!("\n✅ Validation benchmarks complete!");
}
