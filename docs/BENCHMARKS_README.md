# Proton Beam Benchmarks

This directory contains comprehensive performance benchmarks for Proton Beam.

## Quick Start

**Using just (recommended):**
```bash
# Run all benchmarks
just bench

# Run specific benchmarks
just bench-core          # Core library only
just bench-cli           # CLI only
just bench-conversion    # Individual benchmark
```

**Using the shell script:**
```bash
./scripts/run-benchmarks.sh --release
```

**Using cargo directly:**
```bash
cargo bench --workspace
cargo bench --package proton-beam-core --bench conversion_bench
```

## Benchmark Suite Overview

### Core Library Benchmarks (proton-beam-core)

| Benchmark | Focus Area | Key Metrics |
|-----------|------------|-------------|
| **conversion_bench** | JSON ↔ Protobuf conversion | Conversions/sec, latency |
| **validation_bench** | Event validation (basic & full) | Validations/sec, crypto overhead |
| **storage_bench** | File I/O & serialization | MB/sec throughput, compression ratio |
| **builder_bench** | Builder pattern performance | Builds/sec, overhead analysis |
| **index_bench** | SQLite index operations | Inserts/sec, lookups/sec |

### CLI Benchmarks (proton-beam-cli)

| Benchmark | Focus Area | Key Metrics |
|-----------|------------|-------------|
| **pipeline_bench** | End-to-end conversion pipeline | Events/sec, memory efficiency |

## What Gets Measured

### 1. Conversion Benchmarks
- JSON → Proto conversion speed (small and large events)
- Proto → JSON conversion speed
- Round-trip conversions
- Batch conversion performance
- TryFrom trait overhead

**Why it matters**: Conversion is the core operation - faster conversion means faster processing.

### 2. Validation Benchmarks
- Basic field validation (hex checks, ranges)
- Full cryptographic validation (SHA-256, Schnorr)
- Invalid event detection speed
- Batch validation efficiency

**Why it matters**: Validation is optional but important for untrusted data. Understanding the overhead helps users make informed decisions.

### 3. Storage Benchmarks
- Sequential write performance
- Batch write performance
- Sequential read performance
- Streaming read (memory-efficient)
- Large event handling (1KB+ content)
- Round-trip storage performance
- Compression ratio analysis

**Why it matters**: Storage I/O is often the bottleneck in data processing. These benchmarks help optimize file handling.

### 4. Builder Benchmarks
- Minimal event construction
- Event with tags
- Many tags (20+)
- Direct construction comparison
- Tag construction methods
- String conversion overhead

**Why it matters**: The builder pattern should have minimal overhead. These benchmarks ensure ergonomics don't sacrifice performance.

### 5. Index Benchmarks
- Single event insertion
- Batch insertion (500 events)
- Contains lookups
- Query by kind
- Statistics calculation

**Why it matters**: The index enables fast deduplication and queries. Poor index performance would slow down the entire pipeline.

### 6. Pipeline Benchmarks
- End-to-end conversion (JSON file → Parse → Validate → Write PB)
- Parsing-only performance
- Validation overhead measurement
- Batch size optimization
- Memory-efficient streaming
- Error handling performance
- Large file processing (100k+ events)

**Why it matters**: These benchmarks simulate real-world usage and help identify bottlenecks in the complete workflow.

## Performance Results (Reference Hardware)

**Hardware**: Apple Silicon M1 Pro, 32GB RAM, macOS

### Core Library Performance

| Operation | Throughput | Notes |
|-----------|-----------|-------|
| JSON → Proto | ~195k/sec | Small events with tags |
| Proto → JSON | ~845k/sec | 4x faster than parsing |
| Round-trip | ~159k/sec | Full conversion cycle |
| Basic validation | ~500k+/sec | Fast field checks |
| Full validation | ~10k/sec | Crypto-heavy |
| Storage write | ~30-50 MB/sec | Depends on event size |
| Storage read | ~50+ MB/sec | Streaming mode |
| Index insert (batch) | ~50k/sec | 500-event batches |
| Index lookup | ~100k+/sec | Contains queries |

### CLI Pipeline Performance

| Pipeline | Throughput | Notes |
|----------|-----------|-------|
| End-to-end (validated) | ~10-15k/sec | Full pipeline with validation |
| End-to-end (no validation) | ~20-30k/sec | Maximum speed |
| Streaming mode | ~20k+/sec | Constant memory |

### Space Efficiency

| Metric | Value |
|--------|-------|
| Compression ratio | 2-3x | vs JSON |
| Avg event (JSON) | ~400 bytes | With 2 tags |
| Avg event (Protobuf) | ~150 bytes | 62% smaller |

## Optimization Insights

Based on benchmark results:

1. **✅ Batch operations are highly recommended**
   - Index: 5-10x faster
   - Storage: 20-30% faster
   - Use batch sizes of 1000-5000 for best results

2. **✅ Validation has significant overhead**
   - Basic: ~10-20% overhead
   - Full (crypto): ~200x overhead
   - Skip when processing trusted data

3. **✅ Streaming mode is memory-efficient**
   - Only ~5-10% slower than buffering
   - Constant memory regardless of file size
   - Recommended for large files

4. **✅ Builder pattern has minimal overhead**
   - <5% vs direct construction
   - Use freely for ergonomics

5. **✅ Protobuf compression is excellent**
   - 2-3x smaller than JSON
   - Faster to parse
   - Better for long-term storage

## Running Benchmarks in CI

Example GitHub Actions workflow:

```yaml
name: Benchmarks

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          profile: minimal

      - name: Run Benchmarks
        run: |
          ./scripts/run-benchmarks.sh --release > benchmark-results.txt

      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: benchmark-results.txt
```

## Comparing Performance

To compare performance across changes:

```bash
# Baseline
git checkout main
./scripts/run-benchmarks.sh --release 2>&1 | tee baseline.txt

# Your changes
git checkout feature-branch
./scripts/run-benchmarks.sh --release 2>&1 | tee feature.txt

# Compare
diff baseline.txt feature.txt
```

## Adding New Benchmarks

1. Create a new file in the appropriate benches directory:
   - `proton-beam-core/benches/` for core functionality
   - `proton-beam-cli/benches/` for CLI functionality

2. Follow the existing benchmark pattern:
   ```rust
   use std::time::Instant;

   fn benchmark_something() {
       println!("\n=== Benchmark: Something ===");
       let start = Instant::now();
       // ... benchmark code ...
       let duration = start.elapsed();
       println!("  Time taken: {:.2}s", duration.as_secs_f64());
   }

   fn main() {
       benchmark_something();
   }
   ```

3. Register in `Cargo.toml`:
   ```toml
   [[bench]]
   name = "my_bench"
   harness = false
   ```

4. Update `scripts/run-benchmarks.sh` to include the new benchmark

## Troubleshooting

### Benchmarks are slow to compile
- Use debug mode for development: `./scripts/run-benchmarks.sh` (no --release)
- Only use --release for final measurements

### Results are inconsistent
- Close other applications
- Run multiple times and average
- Check CPU throttling
- Disable power saving mode

### Out of disk space
- Benchmarks create temporary files
- Clean up: `cargo clean`
- Check available space: `df -h`

## Documentation

For detailed information, see:
- [BENCHMARKING.md](docs/BENCHMARKING.md) - Complete benchmarking guide
- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - System architecture
- [DEVELOPER_GUIDE.md](docs/DEVELOPER_GUIDE.md) - Development guide

## Contributing

When submitting performance improvements:
1. ✅ Run benchmarks before and after
2. ✅ Include results in PR description
3. ✅ Explain the optimization
4. ✅ Note any tradeoffs
5. ✅ Ensure no regressions elsewhere

---

**Note**: Benchmark results vary by hardware. The numbers above are reference values and may differ on your system.

