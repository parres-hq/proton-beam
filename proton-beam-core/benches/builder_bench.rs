use proton_beam_core::{ProtoEvent, ProtoEventBuilder, Tag};
use std::time::Instant;

fn benchmark_builder_minimal() {
    println!("\n=== Benchmark: Builder Pattern (Minimal Event) ===");

    let num_builds = 1_000_000;
    let start = Instant::now();

    for i in 0..num_builds {
        let _ = ProtoEventBuilder::new()
            .id(format!("{:064x}", i))
            .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
            .created_at(1671217411)
            .kind(1)
            .content("Test")
            .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
            .build();
    }

    let duration = start.elapsed();
    let builds_per_sec = num_builds as f64 / duration.as_secs_f64();

    println!("  Events built: {}", num_builds);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Builds/sec: {:.0}", builds_per_sec);
    println!("  Avg time per build: {:.2}ns", duration.as_nanos() as f64 / num_builds as f64);
}

fn benchmark_builder_with_tags() {
    println!("\n=== Benchmark: Builder Pattern (Event with Tags) ===");

    let num_builds = 500_000;
    let start = Instant::now();

    for i in 0..num_builds {
        let _ = ProtoEventBuilder::new()
            .id(format!("{:064x}", i))
            .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
            .created_at(1671217411)
            .kind(1)
            .add_tag(vec!["e", "5c83da77af1dec6d7289834998ad7aafbd9e2191396d75ec3cc27f5a77226f36"])
            .add_tag(vec!["p", "f7234bd4c1394dda46d09f35bd384dd30cc552ad5541990f98844fb06676e9ca"])
            .add_tag(vec!["t", "nostr"])
            .content("Test event with tags")
            .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
            .build();
    }

    let duration = start.elapsed();
    let builds_per_sec = num_builds as f64 / duration.as_secs_f64();

    println!("  Events built: {}", num_builds);
    println!("  Tags per event: 3");
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Builds/sec: {:.0}", builds_per_sec);
    println!("  Avg time per build: {:.2}ns", duration.as_nanos() as f64 / num_builds as f64);
}

fn benchmark_builder_many_tags() {
    println!("\n=== Benchmark: Builder Pattern (Event with Many Tags) ===");

    let num_builds = 100_000;
    let start = Instant::now();

    for i in 0..num_builds {
        let mut builder = ProtoEventBuilder::new()
            .id(format!("{:064x}", i))
            .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
            .created_at(1671217411)
            .kind(1);

        // Add 20 tags
        for j in 0..20 {
            builder = builder.add_tag(vec!["t", &format!("tag_{}", j)]);
        }

        let _ = builder
            .content("Event with many tags")
            .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
            .build();
    }

    let duration = start.elapsed();
    let builds_per_sec = num_builds as f64 / duration.as_secs_f64();

    println!("  Events built: {}", num_builds);
    println!("  Tags per event: 20");
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Builds/sec: {:.0}", builds_per_sec);
    println!("  Avg time per build: {:.2}µs", duration.as_micros() as f64 / num_builds as f64);
}

fn benchmark_direct_construction() {
    println!("\n=== Benchmark: Direct ProtoEvent Construction (vs Builder) ===");

    let num_constructions = 1_000_000;
    let start = Instant::now();

    for i in 0..num_constructions {
        let _ = ProtoEvent {
            id: format!("{:064x}", i),
            pubkey: "79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3".to_string(),
            created_at: 1671217411,
            kind: 1,
            tags: vec![],
            content: "Test".to_string(),
            sig: "908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262".to_string(),
        };
    }

    let duration = start.elapsed();
    let constructions_per_sec = num_constructions as f64 / duration.as_secs_f64();

    println!("  Events constructed: {}", num_constructions);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Constructions/sec: {:.0}", constructions_per_sec);
    println!("  Avg time per construction: {:.2}ns", duration.as_nanos() as f64 / num_constructions as f64);
}

fn benchmark_builder_vs_direct_overhead() {
    println!("\n=== Benchmark: Builder Overhead Analysis ===");

    let num_iterations = 500_000;

    // Benchmark builder
    let start_builder = Instant::now();
    for i in 0..num_iterations {
        let _ = ProtoEventBuilder::new()
            .id(format!("{:064x}", i))
            .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
            .created_at(1671217411)
            .kind(1)
            .content("Test")
            .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
            .build();
    }
    let builder_duration = start_builder.elapsed();

    // Benchmark direct construction
    let start_direct = Instant::now();
    for i in 0..num_iterations {
        let _ = ProtoEvent {
            id: format!("{:064x}", i),
            pubkey: "79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3".to_string(),
            created_at: 1671217411,
            kind: 1,
            tags: vec![],
            content: "Test".to_string(),
            sig: "908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262".to_string(),
        };
    }
    let direct_duration = start_direct.elapsed();

    let overhead_percent = ((builder_duration.as_nanos() as f64 - direct_duration.as_nanos() as f64)
        / direct_duration.as_nanos() as f64) * 100.0;

    println!("  Iterations: {}", num_iterations);
    println!("  Builder time: {:.2}s", builder_duration.as_secs_f64());
    println!("  Direct time: {:.2}s", direct_duration.as_secs_f64());
    println!("  Builder avg: {:.2}ns", builder_duration.as_nanos() as f64 / num_iterations as f64);
    println!("  Direct avg: {:.2}ns", direct_duration.as_nanos() as f64 / num_iterations as f64);
    println!("  Builder overhead: {:.1}%", overhead_percent);
}

fn benchmark_tag_construction_methods() {
    println!("\n=== Benchmark: Tag Construction Methods ===");

    let num_iterations = 500_000;

    // Method 1: add_tag with vec![]
    let start1 = Instant::now();
    for i in 0..num_iterations {
        let _ = ProtoEventBuilder::new()
            .id(format!("{:064x}", i))
            .pubkey("test")
            .add_tag(vec!["e", "event_id"])
            .add_tag(vec!["p", "pubkey_id"])
            .build();
    }
    let duration1 = start1.elapsed();

    // Method 2: add_tag_instance with Tag
    let start2 = Instant::now();
    for i in 0..num_iterations {
        let tag1 = Tag { values: vec!["e".to_string(), "event_id".to_string()] };
        let tag2 = Tag { values: vec!["p".to_string(), "pubkey_id".to_string()] };
        let _ = ProtoEventBuilder::new()
            .id(format!("{:064x}", i))
            .pubkey("test")
            .add_tag_instance(tag1)
            .add_tag_instance(tag2)
            .build();
    }
    let duration2 = start2.elapsed();

    // Method 3: tags() with pre-built Vec
    let start3 = Instant::now();
    for i in 0..num_iterations {
        let tags = vec![
            Tag { values: vec!["e".to_string(), "event_id".to_string()] },
            Tag { values: vec!["p".to_string(), "pubkey_id".to_string()] },
        ];
        let _ = ProtoEventBuilder::new()
            .id(format!("{:064x}", i))
            .pubkey("test")
            .tags(tags)
            .build();
    }
    let duration3 = start3.elapsed();

    println!("  Iterations: {}", num_iterations);
    println!("  Method 1 (add_tag with vec![]): {:.2}s ({:.0} ops/s)",
        duration1.as_secs_f64(),
        num_iterations as f64 / duration1.as_secs_f64());
    println!("  Method 2 (add_tag_instance): {:.2}s ({:.0} ops/s)",
        duration2.as_secs_f64(),
        num_iterations as f64 / duration2.as_secs_f64());
    println!("  Method 3 (tags() bulk): {:.2}s ({:.0} ops/s)",
        duration3.as_secs_f64(),
        num_iterations as f64 / duration3.as_secs_f64());
}

fn benchmark_string_conversion_in_builder() {
    println!("\n=== Benchmark: String Conversion Overhead in Builder ===");

    let num_iterations = 500_000;

    // Using &str (no conversion needed)
    let start_str = Instant::now();
    for i in 0..num_iterations {
        let id = format!("{:064x}", i);
        let _ = ProtoEventBuilder::new()
            .id(&id)
            .pubkey("test")
            .content("content")
            .sig("sig")
            .build();
    }
    let str_duration = start_str.elapsed();

    // Using String (conversion happens)
    let start_string = Instant::now();
    for i in 0..num_iterations {
        let id = format!("{:064x}", i);
        let _ = ProtoEventBuilder::new()
            .id(id.clone())
            .pubkey("test".to_string())
            .content("content".to_string())
            .sig("sig".to_string())
            .build();
    }
    let string_duration = start_string.elapsed();

    println!("  Iterations: {}", num_iterations);
    println!("  Using &str: {:.2}s ({:.0} ops/s)",
        str_duration.as_secs_f64(),
        num_iterations as f64 / str_duration.as_secs_f64());
    println!("  Using String: {:.2}s ({:.0} ops/s)",
        string_duration.as_secs_f64(),
        num_iterations as f64 / string_duration.as_secs_f64());
    println!("  Difference: {:.1}%",
        ((string_duration.as_nanos() as f64 - str_duration.as_nanos() as f64)
        / str_duration.as_nanos() as f64) * 100.0);
}

fn main() {
    println!("╔════════════════════════════════════════════════╗");
    println!("║    Proton Beam Builder Performance Tests      ║");
    println!("╚════════════════════════════════════════════════╝");

    benchmark_builder_minimal();
    benchmark_builder_with_tags();
    benchmark_builder_many_tags();
    benchmark_direct_construction();
    benchmark_builder_vs_direct_overhead();
    benchmark_tag_construction_methods();
    benchmark_string_conversion_in_builder();

    println!("\n✅ Builder benchmarks complete!");
}

