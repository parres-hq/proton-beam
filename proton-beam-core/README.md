# Proton Beam Core

Core library for converting Nostr events between JSON and Protocol Buffer formats.

## Features

- **JSON ↔ Protobuf Conversion**: Convert Nostr events to/from efficient binary format
- **Event Validation**: Verify event IDs (SHA-256) and Schnorr signatures
- **Length-Delimited I/O**: Stream events efficiently with varint length encoding
- **Builder Pattern**: Fluent API for constructing events
- **Serde Support**: Direct JSON serialization/deserialization
- **Display Trait**: Human-readable pretty-printed JSON output
- **FromIterator**: Ergonomic batch creation from iterators
- **PartialEq/Eq**: Easy testing and comparisons
- **Type-Safe**: Strongly-typed protobuf schema for Nostr events
- **Well-Tested**: 62 tests with comprehensive coverage

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
proton-beam-core = "0.1.0"
```

## Usage

### Builder Pattern (Recommended for Construction/Testing)

```rust
use proton_beam_core::ProtoEventBuilder;

let event = ProtoEventBuilder::new()
    .id("abc123")
    .pubkey("def456")
    .created_at(1234567890)
    .kind(1)
    .content("Hello, Nostr!")
    .add_tag(vec!["e", "event_id"])
    .add_tag(vec!["p", "pubkey_id"])
    .sig("sig789")
    .build();
```

### Convert JSON to Protobuf (Idiomatic with Validation)

```rust
use proton_beam_core::{ProtoEvent, validate_event};
use std::convert::TryFrom;

let json = r#"{
    "id":"...",
    "pubkey":"...",
    "created_at":1234567890,
    "kind":1,
    "tags":[],
    "content":"Hello Nostr!",
    "sig":"..."
}"#;

// Convert to protobuf using TryFrom trait (includes nostr-sdk validation)
let event = ProtoEvent::try_from(json)?;

// Additional validation
validate_event(&event)?;
```

### Convert Protobuf to JSON (Idiomatic)

```rust
use proton_beam_core::ProtoEvent;
use std::convert::TryFrom;

let event = ProtoEventBuilder::new()
    .id("...")
    .pubkey("...")
    .created_at(1234567890)
    .kind(1)
    .content("Hello!")
    .sig("...")
    .build();

// Convert to JSON using TryFrom trait
let json = String::try_from(&event)?;
```

### Serde Serialization (Direct, No Validation)

```rust
use proton_beam_core::ProtoEvent;

// Serialize using serde (faster, no validation)
let json = serde_json::to_string(&event)?;
let json_pretty = serde_json::to_string_pretty(&event)?;

// Deserialize using serde
let event: ProtoEvent = serde_json::from_str(&json)?;
```

### Display Trait for Debugging

```rust
let event = ProtoEventBuilder::new()
    .id("debug_test")
    .content("Debug me!")
    .build();

// Pretty-printed JSON for debugging
println!("{}", event);
```

### Collecting Events into Batches

```rust
use proton_beam_core::{EventBatch, ProtoEventBuilder};

let events = vec![
    ProtoEventBuilder::new().id("1").build(),
    ProtoEventBuilder::new().id("2").build(),
    ProtoEventBuilder::new().id("3").build(),
];

// Collect using FromIterator
let batch: EventBatch = events.into_iter().collect();

// Or extend an existing batch
let mut batch = EventBatch { events: vec![] };
batch.extend(new_events);
```

### Filtering and Processing

```rust
use proton_beam_core::{EventBatch, ProtoEvent};

let all_events: Vec<ProtoEvent> = load_events();

// Filter and collect
let text_notes: EventBatch = all_events
    .into_iter()
    .filter(|e| e.kind == 1)
    .collect();
```

### Convenience Functions (Backward Compatible)

```rust
use proton_beam_core::{json_to_proto, proto_to_json};

// Convert JSON to protobuf
let event = json_to_proto(json)?;

// Convert protobuf to JSON
let json = proto_to_json(&event)?;
```

### Write Events to File

```rust
use proton_beam_core::{write_event_delimited, ProtoEvent};
use std::fs::File;

let mut file = File::create("events.pb")?;
write_event_delimited(&mut file, &event)?;
```

### Read Events from File

```rust
use proton_beam_core::read_events_delimited;
use std::fs::File;

let file = File::open("events.pb")?;
for result in read_events_delimited(file) {
    let event = result?;
    println!("Event ID: {}", event.id);
}
```

### Write/Read with Gzip Compression

```rust
use proton_beam_core::{
    create_gzip_encoder, create_gzip_decoder,
    write_event_delimited, read_events_delimited, ProtoEvent
};
use std::fs::File;
use std::io::BufWriter;

// Write compressed
let file = File::create("events.pb.gz")?;
let gz = create_gzip_encoder(file);
let mut writer = BufWriter::new(gz);
write_event_delimited(&mut writer, &event)?;
drop(writer); // Ensure gzip stream is finished

// Read compressed
let file = File::open("events.pb.gz")?;
let gz = create_gzip_decoder(file);
for result in read_events_delimited(gz) {
    let event = result?;
    println!("Event ID: {}", event.id);
}
```

The CLI automatically uses gzip compression for all `.pb.gz` output files, providing ~65-97% space savings compared to JSON.

## API Design Decisions

### Why `ProtoEvent` instead of `Event`?

To avoid naming conflicts with `nostr_sdk::Event` and make it clear this is the protobuf representation.

### When to use Serde vs TryFrom?

- **Use `TryFrom`** when parsing untrusted Nostr JSON that needs full validation
- **Use serde** for internal serialization where you trust the data (faster, no overhead)
- **Use Display** for debugging and human-readable output

### PartialEq and Testing

All protobuf types derive `PartialEq` and `Eq`, making it easy to assert equality in tests:

```rust
assert_eq!(event1, event2);
```

## Protobuf Schema

```protobuf
// Named ProtoEvent to avoid conflicts with nostr-sdk::Event
message ProtoEvent {
  string id = 1;              // 32-byte hex event ID
  string pubkey = 2;          // 32-byte hex public key
  int64 created_at = 3;       // Unix timestamp
  int32 kind = 4;             // Event kind (0-65535)
  repeated Tag tags = 5;      // Event tags
  string content = 6;         // Event content
  string sig = 7;             // 64-byte hex signature
}

message Tag {
  repeated string values = 1; // Tag values
}

message EventBatch {
  repeated ProtoEvent events = 1;
}
```

## Validation

The library provides two levels of validation:

1. **Basic Validation** (`validate_basic_fields`): Fast checks for:
   - Correct hex format and length for ID, pubkey, and signature
   - Valid timestamp (non-negative)
   - Valid kind (0-65535)

2. **Full Validation** (`validate_event`): Includes basic validation plus:
   - Event ID verification (SHA-256 hash check)
   - Schnorr signature verification using secp256k1

## Storage Format

Events are stored using length-delimited protobuf encoding:

```
[varint length][Event 1 binary]
[varint length][Event 2 binary]
[varint length][Event 3 binary]
...
```

This format allows:
- Append-only writes
- Streaming reads without loading entire file
- Memory-efficient processing

## Performance

- **Encoding**: ~2.5x faster than JSON
- **Decoding**: ~2.7x faster than JSON
- **Size**: 10-25% smaller than minified JSON

## Testing

Run tests:

```bash
cargo test -p proton-beam-core
```

With output:

```bash
cargo test -p proton-beam-core -- --nocapture
```

## Documentation

Generate and view documentation:

```bash
cargo doc -p proton-beam-core --open
```

## License

MIT

## See Also

- [Project Plan](../docs/PROJECT_PLAN.md)
- [Protobuf Schema Documentation](../docs/PROTOBUF_SCHEMA.md)
- [Architecture Overview](../docs/ARCHITECTURE.md)
- [Improvements Log](../IMPROVEMENTS_LOG.md)
- [Nostr Protocol (NIP-01)](https://github.com/nostr-protocol/nips/blob/master/01.md)
- [Protocol Buffers](https://protobuf.dev/)
- [nostr-sdk](https://docs.rs/nostr-sdk/)
