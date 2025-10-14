# Proton Beam CLI Guide

Complete guide to using the `proton-beam` CLI tool for converting Nostr events from JSON to Protocol Buffers.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Basic Usage](#basic-usage)
3. [Command Reference](#command-reference)
4. [Example Scripts](#example-scripts)
5. [Common Workflows](#common-workflows)
6. [Integration Examples](#integration-examples)
7. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Installation

```bash
# Build the CLI
cargo build --release -p proton-beam-cli

# Add to PATH (optional)
export PATH="$PATH:$(pwd)/target/release"

# Or install globally
cargo install --path proton-beam-cli
```

### First Conversion

```bash
# Convert sample events
proton-beam convert examples/sample_events.jsonl

# Convert your own file
proton-beam convert events.jsonl --output-dir ./pb_data
```

### Try Example Scripts

```bash
# Simple conversion
./examples/scripts/basic_conversion.sh

# Compare JSON vs Protobuf sizes
./examples/scripts/compare_sizes.sh
```

---

## Basic Usage

### Convert a file

```bash
proton-beam convert events.jsonl
```

**Output:**
```
⠋ [00:00:05] Processed: 1000 | Valid: 987 | Errors: 13

📊 Conversion Summary:
  Total lines processed: 1000
  ✅ Valid events:       987
  ❌ Invalid events:     13
  Success rate:         98.7%
```

### Read from stdin

```bash
cat events.jsonl | proton-beam convert -
```

### Stream from relay

Requires [nak](https://github.com/fiatjaf/nak):

```bash
nak req -k 1 --limit 1000 wss://relay.damus.io | proton-beam convert -
```

### Output structure

Events are organized by date:

```
./pb_data/
├── 2025_10_13.pb      # Events from Oct 13
├── 2025_10_14.pb      # Events from Oct 14
└── proton-beam.log    # Error and warning logs
```

---

## Command Reference

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--output-dir` | `-o` | `./pb_data` | Output directory for protobuf files |
| `--batch-size` | `-b` | `500` | Number of events to buffer before writing |
| `--no-validate` | | `false` | Skip event validation (faster, less safe) |
| `--verbose` | `-v` | `false` | Enable detailed logging |
| `--no-progress` | | `false` | Disable progress bar |
| `--help` | `-h` | | Show help information |

### Performance Guidelines

| Scenario | Batch Size | Validate | Progress |
|----------|-----------|----------|----------|
| **Streaming** | 50-100 | ✅ Yes | ❌ No |
| **Files** | 500-1000 | ✅ Yes | ✅ Yes |
| **Large Batch** | 2000+ | ❌ No* | ❌ No |
| **Debugging** | 100 | ✅ Yes | ✅ Yes |

*Only skip validation for trusted, pre-validated data

### Examples

```bash
# Basic conversion
proton-beam convert events.jsonl

# Custom output directory
proton-beam convert events.jsonl --output-dir ~/archive

# High-performance mode (trusted data only)
proton-beam convert events.jsonl --no-validate --batch-size 2000

# Verbose debugging
proton-beam convert events.jsonl --verbose

# Background job (no progress bar)
proton-beam convert events.jsonl --no-progress
```

---

## Example Scripts

The `scripts/` directory contains ready-to-run examples. All scripts are executable and well-documented.

### Available Scripts

| Script | Purpose | Prerequisites |
|--------|---------|---------------|
| **[basic_conversion.sh](scripts/basic_conversion.sh)** | Simple file conversion | proton-beam |
| **[stream_from_relay.sh](scripts/stream_from_relay.sh)** | Fetch & convert from relay | proton-beam, nak |
| **[fast_conversion.sh](scripts/fast_conversion.sh)** | Maximum performance mode | proton-beam |
| **[daily_backup.sh](scripts/daily_backup.sh)** | Automated backup workflow | proton-beam, nak |
| **[analyze_errors.sh](scripts/analyze_errors.sh)** | Error analysis & reporting | proton-beam, jq |
| **[compare_sizes.sh](scripts/compare_sizes.sh)** | JSON vs Protobuf comparison | proton-beam, bc |
| **[test_examples.sh](scripts/test_examples.sh)** | Validate all scripts | bash |

### Usage

```bash
# Make scripts executable (first time only)
chmod +x scripts/*.sh

# Run a script
./scripts/basic_conversion.sh

# Run with custom input
./scripts/fast_conversion.sh my_events.jsonl

# Analyze errors from previous conversion
tail -n 50 ./pb_data/proton-beam.log  # View recent errors
```

See **[scripts/README.md](scripts/README.md)** for detailed documentation.

---

## Common Workflows

### Daily Backup

Automated backup from multiple relays:

```bash
#!/bin/bash
DATE=$(date +%Y-%m-%d)
OUTPUT_DIR="$HOME/nostr_backups/$DATE"

# Fetch last 24 hours
nak req --since "24 hours ago" \
  wss://relay.damus.io \
  wss://nos.lol \
  wss://relay.primal.net | \
  proton-beam convert - --output-dir "$OUTPUT_DIR"

# Compress
tar -czf "$OUTPUT_DIR.tar.gz" "$OUTPUT_DIR"
rm -rf "$OUTPUT_DIR"
```

Or use: `./scripts/daily_backup.sh`

### Archive by Event Kind

```bash
# Archive specific kinds
for kind in 0 1 3 7; do
  nak req -k $kind --limit 1000 wss://relay.damus.io | \
    proton-beam convert - --output-dir "./kind_$kind"
done
```

### Author-Specific Archive

```bash
AUTHOR="3bf0c63fcb93463407af97a5e5ee64fa883d107ef9e558472c4eb9aaaefa459d"

nak req -a "$AUTHOR" wss://relay.damus.io | \
  proton-beam convert - --output-dir "./author_archive"
```

### Process Multiple Files

```bash
# Sequential processing
for file in events/*.jsonl; do
  proton-beam convert "$file" --output-dir ./pb_data
done

# Parallel processing (requires GNU parallel)
ls events/*.jsonl | parallel -j 4 \
  'proton-beam convert {} --output-dir ./pb_data --no-progress'
```

### Size Comparison

```bash
# Compare storage efficiency
JSON_SIZE=$(wc -c < events.jsonl)
proton-beam convert events.jsonl --output-dir /tmp/pb
PB_SIZE=$(find /tmp/pb -name "*.pb" -exec wc -c {} + | awk '{sum+=$1} END {print sum}')
SAVED=$((100 - (PB_SIZE * 100 / JSON_SIZE)))
echo "Space saved: ${SAVED}%"
```

Or use: `./scripts/compare_sizes.sh`

---

## Integration Examples

### With jq (JSON filtering)

```bash
# Filter by kind before conversion
cat events.jsonl | jq -c 'select(.kind == 1)' | proton-beam convert -

# Filter by time range
cat events.jsonl | \
  jq -c 'select(.created_at > 1697000000)' | \
  proton-beam convert -

# Remove specific tags
cat events.jsonl | \
  jq -c '.tags |= map(select(.[0] != "proxy"))' | \
  proton-beam convert -
```

### With nak (Nostr CLI)

```bash
# Fetch recent notes
nak req -k 1 --limit 1000 wss://relay.damus.io | proton-beam convert -

# Fetch by multiple authors
nak req -a <pubkey1> -a <pubkey2> wss://nos.lol | proton-beam convert -

# Stream continuously
nak req --stream wss://relay.damus.io | \
  proton-beam convert - --batch-size 100
```

### With Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p proton-beam-cli

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/proton-beam /usr/local/bin/
ENTRYPOINT ["proton-beam"]
```

```bash
docker build -t proton-beam .
docker run -v $(pwd)/data:/data proton-beam convert /data/events.jsonl -o /data/output
```

### Systemd Timer (Scheduled Backup)

```ini
# /etc/systemd/system/proton-backup.service
[Unit]
Description=Proton Beam Nostr Backup

[Service]
Type=oneshot
ExecStart=/usr/local/bin/backup_script.sh
User=nostr
```

```ini
# /etc/systemd/system/proton-backup.timer
[Unit]
Description=Daily Nostr Backup

[Timer]
OnCalendar=daily
Persistent=true

[Install]
WantedBy=timers.target
```

```bash
sudo systemctl enable --now proton-backup.timer
```

---

## Troubleshooting

### Error Checking

```bash
# Check if errors occurred
[ -f ./pb_data/proton-beam.log ] && echo "Log file exists" || echo "No log file"

# View recent errors
tail -n 50 ./pb_data/proton-beam.log

# Count error lines
grep -c "ERROR" ./pb_data/proton-beam.log

# Analyze error types
grep "ERROR" ./pb_data/proton-beam.log | cut -d' ' -f3 | sort | uniq -c | sort -rn

# View specific line errors
grep "line=" ./pb_data/proton-beam.log
```

### Common Issues

| Problem | Solution |
|---------|----------|
| **"No such file or directory"** | Check file path or create output directory: `mkdir -p ./pb_data` |
| **"Permission denied"** | Check write permissions: `chmod u+w ./pb_data` |
| **High memory usage** | Reduce batch size: `--batch-size 100` |
| **Slow processing** | Skip validation if safe: `--no-validate` |
| **No progress bar showing** | Don't redirect stdout, or use `--no-progress` explicitly |
| **All events failing** | Check JSON format, ensure one event per line |

### Analyzing Errors

```bash
# View all errors with line numbers
grep "ERROR" ./pb_data/proton-beam.log

# Filter by error type
grep "parse_error" ./pb_data/proton-beam.log
grep "validation_error" ./pb_data/proton-beam.log
grep "storage_error" ./pb_data/proton-beam.log

# View errors from specific event IDs
grep "id=" ./pb_data/proton-beam.log
```

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success - at least one event converted |
| `1` | All events failed conversion |
| Other | Fatal error (file not found, permissions, etc.) |

### Getting Help

```bash
# Show help
proton-beam --help
proton-beam convert --help

# Show version
proton-beam --version

# Check installation
which proton-beam
```

---

## Tips & Best Practices

1. **Always check proton-beam.log** after conversion
2. **Start with small datasets** when testing
3. **Use appropriate batch sizes** for your use case
4. **Validate when possible** - only skip for trusted data
5. **Monitor disk space** during large conversions
6. **Use the example scripts** - they handle edge cases
7. **Test with sample_events.jsonl** first

---

## External Tools

- **nak**: https://github.com/fiatjaf/nak - Nostr CLI tool
  ```bash
  go install github.com/fiatjaf/nak@latest
  ```

- **jq**: https://stedolan.github.io/jq/ - JSON processor
  ```bash
  brew install jq      # macOS
  apt install jq       # Debian/Ubuntu
  ```

- **GNU parallel**: https://www.gnu.org/software/parallel/
  ```bash
  brew install parallel    # macOS
  apt install parallel     # Debian/Ubuntu
  ```

---

## Related Documentation

- **[Main README](../README.md)** - Project overview and features
- **[Scripts README](scripts/README.md)** - Detailed script documentation
- **[Sample Events](sample_events.jsonl)** - Test data
- **[Developer Guide](../docs/DEVELOPER_GUIDE.md)** - Development information

---

**Need help?** Open an issue on GitHub or check the documentation above.

