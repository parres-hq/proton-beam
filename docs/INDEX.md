# Proton Beam Documentation Index

Welcome to the Proton Beam documentation! This index will help you find the information you need.

## 📚 Documentation Overview

### For Users

- **[README](../README.md)** - Quick start guide and project overview
- **[Configuration Examples](../examples/)** - Sample configuration files and test data

### For Developers

- **[Project Status](STATUS.md)** ⭐ *Start here for current progress*
  - Implementation status
  - Phase completion summary
  - API overview
  - Next steps
  - Quality metrics

- **[Project Plan](PROJECT_PLAN.md)** - Complete project overview
  - Executive summary
  - Architecture overview
  - Implementation phases
  - Component specifications
  - Success metrics

- **[Architecture](ARCHITECTURE.md)** - System architecture and design
  - System diagrams
  - Data flow
  - Concurrency model
  - Performance characteristics

- **[Protobuf Schema](PROTOBUF_SCHEMA.md)** - Protobuf schema documentation
  - Schema definition
  - Design rationale
  - Conversion examples
  - Size comparisons

- **[Developer Guide](DEVELOPER_GUIDE.md)** - Development workflows and best practices
  - Setup instructions
  - Code style guidelines
  - Testing strategies
  - Common tasks
  - Release process

## 🚀 Quick Navigation

### I want to...

#### Use Proton Beam
- **Convert JSON events to protobuf** → [README - Quick Start](../README.md#quick-start)
- **Run the relay daemon** → [README - Daemon Usage](../README.md#daemon-usage)
- **Configure the daemon** → [Example Config](../examples/config.toml)

#### Understand the Project
- **Learn about the architecture** → [Architecture Document](ARCHITECTURE.md)
- **Understand the data format** → [Protobuf Schema](PROTOBUF_SCHEMA.md)
- **See the full plan** → [Project Plan](PROJECT_PLAN.md)

#### Develop & Contribute
- **Set up development environment** → [Developer Guide - Setup](DEVELOPER_GUIDE.md#setup)
- **Write tests** → [Developer Guide - Testing](DEVELOPER_GUIDE.md#testing)
- **Add new features** → [Developer Guide - Common Tasks](DEVELOPER_GUIDE.md#common-tasks)
- **Submit changes** → [Contributing Guidelines](CONTRIBUTING.md)

## 📖 Document Summaries

### [PROJECT_PLAN.md](PROJECT_PLAN.md)
**Purpose:** Complete project specification and implementation roadmap

**Contents:**
- Project goals and requirements
- Workspace structure
- Component specifications (CLI, daemon, core)
- Protobuf schema design
- Validation strategy
- Performance targets
- Implementation phases (6 phases)
- Testing strategy
- Dependencies and risks
- Success metrics

**Read this if:** You want to understand the complete scope and plan for Proton Beam.

---

### [ARCHITECTURE.md](ARCHITECTURE.md)
**Purpose:** System architecture and design decisions

**Contents:**
- System overview diagrams
- CLI tool architecture
- Daemon architecture
- Data flow diagrams
- Storage architecture
- Concurrency model
- Key design decisions
- Component dependencies
- Performance characteristics

**Read this if:** You want to understand how Proton Beam is built and why design decisions were made.

---

### [PROTOBUF_SCHEMA.md](PROTOBUF_SCHEMA.md)
**Purpose:** Detailed protobuf schema documentation

**Contents:**
- Complete proto file definition
- Field descriptions
- Design rationale
- Conversion examples
- Storage format
- Size comparisons
- Event kind reference
- Validation rules
- Schema evolution strategy

**Read this if:** You need to understand the data format or work with the protobuf schema.

---

### [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md)
**Purpose:** Practical guide for developers

**Contents:**
- Development setup
- Project structure
- Development workflow
- Code style and best practices
- Testing guide
- Debugging techniques
- Benchmarking
- Common development tasks
- CI/CD and release process
- Useful commands and resources

**Read this if:** You're developing Proton Beam or contributing code.

---

### [../README.md](../README.md)
**Purpose:** Project introduction and quick start

**Contents:**
- Feature highlights
- Installation instructions
- Basic usage examples
- Configuration example
- Project structure
- Development status and roadmap
- Use cases
- Technical overview

**Read this if:** You're new to Proton Beam and want a quick overview.

---

### [../examples/README.md](../examples/README.md)
**Purpose:** Documentation for example files

**Contents:**
- Sample events description
- Event kinds breakdown
- Tag types demonstrated
- Testing scenarios
- Configuration file documentation

**Read this if:** You want to understand the test data or example configurations.

## 🗂️ Document Relationships

```
README.md (Entry Point)
    │
    ├─► PROJECT_PLAN.md (Complete specification)
    │       │
    │       ├─► ARCHITECTURE.md (System design)
    │       │       │
    │       │       └─► PROTOBUF_SCHEMA.md (Data format)
    │       │
    │       └─► DEVELOPER_GUIDE.md (Development practices)
    │
    └─► examples/
            ├─► config.toml (Configuration example)
            ├─► sample_events.jsonl (Test data)
            └─► README.md (Examples documentation)
```

## 📊 Project Status

**Current Phase:** Phase 1.5 Complete ✅

**Implementation Phases:**
1. ✅ Core Library & Protobuf Schema (Complete)
2. ✅ Enhanced API Features (Complete)
3. ⏳ CLI Tool (Next)
4. ⏳ SQLite Index & Deduplication
5. ⏳ Relay Daemon (Core)
6. ⏳ Relay Discovery & Advanced Features
7. ⏳ Testing, Documentation & Polish

See [Project Status](STATUS.md) for detailed progress.

## 🔑 Key Concepts

### Protobuf
Binary serialization format that's more efficient than JSON. Proton Beam converts Nostr events from JSON to protobuf for storage.

### Length-Delimited Format
Storage format where each event is prefixed with its size, allowing multiple events in one file without a wrapper message.

### Event Validation
Verification of event ID (SHA-256 hash) and Schnorr signature to ensure authenticity.

### Deduplication
Ensuring each event is stored only once, even when received from multiple relays.

### Relay Discovery
Automatically finding and connecting to new Nostr relays based on relay hints in event tags.

## 🛠️ Key Technologies

- **Rust** - Programming language
- **Protocol Buffers** - Binary serialization
- **nostr-sdk** - Nostr protocol implementation
- **Tokio** - Async runtime
- **SQLite** - Event index database
- **WebSocket** - Relay communication

## 📝 File Formats

### Input
- `.jsonl` - JSON Lines (one event per line)
- JSON stream via stdin
- WebSocket messages from relays

### Output
- `.pb` - Protobuf binary files (length-delimited)
- `errors.jsonl` - Malformed events with error reasons
- `.index.db` - SQLite database for event index

### Configuration
- `.toml` - TOML configuration files

## 🎯 Common Workflows

### Converting Events (CLI)
1. Prepare `.jsonl` file with Nostr events
2. Run: `proton-beam convert events.jsonl`
3. Find output in `./pb_data/YYYY_MM_DD.pb` files
4. Check `errors.jsonl` for any malformed events

### Running the Daemon
1. Create/customize `config.toml`
2. Run: `proton-beam-daemon start --config config.toml`
3. Daemon connects to relays and processes events
4. Events stored in configured output directory
5. Monitor logs for status and errors

### Developing a Feature
1. Read relevant documentation
2. Write tests first (TDD)
3. Implement feature
4. Run tests: `cargo test`
5. Check with clippy: `cargo clippy`
6. Format code: `cargo fmt`
7. Submit PR

## 🔗 External Resources

### Nostr Protocol
- [NIP-01: Basic Protocol](https://github.com/nostr-protocol/nips/blob/master/01.md)
- [All NIPs](https://github.com/nostr-protocol/nips)
- [Nostr Website](https://nostr.com)

### Protocol Buffers
- [Protobuf Guide](https://protobuf.dev/)
- [Language Guide (proto3)](https://protobuf.dev/programming-guides/proto3/)
- [Encoding](https://protobuf.dev/programming-guides/encoding/)

### Rust
- [Rust Book](https://doc.rust-lang.org/book/)
- [Async Book](https://rust-lang.github.io/async-book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)

### Libraries
- [prost](https://docs.rs/prost/) - Protobuf for Rust
- [nostr-sdk](https://docs.rs/nostr-sdk/) - Nostr SDK
- [tokio](https://tokio.rs/) - Async runtime
- [clap](https://docs.rs/clap/) - CLI parsing

## 💬 Getting Help

- **Documentation Issues**: [File an issue](https://github.com/yourusername/proton-beam/issues/new)
- **Questions**: [GitHub Discussions](https://github.com/yourusername/proton-beam/discussions)
- **Bug Reports**: [Bug Report Template](https://github.com/yourusername/proton-beam/issues/new?template=bug_report.md)
- **Feature Requests**: [Feature Request Template](https://github.com/yourusername/proton-beam/issues/new?template=feature_request.md)

## 📋 Documentation Checklist

Current documentation status:

- ✅ README
- ✅ Project Plan
- ✅ Architecture Document
- ✅ Protobuf Schema Documentation
- ✅ Developer Guide
- ✅ Example Configuration
- ✅ Sample Test Data
- ✅ Examples Documentation
- ⏳ API Documentation (auto-generated from code)
- ⏳ User Guide (detailed usage)
- ⏳ Troubleshooting Guide
- ⏳ FAQ
- ⏳ Contributing Guide
- ⏳ Code of Conduct

## 🤝 Contributing to Documentation

Found an error or want to improve the documentation?

1. **Small fixes**: Edit directly and submit PR
2. **New sections**: Open an issue to discuss first
3. **Examples**: Add to `/examples` directory
4. **Diagrams**: Use ASCII art or link to external diagrams

### Documentation Standards

- Use markdown for all docs
- Keep lines under 100 characters when possible
- Use code blocks with language hints
- Include table of contents for long documents
- Add "Last Updated" dates to major documents
- Use relative links for internal references

---

**Last Updated:** 2025-10-13
**Documentation Version:** 1.0

