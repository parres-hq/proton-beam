use proton_beam_core::{ProtoEvent, json_to_proto, proto_to_json};
use std::time::Instant;

const SAMPLE_EVENT_JSON: &str = r#"{
    "id":"4376c65d2f232afbe9b882a35baa4f6fe8667c4e684749af565f981833ed6a65",
    "pubkey":"79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3",
    "created_at":1671217411,
    "kind":1,
    "tags":[
        ["e","5c83da77af1dec6d7289834998ad7aafbd9e2191396d75ec3cc27f5a77226f36","wss://nostr.example.com"],
        ["p","f7234bd4c1394dda46d09f35bd384dd30cc552ad5541990f98844fb06676e9ca"]
    ],
    "content":"This is a test event with some content to benchmark conversion performance.",
    "sig":"908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262"
}"#;

const LARGE_CONTENT_EVENT_JSON: &str = r#"{
    "id":"4376c65d2f232afbe9b882a35baa4f6fe8667c4e684749af565f981833ed6a65",
    "pubkey":"79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3",
    "created_at":1671217411,
    "kind":30023,
    "tags":[
        ["d","benchmark-article"],
        ["title","Benchmark Article"],
        ["published_at","1671217411"],
        ["t","nostr"],
        ["t","benchmark"],
        ["t","performance"]
    ],
    "content":"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt. Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
    "sig":"908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262"
}"#;

fn benchmark_json_to_proto_small() {
    println!("\n=== Benchmark: JSON → Proto (Small Event) ===");

    let num_conversions = 100_000;
    let start = Instant::now();

    for _ in 0..num_conversions {
        let _ = json_to_proto(SAMPLE_EVENT_JSON).unwrap();
    }

    let duration = start.elapsed();
    let conversions_per_sec = num_conversions as f64 / duration.as_secs_f64();

    println!("  Conversions: {}", num_conversions);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Conversions/sec: {:.0}", conversions_per_sec);
    println!("  Avg time per conversion: {:.2}µs", duration.as_micros() as f64 / num_conversions as f64);
}

fn benchmark_json_to_proto_large() {
    println!("\n=== Benchmark: JSON → Proto (Large Content Event) ===");

    let num_conversions = 50_000;
    let start = Instant::now();

    for _ in 0..num_conversions {
        let _ = json_to_proto(LARGE_CONTENT_EVENT_JSON).unwrap();
    }

    let duration = start.elapsed();
    let conversions_per_sec = num_conversions as f64 / duration.as_secs_f64();

    println!("  Conversions: {}", num_conversions);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Conversions/sec: {:.0}", conversions_per_sec);
    println!("  Avg time per conversion: {:.2}µs", duration.as_micros() as f64 / num_conversions as f64);
}

fn benchmark_proto_to_json() {
    println!("\n=== Benchmark: Proto → JSON ===");

    let event = json_to_proto(SAMPLE_EVENT_JSON).unwrap();
    let num_conversions = 100_000;
    let start = Instant::now();

    for _ in 0..num_conversions {
        let _ = proto_to_json(&event).unwrap();
    }

    let duration = start.elapsed();
    let conversions_per_sec = num_conversions as f64 / duration.as_secs_f64();

    println!("  Conversions: {}", num_conversions);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Conversions/sec: {:.0}", conversions_per_sec);
    println!("  Avg time per conversion: {:.2}µs", duration.as_micros() as f64 / num_conversions as f64);
}

fn benchmark_round_trip() {
    println!("\n=== Benchmark: Round-Trip Conversion (JSON → Proto → JSON) ===");

    let num_conversions = 50_000;
    let start = Instant::now();

    for _ in 0..num_conversions {
        let event = json_to_proto(SAMPLE_EVENT_JSON).unwrap();
        let _ = proto_to_json(&event).unwrap();
    }

    let duration = start.elapsed();
    let conversions_per_sec = num_conversions as f64 / duration.as_secs_f64();

    println!("  Round trips: {}", num_conversions);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Round trips/sec: {:.0}", conversions_per_sec);
    println!("  Avg time per round trip: {:.2}µs", duration.as_micros() as f64 / num_conversions as f64);
}

fn benchmark_try_from_trait() {
    println!("\n=== Benchmark: TryFrom<&str> Trait (Idiomatic API) ===");

    let num_conversions = 100_000;
    let start = Instant::now();

    for _ in 0..num_conversions {
        let _ = ProtoEvent::try_from(SAMPLE_EVENT_JSON).unwrap();
    }

    let duration = start.elapsed();
    let conversions_per_sec = num_conversions as f64 / duration.as_secs_f64();

    println!("  Conversions: {}", num_conversions);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Conversions/sec: {:.0}", conversions_per_sec);
    println!("  Avg time per conversion: {:.2}µs", duration.as_micros() as f64 / num_conversions as f64);
}

fn benchmark_batch_conversion() {
    println!("\n=== Benchmark: Batch JSON Conversion ===");

    let json_events = vec![SAMPLE_EVENT_JSON; 10_000];
    let start = Instant::now();

    let events: Vec<ProtoEvent> = json_events
        .iter()
        .filter_map(|json| json_to_proto(json).ok())
        .collect();

    let duration = start.elapsed();
    let conversions_per_sec = events.len() as f64 / duration.as_secs_f64();

    println!("  Events converted: {}", events.len());
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Conversions/sec: {:.0}", conversions_per_sec);
    println!("  Avg time per event: {:.2}µs", duration.as_micros() as f64 / events.len() as f64);
}

fn main() {
    println!("╔════════════════════════════════════════════════╗");
    println!("║   Proton Beam Conversion Performance Tests    ║");
    println!("╚════════════════════════════════════════════════╝");

    benchmark_json_to_proto_small();
    benchmark_json_to_proto_large();
    benchmark_proto_to_json();
    benchmark_round_trip();
    benchmark_try_from_trait();
    benchmark_batch_conversion();

    println!("\n✅ Conversion benchmarks complete!");
}

