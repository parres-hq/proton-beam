use proton_beam_core::write_events_delimited;
use proton_beam_core::{
    ProtoEvent, ProtoEventBuilder, validate_event, validation::validate_basic_fields,
};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::time::Instant;
use tempfile::TempDir;

fn create_test_event_json(id: u64) -> String {
    format!(
        r#"{{"id":"{:064x}","pubkey":"79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3","created_at":{},"kind":1,"tags":[["p","79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3"],["t","test"]],"content":"Test event {}","sig":"908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262"}}"#,
        id,
        1671217411 + id as i64,
        id
    )
}

fn create_test_event_proto(id: u64) -> ProtoEvent {
    ProtoEventBuilder::new()
        .id(format!("{:064x}", id))
        .pubkey("79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3")
        .created_at(1671217411 + id as i64)
        .kind(1)
        .add_tag(vec!["p", "79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3"])
        .add_tag(vec!["t", "test"])
        .content(format!("Test event {}", id))
        .sig("908a15e46fb4d8675bab026fc230a0e3542bfade63da02d542fb78b2a8513fcd0092619a2c8c1221e581946e0191f2af505dfdf8657a414dbca329186f009262")
        .build()
}

fn benchmark_end_to_end_conversion() {
    println!("\n=== Benchmark: End-to-End Conversion Pipeline ===");
    println!("  (JSON file → Parse → Validate → Write Protobuf)");

    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.jsonl");
    let output_file = temp_dir.path().join("output.pb");

    // Create input JSONL file
    let num_events = 10_000;
    {
        let mut file = File::create(&input_file).unwrap();
        for i in 0..num_events {
            writeln!(file, "{}", create_test_event_json(i)).unwrap();
        }
    }

    let start = Instant::now();

    // Step 1: Read and parse JSON
    let file = File::open(&input_file).unwrap();
    let reader = BufReader::new(file);
    let events: Vec<ProtoEvent> = std::io::BufRead::lines(reader)
        .map_while(Result::ok)
        .filter_map(|line| ProtoEvent::try_from(line.as_str()).ok())
        .collect();

    // Step 2: Validate (basic only for speed)
    let valid_events: Vec<ProtoEvent> = events
        .into_iter()
        .filter(|e| validate_basic_fields(e).is_ok())
        .collect();

    // Step 3: Write to protobuf
    {
        let file = File::create(&output_file).unwrap();
        let mut writer = BufWriter::new(file);
        write_events_delimited(&mut writer, &valid_events).unwrap();
    }

    let duration = start.elapsed();

    let input_size = std::fs::metadata(&input_file).unwrap().len();
    let output_size = std::fs::metadata(&output_file).unwrap().len();
    let events_per_sec = valid_events.len() as f64 / duration.as_secs_f64();
    let mb_per_sec = (input_size as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();

    println!("  Input events: {}", num_events);
    println!("  Valid events: {}", valid_events.len());
    println!(
        "  Input size: {:.2} MB",
        input_size as f64 / (1024.0 * 1024.0)
    );
    println!(
        "  Output size: {:.2} MB",
        output_size as f64 / (1024.0 * 1024.0)
    );
    println!(
        "  Compression: {:.1}%",
        ((input_size - output_size) as f64 / input_size as f64) * 100.0
    );
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Events/sec: {:.0}", events_per_sec);
    println!("  Throughput: {:.2} MB/s", mb_per_sec);
}

fn benchmark_parsing_only() {
    println!("\n=== Benchmark: JSON Parsing Only (No Validation) ===");

    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.jsonl");

    let num_events = 50_000;
    {
        let mut file = File::create(&input_file).unwrap();
        for i in 0..num_events {
            writeln!(file, "{}", create_test_event_json(i)).unwrap();
        }
    }

    let start = Instant::now();

    let file = File::open(&input_file).unwrap();
    let reader = BufReader::new(file);
    let events: Vec<ProtoEvent> = std::io::BufRead::lines(reader)
        .map_while(Result::ok)
        .filter_map(|line| ProtoEvent::try_from(line.as_str()).ok())
        .collect();

    let duration = start.elapsed();

    let events_per_sec = events.len() as f64 / duration.as_secs_f64();

    println!("  Events parsed: {}", events.len());
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Events/sec: {:.0}", events_per_sec);
    println!(
        "  Avg time per event: {:.2}µs",
        duration.as_micros() as f64 / events.len() as f64
    );
}

fn benchmark_validation_overhead() {
    println!("\n=== Benchmark: Validation Overhead in Pipeline ===");

    let num_events = 10_000;
    let events: Vec<ProtoEvent> = (0..num_events).map(create_test_event_proto).collect();

    // Without validation
    let start_no_validation = Instant::now();
    let _count1 = events.len();
    let duration_no_validation = start_no_validation.elapsed();

    // With basic validation
    let start_basic = Instant::now();
    let count_basic = events
        .iter()
        .filter(|e| validate_basic_fields(e).is_ok())
        .count();
    let duration_basic = start_basic.elapsed();

    // With full validation (will fail, but measures overhead)
    let start_full = Instant::now();
    let count_full = events.iter().filter(|e| validate_event(e).is_ok()).count();
    let duration_full = start_full.elapsed();

    println!("  Events: {}", num_events);
    println!(
        "  No validation: {:.2}s ({:.0} events/s)",
        duration_no_validation.as_secs_f64(),
        num_events as f64 / duration_no_validation.as_secs_f64()
    );
    println!(
        "  Basic validation: {:.2}s ({:.0} events/s, passed: {})",
        duration_basic.as_secs_f64(),
        num_events as f64 / duration_basic.as_secs_f64(),
        count_basic
    );
    println!(
        "  Full validation: {:.2}s ({:.0} events/s, passed: {})",
        duration_full.as_secs_f64(),
        num_events as f64 / duration_full.as_secs_f64(),
        count_full
    );

    if duration_no_validation.as_nanos() > 0 {
        let basic_overhead = ((duration_basic.as_nanos() as f64
            - duration_no_validation.as_nanos() as f64)
            / duration_no_validation.as_nanos() as f64)
            * 100.0;
        let full_overhead = ((duration_full.as_nanos() as f64
            - duration_no_validation.as_nanos() as f64)
            / duration_no_validation.as_nanos() as f64)
            * 100.0;

        println!("  Basic validation overhead: {:.1}%", basic_overhead);
        println!("  Full validation overhead: {:.1}%", full_overhead);
    }
}

fn benchmark_batch_sizes() {
    println!("\n=== Benchmark: Batch Size Performance Analysis ===");

    let temp_dir = TempDir::new().unwrap();
    let num_events = 10_000;
    let events: Vec<ProtoEvent> = (0..num_events).map(create_test_event_proto).collect();

    let batch_sizes = vec![100, 500, 1000, 2000, 5000];

    for batch_size in batch_sizes {
        let output_file = temp_dir.path().join(format!("batch_{}.pb", batch_size));

        let start = Instant::now();
        {
            let file = File::create(&output_file).unwrap();
            let mut writer = BufWriter::new(file);

            for chunk in events.chunks(batch_size) {
                write_events_delimited(&mut writer, chunk).unwrap();
            }
        }
        let duration = start.elapsed();

        let events_per_sec = num_events as f64 / duration.as_secs_f64();

        println!(
            "  Batch size {}: {:.2}s ({:.0} events/s)",
            batch_size,
            duration.as_secs_f64(),
            events_per_sec
        );
    }
}

fn benchmark_memory_efficient_streaming() {
    println!("\n=== Benchmark: Memory-Efficient Streaming Pipeline ===");
    println!("  (Process one event at a time without buffering)");

    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.jsonl");
    let output_file = temp_dir.path().join("output.pb");

    let num_events = 50_000;
    {
        let mut file = File::create(&input_file).unwrap();
        for i in 0..num_events {
            writeln!(file, "{}", create_test_event_json(i)).unwrap();
        }
    }

    let start = Instant::now();
    let mut processed = 0;

    {
        let input = File::open(&input_file).unwrap();
        let reader = BufReader::new(input);
        let output = File::create(&output_file).unwrap();
        let mut writer = BufWriter::new(output);

        for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
            if let Ok(event) = ProtoEvent::try_from(line.as_str())
                && validate_basic_fields(&event).is_ok()
                && proton_beam_core::write_event_delimited(&mut writer, &event).is_ok()
            {
                processed += 1;
            }
        }
    }

    let duration = start.elapsed();

    let input_size = std::fs::metadata(&input_file).unwrap().len();
    let events_per_sec = processed as f64 / duration.as_secs_f64();

    println!("  Events processed: {}", processed);
    println!(
        "  Input size: {:.2} MB",
        input_size as f64 / (1024.0 * 1024.0)
    );
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Events/sec: {:.0}", events_per_sec);
    println!("  (Peak memory: minimal - streaming one event at a time)");
}

fn benchmark_error_handling_overhead() {
    println!("\n=== Benchmark: Error Handling Performance ===");

    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.jsonl");

    // Create mix of valid and invalid events
    let num_events = 10_000;
    {
        let mut file = File::create(&input_file).unwrap();
        for i in 0..num_events {
            if i % 5 == 0 {
                // 20% invalid JSON
                writeln!(file, "{{invalid json").unwrap();
            } else {
                writeln!(file, "{}", create_test_event_json(i)).unwrap();
            }
        }
    }

    let start = Instant::now();

    let file = File::open(&input_file).unwrap();
    let reader = BufReader::new(file);
    let mut valid_count = 0;
    let mut error_count = 0;

    for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
        match ProtoEvent::try_from(line.as_str()) {
            Ok(_) => valid_count += 1,
            Err(_) => error_count += 1,
        }
    }

    let duration = start.elapsed();

    let total_per_sec = num_events as f64 / duration.as_secs_f64();

    println!("  Total lines: {}", num_events);
    println!("  Valid events: {}", valid_count);
    println!("  Errors: {}", error_count);
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Lines/sec: {:.0}", total_per_sec);
    println!(
        "  Error rate: {:.1}%",
        (error_count as f64 / num_events as f64) * 100.0
    );
}

fn benchmark_large_file_processing() {
    println!("\n=== Benchmark: Large File Processing (100k events) ===");

    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("large_input.jsonl");
    let output_file = temp_dir.path().join("large_output.pb");

    let num_events = 100_000;
    {
        let mut file = File::create(&input_file).unwrap();
        for i in 0..num_events {
            writeln!(file, "{}", create_test_event_json(i)).unwrap();
        }
    }

    let input_size = std::fs::metadata(&input_file).unwrap().len();

    let start = Instant::now();

    // Streaming pipeline
    {
        let input = File::open(&input_file).unwrap();
        let reader = BufReader::new(input);
        let output = File::create(&output_file).unwrap();
        let mut writer = BufWriter::new(output);

        for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
            if let Ok(event) = ProtoEvent::try_from(line.as_str()) {
                let _ = proton_beam_core::write_event_delimited(&mut writer, &event);
            }
        }
    }

    let duration = start.elapsed();

    let output_size = std::fs::metadata(&output_file).unwrap().len();
    let events_per_sec = num_events as f64 / duration.as_secs_f64();
    let mb_per_sec = (input_size as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();

    println!("  Events processed: {}", num_events);
    println!(
        "  Input size: {:.2} MB",
        input_size as f64 / (1024.0 * 1024.0)
    );
    println!(
        "  Output size: {:.2} MB",
        output_size as f64 / (1024.0 * 1024.0)
    );
    println!(
        "  Compression: {:.1}%",
        ((input_size - output_size) as f64 / input_size as f64) * 100.0
    );
    println!("  Time taken: {:.2}s", duration.as_secs_f64());
    println!("  Events/sec: {:.0}", events_per_sec);
    println!("  Throughput: {:.2} MB/s", mb_per_sec);
}

fn main() {
    println!("╔════════════════════════════════════════════════╗");
    println!("║     Proton Beam CLI Pipeline Benchmarks       ║");
    println!("╚════════════════════════════════════════════════╝");

    benchmark_end_to_end_conversion();
    benchmark_parsing_only();
    benchmark_validation_overhead();
    benchmark_batch_sizes();
    benchmark_memory_efficient_streaming();
    benchmark_error_handling_overhead();
    benchmark_large_file_processing();

    println!("\n✅ Pipeline benchmarks complete!");
    println!("\n💡 Tips:");
    println!("  - Use larger batch sizes (1000-5000) for better performance");
    println!(
        "  - Skip validation with --validate-signatures=false --validate-event-ids=false for maximum speed"
    );
    println!("  - Streaming mode keeps memory usage constant regardless of file size");
}
