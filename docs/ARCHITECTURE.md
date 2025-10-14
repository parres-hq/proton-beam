# Proton Beam Architecture

**Version:** 1.0
**Last Updated:** 2025-10-13

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

## CLI Tool Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    proton-beam CLI                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Input Sources:                                             │
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
│         └────────────┬───────────────────┘                 │
│                      ▼                                     │
│         ┌────────────────────────────────┐                 │
│         │   Validation Pipeline          │                 │
│         │   - JSON parse                 │                 │
│         │   - Event ID check             │                 │
│         │   - Signature verify           │                 │
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
│  │(500 events) │   │.log          │                       │
│  └──────┬──────┘                                           │
│         ▼                                                  │
│  ┌─────────────────────────────┐                          │
│  │  Date-based File Writer     │                          │
│  │  - Group by created_at      │                          │
│  │  - Length-delimited format  │                          │
│  │  - YYYY_MM_DD.pb.gz files   │                          │
│  │  - Gzip compressed          │                          │
│  └──────┬──────────────────────┘                          │
│         ▼                                                  │
│  ┌──────────────────────┐                                 │
│  │  Output Files:       │                                 │
│  │  2025_10_13.pb.gz    │                                 │
│  │  2025_10_14.pb.gz    │                                 │
│  │  ...                 │                                 │
│  └──────────────────────┘                                 │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

## Daemon Architecture

```
┌───────────────────────────────────────────────────────────────────────┐
│                     proton-beam-daemon                                │
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
  [File Writer] ──► 2025_10_13.pb
                    2025_10_14.pb
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
│  ./nostr_events/                   │
│                                    │
│  ├── 2025_10_13.pb                 │ ◄── Events from Oct 13
│  │   ┌────────────────────────┐    │
│  │   │ [len][Event 1 binary]  │    │
│  │   │ [len][Event 2 binary]  │    │     Length-delimited
│  │   │ [len][Event 3 binary]  │    │     protobuf format
│  │   │ ...                    │    │
│  │   └────────────────────────┘    │
│  │                                 │
│  ├── 2025_10_14.pb                 │ ◄── Events from Oct 14
│  │                                 │
│  ├── 2025_10_15.pb                 │ ◄── Events from Oct 15
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

- Easy to archive/compress old dates
- Reasonable file sizes (depends on relay traffic)
- Out-of-order events handled gracefully

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
- Configurable batch size for different use cases

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

### V2.0: Query API
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
           └─► YYYY_MM_DD.pb.gz files (full events, gzip compressed)
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

### CLI Tool
- **Throughput**: ~200-500 events/sec (with validation)
- **Memory**: ~50-100 MB
- **Bottleneck**: Signature verification (CPU-bound)

### Daemon
- **Throughput**: 100+ events/sec sustained
- **Memory**: 50-200 MB (depends on batch size and relay count)
- **Bottlenecks**:
  - Network I/O (relay connections)
  - Signature verification (CPU-bound)
  - Disk I/O (mitigated by batching)

### Storage
- **Size**: ~10-25% smaller than minified JSON (raw protobuf)
- **Compression**: Gzip compression enabled by default (~65-97% space savings vs JSON depending on data patterns)
- **Combined**: ~3-40x smaller than JSON (protobuf + gzip)
- **Index**: ~1-2% of event data size

---

**Document Status:** Complete
**Last Updated:** 2025-10-13

