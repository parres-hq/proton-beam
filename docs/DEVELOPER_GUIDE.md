# Developer Guide

**Version:** 1.0
**Last Updated:** 2025-10-13

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Protocol Buffers compiler (`protoc`) - optional, `prost-build` handles it
- SQLite 3.x (bundled with `rusqlite`)

### Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/proton-beam.git
cd proton-beam

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Build with release optimizations
cargo build --workspace --release
```

## Project Structure

```
proton-beam/
├── Cargo.toml                 # Workspace manifest
├── proton-beam-core/          # Core library
│   ├── Cargo.toml
│   ├── build.rs               # Protobuf codegen
│   ├── proto/
│   │   └── nostr.proto        # Protobuf schema
│   └── src/
│       ├── lib.rs             # Public API
│       ├── event.rs           # Event conversion
│       ├── validation.rs      # Validation logic
│       ├── error.rs           # Error types
│       └── storage.rs         # I/O operations
├── proton-beam-cli/           # CLI tool
│   └── src/
│       ├── main.rs
│       ├── input.rs
│       └── progress.rs
├── proton-beam-daemon/        # Daemon
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── relay_manager.rs
│       ├── event_processor.rs
│       ├── storage_manager.rs
│       ├── index.rs
│       └── discovery.rs
├── docs/                      # Documentation
└── examples/                  # Examples and samples
```

## Development Workflow

### 1. Working on Core Library

```bash
cd proton-beam-core

# Build
cargo build

# Test
cargo test

# Watch mode (requires cargo-watch)
cargo watch -x test

# Generate documentation
cargo doc --open
```

### 2. Working on CLI

```bash
cd proton-beam-cli

# Build
cargo build

# Run with sample data
cargo run -- convert ../examples/sample_events.jsonl

# Run with debug logging
RUST_LOG=debug cargo run -- convert ../examples/sample_events.jsonl

# Install locally for testing
cargo install --path .
```

### 3. Working on Daemon

```bash
cd proton-beam-daemon

# Build
cargo build

# Run with example config
cargo run -- start --config ../examples/config.toml

# Run with debug logging
RUST_LOG=debug cargo run -- start --config ../examples/config.toml
```

## Code Style

### Formatting

```bash
# Format all code
cargo fmt --all

# Check formatting without modifying
cargo fmt --all -- --check
```

### Linting

```bash
# Run clippy on all targets
cargo clippy --all-targets --all-features

# Clippy with strict warnings
cargo clippy --all-targets --all-features -- -D warnings
```

### Best Practices

1. **Error Handling**
   - Use `anyhow::Result` for application errors
   - Use `thiserror` for library errors
   - Provide context with `.context()` or `.with_context()`

2. **Documentation**
   - Add doc comments to all public APIs
   - Include examples in doc comments
   - Document error conditions

3. **Testing**
   - Write unit tests for core logic
   - Write integration tests for end-to-end flows
   - Use `#[cfg(test)]` modules

4. **Async Code**
   - Use `tokio` for async runtime
   - Prefer structured concurrency
   - Avoid blocking in async contexts

## Testing

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p proton-beam-core

# Specific test
cargo test -p proton-beam-core validation_tests

# With output
cargo test --workspace -- --nocapture

# With logging
RUST_LOG=debug cargo test --workspace -- --nocapture
```

### Writing Tests

#### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_conversion() {
        let json = r#"{"id":"...", "pubkey":"...", ...}"#;
        let proto = json_to_proto(json).unwrap();
        assert_eq!(proto.id, "...");
    }

    #[test]
    fn test_validation_invalid_id() {
        let event = create_invalid_event();
        assert!(validate_event(&event).is_err());
    }
}
```

#### Integration Tests

```rust
// tests/integration_test.rs
use proton_beam_core::*;

#[test]
fn test_cli_converts_jsonl_file() {
    let input = "examples/sample_events.jsonl";
    let output = tempdir().unwrap();

    // Run CLI
    convert_file(input, output.path()).unwrap();

    // Verify output
    assert!(output.path().join("2025_10_13.pb").exists());
}
```

### Test Data

Use the provided sample events:

```rust
const SAMPLE_EVENT: &str = include_str!("../examples/sample_events.jsonl");

#[test]
fn test_with_sample_data() {
    for line in SAMPLE_EVENT.lines() {
        // Test each event
    }
}
```

## Building Protobuf Schema

The protobuf schema is automatically compiled during build:

```bash
# Trigger rebuild
cargo clean -p proton-beam-core
cargo build -p proton-beam-core
```

Generated code location:
```
target/debug/build/proton-beam-core-*/out/nostr.rs
```

Manual protobuf compilation (optional):

```bash
protoc --rust_out=. proto/nostr.proto
```

## Debugging

### Debug Logging

```bash
# Set log level
export RUST_LOG=debug
export RUST_LOG=proton_beam_core=trace

# Run with logging
cargo run --bin proton-beam-daemon
```

### Debug Builds

```bash
# Build with debug symbols
cargo build

# Run with debugger (lldb on macOS, gdb on Linux)
rust-lldb target/debug/proton-beam-daemon
```

### Profiling

```bash
# CPU profiling with cargo-flamegraph
cargo install flamegraph
cargo flamegraph --bin proton-beam-cli -- convert large_file.jsonl

# Memory profiling with valgrind
cargo build
valgrind --tool=massif target/debug/proton-beam-cli convert test.jsonl
```

## Benchmarking

### Writing Benchmarks

```rust
// benches/conversion_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use proton_beam_core::json_to_proto;

fn benchmark_conversion(c: &mut Criterion) {
    let json = r#"{"id":"...", ...}"#;

    c.bench_function("json_to_proto", |b| {
        b.iter(|| json_to_proto(black_box(json)))
    });
}

criterion_group!(benches, benchmark_conversion);
criterion_main!(benches);
```

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench --workspace

# Specific benchmark
cargo bench --bench conversion_bench

# With baseline comparison
cargo bench --workspace -- --save-baseline main
# ... make changes ...
cargo bench --workspace -- --baseline main
```

## Common Tasks

### Adding a New Event Field

1. Update `proto/nostr.proto`:
```protobuf
message Event {
    // ... existing fields ...
    string new_field = 8;  // Use next available number
}
```

2. Rebuild:
```bash
cargo build -p proton-beam-core
```

3. Update conversion code in `event.rs`:
```rust
proto_event.new_field = json_event.new_field.clone();
```

4. Write tests:
```rust
#[test]
fn test_new_field_conversion() { ... }
```

### Adding a New CLI Command

1. Update `proton-beam-cli/src/main.rs`:
```rust
#[derive(Parser)]
enum Commands {
    Convert(ConvertArgs),
    NewCommand(NewCommandArgs),  // Add this
}
```

2. Implement command handler:
```rust
fn handle_new_command(args: NewCommandArgs) -> Result<()> {
    // Implementation
}
```

3. Add tests:
```rust
#[test]
fn test_new_command() { ... }
```

### Adding a New Configuration Option

1. Update `proton-beam-daemon/src/config.rs`:
```rust
#[derive(Deserialize)]
pub struct DaemonConfig {
    // ... existing fields ...
    pub new_option: bool,
}
```

2. Update `examples/config.toml`:
```toml
[daemon]
new_option = true
```

3. Use in daemon code:
```rust
if config.daemon.new_option {
    // Do something
}
```

## Continuous Integration

### GitHub Actions

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --workspace
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo fmt --all -- --check
```

## Release Process

### Version Bumping

1. Update version in all `Cargo.toml` files:
```toml
[package]
version = "0.2.0"
```

2. Update CHANGELOG.md

3. Commit and tag:
```bash
git commit -am "Bump version to 0.2.0"
git tag v0.2.0
git push && git push --tags
```

### Building Release Binaries

```bash
# Build optimized binaries
cargo build --workspace --release

# Binaries location
ls target/release/proton-beam*
```

### Publishing to crates.io

```bash
# Publish core library first
cd proton-beam-core
cargo publish

# Then CLI
cd ../proton-beam-cli
cargo publish

# Finally daemon
cd ../proton-beam-daemon
cargo publish
```

## Useful Commands

```bash
# Check for outdated dependencies
cargo outdated

# Update dependencies
cargo update

# Security audit
cargo audit

# Show dependency tree
cargo tree

# Clean build artifacts
cargo clean

# Build documentation for all crates
cargo doc --workspace --no-deps --open

# Count lines of code
tokei

# Check code coverage (requires cargo-tarpaulin)
cargo tarpaulin --workspace --out Html
```

## IDE Setup

### VS Code

Recommended extensions:
- `rust-analyzer`: Rust language support
- `CodeLLDB`: Debugging support
- `Even Better TOML`: TOML syntax
- `Proto`: Protobuf syntax

`.vscode/settings.json`:
```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.features": "all"
}
```

### IntelliJ IDEA / CLion

- Install Rust plugin
- Enable "Use clippy" in Rust settings
- Enable "Use rustfmt" for formatting

## Resources

### Documentation
- [Rust Book](https://doc.rust-lang.org/book/)
- [Async Book](https://rust-lang.github.io/async-book/)
- [Protobuf Guide](https://protobuf.dev/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)

### Nostr Resources
- [NIP-01](https://github.com/nostr-protocol/nips/blob/master/01.md)
- [nostr-sdk docs](https://docs.rs/nostr-sdk/)
- [Nostr Protocol](https://nostr.com)

### Crates
- [prost](https://docs.rs/prost/)
- [clap](https://docs.rs/clap/)
- [tokio](https://docs.rs/tokio/)
- [rusqlite](https://docs.rs/rusqlite/)

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/yourusername/proton-beam/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/proton-beam/discussions)
- **Rust Community**: [Rust Users Forum](https://users.rust-lang.org/)

---

**Document Status:** Complete
**Last Updated:** 2025-10-13

