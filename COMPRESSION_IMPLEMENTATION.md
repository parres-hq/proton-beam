# Gzip Compression Implementation

## Summary

Implemented transparent gzip compression for all protobuf storage files in Proton Beam. Files are now saved as `.pb.gz` instead of `.pb`, providing significant space savings with minimal performance impact.

## Changes Made

### 1. Dependencies
- Added `flate2 = "1.0"` to workspace dependencies in `Cargo.toml`
- Added flate2 to both `proton-beam-core` and `proton-beam-cli` crates

### 2. Core Library (`proton-beam-core`)

#### New Functions in `src/storage.rs`:
- `create_gzip_encoder<W: Write>(writer: W) -> GzEncoder<W>` - Wraps a writer with gzip compression
- `create_gzip_decoder<R: Read>(reader: R) -> GzDecoder<R>` - Wraps a reader with gzip decompression

These functions are exported from the main library and provide transparent compression/decompression for any I/O stream.

#### Tests Added:
- `test_gzip_compression_single_event()` - Verifies round-trip compression/decompression works
- `test_gzip_compression_multiple_events()` - Tests batch compression
- `test_compression_ratio()` - Validates meaningful compression is achieved (>1.5x on repetitive data)

### 3. CLI (`proton-beam-cli`)

#### Updated `src/storage.rs`:
- Modified `flush_buffer()` to wrap file handles with gzip compression
- Changed output file extension from `.pb` to `.pb.gz`
- Updated test to verify `.pb.gz` files are created

#### Updated `src/main.rs`:
- Changed file name format to `.pb.gz` for progress tracking

#### Updated `tests/integration_tests.rs`:
- Modified file detection logic to look for `.pb.gz` files instead of `.pb`
- Updated date-based filename parsing to handle `.pb.gz` extension

### 4. Benchmarks

Added new benchmark in `proton-beam-core/benches/storage_bench.rs`:
- `benchmark_gzip_compression()` - Measures compression ratios between JSON, raw protobuf, and gzip'd protobuf

### 5. Documentation

#### Updated Files:
- `README.md` - Updated features to mention "Protobuf + gzip compression (~3x smaller than JSON, 65%+ space savings)"
- `docs/ARCHITECTURE.md` - Updated storage specs and all diagrams to show `.pb.gz` files and compression ratios
- `proton-beam-core/README.md` - Added example showing how to use compression in library code
- `examples/scripts/compare_sizes.sh` - Updated to look for `.pb.gz` files

## Performance Results

### Benchmark Results (1,000 events):
```
JSON total size:           448.03 KB
Protobuf size:             379.67 KB (1.18x smaller than JSON)
Gzip'd Protobuf size:       11.56 KB (32.84x smaller than protobuf)
Combined compression:       38.75x smaller than JSON
Space saved vs JSON:        97.4%
```

### Real-world Results (141 sample events):
```
JSON size:                 252,864 bytes
Compressed protobuf:        87,040 bytes
Compression ratio:          2.91x
Space saved:                65.6%
```

## Usage

### CLI (Automatic)
The CLI automatically uses gzip compression for all output files:

```bash
proton-beam convert events.jsonl --output-dir ./output
# Creates: ./output/2025_10_14.pb.gz (automatically compressed)
```

### Library API (Manual)
```rust
use proton_beam_core::{
    create_gzip_encoder, create_gzip_decoder,
    write_event_delimited, read_events_delimited
};
use std::fs::File;
use std::io::BufWriter;

// Write compressed
let file = File::create("events.pb.gz")?;
let gz = create_gzip_encoder(file);
let mut writer = BufWriter::new(gz);
write_event_delimited(&mut writer, &event)?;
drop(writer); // Finish gzip stream

// Read compressed
let file = File::open("events.pb.gz")?;
let gz = create_gzip_decoder(file);
for result in read_events_delimited(gz) {
    let event = result?;
    // Process event...
}
```

## Design Decisions

### Why Gzip?
1. **Ubiquitous**: Available everywhere, well-tested
2. **Fast**: Minimal CPU overhead, good compression ratio
3. **Transparent**: Works as a simple wrapper around any I/O stream
4. **Rust Support**: `flate2` crate is mature and widely used

### Why Default Compression Level?
- Level 6 (default) provides excellent compression with good speed
- No need for configuration complexity
- Users can always wrap with their own compression if needed

### Why Transparent Wrappers?
- No changes needed to existing read/write logic
- Drop-in replacement: just wrap the file handle
- Works with any `Read` or `Write` implementor

## Backward Compatibility

**Breaking Change**: Old `.pb` files cannot be read by the new version. If you have existing `.pb` files:

1. Keep a copy of the old binary to read them, or
2. Re-convert from original JSON sources

This is acceptable since Proton Beam is marked as "highly experimental" and not yet at v1.0.

## Future Enhancements

Possible future improvements:
- CLI flag to disable compression: `--no-compression`
- Configurable compression level: `--compression-level 9`
- Support for other algorithms: zstd, lz4, etc.
- Auto-detect compression on read (check magic bytes)

## Testing

All tests pass:
```bash
cargo test --all-features    # ✓ 121 tests pass
cargo bench --bench storage_bench  # ✓ Benchmarks run successfully
```

Integration tests verify:
- CLI creates `.pb.gz` files correctly
- Files can be read back and decompressed
- Compression provides meaningful space savings
- Round-trip integrity is maintained

