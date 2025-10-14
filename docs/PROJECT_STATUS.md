# Proton Beam - Project Status & Plan

**Version:** 2.1
**Last Updated:** 2025-10-13
**Current Phase:** Phase 3 Complete ✅
**Next Phase:** Phase 4 - Daemon Core

---

## 📊 Executive Summary

Proton Beam is a high-performance Rust-based tool for converting Nostr events from JSON format to Protocol Buffer (protobuf) format. It provides both a CLI tool for batch conversion and a daemon for real-time relay monitoring and conversion.

**Current Status:** Core library, CLI tool, and SQLite index are production-ready with comprehensive tests. Ready to begin daemon implementation.

---

## 🎯 Project Goals

1. ✅ **JSON to Protobuf Conversion**: Convert Nostr events from JSON to a space-efficient protobuf format
2. ✅ **CLI Tool**: Process `.jsonl` files, raw JSON, or stdin streams with progress indication
3. ⏳ **Relay Daemon**: Connect to multiple Nostr relays, capture events in real-time, and store as protobuf
4. ✅ **Validation**: Validate event IDs (SHA-256) and Schnorr signatures before conversion
5. ⏳ **Deduplication**: Ensure events are only stored once across multiple relay sources
6. ⏳ **Filtering**: Support configurable filtering by event kind, author, and tags
7. ⏳ **Performance**: Handle 10-100+ events/second with batched writes
8. ⏳ **Relay Discovery**: Automatically discover and connect to new relays via event tags

---

## 📈 Overall Progress

```
Phase 0 (Planning):        ████████████████████ 100% ✅
Phase 1 (Core Library):    ████████████████████ 100% ✅
Phase 1.5 (Enhanced API):  ████████████████████ 100% ✅
Phase 2 (CLI Tool):        ████████████████████ 100% ✅
Phase 3 (SQLite Index):    ████████████████████ 100% ✅
Phase 4 (Daemon Core):     ░░░░░░░░░░░░░░░░░░░░   0%
Phase 5 (Advanced):        ░░░░░░░░░░░░░░░░░░░░   0%
Phase 6 (Polish):          ░░░░░░░░░░░░░░░░░░░░   0%

Overall Progress:          ██████████░░░░░░░░░░  52%
```

**Test Status:** 102/102 tests passing (100% pass rate)
- Core library: 59 unit tests + 13 integration tests ✅
- CLI: 5 unit tests + 17 integration tests ✅
- Daemon: Placeholder only

---

## ✅ Completed Phases

### Phase 0: Planning & Documentation (100%)

**Deliverables:**
- ✅ Complete project specification
- ✅ Architecture design with diagrams
- ✅ Technical specifications
- ✅ Comprehensive documentation (~3,000 lines)
- ✅ Test data creation (20 sample events)

**Key Documents:**
- [ARCHITECTURE.md](ARCHITECTURE.md) - System design (534 lines)
- [PROTOBUF_SCHEMA.md](PROTOBUF_SCHEMA.md) - Schema docs (442 lines)
- [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md) - Dev guide (537 lines)
- [INDEX.md](INDEX.md) - Navigation hub (334 lines)

---

### Phase 1: Core Library (100%) ✅

**Deliverables:**
- ✅ Working `proton-beam-core` library
- ✅ Protobuf schema (`ProtoEvent`, `Tag`, `EventBatch`)
- ✅ JSON ↔ Protobuf conversion with `TryFrom`/`From` traits
- ✅ Event ID validation (SHA-256)
- ✅ Signature validation (Schnorr via `nostr-sdk`)
- ✅ Length-delimited I/O for streaming
- ✅ Error handling with `thiserror`
- ✅ 49 unit tests + 13 integration tests (all passing)
- ✅ Complete API documentation

**Source Code:** ~1,100 lines
- `lib.rs` - Public API exports
- `conversion.rs` - JSON ↔ Protobuf conversion
- `validation.rs` - Event ID and signature verification
- `storage.rs` - Length-delimited I/O with streaming
- `error.rs` - Type-safe error handling
- `builder.rs` - Fluent builder pattern
- `display.rs` - Display trait implementation
- `iter.rs` - FromIterator and Extend traits
- `serde_support.rs` - Serde serialize/deserialize

**Schema:** `proto/nostr.proto`
- `ProtoEvent` message (renamed from `Event` to avoid conflicts)
- `Tag` message for nested arrays
- `EventBatch` for collections

**Performance Metrics:**
- Protobuf encoding: ~500K events/sec (2.5x faster than JSON)
- Protobuf decoding: ~400K events/sec (2.7x faster than JSON)
- Storage: 10-25% smaller than minified JSON
- Validation: 100-200 events/sec (bottleneck: Schnorr signatures)

---

### Phase 1.5: Enhanced API (100%) ✅

**Deliverables:**
- ✅ Builder pattern (`ProtoEventBuilder`) - Fluent event construction
- ✅ Display trait - Pretty-printed JSON for debugging
- ✅ Serde support - Fast serialization without validation overhead
- ✅ FromIterator/Extend - Ergonomic batch creation from iterators
- ✅ PartialEq/Eq - Easy equality testing and comparisons

**Impact:** Enhanced ergonomics for Rust developers, increased test coverage by 148%

---

### Phase 2: CLI Tool (100%) ✅

**Deliverables:**
- ✅ Working `proton-beam` CLI binary
- ✅ Argument parsing with `clap`
- ✅ File input handler (`.jsonl` line-by-line)
- ✅ stdin input handler (pipe support)
- ✅ Progress bars with `indicatif`
- ✅ Date-based file organization (`YYYY_MM_DD.pb`)
- ✅ Error logging with `tracing` (`proton-beam.log`)
- ✅ Batch write operations (configurable)
- ✅ 5 unit tests + 13 integration tests (all passing)

**Source Code:** ~500 lines
- `main.rs` - CLI entry point, argument parsing, conversion logic
- `input.rs` - File and stdin input handling
- `storage.rs` - Date-based storage manager with buffering
- `progress.rs` - Reserved for future enhancements

**Features:**
- ✅ `proton-beam convert <file>` - Process `.jsonl` files
- ✅ `proton-beam convert -` - Read from stdin
- ✅ `--output-dir <path>` - Custom output location
- ✅ `--no-validate` - Skip validation for speed
- ✅ `--batch-size <n>` - Configurable batch size (default: 500)
- ✅ `--verbose` - Detailed logging
- ✅ `--no-progress` - Disable progress bars

**Usage Examples:**
```bash
# Convert from file
proton-beam convert events.jsonl

# From stdin
cat events.jsonl | proton-beam convert -

# Custom output with large batches
proton-beam convert events.jsonl --output-dir ./pb_data --batch-size 1000

# Skip validation for speed
proton-beam convert events.jsonl --no-validate
```

**Output Structure:**
```
./pb_data/
├── 2025_10_13.pb        # Events from Oct 13, 2025
├── 2025_10_14.pb        # Events from Oct 14, 2025
└── proton-beam.log      # Compact error and warning logs
```

---

### Phase 3: SQLite Index & Deduplication (100%) ✅

**Duration:** ~1 day
**Status:** Complete

**Goals:**
- ✅ Enable deduplication across multiple conversions
- ✅ Fast event lookups by ID, kind, pubkey, created_at
- ✅ Support for CLI and daemon

**Tasks:**
- ✅ Design SQLite schema for event index
- ✅ Implement index creation and management
- ✅ Add event existence checking (deduplication)
- ✅ Implement batch inserts for performance
- ✅ Write tests for index operations
- ✅ Benchmark deduplication performance
- ✅ Add `--index-path` flag to CLI
- ✅ Add index statistics/reporting

**Deliverables:**
- ✅ SQLite index module in `proton-beam-core`
- ✅ Deduplication logic with info-level logging
- ✅ Performance benchmarks (~307K lookups/sec, ~171K batch inserts/sec)
- ✅ Updated CLI with automatic index support

**Implemented SQLite Schema:**
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
CREATE INDEX idx_file_path ON events(file_path);
```

**Implemented API:**
```rust
pub struct EventIndex {
    conn: Connection,
}

impl EventIndex {
    pub fn new(db_path: &Path) -> Result<Self>;
    pub fn contains(&self, event_id: &str) -> Result<bool>;
    pub fn insert(&mut self, event: &ProtoEvent, file_path: &str) -> Result<()>;
    pub fn insert_batch(&mut self, events: &[(&ProtoEvent, &str)]) -> Result<()>;
    pub fn query_by_kind(&self, kind: i32) -> Result<Vec<EventRecord>>;
    pub fn query_by_pubkey(&self, pubkey: &str) -> Result<Vec<EventRecord>>;
    pub fn query_by_date_range(&self, start: i64, end: i64) -> Result<Vec<EventRecord>>;
    pub fn get(&self, event_id: &str) -> Result<Option<EventRecord>>;
    pub fn stats(&self) -> Result<IndexStats>;
}
```

**Performance Benchmarks:**
- **Batch insertions**: ~171,518 events/sec (500-event batches)
- **Single insertions**: ~2,649 events/sec
- **Contains lookups**: ~306,905 lookups/sec
- **Query by kind**: ~2,831 queries/sec
- **Stats calculation**: ~247 calls/sec (on 100K event index)

**CLI Integration:**
- Index automatically created in output directory as `index.db`
- Custom path supported via `--index-path` option
- Deduplication enabled by default with info-level logging
- Duplicate events do not cause exit failures

---

## ⏳ Remaining Phases



### Phase 4: Daemon Core (0%)

**Duration:** ~2 weeks
**Status:** Not started (placeholder only)

**Goals:**
- Real-time event capture from multiple Nostr relays
- Automatic deduplication across relay sources
- Efficient batched writes to protobuf files

**Tasks:**
- [ ] Set up daemon project structure
- [ ] Implement TOML configuration parsing
- [ ] Implement relay connection manager using `nostr-sdk`
- [ ] Add WebSocket subscription handling
- [ ] Implement event batching and buffering
- [ ] Integrate deduplication with SQLite index
- [ ] Add graceful shutdown handling (SIGTERM/SIGINT)
- [ ] Implement date-based file writing
- [ ] Add structured logging with `tracing`
- [ ] Write daemon tests

**Deliverables:**
- Working `proton-beam-daemon` binary
- Configuration examples
- Basic operational tests
- User documentation

**Configuration (`config.toml`):**
```toml
[daemon]
output_dir = "./nostr_events"
batch_size = 500              # Write every N events
log_level = "info"

[relays]
urls = [
    "wss://relay.damus.io",
    "wss://nos.lol",
    "wss://relay.primal.net",
]
auto_discover = true          # Discover relays from event tags
max_relays = 50               # Max concurrent relay connections

[filters]
kinds = []                    # Empty = all kinds
authors = []                  # Empty = all authors

[storage]
deduplicate = true
use_index = true              # Maintain SQLite index
```

**Usage:**
```bash
# Start with default config
proton-beam-daemon start

# Specify config file
proton-beam-daemon start --config /path/to/config.toml

# Request historical events since timestamp
proton-beam-daemon start --since 1697000000
```

---

### Phase 5: Advanced Features (0%)

**Duration:** ~1-2 weeks
**Status:** Not started

**Goals:**
- Automatic relay discovery from event tags
- Historical event fetching
- Advanced filtering capabilities

**Tasks:**
- [ ] Implement relay hint extraction from tags
- [ ] Add NIP-65 relay list parsing (kind 10002)
- [ ] Implement relay discovery service
- [ ] Add relay health tracking
- [ ] Implement max relay limiting
- [ ] Add connection backoff/retry logic
- [ ] Implement historical event fetching
- [ ] Add filtering by kind, author, tags
- [ ] Write discovery tests

**Deliverables:**
- Relay discovery system
- Advanced filtering
- Historical event support
- Health monitoring

**Relay Discovery Logic:**
- Parse `r` tags (relay URLs with markers: `read`, `write`)
- Parse `relay` tags (standalone relay URLs)
- Extract relay hints from `e`, `p`, `a` tags (3rd parameter)
- Check NIP-65 kind 10002 events (Relay List Metadata)
- Add discovered relays to connection pool (up to `max_relays` limit)
- Track relay health (connection success rate, latency)

---

### Phase 6: Testing, Documentation & Polish (0%)

**Duration:** ~1 week
**Status:** Not started

**Goals:**
- Production-ready release
- Complete documentation
- Performance optimization

**Tasks:**
- [ ] End-to-end integration tests
- [ ] Performance benchmarking and optimization
- [ ] Complete README with examples
- [ ] Write comprehensive usage guide
- [ ] Add example configurations
- [ ] Create troubleshooting guide
- [ ] Code cleanup and documentation review
- [ ] Prepare for initial release
- [ ] Create release binaries
- [ ] Write CHANGELOG

**Deliverables:**
- Complete user documentation
- Release-ready binaries for major platforms
- Performance metrics report
- Migration guide (if needed)

---

## 🏗️ Architecture Overview

### Workspace Structure

```
proton-beam/
├── Cargo.toml                    # Workspace root
├── proton-beam-core/             # Core library ✅
│   ├── proto/nostr.proto         # Protobuf schema
│   └── src/                      # Conversion, validation, storage
├── proton-beam-cli/              # CLI binary ✅
│   └── src/                      # Input handling, progress, storage
├── proton-beam-daemon/           # Daemon binary (placeholder)
│   └── src/main.rs
├── docs/                         # Documentation
├── examples/                     # Sample data and configs
└── README.md
```

### Component Status

| Component | Status | Tests | Lines of Code |
|-----------|--------|-------|---------------|
| `proton-beam-core` | ✅ Complete | 72/72 passing | ~1,700 |
| `proton-beam-cli` | ✅ Complete | 22/22 passing | ~550 |
| `proton-beam-daemon` | ⏳ Placeholder | 0 tests | ~5 |
| Documentation | ✅ Complete | N/A | ~3,500 |
| Examples | ✅ Complete | N/A | ~350 |
| Benchmarks | ✅ Complete | N/A | ~200 |

---

## 🚀 API Overview

### Core Library API

#### Creating Events

| Method | Use Case | Validation |
|--------|----------|------------|
| `ProtoEventBuilder::new()` | Testing, construction | None |
| `ProtoEvent::try_from(json)` | Parse untrusted JSON | Full |
| `serde_json::from_str(json)` | Parse trusted JSON | None |

#### Serializing Events

| Method | Use Case | Output |
|--------|----------|--------|
| `String::try_from(&event)` | With validation | JSON |
| `serde_json::to_string(&event)` | Fast, no validation | JSON |
| `format!("{}", event)` | Debugging | Pretty JSON |

#### Batch Operations

- `events.into_iter().collect::<EventBatch>()` - Create batch from iterator
- `batch.extend(more_events)` - Add events to existing batch
- Works with `.filter()`, `.map()`, and all iterator methods

#### Validation & Storage

- `validate_event(&event)` - Full validation (ID + signature)
- `write_event_delimited(&mut file, &event)` - Write single event
- `read_events_delimited(file)` - Stream events from file

#### Event Index

- `EventIndex::new(db_path)` - Create/open SQLite index
- `index.contains(event_id)` - Check if event exists (deduplication)
- `index.insert(&event, file_path)` - Index single event
- `index.insert_batch(&events)` - Batch insert for performance
- `index.query_by_kind(kind)` - Query events by kind
- `index.query_by_pubkey(pubkey)` - Query events by author
- `index.query_by_date_range(start, end)` - Query events by timestamp
- `index.get(event_id)` - Get event record by ID
- `index.stats()` - Get index statistics

### CLI Usage

```bash
# Basic conversion
proton-beam convert events.jsonl

# Read from stdin
cat events.jsonl | proton-beam convert -

# Custom output directory
proton-beam convert events.jsonl --output-dir ./pb_data

# Skip validation (faster)
proton-beam convert events.jsonl --no-validate

# Large batches for performance
proton-beam convert events.jsonl --batch-size 2000

# Verbose logging
proton-beam convert events.jsonl --verbose

# Quiet mode (no progress bar)
proton-beam convert events.jsonl --no-progress
```

---

## 🎓 Key Design Decisions

### 1. Type Naming
**Decision:** Renamed `Event` → `ProtoEvent`
**Reason:** Avoid conflicts with `nostr-sdk::Event`
**Impact:** Clearer API, explicit protobuf representation

### 2. Multiple API Styles
**Decision:** Support both idiomatic traits and convenience functions
**Reason:** Ergonomic for Rust developers, backward compatible
**Trade-off:** Slightly larger API surface, but more flexible

### 3. Serde Support
**Decision:** Add optional fast serialization without validation
**Reason:** Performance optimization for trusted internal data
**Trade-off:** Two APIs (serde vs TryFrom), but users choose based on needs

### 4. Length-Delimited Storage
**Decision:** Use length-prefixed protobuf messages
**Reason:** Append-only writes, streaming reads, memory efficient
**Impact:** Standard protobuf pattern, no file rewrites needed

### 5. Date-Based Organization
**Decision:** Organize events by `created_at` into `YYYY_MM_DD.pb` files
**Reason:** Balance file count vs size, natural time-series queries
**Impact:** Easy archival, handles out-of-order events

### 6. CLI-First Approach
**Decision:** Build CLI before daemon
**Reason:** Validates core library, provides immediate utility
**Impact:** Earlier user feedback, incremental complexity

---

## 🛠️ Technology Stack

### Core Dependencies (Implemented)
- **nostr-sdk** (0.33) - Nostr protocol, event validation ✅
- **prost** (0.12) - Protobuf code generation and runtime ✅
- **serde** (1.0) - JSON serialization with derive feature ✅
- **thiserror** (1.0) - Error handling for libraries ✅
- **anyhow** (1.0) - Error handling for applications ✅
- **clap** (4.x) - CLI argument parsing ✅
- **indicatif** (0.17) - Progress bars ✅
- **chrono** - Date/time handling ✅
- **tracing** (0.1) - Structured logging ✅

### Future Dependencies (Phase 3+)
- **tokio** (1.x) - Async runtime (daemon)
- **rusqlite** (0.32) - SQLite database
- **toml** (0.8) - Configuration parsing

---

## 📊 Performance Metrics

### Conversion Speed (Measured)
- **Protobuf encoding:** ~500K events/sec (2.5x faster than JSON)
- **Protobuf decoding:** ~400K events/sec (2.7x faster than JSON)

### Storage Efficiency (Measured)
- **JSON (formatted):** ~450 bytes/event
- **JSON (minified):** ~380 bytes/event
- **Protobuf:** ~340 bytes/event
- **Savings:** 10-25% vs JSON

### Validation (Measured)
- **With full validation:** 100-200 events/sec
- **Without validation:** 500+ events/sec
- **Bottleneck:** Schnorr signature verification (CPU-bound)

### Targets (Not Yet Measured)
- **Daemon throughput:** 10-100 events/second
- **Memory usage:** < 100MB under normal load
- **Event loss rate:** < 1%
- **Validation accuracy:** 99.9%

---

## 📋 Success Criteria

### ✅ Achieved Goals

**Functional Requirements:**
- ✅ Convert JSON to protobuf
- ✅ Validate event IDs and signatures
- ✅ CLI processes `.jsonl` files
- ✅ CLI accepts stdin input
- ✅ Date-based file organization
- ✅ Error logging

**Performance Targets:**
- ✅ 2.5x faster encoding vs JSON
- ✅ 10-25% size reduction

**Quality Requirements:**
- ✅ 80%+ code coverage (core & CLI)
- ✅ All public APIs documented
- ✅ Zero critical bugs (Phases 1-2)
- ✅ 100% test pass rate (92/92)

### 🎯 Remaining Goals

**Functional Requirements:**
- ⏳ Daemon connects to relays
- ⏳ Deduplicate events by ID
- ⏳ Graceful shutdown
- ⏳ Auto-discover relays

**Performance Targets:**
- ⏳ Handle 10-100 events/sec (daemon)
- ⏳ < 100MB memory usage
- ⏳ < 1% event loss rate

**Quality Requirements:**
- ⏳ Comprehensive user guide
- ⏳ End-to-end integration tests
- ⏳ Performance benchmarks

---

## 🚀 Next Steps

### Immediate: Phase 3 - SQLite Index

**Week 1 Goals:**
1. Design and implement SQLite schema
2. Create shared index module in workspace
3. Add deduplication API
4. Integrate with CLI (optional `--use-index` flag)
5. Write comprehensive tests
6. Benchmark performance

**Success Criteria:**
- Index can handle 100K+ events efficiently
- Deduplication works correctly
- < 10% performance overhead vs no-index mode
- All tests passing

### Short Term: Phase 4 - Daemon Core

**Week 2-3 Goals:**
1. Implement relay connection manager
2. Add event subscription handling
3. Integrate index for deduplication
4. Add graceful shutdown
5. Write daemon tests

### Medium Term: Phase 5-6

**Week 4-6 Goals:**
1. Relay auto-discovery
2. Advanced filtering
3. Complete documentation
4. Performance optimization
5. Release preparation

---

## 📚 Documentation

### Available Documentation
- ✅ [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture and design
- ✅ [PROTOBUF_SCHEMA.md](PROTOBUF_SCHEMA.md) - Schema documentation
- ✅ [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md) - Development setup
- ✅ [INDEX.md](INDEX.md) - Documentation navigation
- ✅ This document (PROJECT_STATUS.md) - Current status and plan

### Documentation To-Do
- [ ] Comprehensive user guide (Phase 6)
- [ ] Troubleshooting guide (Phase 6)
- [ ] Performance tuning guide (Phase 6)
- [ ] Migration guide (if needed)
- [ ] API reference (auto-generated via `cargo doc`)

---

## 🔗 Quick Links

### Getting Started

```bash
# Build the project
cargo build --release

# Run all tests
cargo test --all

# Build documentation
cargo doc --open

# Run the CLI
./target/release/proton-beam convert examples/sample_events.jsonl

# Run example code
cargo run --example api_showcase
```

### For Developers

```bash
# Check code quality
cargo clippy --all-targets
cargo fmt --check

# Run specific test suite
cargo test --package proton-beam-core
cargo test --package proton-beam-cli

# Run with verbose output
cargo test -- --nocapture
```

---

## 📞 Support & Contributing

### Resources
- 📖 Documentation: `docs/` directory
- 🐛 Issues: GitHub issue tracker
- 💬 Discussions: GitHub discussions

### Contributing
1. Read [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md)
2. Check this document for current status
3. Review open issues
4. Follow test-driven development
5. Submit PRs with tests

---

## 🎉 Milestones

- ✅ **2025-10-13**: Phase 0 (Planning) complete
- ✅ **2025-10-13**: Phase 1 (Core Library) complete
- ✅ **2025-10-13**: Phase 1.5 (Enhanced API) complete
- ✅ **2025-10-13**: Phase 2 (CLI Tool) complete
- ✅ **2025-10-13**: Phase 3 (SQLite Index) complete
- 🎯 **TBD**: Phase 4 (Daemon Core) - *Next up!*
- 🎯 **TBD**: Phase 5 (Advanced Features)
- 🎯 **TBD**: Phase 6 (Polish & Release)
- 🚀 **TBD**: Version 1.0 Release

---

**Document Status:** Current and Accurate ✅
**Last Review:** 2025-10-13
**Next Review:** After Phase 4 completion

