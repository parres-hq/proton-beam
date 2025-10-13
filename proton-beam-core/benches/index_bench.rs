use proton_beam_core::{EventIndex, ProtoEventBuilder};
use std::time::Instant;
use tempfile::TempDir;

fn create_test_event(id: &str, kind: i32, pubkey: &str, created_at: i64) -> proton_beam_core::ProtoEvent {
    ProtoEventBuilder::new()
        .id(id)
        .kind(kind)
        .pubkey(pubkey)
        .created_at(created_at)
        .content("benchmark content")
        .sig("0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")
        .build()
}

fn benchmark_insert_single() {
    println!("\n=== Benchmark: Single Event Insertions ===");

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");
    let mut index = EventIndex::new(&db_path).unwrap();

    let num_events = 10_000;
    let start = Instant::now();

    for i in 0..num_events {
        let event = create_test_event(
            &format!("{:064x}", i),
            1,
            "pubkey_bench",
            1234567890 + i as i64,
        );
        index.insert(&event, "bench.pb").unwrap();
    }

    let duration = start.elapsed();
    let events_per_sec = num_events as f64 / duration.as_secs_f64();

    println!("  Events inserted: {}", num_events);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Events/sec: {:.0}", events_per_sec);
}

fn benchmark_insert_batch() {
    println!("\n=== Benchmark: Batch Event Insertions ===");

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");
    let mut index = EventIndex::new(&db_path).unwrap();

    let num_events = 10_000;
    let batch_size = 500;

    let start = Instant::now();

    for batch_start in (0..num_events).step_by(batch_size) {
        let batch: Vec<_> = (batch_start..batch_start + batch_size.min(num_events - batch_start))
            .map(|i| {
                let event = create_test_event(
                    &format!("{:064x}", i),
                    1,
                    "pubkey_bench",
                    1234567890 + i as i64,
                );
                (event, "bench.pb")
            })
            .collect();

        let batch_refs: Vec<_> = batch.iter().map(|(e, f)| (e, *f)).collect();
        index.insert_batch(&batch_refs).unwrap();
    }

    let duration = start.elapsed();
    let events_per_sec = num_events as f64 / duration.as_secs_f64();

    println!("  Events inserted: {}", num_events);
    println!("  Batch size: {}", batch_size);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Events/sec: {:.0}", events_per_sec);
}

fn benchmark_contains() {
    println!("\n=== Benchmark: Contains Lookups ===");

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");
    let mut index = EventIndex::new(&db_path).unwrap();

    // Insert events
    let num_events = 10_000;
    let events: Vec<_> = (0..num_events)
        .map(|i| create_test_event(
            &format!("{:064x}", i),
            1,
            "pubkey_bench",
            1234567890 + i as i64,
        ))
        .collect();

    let batch_refs: Vec<_> = events.iter().map(|e| (e, "bench.pb")).collect();
    index.insert_batch(&batch_refs).unwrap();

    // Benchmark lookups
    let num_lookups = 100_000;
    let start = Instant::now();

    for i in 0..num_lookups {
        let id = format!("{:064x}", i % num_events);
        let _ = index.contains(&id).unwrap();
    }

    let duration = start.elapsed();
    let lookups_per_sec = num_lookups as f64 / duration.as_secs_f64();

    println!("  Index size: {} events", num_events);
    println!("  Lookups performed: {}", num_lookups);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Lookups/sec: {:.0}", lookups_per_sec);
}

fn benchmark_query_by_kind() {
    println!("\n=== Benchmark: Query by Kind ===");

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");
    let mut index = EventIndex::new(&db_path).unwrap();

    // Insert events with different kinds
    let num_events = 10_000;
    let events: Vec<_> = (0..num_events)
        .map(|i| create_test_event(
            &format!("{:064x}", i),
            i % 10, // 10 different kinds
            "pubkey_bench",
            1234567890 + i as i64,
        ))
        .collect();

    let batch_refs: Vec<_> = events.iter().map(|e| (e, "bench.pb")).collect();
    index.insert_batch(&batch_refs).unwrap();

    // Benchmark queries
    let num_queries = 100;
    let start = Instant::now();

    for i in 0..num_queries {
        let kind = i % 10;
        let _ = index.query_by_kind(kind).unwrap();
    }

    let duration = start.elapsed();
    let queries_per_sec = num_queries as f64 / duration.as_secs_f64();

    println!("  Index size: {} events", num_events);
    println!("  Queries performed: {}", num_queries);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Queries/sec: {:.0}", queries_per_sec);
}

fn benchmark_stats() {
    println!("\n=== Benchmark: Stats Calculation ===");

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");
    let mut index = EventIndex::new(&db_path).unwrap();

    // Insert events in batches
    let num_events = 100_000;
    let batch_size = 1000;

    for batch_start in (0..num_events).step_by(batch_size) {
        let batch: Vec<_> = (batch_start..batch_start + batch_size.min(num_events - batch_start))
            .map(|i| {
                let event = create_test_event(
                    &format!("{:064x}", i),
                    (i % 10) as i32,
                    &format!("pubkey_{}", i % 100),
                    1234567890 + i as i64);
                (event, format!("file_{}.pb", i / 1000))
            })
            .collect();

        let batch_refs: Vec<_> = batch.iter().map(|(e, f)| (e, f.as_str())).collect();
        index.insert_batch(&batch_refs).unwrap();
    }

    // Benchmark stats
    let num_calls = 1000;
    let start = Instant::now();

    for _ in 0..num_calls {
        let _ = index.stats().unwrap();
    }

    let duration = start.elapsed();
    let calls_per_sec = num_calls as f64 / duration.as_secs_f64();

    let stats = index.stats().unwrap();
    println!("  Index size: {} events", stats.total_events);
    println!("  Unique files: {}", stats.unique_files);
    println!("  Unique pubkeys: {}", stats.unique_pubkeys);
    println!("  Stats calls: {}", num_calls);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Calls/sec: {:.0}", calls_per_sec);
}

fn main() {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║   Proton Beam Index Performance Benchmarks   ║");
    println!("╚═══════════════════════════════════════════════╝");

    benchmark_insert_single();
    benchmark_insert_batch();
    benchmark_contains();
    benchmark_query_by_kind();
    benchmark_stats();

    println!("\n✅ Benchmarks complete!");
}

