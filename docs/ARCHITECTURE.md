# Proton Beam Architecture

**Version:** 1.1
**Last Updated:** 2025-10-29
**Implementation Status:** Phases 1-3 Complete (Core, CLI, Index) | Phase 4 Planned (Daemon)

## System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Proton Beam System                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────────┐              ┌──────────────────┐            │
│  │   CLI Tool       │              │  Relay Daemon    │            │
│  │ proton-beam-cli  │              │ proton-beam-     │            │
│  │                  │              │    daemon        │            │
│  └────────┬─────────┘              └────────┬─────────┘            │
│           │                                 │                      │
│           │  Uses                   Uses    │                      │
│           │                                 │                      │
│           ▼                                 ▼                      │
│  ┌────────────────────────────────────────────────────┐            │
│  │          Core Library (proton-beam-core)           │            │
│  │  ┌──────────────────────────────────────────────┐  │            │
│  │  │  Protobuf Schema (nostr.proto)               │  │            │
│  │  │  - Event, Tag, EventBatch messages           │  │            │
│  │  └──────────────────────────────────────────────┘  │            │
│  │  ┌──────────────────────────────────────────────┐  │            │
│  │  │  Conversion Engine                           │  │            │
│  │  │  - JSON → Protobuf                           │  │            │
│  │  │  - Protobuf → JSON                           │  │            │
│  │  └──────────────────────────────────────────────┘  │            │
│  │  ┌──────────────────────────────────────────────┐  │            │
│  │  │  Validation Layer                            │  │            │
│  │  │  - Event ID (SHA-256) verification           │  │            │
│  │  │  - Schnorr signature verification            │  │            │
│  │  └──────────────────────────────────────────────┘  │            │
│  │  ┌──────────────────────────────────────────────┐  │            │
│  │  │  Storage I/O                                 │  │            │
│  │  │  - Length-delimited write                    │  │            │
│  │  │  - Length-delimited read                     │  │            │
│  │  └──────────────────────────────────────────────┘  │            │
│  └────────────────────────────────────────────────────┘            │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## CLI Tool Architecture (✅ Implemented)

The CLI tool supports three main commands: `convert`, `merge`, and `index`.

```
┌─────────────────────────────────────────────────────────────┐
│                    proton-beam CLI                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Commands:                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │  convert    │  │    merge    │  │    index    │        │
│  │ (primary)   │  │  (parallel) │  │  (rebuild)  │        │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘        │
│                                                             │
│  Convert Mode (Single/Parallel):                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │ .jsonl File │  │   stdin     │  │ Raw JSON    │        │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘        │
│         │                │                │                │
│         └────────────────┼────────────────┘                │
│                          ▼                                 │
│         ┌────────────────────────────────┐                 │
│         │   Input Parser & Reader        │                 │
│         │   - Line-by-line processing    │                 │
│         │   - Progress tracking           │                 │
│         │   - Optional kind pre-filter   │                 │
│         └────────────┬───────────────────┘                 │
│                      ▼                                     │
│         ┌────────────────────────────────┐                 │
│         │   Validation Pipeline          │                 │
│         │   - JSON parse                 │                 │
│         │   - Basic field validation     │                 │
│         │   - Event ID check (optional)  │                 │
│         │   - Signature verify (optional)│                 │
│         └────────┬───────────────────────┘                 │
│                  │                                         │
│         ┌────────┴────────┐                                │
│         ▼                 ▼                                │
│  ┌─────────────┐   ┌─────────────┐                        │
│  │Valid Events │   │   Errors    │                        │
│  └──────┬──────┘   └──────┬──────┘                        │
│         │                 │                                │
│         ▼                 ▼                                │
│  ┌─────────────┐   ┌─────────────┐                        │
│  │JSON→Protobuf│   │  Error Log  │                        │
│  │Conversion   │   │  (tracing)  │                        │
│  └──────┬──────┘   └──────┬──────┘                        │
│         │                 │                                │
│         ▼                 ▼                                │
│  ┌─────────────┐   ┌──────────────┐                       │
│  │Event Batcher│   │proton-beam   │                       │
│  │(1000 events)│   │.log          │                       │
│  └──────┬──────┘                                           │
│         ▼                                                  │
│  ┌─────────────────────────────┐                          │
│  │  Date-based File Writer     │                          │
│  │  - Group by created_at      │                          │
│  │  - Length-delimited format  │                          │
│  │  - YYYY_MM_DD.pb.gz files   │                          │
│  │  - Gzip (level 0-9, def: 6) │                          │
│  │  - Optional S3 upload       │                          │
│  └──────┬──────────────────────┘                          │
│         ▼                                                  │
│  ┌──────────────────────┐                                 │
│  │  Output Directory:   │                                 │
│  │  ./pb_data/          │  (default)                      │
│  │  ├─ 2025_10_13.pb.gz │                                 │
│  │  ├─ 2025_10_14.pb.gz │                                 │
│  │  ├─ index.db         │  (SQLite)                       │
│  │  └─ proton-beam.log  │                                 │
│  └──────────────────────┘                                 │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

## Daemon Architecture (⏳ Planned - Phase 4)

**Status:** Not yet implemented. This section describes the planned architecture for Phase 4.

```
┌───────────────────────────────────────────────────────────────────────┐
│                     proton-beam-daemon (PLANNED)                      │
├───────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  ┌─────────────────────────────────────────────────────────┐         │
│  │                  Configuration Layer                    │         │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐        │         │
│  │  │ config.toml│  │    CLI     │  │Environment │        │         │
│  │  │            │  │   Args     │  │  Variables │        │         │
│  │  └──────┬─────┘  └──────┬─────┘  └──────┬─────┘        │         │
│  │         └────────────────┼────────────────┘             │         │
│  └──────────────────────────┼──────────────────────────────┘         │
│                             ▼                                         │
│  ┌─────────────────────────────────────────────────────────┐         │
│  │              Relay Manager                              │         │
│  │  ┌────────────────────────────────────────────┐         │         │
│  │  │  Initial Relay Pool                        │         │         │
│  │  │  - relay.damus.io                          │         │         │
│  │  │  - nos.lol                                 │         │         │
│  │  │  - relay.primal.net                        │         │         │
│  │  │  - relay.nostr.band                        │         │         │
│  │  │  - relay.snort.social                      │         │         │
│  │  └────────────┬───────────────────────────────┘         │         │
│  │               │                                          │         │
│  │               ▼                                          │         │
│  │  ┌────────────────────────────────────────────┐         │         │
│  │  │  Connection Pool Manager                   │         │         │
│  │  │  - WebSocket connections                   │         │         │
│  │  │  - Health monitoring                       │         │         │
│  │  │  - Reconnection logic                      │         │         │
│  │  │  - Load balancing                          │         │         │
│  │  └────────────┬───────────────────────────────┘         │         │
│  │               │                                          │         │
│  │               ▼                                          │         │
│  │  ┌────────────────────────────────────────────┐         │         │
│  │  │  Relay Discovery Service                   │         │         │
│  │  │  - Extract relay hints from tags           │         │         │
│  │  │  - Parse NIP-65 relay lists (kind 10002)   │         │         │
│  │  │  - Discover from e/p/a/r tags              │         │         │
│  │  │  - Maintain discovered relay queue         │         │         │
│  │  └────────────┬───────────────────────────────┘         │         │
│  └───────────────┼─────────────────────────────────────────┘         │
│                  │                                                    │
│                  ▼                                                    │
│  ┌─────────────────────────────────────────────────────────┐         │
│  │              Event Processing Pipeline                  │         │
│  │                                                          │         │
│  │  ┌────────────────────────────────────────────┐         │         │
│  │  │  WebSocket Message Receiver                │         │         │
│  │  │  - Multiple relay streams                  │         │         │
│  │  │  - Message parsing                         │         │         │
│  │  └────────────┬───────────────────────────────┘         │         │
│  │               ▼                                          │         │
│  │  ┌────────────────────────────────────────────┐         │         │
│  │  │  Filter Matcher                            │         │         │
│  │  │  - Kind filter                             │         │         │
│  │  │  - Author filter                           │         │         │
│  │  │  - Tag filter                              │         │         │
│  │  └────────────┬───────────────────────────────┘         │         │
│  │               ▼                                          │         │
│  │  ┌────────────────────────────────────────────┐         │         │
│  │  │  Deduplication Layer                       │         │         │
│  │  │  - Check SQLite index                      │         │         │
│  │  │  - Event ID lookup                         │         │         │
│  │  │  - Skip if exists                          │         │         │
│  │  └────────────┬───────────────────────────────┘         │         │
│  │               ▼                                          │         │
│  │  ┌────────────────────────────────────────────┐         │         │
│  │  │  Validation Pipeline                       │         │         │
│  │  │  - JSON parsing                            │         │         │
│  │  │  - Event ID verification                   │         │         │
│  │  │  - Signature verification                  │         │         │
│  │  └────────┬───────────────────────────────────┘         │         │
│  │            │                                             │         │
│  │   ┌────────┴────────┐                                   │         │
│  │   ▼                 ▼                                   │         │
│  │ ┌──────┐      ┌──────────┐                             │         │
│  │ │Valid │      │ Invalid  │                             │         │
│  │ └───┬──┘      └────┬─────┘                             │         │
│  │     │              │                                    │         │
│  │     ▼              ▼                                    │         │
│  │ ┌──────┐      ┌──────────┐                             │         │
│  │ │Buffer│      │Error Log │                             │         │
│  │ └───┬──┘      └──────────┘                             │         │
│  │     │                                                   │         │
│  └─────┼───────────────────────────────────────────────────┘         │
│        │                                                             │
│        ▼                                                             │
│  ┌─────────────────────────────────────────────────────────┐         │
│  │              Storage Manager                            │         │
│  │                                                          │         │
│  │  ┌────────────────────────────────────────────┐         │         │
│  │  │  Event Batcher                             │         │         │
│  │  │  - Accumulate 500 events (configurable)    │         │         │
│  │  │  - Trigger on count or time                │         │         │
│  │  └────────────┬───────────────────────────────┘         │         │
│  │               ▼                                          │         │
│  │  ┌────────────────────────────────────────────┐         │         │
│  │  │  Date-based Router                         │         │         │
│  │  │  - Group by created_at timestamp           │         │         │
│  │  │  - Map to YYYY_MM_DD.pb.gz files           │         │         │
│  │  └────────────┬───────────────────────────────┘         │         │
│  │               ▼                                          │         │
│  │  ┌────────────────────────────────────────────┐         │         │
│  │  │  File Writer                               │         │         │
│  │  │  - Length-delimited protobuf               │         │         │
│  │  │  - Append-only writes                      │         │         │
│  │  │  - Atomic operations                       │         │         │
│  │  └────────────┬───────────────────────────────┘         │         │
│  │               ▼                                          │         │
│  │  ┌────────────────────────────────────────────┐         │         │
│  │  │  SQLite Index Updater                      │         │         │
│  │  │  - Batch inserts                           │         │         │
│  │  │  - Track event ID → file mapping           │         │         │
│  │  └────────────────────────────────────────────┘         │         │
│  └─────────────────────────────────────────────────────────┘         │
│                                                                       │
│  ┌─────────────────────────────────────────────────────────┐         │
│  │              Monitoring & Control                       │         │
│  │  - Metrics (events/sec, errors, relays)                │         │
│  │  - Graceful shutdown handler (SIGTERM/SIGINT)          │         │
│  │  - Health checks                                        │         │
│  └─────────────────────────────────────────────────────────┘         │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
```

## Data Flow

### CLI Tool Data Flow

```
Input File/Stream
       │
       ▼
   [Line Reader]
       │
       ▼
  [JSON Parser] ──► (parse error) ──► proton-beam.log
       │
       ▼
  [Validator] ──► (validation error) ──► proton-beam.log
       │
       ▼
  [JSON→Proto Converter]
       │
       ▼
  [Event Batcher]
       │
       ▼
  [Date Grouper]
       │
       ▼
  [File Writer] ──► 2025_10_13.pb.gz
                    2025_10_14.pb.gz
                    ...
```

### Daemon Data Flow

```
Multiple Relays (WebSocket)
       │
       │ (EVENT messages)
       ▼
  [Message Receiver]
       │
       ▼
  [Filter Check] ──► (filtered out) ──► (discard)
       │
       ▼
  [Dedup Check] ──► (duplicate) ──► (discard)
       │
       ▼
  [Validator] ──► (invalid) ──► proton-beam.log
       │
       ▼
  [Relay Discovery] ──► [Discovered Relays] ──► [Connection Pool]
       │
       ▼
  [JSON→Proto Converter]
       │
       ▼
  [Event Buffer]
       │
       ▼ (batch trigger: 500 events OR 30 seconds)
  [Batch Processor]
       │
       ├─► [File Writer] ──► YYYY_MM_DD.pb.gz files (gzip compressed)
       │
       └─► [Index Updater] ──► index.db
```

## Storage Architecture

```
Output Directory Structure:
┌────────────────────────────────────┐
│  ./pb_data/                        │  (CLI default)
│  OR ./nostr_events/                │  (Daemon default)
│                                    │
│  ├── 2025_10_13.pb.gz              │ ◄── Events from Oct 13 (gzip compressed)
│  │   ┌────────────────────────┐    │
│  │   │ [len][Event 1 binary]  │    │
│  │   │ [len][Event 2 binary]  │    │     Length-delimited
│  │   │ [len][Event 3 binary]  │    │     protobuf format
│  │   │ ...                    │    │     (inside gzip stream)
│  │   └────────────────────────┘    │
│  │                                 │
│  ├── 2025_10_14.pb.gz              │ ◄── Events from Oct 14
│  │                                 │
│  ├── 2025_10_15.pb.gz              │ ◄── Events from Oct 15
│  │                                 │
│  ├── proton-beam.log               │ ◄── Error and warning logs
│  │   ┌────────────────────────┐    │
│  │   │ 2025-10-14T13:48:12Z   │    │
│  │   │  ERROR parse_error:... │    │     Compact log format
│  │   │        line=1 id=abcd  │    │     (tracing subscriber)
│  │   │ ...                    │    │
│  │   └────────────────────────┘    │
│  │                                 │
│  └── index.db                      │ ◄── SQLite index
│      ┌────────────────────────┐    │
│      │ events table:          │    │
│      │  - id (PK)             │    │     Event lookup index
│      │  - kind                │    │     for deduplication
│      │  - pubkey              │    │     and future queries
│      │  - created_at          │    │
│      │  - file_path           │    │
│      │  - indexed_at          │    │
│      └────────────────────────┘    │
│                                    │
└────────────────────────────────────┘
```

## Concurrency Model

### CLI Tool
```
Single-threaded with async I/O:
┌─────────────────────────────────┐
│  Main Thread                    │
│  ├─ Read events                 │
│  ├─ Validate (sequential)       │
│  ├─ Convert (sequential)        │
│  ├─ Buffer (in-memory)          │
│  └─ Write (async I/O)           │
└─────────────────────────────────┘
```

### Daemon
```
Multi-threaded async model:
┌─────────────────────────────────┐
│  Main Async Runtime (Tokio)    │
│                                 │
│  ┌─────────────────────────┐   │
│  │ Relay Connections       │   │
│  │ (multiple concurrent)   │   │
│  │  - Task per relay       │   │
│  │  - WebSocket streams    │   │
│  └────────┬────────────────┘   │
│           │                     │
│           ▼                     │
│  ┌─────────────────────────┐   │
│  │ Event Processing Pool   │   │
│  │  - Validate (parallel)  │   │
│  │  - Convert (parallel)   │   │
│  └────────┬────────────────┘   │
│           │                     │
│           ▼                     │
│  ┌─────────────────────────┐   │
│  │ Storage Task            │   │
│  │  - Single writer        │   │
│  │  - Batch operations     │   │
│  └─────────────────────────┘   │
│                                 │
│  ┌─────────────────────────┐   │
│  │ Discovery Task          │   │
│  │  - Periodic discovery   │   │
│  │  - Connect to new relays│   │
│  └─────────────────────────┘   │
│                                 │
└─────────────────────────────────┘
```

## Key Design Decisions

### 1. Length-Delimited Protobuf
**Why:** Allows append-only writes without rewriting entire files. Each event is independent.

```
File Structure:
┌──────┬─────────┬──────┬─────────┬──────┬─────────┐
│ len1 │ Event1  │ len2 │ Event2  │ len3 │ Event3  │
└──────┴─────────┴──────┴─────────┴──────┴─────────┘
 varint  binary   varint  binary   varint  binary
```

### 2. Date-based File Organization
**Why:** Balance between too many files (one per event) and too few (one massive file).

- Easy to archive old dates (files already gzip compressed)
- Reasonable file sizes (depends on relay traffic)
- Out-of-order events handled gracefully
- Format: `YYYY_MM_DD.pb.gz` (always gzip compressed)

### 3. SQLite for Deduplication
**Why:** Fast lookups, ACID properties, no external dependencies.

- O(1) event existence check
- Enables future querying without scanning .pb.gz files
- Small overhead (~1-2% of event data size)

### 4. Validation with nostr-sdk
**Why:** Battle-tested implementation, correct cryptography.

- Don't roll our own crypto
- Get NIP updates automatically
- Proper Schnorr signature verification

### 5. Batch Writes
**Why:** Dramatically reduce I/O overhead.

- Amortize file open/close costs
- Better disk I/O patterns
- Configurable batch size (default: 1000 for CLI, 500 for daemon)
- Parallel processing with thread-local temp files (CLI only)

### 6. Gzip Compression
**Why:** Significant space savings with minimal CPU overhead.

- Always enabled with configurable level (0-9, default: 6)
- Reduces storage by ~65-97% compared to raw protobuf
- Combined with protobuf: ~3-40x smaller than JSON
- Streaming compression during write (memory efficient)

## Component Dependencies

```
proton-beam-cli
    │
    └─► proton-beam-core
            │
            ├─► prost (protobuf)
            ├─► nostr-sdk (validation)
            ├─► serde_json (JSON parsing)
            └─► sha2, secp256k1 (crypto)

proton-beam-daemon
    │
    ├─► proton-beam-core
    │       │
    │       └─► (same as above)
    │
    ├─► nostr-sdk (relay connections)
    ├─► rusqlite (index)
    ├─► tokio (async runtime)
    └─► toml (config parsing)
```

## Future Architecture Enhancements

### V2.0: Query API (Future)
```
┌──────────────────────┐
│   REST API Server    │
│   (new component)    │
└──────────┬───────────┘
           │
           ▼
    ┌──────────────┐
    │ Query Engine │
    └──────┬───────┘
           │
           ├─► index.db (fast queries)
           │
           └─► YYYY_MM_DD.pb.gz files (full events)
```

### V3.0: Distributed Storage
```
┌───────────┐     ┌───────────┐     ┌───────────┐
│  Node 1   │◄───►│  Node 2   │◄───►│  Node 3   │
│ (Primary) │     │ (Replica) │     │ (Replica) │
└───────────┘     └───────────┘     └───────────┘
      │                 │                 │
      └─────────────────┴─────────────────┘
                        │
                        ▼
              ┌─────────────────┐
              │ Consensus Layer │
              └─────────────────┘
```

## Performance Characteristics

### CLI Tool (✅ Measured)
- **Throughput**: ~200-500 events/sec with full validation (single-threaded)
- **Parallel Mode**: Scales near-linearly with CPU cores
- **Memory**: ~50-100 MB (single-threaded), ~100-500 MB (parallel)
- **Bottleneck**: Signature verification (CPU-bound)
- **Protobuf encoding**: ~500K events/sec (2.5x faster than JSON)
- **Protobuf decoding**: ~400K events/sec (2.7x faster than JSON)

### Daemon (⏳ Not Yet Measured)
- **Throughput**: Target 100+ events/sec sustained
- **Memory**: Target 50-200 MB (depends on batch size and relay count)
- **Expected Bottlenecks**:
  - Network I/O (relay connections)
  - Signature verification (CPU-bound)
  - Disk I/O (mitigated by batching)

### Storage (✅ Measured)
- **Raw Protobuf**: ~10-25% smaller than minified JSON
- **With Gzip**: ~65-97% compression ratio (depending on content patterns)
- **Combined**: ~3-40x smaller than formatted JSON (protobuf + gzip)
- **SQLite Index**: ~1-2% of event data size
- **File Format**: `YYYY_MM_DD.pb.gz` (gzip level 6 by default)

### Index Performance (✅ Measured)
- **Batch insertions**: ~171K events/sec (500-event batches)
- **Single insertions**: ~2.6K events/sec
- **Lookups**: ~307K lookups/sec
- **Queries by kind**: ~2.8K queries/sec

## CLI Commands Reference

### `convert` - Convert JSON events to protobuf

**Basic Usage:**
```bash
proton-beam convert events.jsonl
proton-beam convert - < events.jsonl  # from stdin
```

**Key Options:**
- `--output-dir <path>` - Output directory (default: `./pb_data`)
- `--batch-size <n>` - Events per batch (default: 1000)
- `--parallel <n>` - Number of threads (default: CPU count)
- `--validate-signatures=<bool>` - Validate signatures (default: true)
- `--validate-event-ids=<bool>` - Validate event IDs (default: true)
- `--filter-invalid-kinds` - Pre-filter invalid kinds (default: true)
- `--compression-level <0-9>` - Gzip level (default: 6)
- `--s3-output <uri>` - Upload to S3 (requires `s3` feature)
- `--no-progress` - Disable progress bar
- `--verbose` - Detailed logging

**Output:**
- `YYYY_MM_DD.pb.gz` - Date-organized event files
- `index.db` - SQLite index for deduplication
- `proton-beam.log` - Error and warning logs

### `merge` - Merge temporary parallel conversion files

**Usage:**
```bash
proton-beam merge ./pb_data
proton-beam merge ./pb_data --cleanup  # remove temp files after
```

**Purpose:** Combines thread-local temporary files created during parallel conversion into final date-organized files.

**Key Options:**
- `--compression-level <0-9>` - Gzip level (default: 6)
- `--cleanup` - Remove temp directory after successful merge
- `--verbose` - Detailed logging

### `index rebuild` - Rebuild SQLite index from protobuf files

**Usage:**
```bash
proton-beam index rebuild ./pb_data
proton-beam index rebuild ./pb_data --index-path ./custom/index.db
```

**Purpose:** Scans all `.pb.gz` files and rebuilds the SQLite index. Useful for:
- Recovering from index corruption
- Building index after importing files
- Migrating to new index schema

**Key Options:**
- `--index-path <path>` - Custom index location (default: `<pb_dir>/index.db`)
- `--s3-output <uri>` - Upload to S3 after rebuild (requires `s3` feature)
- `--verbose` - Detailed logging

**Performance:** Uses bulk insert mode with optimized SQLite settings (~171K events/sec).

## What's Implemented vs. Planned

### ✅ Fully Implemented (Phases 1-3)
- **Core Library** (`proton-beam-core`)
  - Protobuf schema (ProtoEvent, Tag, EventBatch)
  - JSON ↔ Protobuf conversion with validation
  - Length-delimited I/O with gzip compression
  - SQLite index with deduplication
  - Builder pattern, Display, Serde support

- **CLI Tool** (`proton-beam-cli`)
  - Single-threaded and parallel conversion modes
  - Three commands: `convert`, `merge`, `index rebuild`
  - Progress bars and error tracking
  - Date-based file organization (`.pb.gz` format)
  - Optional S3 upload support
  - Configurable validation and compression

- **ClickHouse Integration** (`proton-beam-clickhouse-import`)
  - Import protobuf events to ClickHouse
  - Schema and bootstrap scripts

### ⏳ Planned (Phase 4+)
- **Daemon** (`proton-beam-daemon`)
  - Real-time relay connections
  - WebSocket event streaming
  - Automatic relay discovery
  - Configuration file support
  - Health monitoring and metrics
  - Graceful shutdown handling

- **Advanced Features** (Phase 5+)
  - REST API for queries
  - Historical event fetching
  - Advanced filtering by tags
  - Distributed storage (Phase 6)

---

**Document Status:** Current and Accurate ✅
**Last Updated:** 2025-10-29
**Implementation Status:** CLI Complete | Daemon Planned

