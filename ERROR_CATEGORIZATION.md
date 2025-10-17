# Error Categorization and Statistics

## Overview

Proton Beam now categorizes conversion errors and displays a summary at the end of processing, similar to how it handles invalid event kinds with filtering. This makes it much easier to understand what types of errors occurred during conversion without flooding the logs with individual error messages.

## Features

### Error Categories

The following error types are tracked:

1. **Invalid Tag Values** - Tag values that are not strings (e.g., numbers, booleans, objects)
2. **Invalid Event Kinds** - Event kinds outside the valid range (0-65535)
3. **Invalid Signatures** - Failed Schnorr signature verification
4. **Invalid Event IDs** - Event ID doesn't match the computed hash
5. **Hash Computation Errors** - Errors while computing event hashes
6. **Storage Errors** - I/O errors while writing to disk
7. **Parse Errors** - JSON parsing and conversion errors
8. **Other Validation Errors** - Other validation failures

### Logging Behavior

- **Critical errors** (Storage, Hash) are logged at ERROR level for immediate visibility
- **Less critical errors** (Invalid tags, signatures, etc.) are logged at DEBUG level to reduce log noise
- All errors are counted and summarized at the end of processing

### Example Output

```
📊 Conversion Summary:
  Total lines processed: 1000000
  ✅ Valid events:       950000
  ❌ Invalid events:     45000
  ⏭️  Skipped lines:      5000
  Success rate:         95.0%

📋 Error Breakdown:
  Invalid Tag Values: 30000
  Invalid Signatures: 10000
  Invalid Event IDs: 3000
  Parse Errors: 2000
```

## Benefits

1. **Reduced Log Spam** - Individual errors for common issues (like invalid tag values) are logged at DEBUG level instead of ERROR
2. **Better Visibility** - Easy to see at a glance what types of errors occurred and how many of each
3. **Actionable Insights** - Helps identify systematic problems in the input data
4. **Consistent with Existing Patterns** - Similar to how invalid kinds are pre-filtered and counted

## Implementation Details

### Single-threaded Conversion

Error statistics are tracked within the `StorageManager` and displayed at the end of conversion.

### Parallel Conversion

Each thread tracks its own error statistics, which are then merged and displayed as a unified summary at the end.

## Usage

No changes to the CLI are required. Error categorization is automatic:

```bash
# Single-threaded
proton-beam convert input.jsonl --output-dir ./pb_data

# Parallel (automatic error stats aggregation)
proton-beam convert input.jsonl --output-dir ./pb_data --parallel 16
```

To see individual error details, use verbose mode:

```bash
proton-beam convert input.jsonl --output-dir ./pb_data --verbose
```

This will log all errors (including DEBUG level) to the log file in the output directory.

## Error Handling Policy

**Important**: Proton Beam does NOT coerce invalid values. Events with malformed data are rejected:

- Tag values MUST be strings (as per Nostr spec)
- Event kinds MUST be in the range 0-65535
- Signatures MUST be valid Schnorr signatures
- Event IDs MUST match the computed hash

This ensures data integrity and compliance with the Nostr protocol specification.

