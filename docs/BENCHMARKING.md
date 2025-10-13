# Proton Beam Benchmarking Guide

This guide explains how to run and interpret performance benchmarks for Proton Beam.

## Overview

Proton Beam includes a comprehensive benchmark suite to measure performance across all critical paths:

### Core Library Benchmarks

1. **Conversion Benchmarks** (`conversion_bench`)
   - JSON → Protobuf conversion
   - Protobuf → JSON conversion
   - Round-trip conversions
   - Large content handling
   - Batch conversion performance

2. **Validation Benchmarks** (`validation_bench`)
   - Basic field validation (hex checks, ranges)
   - Full cryptographic validation
   - Invalid event detection
   - Batch validation performance

3. **Storage Benchmarks** (`storage_bench`)
   - Length-delimited write performance
   - Sequential read performance
   - Streaming read (memory-efficient)
   - Large event handling
   - Compression ratio analysis

4. **Builder Benchmarks** (`builder_bench`)
   - Builder pattern overhead
   - Tag construction methods
   - String conversion costs
   - Direct construction comparison

5. **Index Benchmarks** (`index_bench`)
   - Single event insertion
   - Batch insertion performance
   - Contains lookups
   - Query by kind
   - Statistics calculation

### CLI Benchmarks

6. **Pipeline Benchmarks** (`pipeline_bench`)
   - End-to-end conversion pipeline
   - Validation overhead measurement
   - Batch size optimization
   - Memory-efficient streaming
   - Error handling performance
   - Large file processing (100k+ events)

## Running Benchmarks

### Quick Start

**Using just (recommended):**
```bash
# Run all benchmarks
just bench

# Run specific benchmark
just bench-conversion
```

**Using the shell script:**
```bash
# Run all benchmarks
./scripts/run-benchmarks.sh --release

# Fast debug mode
./scripts/run-benchmarks.sh
```

### Selective Benchmarks

**Using just:**
```bash
# Core library only
just bench-core

# CLI only
just bench-cli

# Individual benchmarks
just bench-conversion
just bench-validation
just bench-storage
just bench-builder
just bench-index
just bench-pipeline
```

**Using the shell script:**
```bash
./scripts/run-benchmarks.sh --core --release
./scripts/run-benchmarks.sh --cli --release
./scripts/run-benchmarks.sh --conversion --release
# ... etc
```

### Direct Cargo Commands

You can also run benchmarks directly with cargo:

```bash
# Run a specific benchmark
cargo bench --package proton-beam-core --bench conversion_bench --release

# Run all core benchmarks
cargo bench --package proton-beam-core --release

# Run all CLI benchmarks
cargo bench --package proton-beam-cli --release

# Run all benchmarks in the workspace
cargo bench --workspace --release
```

## Performance Targets

These are the target performance characteristics for Proton Beam:

### Conversion Performance
- **JSON → Proto**: >50,000 events/sec
- **Proto → JSON**: >100,000 events/sec
- **Round-trip**: >25,000 events/sec

### Validation Performance
- **Basic validation**: >500,000 validations/sec
- **Full validation**: >5,000 validations/sec (crypto-heavy)

### Storage Performance
- **Write throughput**: >30 MB/sec
- **Read throughput**: >50 MB/sec
- **Compression ratio**: 2-3x vs JSON

### Index Performance
- **Batch insertions**: >50,000 events/sec
- **Contains lookups**: >100,000 lookups/sec
- **Query by kind**: >1,000 queries/sec

### Pipeline Performance
- **End-to-end**: >10,000 events/sec (with validation)
- **Streaming mode**: >20,000 events/sec (no validation)

## Interpreting Results

### Reading Benchmark Output

Each benchmark displays several metrics:

```
=== Benchmark: JSON → Proto (Small Event) ===
  Conversions: 100000
  Time taken: 1.85s
  Conversions/sec: 54054
  Avg time per conversion: 18.52µs
```

Key metrics:
- **Operations/sec**: Higher is better
- **Avg time per operation**: Lower is better
- **Throughput (MB/s)**: For I/O operations, higher is better

### Regression Detection

To detect performance regressions:

**Using just:**
```bash
# 1. Save baseline
just bench-save
# Note the filename in benchmark-results/

# 2. Make your changes

# 3. Save new results
just bench-save

# 4. Compare
just bench-compare benchmark-results/bench-BASELINE.txt benchmark-results/bench-CURRENT.txt
```

**Manual approach:**
```bash
# 1. Run benchmarks before changes
just bench 2>&1 | tee baseline.txt

# 2. Make your changes

# 3. Run benchmarks again
just bench 2>&1 | tee current.txt

# 4. Compare results
diff baseline.txt current.txt
```

Look for:
- **>10% decrease** in throughput: Investigate immediately
- **>5% decrease**: Worth investigating
- **>10% increase**: Great! Document what improved it

## Optimization Tips

### Based on Benchmark Results

1. **Use batch operations** when processing multiple events:
   - `index.insert_batch()` is ~5-10x faster than individual inserts
   - `write_events_delimited()` is more efficient than multiple writes

2. **Skip validation** when processing trusted data:
   - Full validation adds ~200x overhead due to cryptography
   - Basic validation adds ~10-20% overhead
   - Use `--no-validate` flag for maximum speed

3. **Choose appropriate batch sizes**:
   - 1000-5000 events per batch gives best performance
   - Too small: More overhead
   - Too large: Memory pressure

4. **Use streaming mode** for large files:
   - Constant memory usage regardless of file size
   - Process events one at a time
   - Only slight performance penalty vs buffering

5. **Builder pattern overhead** is minimal:
   - <5% overhead vs direct construction
   - Use it freely for better ergonomics

## Advanced Usage

### Custom Benchmarks

To add new benchmarks:

1. Create a new file in `proton-beam-core/benches/` or `proton-beam-cli/benches/`
2. Follow the pattern of existing benchmarks
3. Register in `Cargo.toml`:
   ```toml
   [[bench]]
   name = "my_bench"
   harness = false
   ```

### Profiling

For detailed profiling, use tools like:

```bash
# CPU profiling with perf (Linux)
cargo bench --package proton-beam-core --bench conversion_bench --release -- --profile-time=5

# Memory profiling with valgrind
valgrind --tool=massif cargo bench --package proton-beam-core --bench storage_bench

# Flame graphs
cargo flamegraph --bench conversion_bench --release
```

### CI/CD Integration

Example GitHub Actions workflow:

```yaml
- name: Run Benchmarks
  run: |
    ./scripts/run-benchmarks.sh --release > benchmark-results.txt

- name: Upload Results
  uses: actions/upload-artifact@v3
  with:
    name: benchmark-results
    path: benchmark-results.txt
```

## Continuous Performance Monitoring

### Tracking Over Time

1. Save benchmark results with git commit info:
   ```bash
   git_hash=$(git rev-parse --short HEAD)
   ./scripts/run-benchmarks.sh --release 2>&1 | tee "benchmarks-${git_hash}.txt"
   ```

2. Store results in a separate branch:
   ```bash
   git checkout -b benchmark-results
   git add "benchmarks-${git_hash}.txt"
   git commit -m "Benchmark results for ${git_hash}"
   git push origin benchmark-results
   ```

### Performance Goals

When optimizing, focus on:

1. **End-to-end throughput**: Most important for users
2. **Memory efficiency**: Especially for large files
3. **Compression ratio**: Balance speed vs size
4. **Index performance**: Critical for deduplication at scale

## Troubleshooting

### Inconsistent Results

If benchmark results vary significantly:

1. **Close other applications** to reduce system noise
2. **Run multiple times** and average results
3. **Check CPU throttling**: Ensure adequate cooling
4. **Disable power saving**: Set CPU governor to performance mode

### Benchmark Failures

If benchmarks fail to compile or run:

1. **Clean build artifacts**:
   ```bash
   cargo clean
   cargo bench --workspace --release
   ```

2. **Check dependencies**:
   ```bash
   cargo update
   ```

3. **Verify test data**:
   Ensure temporary directories have sufficient space

## Contributing

When submitting performance improvements:

1. Run benchmarks before and after your changes
2. Include benchmark results in PR description
3. Explain what improved and why
4. Ensure no regressions in other areas

Example PR description:
```markdown
## Performance Improvement: Index Batch Insertions

### Changes
- Implemented prepared statements for batch inserts
- Reduced transaction overhead

### Benchmark Results
Before: 30,000 inserts/sec
After: 55,000 inserts/sec
Improvement: +83%

### Tradeoffs
None - pure optimization with no functional changes
```

## See Also

- [Architecture Documentation](ARCHITECTURE.md)
- [Developer Guide](DEVELOPER_GUIDE.md)
- [Testing Checklist](TESTING_CHECKLIST.md)

