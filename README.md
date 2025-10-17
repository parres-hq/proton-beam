# Proton Beam ⚡

> Convert Nostr events from JSON to Protocol Buffers at lightspeed

[![Rust](https://img.shields.io/badge/rust-1.90%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Proton Beam is a highly experimental (and will eventually be a high-performance) Rust tool for converting [Nostr](https://nostr.com) events from JSON format to Protocol Buffers (protobuf). It provides both a CLI tool for batch processing and a daemon for real-time relay monitoring.

## Features

- 🚀 **High Performance**: Process 100+ events/second with validated signatures
- 🔒 **Full Validation**: Verify event IDs (SHA-256) and Schnorr signatures
- 📦 **Efficient Storage**: Protobuf + gzip compression (~3x smaller than JSON, 65%+ space savings)
- 🗄️ **Optimized SQLite Index**: Fast event lookups and deduplication (~307K lookups/sec)
  - Bulk insert mode: 2-3x faster for large-scale index rebuilds (500K+ events/sec)
  - Optimized PRAGMAs for multi-billion event datasets
- 🔄 **Real-time Processing**: Connect to multiple Nostr relays simultaneously
- 🎯 **Smart Deduplication**: Events stored once across all relay sources
- 🔍 **Advanced Filtering**: Filter by event kind, author, or tags
- ⚡ **Input Preprocessing**: Ultra-fast regex-based filtering before JSON parsing
- 🌐 **Auto-discovery**: Automatically discover and connect to new relays
- 📊 **Progress Tracking**: Beautiful progress bars for batch operations
- 🔀 **Parallel Processing**: Multi-threaded conversion for maximum throughput
- ☁️ **AWS S3 Support**: Direct upload to S3 buckets (optional feature)
- 💾 **Scalable**: Tested with 1TB+ datasets on commodity hardware

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/proton-beam.git
cd proton-beam

# Build all binaries
cargo build --release

# Install binaries
cargo install --path proton-beam-cli
cargo install --path proton-beam-daemon
```

### CLI Usage

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

Skip validation for faster processing:

```bash
# Skip both signature and ID validation
proton-beam convert events.jsonl --validate-signatures=false --validate-event-ids=false

# Or skip just signatures
proton-beam convert events.jsonl --validate-signatures=false
```

Disable preprocessing filter (enabled by default):

```bash
proton-beam convert events.jsonl --no-filter-kinds
```

Parallel processing with multiple threads:

```bash
proton-beam convert events.jsonl --parallel 8
```

Recover from a failed parallel conversion (merge existing temp files):

```bash
proton-beam merge ./pb_data --cleanup
```

Adjust compression level:

```bash
proton-beam convert events.jsonl --compression-level 9
```

Rebuild the event index from protobuf files:

```bash
# Rebuild index with optimized bulk insert mode (2-3x faster)
proton-beam index rebuild ./pb_data

# Custom index location
proton-beam index rebuild ./pb_data --index-path ./custom/index.db
```

Upload to S3 after conversion (requires `--features s3`):

```bash
# Build with S3 support
cargo build --release --features s3 -p proton-beam-cli

# Convert and upload
proton-beam convert events.jsonl --s3-output s3://my-bucket/output/
```

### AWS Deployment

For processing large datasets on AWS EC2 with automatic S3 upload:

```bash
# Set your configuration
export INPUT_URL="https://example.com/data.jsonl"
export S3_OUTPUT_BUCKET="my-bucket"
export KEY_NAME="my-ec2-keypair"

# Deploy via CloudFormation
./scripts/deploy-cloudformation.sh
```

**Complete guides:**
- [Quick Start](QUICKSTART_CLOUDFORMATION.md) - Get started in 3 steps
- [Complete Guide](CLOUDFORMATION_GUIDE.md) - Full documentation
- [1.2TB Dataset Guide](DEPLOYMENT_1.2TB.md) - Specific configuration example

### Daemon Usage

Start with default configuration:

```bash
proton-beam-daemon start
```

Use custom config:

```bash
proton-beam-daemon start --config config.toml
```

Request historical events:

```bash
proton-beam-daemon start --since 1697000000
```

## Project Structure

```
proton-beam/
├── proton-beam-core/      # Core library (protobuf + conversion)
├── proton-beam-cli/       # CLI tool
├── proton-beam-daemon/    # Relay monitoring daemon
├── docs/                  # Documentation
│   ├── PROJECT_PLAN.md    # Complete project plan
│   └── PROTOBUF_SCHEMA.md # Protobuf schema documentation
└── examples/              # Sample events and configs
```

## Documentation

- **[Project Status & Plan](docs/PROJECT_STATUS.md)**: Current status, progress, and complete roadmap
- **[Architecture](docs/ARCHITECTURE.md)**: System architecture and design decisions
- **[Protobuf Schema](docs/PROTOBUF_SCHEMA.md)**: Detailed schema documentation
- **[Developer Guide](docs/DEVELOPER_GUIDE.md)**: Development setup and workflows
- **[Benchmarking Guide](docs/BENCHMARKING.md)**: Performance benchmarks and optimization tips
- **[Preprocessing Guide](docs/PREPROCESSING.md)**: Input filtering and preprocessing options
- **[Documentation Index](docs/INDEX.md)**: Complete documentation navigation
- **API Documentation**: Run `cargo doc --open`

## Performance Benchmarks

Proton Beam includes comprehensive benchmarks covering all critical paths. Run them with:

```bash
# Using just (recommended)
just bench

# Or using the shell script
./scripts/run-benchmarks.sh --release
```

**Sample Results** (Apple M1 Pro):
- JSON → Protobuf: ~195k events/sec
- Protobuf → JSON: ~845k events/sec
- Basic validation: ~7M validations/sec
- Storage throughput: ~473 MB/sec write, ~810 MB/sec read
- End-to-end pipeline: ~155k events/sec

See [BENCHMARKS_README.md](docs/BENCHMARKS_README.md) for detailed information.

## Configuration Example

`config.toml` for daemon:

```toml
[daemon]
output_dir = "./nostr_events"
batch_size = 500
log_level = "info"

[relays]
urls = [
    "wss://relay.damus.io",
    "wss://nos.lol",
    "wss://relay.primal.net",
    "wss://relay.nostr.band",
    "wss://relay.snort.social",
]
auto_discover = true
max_relays = 50

[filters]
kinds = []        # Empty = all kinds
authors = []      # Empty = all authors

[storage]
deduplicate = true
use_index = true
```

## How It Works

1. **JSON Input**: Accepts Nostr events in JSON format (from files, stdin, or relays)
2. **Validation**: Verifies event ID (SHA-256) and Schnorr signature
3. **Conversion**: Converts to efficient protobuf binary format
4. **Storage**: Organizes events by date (`YYYY_MM_DD.pb`) with length-delimited encoding
5. **Indexing**: Maintains SQLite index for fast deduplication and querying

## Performance

- **Throughput**: 100+ events/second (with full validation)
- **Storage**: ~10-25% smaller than minified JSON
- **Memory**: < 100MB under normal load
- **Validation**: 100% accuracy using nostr-sdk

## Development Status

✅ **Phase 1 Complete**: Core library fully implemented and tested (62/62 tests passing)
✅ **Phase 1.5 Complete**: Enhanced API with builder, Display, Serde, FromIterator
✅ **Phase 2 Complete**: CLI tool with progress bars, date-based storage (18/18 tests passing)
✅ **CI/CD**: Automated testing, linting, formatting, and benchmarks
🚧 **Next Phase**: SQLite Index & Deduplication

See [PROJECT_STATUS.md](docs/PROJECT_STATUS.md) for detailed progress.

### Roadmap

- [x] Phase 1: Core library & protobuf schema ✅
- [x] Phase 1.5: Enhanced API features ✅
- [x] Phase 2: CLI tool ✅
- [ ] Phase 3: SQLite index & deduplication ⏳
- [ ] Phase 4: Relay daemon (core)
- [ ] Phase 5: Relay discovery & advanced features
- [ ] Phase 6: Testing, documentation & polish

## Use Cases

- **Event Archival**: Efficiently archive Nostr events for long-term storage
- **Data Analysis**: Process large datasets of Nostr events
- **Relay Backups**: Create compressed backups of relay data
- **Research**: Analyze Nostr protocol usage and patterns
- **Integration**: Use as a library in other Rust projects

## Technical Details

### Protobuf Schema

```protobuf
// Named ProtoEvent to avoid conflicts with nostr-sdk::Event
message ProtoEvent {
  string id = 1;              // Event ID (hex)
  string pubkey = 2;          // Public key (hex)
  int64 created_at = 3;       // Unix timestamp
  int32 kind = 4;             // Event kind
  repeated Tag tags = 5;      // Tags
  string content = 6;         // Content
  string sig = 7;             // Signature (hex)
}

message Tag {
  repeated string values = 1;
}
```

### Storage Format

Events are stored in date-organized files using length-delimited protobuf:

```
./nostr_events/
├── 2025_10_13.pb       # All events from Oct 13
├── 2025_10_14.pb
├── proton-beam.log     # Error and warning logs
└── index.db            # SQLite index
```

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting PRs.

### Development Setup

```bash
# Clone the repo
git clone https://github.com/yourusername/proton-beam.git
cd proton-beam

# Run tests
cargo test --all

# Run tests with output
cargo test --all -- --nocapture

# Format code
cargo fmt --all

# Lint
cargo clippy --all-targets --all-features

# Or use just commands (recommended)
just test      # Run tests
just fmt       # Check formatting
just lint      # Run clippy
just precommit # Run all pre-commit checks (format, lint, tests, MSRV)
```

### CI/CD

All code is automatically checked on pull requests:
- ✅ Format validation (`rustfmt`)
- ✅ Lint checks (`clippy`)
- ✅ Documentation builds
- ✅ Tests on Linux, macOS, and Windows
- ✅ MSRV compatibility (Rust 1.90+)
- ✅ Security audit
- 📊 Performance benchmarks

See [CI Workflows](.github/workflows/README.md) for details.

## Dependencies

- [nostr-sdk](https://github.com/rust-nostr/nostr): Nostr protocol implementation
- [prost](https://github.com/tokio-rs/prost): Protocol Buffers for Rust
- [tokio](https://tokio.rs/): Async runtime
- [clap](https://github.com/clap-rs/clap): CLI argument parsing
- [rusqlite](https://github.com/rusqlite/rusqlite): SQLite bindings

## License

MIT License - see [LICENSE](LICENSE) for details

## Acknowledgments

- [Nostr Protocol](https://github.com/nostr-protocol/nips) - The protocol this tool supports
- [nostr-sdk](https://github.com/rust-nostr/nostr) - Excellent Rust implementation
- Protocol Buffers - Efficient serialization format

## Support

- 📖 [Documentation](docs/)
- 🐛 [Issue Tracker](https://github.com/yourusername/proton-beam/issues)
- 💬 [Discussions](https://github.com/yourusername/proton-beam/discussions)

## Related Projects

- [nostr-tools](https://github.com/nbd-wtf/nostr-tools) - JavaScript Nostr library
- [nostr-rs-relay](https://github.com/scsibug/nostr-rs-relay) - Rust Nostr relay
- [nostcat](https://github.com/blakejakopovic/nostcat) - Nostr CLI tool

---

**Built with ⚡ and 🦀 by the Nostr community**

