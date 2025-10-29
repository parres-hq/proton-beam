# Proton Beam CLI

Command-line tool for converting Nostr events from JSON to Protocol Buffers.

## Features

- 🚀 **High Performance**: Process 100+ events/second with validated signatures
- 📦 **Efficient Storage**: Protobuf + gzip compression (~3x smaller than JSON)
- 🗄️ **SQLite Index**: Fast event lookups and deduplication (~307K lookups/sec)
- 🔄 **Multiple Input Modes**: Files, stdin, or stream from relays
- 🎯 **Smart Deduplication**: Events stored once across all sources
- 🔍 **Advanced Filtering**: Filter by event kind, author, or tags
- ⚡ **Input Preprocessing**: Ultra-fast regex-based filtering before JSON parsing
- 📊 **Progress Tracking**: Beautiful progress bars for batch operations
- 🔀 **Parallel Processing**: Multi-threaded conversion for maximum throughput
- ☁️ **AWS S3 Support**: Direct upload to S3 buckets (optional feature)
- 💾 **ClickHouse Integration**: Import protobuf data to ClickHouse (optional feature)

## Installation

```bash
# Install from source
cargo install --path .

# With optional features
cargo install --path . --features s3
cargo install --path . --features clickhouse
cargo install --path . --features s3,clickhouse
```

## Usage

### Convert Events

Convert a `.jsonl` file:

```bash
proton-beam convert events.jsonl
```

Read from stdin:

```bash
cat events.jsonl | proton-beam convert -
```

Specify output directory:

```bash
proton-beam convert events.jsonl --output-dir ./pb_data
```

### Validation Options

Skip validation for faster processing:

```bash
# Skip both signature and ID validation
proton-beam convert events.jsonl --validate-signatures=false --validate-event-ids=false

# Or skip just signatures
proton-beam convert events.jsonl --validate-signatures=false
```

### Parallel Processing

Process with multiple threads:

```bash
proton-beam convert events.jsonl --parallel 8
```

Recover from a failed parallel conversion:

```bash
proton-beam merge ./pb_data --cleanup
```

### Compression

Adjust compression level (1-9, default 6):

```bash
proton-beam convert events.jsonl --compression-level 9
```

### Filtering

Input preprocessing (enabled by default):

```bash
# Disable preprocessing filter
proton-beam convert events.jsonl --no-filter-kinds
```

### Index Management

Rebuild the event index:

```bash
# Standard rebuild
proton-beam index rebuild ./pb_data

# With optimized bulk insert mode (2-3x faster)
proton-beam index rebuild ./pb_data --bulk

# Custom index location
proton-beam index rebuild ./pb_data --index-path ./custom/index.db
```

### AWS S3 Upload

Upload to S3 after conversion (requires `--features s3`):

```bash
# Build with S3 support
cargo build --release --features s3

# Convert and upload
proton-beam convert events.jsonl --s3-output s3://my-bucket/output/
```

### ClickHouse Import

Import protobuf data to ClickHouse (requires `--features clickhouse`):

```bash
# Build with ClickHouse support
cargo build --release --features clickhouse

# Import data
proton-beam-clickhouse-import --config clickhouse-import.toml
```

See [ClickHouse documentation](../clickhouse/IMPORT_README.md) for detailed setup instructions.

## Output Format

Events are stored in date-organized files:

```
./pb_data/
├── 2025_10_13.pb.gz    # All events from Oct 13 (gzip compressed)
├── 2025_10_14.pb.gz
├── index.db            # SQLite index for deduplication
└── proton-beam.log     # Error and warning logs
```

Each `.pb.gz` file contains length-delimited protobuf events with gzip compression.

## Performance

**Benchmarks** (Apple M1 Pro):
- End-to-end pipeline: ~155k events/sec
- JSON → Protobuf: ~195k events/sec
- Protobuf → JSON: ~845k events/sec
- Storage throughput: ~473 MB/sec write, ~810 MB/sec read
- Index lookups: ~307k lookups/sec

Run benchmarks:

```bash
cargo bench -p proton-beam-cli
```

## Configuration

The CLI uses command-line flags for configuration. For daemon-based processing with configuration files, see [proton-beam-daemon](../proton-beam-daemon/).

## Storage Format

Events use length-delimited protobuf encoding:

```
[varint length][Event 1 binary]
[varint length][Event 2 binary]
...
```

This allows:
- Append-only writes
- Streaming reads without loading entire file
- Memory-efficient processing

## Examples

See the [examples directory](../examples/) for:
- Sample event files
- Conversion scripts
- Common workflows
- Configuration examples

## License

MIT

## See Also

- [Core Library](../proton-beam-core/README.md) - Conversion library API
- [Examples](../examples/README.md) - Usage examples and scripts
- [Architecture](../docs/ARCHITECTURE.md) - System design
- [Main README](../README.md) - Project overview

