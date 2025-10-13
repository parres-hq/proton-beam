# Proton Beam - Project Plan

**Version:** 1.0
**Last Updated:** 2025-10-13

## Executive Summary

Proton Beam is a high-performance Rust-based tool for converting Nostr events from JSON format to Protocol Buffer (protobuf) format. It provides both a CLI tool for batch conversion and a daemon for real-time relay monitoring and conversion.

## Project Goals

1. **JSON to Protobuf Conversion**: Convert Nostr events from JSON to a space-efficient protobuf format
2. **CLI Tool**: Process `.jsonl` files, raw JSON, or stdin streams with progress indication
3. **Relay Daemon**: Connect to multiple Nostr relays, capture events in real-time, and store as protobuf
4. **Validation**: Validate event IDs (SHA-256) and Schnorr signatures before conversion
5. **Deduplication**: Ensure events are only stored once across multiple relay sources
6. **Filtering**: Support configurable filtering by event kind, author, and tags
7. **Performance**: Handle 10-100+ events/second with batched writes
8. **Relay Discovery**: Automatically discover and connect to new relays via event tags

## Architecture Overview

### Workspace Structure

```
proton-beam/
├── Cargo.toml                    # Workspace root
├── proton-beam-core/             # Core library (protobuf schema + conversion)
│   ├── Cargo.toml
│   ├── build.rs                  # Build script for protobuf codegen
│   ├── proto/
│   │   └── nostr.proto           # Protobuf schema definition
│   └── src/
│       ├── lib.rs
│       ├── event.rs              # Event conversion logic
│       ├── validation.rs         # ID & signature validation
│       ├── error.rs              # Error types
│       └── storage.rs            # Length-delimited protobuf I/O
├── proton-beam-cli/              # CLI binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── input.rs              # Handle file, stdin, raw JSON input
│       └── progress.rs           # Progress bar implementation
├── proton-beam-daemon/           # Relay monitoring daemon
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs             # TOML configuration
│       ├── relay_manager.rs     # Relay connection management
│       ├── event_processor.rs   # Event batching & deduplication
│       ├── storage_manager.rs   # File writing & organization
│       ├── index.rs              # SQLite event index
│       └── discovery.rs          # Relay discovery logic
├── docs/
│   ├── PROJECT_PLAN.md           # This file
│   ├── PROTOBUF_SCHEMA.md        # Protobuf schema documentation
│   └── USAGE.md                  # User guide
├── examples/
│   └── sample_events.jsonl       # Sample Nostr events for testing
└── README.md
```

## Protobuf Schema Design

### Core Event Message

```protobuf
syntax = "proto3";

package nostr;

message Event {
  string id = 1;              // 32-byte hex-encoded event ID
  string pubkey = 2;          // 32-byte hex-encoded public key
  int64 created_at = 3;       // Unix timestamp
  int32 kind = 4;             // Event kind (0-65535)
  repeated Tag tags = 5;      // Event tags
  string content = 6;         // Event content
  string sig = 7;             // 64-byte hex-encoded signature
}

message Tag {
  repeated string values = 1; // Tag values (array of strings)
}

message EventBatch {
  repeated Event events = 1;  // For testing/convenience
}
```

**Design Rationale:**
- **Single generic Event message**: Supports all event kinds without specialized messages
- **Typed Tag message**: Clean structure for tag arrays
- **Length-delimited streaming**: Events written with length prefixes for append-only storage
- **Hex-encoded strings**: Preserve Nostr's standard encoding (id, pubkey, sig)

## Component Specifications

### 1. Core Library (`proton-beam-core`)

**Responsibilities:**
- Define and compile protobuf schema
- Convert JSON events to/from protobuf
- Validate event IDs (SHA-256 hash verification)
- Validate Schnorr signatures on secp256k1
- Read/write length-delimited protobuf streams

**Key Dependencies:**
- `prost` + `prost-build`: Protobuf code generation
- `nostr-sdk`: Event validation and signature verification
- `serde_json`: JSON parsing
- `sha2`, `secp256k1`: Cryptographic validation

**Public API:**
```rust
pub struct NostrEvent { /* protobuf-generated */ }

pub fn json_to_proto(json: &str) -> Result<NostrEvent, Error>;
pub fn proto_to_json(event: &NostrEvent) -> Result<String, Error>;
pub fn validate_event(event: &NostrEvent) -> Result<(), ValidationError>;
pub fn write_event_delimited<W: Write>(writer: W, event: &NostrEvent) -> Result<()>;
pub fn read_events_delimited<R: Read>(reader: R) -> impl Iterator<Item = Result<NostrEvent>>;
```

### 2. CLI Tool (`proton-beam-cli`)

**Usage:**
```bash
# Convert from file
proton-beam convert events.jsonl

# From stdin
cat events.jsonl | proton-beam convert -

# Output to specific directory
proton-beam convert events.jsonl --output-dir ./pb_data

# Skip validation (faster, but dangerous)
proton-beam convert events.jsonl --no-validate
```

**Features:**
- Read `.jsonl` files line-by-line
- Accept input from stdin
- Display progress bar based on file size or line count
- Write events to date-organized protobuf files (`YYYY_MM_DD.pb`)
- Log malformed events to `errors.jsonl` with error reasons
- Batch writes (configurable, default 500 events)

**Output Structure:**
```
./pb_data/
├── 2025_10_13.pb        # Events from Oct 13, 2025
├── 2025_10_14.pb
├── errors.jsonl         # Malformed events with error reasons
└── .index.db            # SQLite index (optional for CLI)
```

### 3. Relay Daemon (`proton-beam-daemon`)

**Usage:**
```bash
# Start with default config
proton-beam-daemon start

# Specify config file
proton-beam-daemon start --config /path/to/config.toml

# Request historical events since timestamp
proton-beam-daemon start --since 1697000000
```

**Configuration (`config.toml`):**
```toml
[daemon]
output_dir = "./nostr_events"
batch_size = 500              # Write every N events
log_level = "info"

[relays]
# Initial relay list
urls = [
    "wss://relay.damus.io",
    "wss://nos.lol",
    "wss://relay.primal.net",
    "wss://relay.nostr.band",
    "wss://relay.snort.social",
]
auto_discover = true          # Discover relays from event tags
max_relays = 50               # Max concurrent relay connections

[filters]
# Only store these kinds (empty = all)
kinds = []
# Only from these authors (empty = all)
authors = []
# Tag filters (e.g., only events with 'p' tag containing specific pubkey)
# tags.p = ["pubkey1", "pubkey2"]

[historical]
enabled = false
since_timestamp = 0           # Unix timestamp; 0 = no historical

[storage]
deduplicate = true
use_index = true              # Maintain SQLite index for deduplication
```

**Features:**
- Connect to multiple relays via WebSocket
- Subscribe to filtered events (by kind, author, tags)
- Deduplicate events by ID across all relays
- Batch write events every N events or M seconds
- Automatically discover relays from event tags (`r`, `relay`, `e`, `p`, `a` tag hints)
- Maintain SQLite index of event IDs and file locations
- Graceful shutdown (SIGTERM/SIGINT) with buffer flush
- Reconnect on relay disconnections with exponential backoff

**SQLite Index Schema:**
```sql
CREATE TABLE events (
    id TEXT PRIMARY KEY,      -- Event ID (hex)
    kind INTEGER NOT NULL,
    pubkey TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    file_path TEXT NOT NULL,  -- Path to .pb file containing this event
    indexed_at INTEGER NOT NULL
);

CREATE INDEX idx_kind ON events(kind);
CREATE INDEX idx_pubkey ON events(pubkey);
CREATE INDEX idx_created_at ON events(created_at);
```

**Relay Discovery Logic:**
- Parse `r` tags (relay URLs with markers: `read`, `write`)
- Parse `relay` tags (standalone relay URLs)
- Extract relay hints from `e`, `p`, `a` tags (3rd parameter)
- Check NIP-65 kind 10002 events (Relay List Metadata)
- Add discovered relays to connection pool (up to `max_relays` limit)
- Track relay health (connection success rate, latency)

### 4. Storage Format

**File Organization:**
```
<output_dir>/
├── 2025_10_13.pb       # All events from Oct 13, 2025 (by created_at)
├── 2025_10_14.pb
├── 2025_10_15.pb
├── errors.jsonl        # Malformed events with reasons
└── .index.db           # SQLite index
```

**Protobuf Encoding:**
- **Length-delimited format**: Each event prefixed with varint length
- Allows append-only writes without rewriting files
- Enables streaming reads without loading entire file
- Standard protobuf streaming format

**Date-based Organization:**
- Events grouped by `created_at` timestamp (not received time)
- File naming: `YYYY_MM_DD.pb`
- Handles events arriving out-of-order (append to appropriate file)

## Validation Strategy

### Event ID Validation
1. Parse event JSON
2. Reconstruct serialization array: `[0, pubkey, created_at, kind, tags, content]`
3. Serialize to canonical JSON (UTF-8, no whitespace, escaped characters)
4. Compute SHA-256 hash
5. Compare with event's `id` field (must match)

### Signature Validation
1. Verify event ID is valid (see above)
2. Parse `sig` as hex-encoded Schnorr signature
3. Parse `pubkey` as hex-encoded secp256k1 public key
4. Verify signature against event ID using Schnorr signature algorithm
5. Use `nostr-sdk` for validation implementation

### Error Handling
- **Malformed JSON**: Log to `errors.jsonl`, continue processing
- **Invalid ID**: Log with reason "invalid_id: hash mismatch", continue
- **Invalid signature**: Log with reason "invalid_signature: verification failed", continue
- **CLI**: Show error count in summary, exit code 0 if any events succeeded
- **Daemon**: Log warnings, increment metrics counter, continue

## Performance Targets

### Initial Targets
- **Throughput**: 10-100 events/second (daemon)
- **Batch writes**: Every 500 events (configurable)
- **Memory**: < 100MB for daemon under normal load
- **Latency**: < 100ms from event receipt to buffer

### Future Optimization
- Increase throughput to 1000+ events/second
- Parallel validation using thread pool
- Compression (gzip, zstd) for protobuf files
- Sharding by event kind or date ranges

## Implementation Phases

### Phase 1: Core Library & Protobuf Schema
**Duration:** 1-2 weeks

**Tasks:**
1. ✅ Create workspace structure
2. Define protobuf schema in `proton-beam-core/proto/nostr.proto`
3. Set up `build.rs` for protobuf code generation
4. Implement JSON to protobuf conversion
5. Implement event ID validation
6. Implement signature validation using `nostr-sdk`
7. Implement length-delimited I/O functions
8. Write unit tests for conversions and validation
9. Create sample events file for testing

**Deliverables:**
- Working `proton-beam-core` library
- Comprehensive unit tests
- Documentation for public API

### Phase 2: CLI Tool
**Duration:** 1 week

**Tasks:**
1. Set up CLI project structure with `clap` for argument parsing
2. Implement file input handler (`.jsonl`)
3. Implement stdin input handler
4. Add progress bar using `indicatif`
5. Implement date-based file organization
6. Add error file writing (`errors.jsonl`)
7. Implement batched writes
8. Add CLI tests and integration tests
9. Write user documentation

**Deliverables:**
- Working `proton-beam` CLI binary
- Integration tests with sample data
- Usage documentation

### Phase 3: SQLite Index & Deduplication
**Duration:** 1 week

**Tasks:**
1. Design SQLite schema for event index
2. Implement index creation and management
3. Add event existence checking (deduplication)
4. Implement batch inserts for performance
5. Add index compaction/maintenance utilities
6. Write tests for index operations
7. Benchmark deduplication performance

**Deliverables:**
- SQLite index module
- Deduplication logic
- Performance benchmarks

### Phase 4: Relay Daemon (Core)
**Duration:** 2 weeks

**Tasks:**
1. Set up daemon project structure
2. Implement TOML configuration parsing
3. Implement relay connection manager using `nostr-sdk`
4. Add WebSocket subscription handling
5. Implement event batching and buffering
6. Integrate deduplication with SQLite index
7. Add graceful shutdown handling
8. Implement date-based file writing
9. Add logging with `tracing`
10. Write daemon tests

**Deliverables:**
- Working daemon binary
- Configuration examples
- Basic operational tests

### Phase 5: Relay Discovery & Advanced Features
**Duration:** 1-2 weeks

**Tasks:**
1. Implement relay hint extraction from tags
2. Add NIP-65 relay list parsing
3. Implement relay discovery service
4. Add relay health tracking
5. Implement max relay limiting
6. Add connection backoff/retry logic
7. Implement historical event fetching
8. Add filtering by kind, author, tags
9. Write discovery tests

**Deliverables:**
- Relay discovery system
- Advanced filtering
- Historical event support

### Phase 6: Testing, Documentation & Polish
**Duration:** 1 week

**Tasks:**
1. End-to-end integration tests
2. Performance benchmarking and optimization
3. Complete README with examples
4. Write comprehensive usage guide
5. Add example configurations
6. Create troubleshooting guide
7. Code cleanup and documentation
8. Prepare for initial release

**Deliverables:**
- Complete documentation
- Release-ready binaries
- Performance metrics

## Testing Strategy

### Unit Tests
- Event conversion (JSON ↔ protobuf)
- Event validation (ID, signature)
- Length-delimited I/O
- SQLite index operations
- Configuration parsing

### Integration Tests
- CLI: Process sample `.jsonl` file
- CLI: Read from stdin
- Daemon: Connect to test relay
- Daemon: Process events end-to-end
- Deduplication across restarts

### End-to-End Tests
- CLI converts large file (10K+ events)
- Daemon runs for extended period
- Verify data integrity after conversion
- Test graceful shutdown and recovery

### Performance Tests
- Benchmark conversion speed
- Measure throughput under load
- Memory profiling
- Disk I/O optimization

## Dependencies

### Core Dependencies
```toml
[dependencies]
# Protobuf
prost = "0.12"
prost-types = "0.12"

# Nostr
nostr-sdk = "0.33"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Async runtime (daemon)
tokio = { version = "1", features = ["full"] }

# CLI
clap = { version = "4", features = ["derive"] }
indicatif = "0.17"

# Configuration
toml = "0.8"

# Database
rusqlite = { version = "0.32", features = ["bundled"] }

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

[build-dependencies]
prost-build = "0.12"
```

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Performance bottleneck in validation** | High | Use parallelism, consider validation toggle |
| **SQLite lock contention** | Medium | Batch writes, use WAL mode |
| **Relay connection limits** | Medium | Implement connection pooling, backoff |
| **Out-of-order events** | Low | Length-delimited format handles this |
| **Disk space growth** | Medium | Implement compression, archiving |
| **nostr-sdk API changes** | Low | Pin versions, abstract validation layer |

## Success Metrics

### Functional Requirements
- ✅ Convert JSON events to protobuf
- ✅ Validate event IDs and signatures
- ✅ CLI processes `.jsonl` files
- ✅ CLI accepts stdin input
- ✅ Daemon connects to relays
- ✅ Events deduplicated by ID
- ✅ Date-based file organization
- ✅ Graceful shutdown

### Performance Requirements
- Handle 10-100 events/second
- < 100MB memory usage
- < 1% event loss rate
- 99.9% validation accuracy

### Quality Requirements
- 80%+ code coverage
- All public APIs documented
- Comprehensive user guide
- Zero critical bugs

## Future Enhancements

### V2.0 Features
- **Query API**: RESTful API to query stored events
- **Compression**: Optional gzip/zstd compression
- **Sharding**: Distribute events across multiple files
- **Replication**: Sync between multiple storage nodes
- **Metrics**: Prometheus metrics export
- **Web UI**: Monitor daemon status and statistics

### V2.5 Features
- **Event deletion**: Honor NIP-09 deletion requests
- **Replaceable events**: Handle NIP-16 replaceable events
- **Export tools**: Convert protobuf back to JSON
- **Migration tools**: Import from other formats

### V3.0 Features
- **Distributed storage**: Multi-node coordination
- **Query optimization**: Indexes for common queries
- **Real-time subscriptions**: WebSocket API for live events
- **Custom schemas**: Per-kind protobuf schemas

## Glossary

- **Event**: A Nostr event (kind 0-65535)
- **Protobuf**: Protocol Buffers, binary serialization format
- **Length-delimited**: Protobuf streaming format with length prefixes
- **NIP**: Nostr Implementation Possibility (specification document)
- **Relay**: Nostr relay server (WebSocket endpoint)
- **JSONL**: JSON Lines format (one JSON object per line)
- **Deduplication**: Preventing storage of duplicate events

## References

- [Nostr Protocol (NIP-01)](https://github.com/nostr-protocol/nips/blob/master/01.md)
- [Protocol Buffers](https://protobuf.dev/)
- [nostr-sdk Documentation](https://docs.rs/nostr-sdk/)
- [Length-Delimited Messages](https://protobuf.dev/programming-guides/techniques/#streaming)

---

**Document Status:** Draft
**Next Review:** After Phase 1 completion

