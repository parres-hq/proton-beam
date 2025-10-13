use proton_beam_core::{ProtoEvent, ProtoEventBuilder, write_event_delimited, write_events_delimited, read_events_delimited};
use std::fs::File;
use std::io::{BufWriter, BufReader};
use std::time::Instant;
use tempfile::TempDir;

fn create_test_event(id: u64, kind: i32) -> ProtoEvent {
    ProtoEventBuilder::new()
        .id(format!("{:064x}", id))
        .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
        .created_at(1671217411 + id as i64)
        .kind(kind)
        .content(format!("Test event content number {}", id))
        .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
        .add_tag(vec!["e", &format!("ref_{}", id)])
        .add_tag(vec!["p", "79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3"])
        .build()
}

fn benchmark_write_single() {
    println!("\n=== Benchmark: Write Single Events (Sequential) ===");

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("events.pb");

    let num_events = 10_000;
    let events: Vec<ProtoEvent> = (0..num_events)
        .map(|i| create_test_event(i, 1))
        .collect();

    let start = Instant::now();
    {
        let file = File::create(&file_path).unwrap();
        let mut writer = BufWriter::new(file);

        for event in &events {
            write_event_delimited(&mut writer, event).unwrap();
        }
    }
    let duration = start.elapsed();

    let file_size = std::fs::metadata(&file_path).unwrap().len();
    let events_per_sec = num_events as f64 / duration.as_secs_f64();
    let mb_per_sec = (file_size as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();

    println!("  Events written: {}", num_events);
    println!("  File size: {:.2} MB", file_size as f64 / (1024.0 * 1024.0));
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Events/sec: {:.0}", events_per_sec);
    println!("  Throughput: {:.2} MB/s", mb_per_sec);
    println!("  Avg time per event: {:.2}µs", duration.as_micros() as f64 / num_events as f64);
}

fn benchmark_write_batch() {
    println!("\n=== Benchmark: Write Batch Events ===");

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("events.pb");

    let num_events = 10_000;
    let events: Vec<ProtoEvent> = (0..num_events)
        .map(|i| create_test_event(i, 1))
        .collect();

    let start = Instant::now();
    {
        let file = File::create(&file_path).unwrap();
        let mut writer = BufWriter::new(file);
        write_events_delimited(&mut writer, &events).unwrap();
    }
    let duration = start.elapsed();

    let file_size = std::fs::metadata(&file_path).unwrap().len();
    let events_per_sec = num_events as f64 / duration.as_secs_f64();
    let mb_per_sec = (file_size as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();

    println!("  Events written: {}", num_events);
    println!("  File size: {:.2} MB", file_size as f64 / (1024.0 * 1024.0));
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Events/sec: {:.0}", events_per_sec);
    println!("  Throughput: {:.2} MB/s", mb_per_sec);
    println!("  Avg time per event: {:.2}µs", duration.as_micros() as f64 / num_events as f64);
}

fn benchmark_read_sequential() {
    println!("\n=== Benchmark: Read Events Sequentially ===");

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("events.pb");

    // First, write events
    let num_events = 10_000;
    let events: Vec<ProtoEvent> = (0..num_events)
        .map(|i| create_test_event(i, 1))
        .collect();

    {
        let file = File::create(&file_path).unwrap();
        let mut writer = BufWriter::new(file);
        write_events_delimited(&mut writer, &events).unwrap();
    }

    let file_size = std::fs::metadata(&file_path).unwrap().len();

    // Now benchmark reading
    let start = Instant::now();
    let file = File::open(&file_path).unwrap();
    let reader = BufReader::new(file);
    let read_events: Vec<ProtoEvent> = read_events_delimited(reader)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let duration = start.elapsed();

    let events_per_sec = read_events.len() as f64 / duration.as_secs_f64();
    let mb_per_sec = (file_size as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();

    println!("  Events read: {}", read_events.len());
    println!("  File size: {:.2} MB", file_size as f64 / (1024.0 * 1024.0));
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Events/sec: {:.0}", events_per_sec);
    println!("  Throughput: {:.2} MB/s", mb_per_sec);
    println!("  Avg time per event: {:.2}µs", duration.as_micros() as f64 / read_events.len() as f64);
}

fn benchmark_read_streaming() {
    println!("\n=== Benchmark: Read Events Streaming (Memory Efficient) ===");

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("events.pb");

    // First, write events
    let num_events = 10_000;
    let events: Vec<ProtoEvent> = (0..num_events)
        .map(|i| create_test_event(i, 1))
        .collect();

    {
        let file = File::create(&file_path).unwrap();
        let mut writer = BufWriter::new(file);
        write_events_delimited(&mut writer, &events).unwrap();
    }

    let file_size = std::fs::metadata(&file_path).unwrap().len();

    // Now benchmark streaming read (process one at a time)
    let start = Instant::now();
    let file = File::open(&file_path).unwrap();
    let reader = BufReader::new(file);
    let mut count = 0;
    for result in read_events_delimited(reader) {
        let _event = result.unwrap();
        count += 1;
        // In a real scenario, we'd process the event here without storing all in memory
    }
    let duration = start.elapsed();

    let events_per_sec = count as f64 / duration.as_secs_f64();
    let mb_per_sec = (file_size as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();

    println!("  Events processed: {}", count);
    println!("  File size: {:.2} MB", file_size as f64 / (1024.0 * 1024.0));
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Events/sec: {:.0}", events_per_sec);
    println!("  Throughput: {:.2} MB/s", mb_per_sec);
    println!("  Avg time per event: {:.2}µs", duration.as_micros() as f64 / count as f64);
}

fn benchmark_write_large_events() {
    println!("\n=== Benchmark: Write Large Events (1KB+ content) ===");

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large_events.pb");

    let num_events = 1_000;
    let large_content = "x".repeat(2048); // 2KB content
    let events: Vec<ProtoEvent> = (0..num_events)
        .map(|i| {
            ProtoEventBuilder::new()
                .id(format!("{:064x}", i))
                .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
                .created_at(1671217411 + i as i64)
                .kind(30023) // Long-form content
                .content(&large_content)
                .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
                .add_tag(vec!["d", &format!("article_{}", i)])
                .add_tag(vec!["title", &format!("Article {}", i)])
                .build()
        })
        .collect();

    let start = Instant::now();
    {
        let file = File::create(&file_path).unwrap();
        let mut writer = BufWriter::new(file);
        write_events_delimited(&mut writer, &events).unwrap();
    }
    let duration = start.elapsed();

    let file_size = std::fs::metadata(&file_path).unwrap().len();
    let events_per_sec = num_events as f64 / duration.as_secs_f64();
    let mb_per_sec = (file_size as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();

    println!("  Events written: {}", num_events);
    println!("  Avg event size: ~{:.2} KB", (file_size as f64 / num_events as f64) / 1024.0);
    println!("  File size: {:.2} MB", file_size as f64 / (1024.0 * 1024.0));
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Events/sec: {:.0}", events_per_sec);
    println!("  Throughput: {:.2} MB/s", mb_per_sec);
}

fn benchmark_round_trip_storage() {
    println!("\n=== Benchmark: Round-Trip Storage (Write + Read) ===");

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("roundtrip.pb");

    let num_events = 5_000;
    let events: Vec<ProtoEvent> = (0..num_events)
        .map(|i| create_test_event(i, 1))
        .collect();

    let start = Instant::now();

    // Write
    {
        let file = File::create(&file_path).unwrap();
        let mut writer = BufWriter::new(file);
        write_events_delimited(&mut writer, &events).unwrap();
    }

    // Read
    let file = File::open(&file_path).unwrap();
    let reader = BufReader::new(file);
    let read_events: Vec<ProtoEvent> = read_events_delimited(reader)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let duration = start.elapsed();

    let file_size = std::fs::metadata(&file_path).unwrap().len();
    let round_trips_per_sec = read_events.len() as f64 / duration.as_secs_f64();

    println!("  Events processed: {}", read_events.len());
    println!("  File size: {:.2} MB", file_size as f64 / (1024.0 * 1024.0));
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Round trips/sec: {:.0}", round_trips_per_sec);
    println!("  Avg time per round trip: {:.2}µs", duration.as_micros() as f64 / read_events.len() as f64);
}

fn benchmark_compression_ratio() {
    println!("\n=== Benchmark: Compression Ratio Analysis ===");

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("compression.pb");

    let num_events = 1_000;
    let events: Vec<ProtoEvent> = (0..num_events)
        .map(|i| create_test_event(i, 1))
        .collect();

    // Calculate JSON size
    use proton_beam_core::proto_to_json;
    let json_size: usize = events
        .iter()
        .map(|e| proto_to_json(e).unwrap().len())
        .sum();

    // Write protobuf
    {
        let file = File::create(&file_path).unwrap();
        let mut writer = BufWriter::new(file);
        write_events_delimited(&mut writer, &events).unwrap();
    }

    let pb_size = std::fs::metadata(&file_path).unwrap().len() as usize;
    let compression_ratio = json_size as f64 / pb_size as f64;
    let space_saved = ((json_size - pb_size) as f64 / json_size as f64) * 100.0;

    println!("  Events analyzed: {}", num_events);
    println!("  JSON total size: {:.2} KB", json_size as f64 / 1024.0);
    println!("  Protobuf total size: {:.2} KB", pb_size as f64 / 1024.0);
    println!("  Compression ratio: {:.2}x", compression_ratio);
    println!("  Space saved: {:.1}%", space_saved);
    println!("  Avg JSON event: {:.2} bytes", json_size as f64 / num_events as f64);
    println!("  Avg Protobuf event: {:.2} bytes", pb_size as f64 / num_events as f64);
}

fn main() {
    println!("╔════════════════════════════════════════════════╗");
    println!("║    Proton Beam Storage Performance Tests      ║");
    println!("╚════════════════════════════════════════════════╝");

    benchmark_write_single();
    benchmark_write_batch();
    benchmark_read_sequential();
    benchmark_read_streaming();
    benchmark_write_large_events();
    benchmark_round_trip_storage();
    benchmark_compression_ratio();

    println!("\n✅ Storage benchmarks complete!");
}

