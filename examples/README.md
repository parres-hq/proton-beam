# Examples Directory

This directory contains example files for testing and understanding Proton Beam.

## Files

### API Showcase Example

Located at `../proton-beam-core/examples/api_showcase.rs`

Demonstrates all the core library features in action. Run with:

```bash
cargo run -p proton-beam-core --example api_showcase
```

**Features Demonstrated:**
1. **Builder Pattern** - Fluent event construction with `ProtoEventBuilder`
2. **Display Trait** - Pretty-printed JSON output for debugging
3. **Serde Support** - Fast serialization and deserialization
4. **PartialEq** - Event comparison and equality testing
5. **FromIterator & Extend** - Ergonomic batch creation from iterators
6. **Filter & Collect** - Powerful event processing with iterator methods

This example is perfect for understanding the API and getting started with the core library.

### `sample_events.jsonl`

A comprehensive collection of sample Nostr events in JSON Lines format for testing Proton Beam. Contains 20 events with various characteristics:

#### Valid Events (14 events)

1. **Text Note with Reply** (kind 1)
   - Contains `e` and `p` tags
   - Demonstrates threaded replies

2. **User Metadata** (kind 0)
   - Profile information in JSON content
   - Name, about, picture fields

3. **Text Note with Hashtags** (kind 1)
   - Multiple `t` tags for hashtags
   - Demonstrates content tagging

4. **Contact List** (kind 3)
   - Follow list with multiple `p` tags
   - Empty content (standard for kind 3)

5. **Reaction** (kind 7)
   - `+` reaction to another event
   - References event and author

6. **Repost** (kind 6)
   - Repost of another note
   - Empty content (standard for kind 6)

7. **Relay List Metadata** (kind 10002)
   - User's relay preferences (NIP-65)
   - `r` tags with read/write markers

8. **Long-form Article** (kind 30023)
   - Addressable event with `d` tag
   - Markdown content
   - Published timestamp, title, tags

9. **Deletion Request** (kind 5)
   - References event to delete
   - Has reason in content

10. **Complex Multi-tag Event** (kind 1)
    - Demonstrates relay discovery
    - Multiple tag types: `e`, `p`, `a`, `r`, `relay`
    - Relay hints in tags

11. **Event with Invalid Tag Reference** (kind 1)
    - Short/invalid event ID in `e` tag
    - Tests tag validation

12. **Ephemeral Event** (kind 20000)
    - Ephemeral kind (20000-29999)
    - Not expected to be stored by relays

13. **Event with Advanced Tags** (kind 1)
    - `client` and `proxy` tags
    - Tests less common tag types

14. **Quote Repost** (kind 1)
    - Uses `q` tag for quote reposts
    - Includes relay hint and author

#### Duplicate Events (2 events)

15-16. **Duplicate Test Events**
    - Same event ID appearing twice
    - Tests deduplication logic

#### Malformed Events (6 events)

17. **Missing Signature**
    - Valid structure but no `sig` field
    - Should fail validation

18. **Invalid JSON Structure**
    - Completely invalid JSON format
    - Should fail parsing

19. **Empty Event ID**
    - Event with empty string as ID
    - Should fail ID validation

20. **Invalid Pubkey Format**
    - Non-hex characters in pubkey
    - Should fail validation

21. **Negative Timestamp**
    - `created_at` is negative
    - Tests boundary conditions

22. **Short Event Reference**
    - Non-standard event ID length in tag
    - Tests tag validation

## Event Kinds Reference

Events in this file demonstrate these kinds:

| Kind | Name | Count | Description |
|------|------|-------|-------------|
| 0 | Metadata | 1 | User profile information |
| 1 | Text Note | 10 | Short-form text posts |
| 3 | Contacts | 1 | Follow list |
| 5 | Deletion | 1 | Event deletion request |
| 6 | Repost | 1 | Repost of another event |
| 7 | Reaction | 1 | Reaction to another event |
| 10002 | Relay List | 1 | User's relay preferences |
| 20000 | Ephemeral | 1 | Ephemeral event (not stored) |
| 30023 | Long-form | 1 | Long-form article |

## Tag Types Demonstrated

- **`e`** - Event references (7 events)
- **`p`** - Pubkey references (10 events)
- **`a`** - Addressable event references (1 event)
- **`t`** - Hashtags (2 events)
- **`r`** - Reference/relay URLs (2 events)
- **`d`** - Identifier (addressable events) (1 event)
- **`q`** - Quote repost (1 event)
- **`title`** - Title tag (1 event)
- **`published_at`** - Publication timestamp (1 event)
- **`client`** - Client application (1 event)
- **`proxy`** - Proxy information (1 event)
- **`relay`** - Standalone relay tag (1 event)

## Relay Discovery Testing

Several events include relay information for testing auto-discovery:

- Event #8: Relay list metadata with read/write markers
- Event #10: Multiple relay hints in `e`, `p`, `a` tags plus `relay` tag
- Event #14: Quote repost with relay hint

Relays mentioned:
- `wss://relay.damus.io`
- `wss://nos.lol`
- `wss://relay.primal.net`
- `wss://relay.snort.social`

## Testing Scenarios

### Basic Conversion
```bash
proton-beam convert sample_events.jsonl
```

Expected: 14 valid events converted, 6 errors logged

### Deduplication Testing
```bash
proton-beam convert sample_events.jsonl
```

Expected: Event with ID starting with `duplicate_test_event_id_` should only be stored once

### Validation Testing

Events that should pass validation:
- Events 1-10, 12-14, 17

Events that should fail validation:
- Event 7: Missing signature
- Event 9: Invalid JSON structure
- Event 11: Empty ID
- Event 13: Invalid pubkey format
- Event 15: Negative timestamp

### Filter Testing

Filter by kind 1 (text notes only):
```bash
# In daemon config
kinds = [1]
```

Expected: 10 events stored

Filter by specific author:
```bash
# In daemon config
authors = ["79dff8f82963424e0bb02708a22e44b4980893e3a4be0fa3cb60a43b946764e3"]
```

Expected: 2 events stored (events with this pubkey)

## Notes

- **Signatures are placeholder values** - These are not real Schnorr signatures. In production, validation will fail for these events unless signature verification is disabled.
- **Event IDs are placeholder values** - These are not real SHA-256 hashes. Real events must have IDs that match the hash of their serialized content.
- **For actual testing**, you may want to generate real events using a Nostr client or the `nostr-sdk` library.

---

## `config.toml`

Example configuration file for `proton-beam-daemon`. Contains:

- Relay connection settings
- Filter configuration
- Storage options
- Historical event fetching
- Extensive comments explaining each option

### Usage

```bash
proton-beam-daemon start --config examples/config.toml
```

### Customization

Copy and modify for your use case:

```bash
cp examples/config.toml my-config.toml
# Edit my-config.toml
proton-beam-daemon start --config my-config.toml
```

## Future Examples

Coming soon:
- `real_events.jsonl` - Real Nostr events with valid signatures
- `benchmark_events.jsonl` - Large dataset for performance testing
- `minimal-config.toml` - Minimal configuration example
- `advanced-config.toml` - Advanced filtering and discovery examples

