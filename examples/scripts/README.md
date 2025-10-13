# Example Scripts

This directory contains practical shell scripts demonstrating various use cases for the Proton Beam CLI.

## Scripts Overview

### 1. `basic_conversion.sh`

The simplest example - converts a JSONL file to protobuf.

```bash
./basic_conversion.sh
```

**What it does:**
- Converts `examples/sample_events.jsonl`
- Stores output in `./pb_data`
- Shows summary and checks for errors

**Good for:** Learning the basics, quick tests

---

### 2. `stream_from_relay.sh`

Fetches events from a Nostr relay and converts them in real-time.

```bash
./stream_from_relay.sh
```

**Prerequisites:** Install [nak](https://github.com/fiatjaf/nak)

```bash
go install github.com/fiatjaf/nak@latest
```

**What it does:**
- Fetches 100 recent text notes from relay.damus.io
- Pipes them directly to proton-beam
- Stores in `./relay_data`

**Good for:** Real-time relay monitoring, testing with live data

---

### 3. `fast_conversion.sh`

Maximum performance conversion with validation disabled.

```bash
./fast_conversion.sh [input_file]
```

**Examples:**
```bash
./fast_conversion.sh                        # Uses sample_events.jsonl
./fast_conversion.sh large_dataset.jsonl    # Custom file
```

**What it does:**
- Disables validation (faster but less safe)
- Uses large batch size (2000 events)
- Disables progress bar
- Measures processing time and rate

**Good for:** Batch processing large trusted datasets

⚠️ **Warning:** Only use for pre-validated, trusted data!

---

### 4. `daily_backup.sh`

Creates daily backups of relay data.

```bash
./daily_backup.sh
```

**What it does:**
- Fetches events from multiple relays (last 24 hours)
- Converts to protobuf
- Compresses into dated archive
- Cleans up old backups (>30 days)
- Logs to syslog

**Good for:** Automated archival, cron jobs

**Cron example:**
```cron
# Run daily at 2 AM
0 2 * * * /path/to/daily_backup.sh
```

---

### 5. `analyze_errors.sh`

Analyzes conversion errors and generates reports.

```bash
./analyze_errors.sh [errors_file]
```

**Examples:**
```bash
./analyze_errors.sh                         # Uses ./pb_data/errors.jsonl
./analyze_errors.sh custom/path/errors.jsonl
```

**What it does:**
- Counts total errors
- Breaks down by error type
- Shows most common messages
- Provides suggestions for fixes

**Good for:** Debugging, understanding data quality issues

**Requires:** `jq` for detailed analysis

---

### 6. `compare_sizes.sh`

Compares JSON vs Protobuf storage sizes.

```bash
./compare_sizes.sh [input_file]
```

**Examples:**
```bash
./compare_sizes.sh                      # Uses sample_events.jsonl
./compare_sizes.sh my_events.jsonl
```

**What it does:**
- Converts events to protobuf
- Calculates size difference
- Shows compression ratio
- Displays per-event averages
- Creates visual comparison

**Good for:** Demonstrating efficiency, capacity planning

---

## Quick Start

The scripts automatically find the `proton-beam` binary in your project's `target/` directory, so you don't need to install it globally. Just build and run:

```bash
# From project root
cargo build --release -p proton-beam-cli

# Run any example script
./examples/scripts/basic_conversion.sh
```

### Making Scripts Executable

Before running, make scripts executable:

```bash
chmod +x examples/scripts/*.sh
```

## Dependencies

Some scripts require additional tools:

| Script | Required Tools |
|--------|---------------|
| `basic_conversion.sh` | proton-beam |
| `stream_from_relay.sh` | proton-beam, nak |
| `fast_conversion.sh` | proton-beam |
| `daily_backup.sh` | proton-beam, nak, tar |
| `analyze_errors.sh` | proton-beam, jq (optional) |
| `compare_sizes.sh` | proton-beam, bc |

### Installing Dependencies

**nak** (Nostr Army Knife):
```bash
go install github.com/fiatjaf/nak@latest
```

**jq** (JSON processor):
```bash
# macOS
brew install jq

# Debian/Ubuntu
apt install jq

# Fedora
dnf install jq
```

**bc** (Calculator - usually pre-installed):
```bash
# macOS
brew install bc

# Debian/Ubuntu
apt install bc
```

---

## Customizing Scripts

All scripts have configuration variables at the top. Edit them to match your needs:

```bash
# Example: Modify relay in stream_from_relay.sh
RELAY="wss://your-relay.com"
OUTPUT_DIR="./my_custom_dir"
EVENT_LIMIT=500
```

---

## Running in Production

### As Systemd Services

Example for daily backups:

```bash
# Copy script to system location
sudo cp daily_backup.sh /usr/local/bin/

# Create systemd service
sudo nano /etc/systemd/system/nostr-backup.service
```

```ini
[Unit]
Description=Nostr Daily Backup
After=network.target

[Service]
Type=oneshot
User=nostr
ExecStart=/usr/local/bin/daily_backup.sh
```

```bash
# Create timer
sudo nano /etc/systemd/system/nostr-backup.timer
```

```ini
[Unit]
Description=Daily Nostr Backup Timer

[Timer]
OnCalendar=daily
Persistent=true

[Install]
WantedBy=timers.target
```

```bash
# Enable and start
sudo systemctl enable --now nostr-backup.timer
```

### Docker Integration

Create a Dockerfile:

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p proton-beam-cli

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y ca-certificates jq bc curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/proton-beam /usr/local/bin/
COPY examples/scripts/*.sh /scripts/

RUN chmod +x /scripts/*.sh

WORKDIR /data
ENTRYPOINT ["/bin/bash"]
```

Run scripts in Docker:

```bash
docker build -t proton-beam .
docker run -v $(pwd)/data:/data proton-beam /scripts/basic_conversion.sh
```

---

## Troubleshooting

### "Permission denied"

Make scripts executable:
```bash
chmod +x script_name.sh
```

### "Command not found: proton-beam"

**This should not happen!** The scripts automatically detect the proton-beam binary from:
1. System PATH (if installed with `cargo install`)
2. `target/release/proton-beam` (release build)
3. `target/debug/proton-beam` (debug build)

If you still see this error, build the project:
```bash
# From the project root
cargo build --release -p proton-beam-cli
```

Or install globally:
```bash
cargo install --path proton-beam-cli
```

### "nak: command not found"

Install nak:
```bash
go install github.com/fiatjaf/nak@latest
```

Ensure `$GOPATH/bin` is in your PATH:
```bash
export PATH="$PATH:$(go env GOPATH)/bin"
```

### Scripts hang or don't complete

- Check if input file exists
- Ensure you have write permissions to output directory
- Check available disk space
- Try with smaller input files first

---

## Contributing

Have a useful script? Submit a PR!

Guidelines:
1. Include clear comments
2. Add configuration section at top
3. Handle errors gracefully
4. Check for required dependencies
5. Update this README

---

## Additional Examples

For more examples and detailed explanations, see:
- [CLI_EXAMPLES.md](../CLI_EXAMPLES.md) - Comprehensive CLI usage guide
- [sample_events.jsonl](../sample_events.jsonl) - Sample data for testing

---

**Need help?** Open an issue on GitHub!

