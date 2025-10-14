# Input Preprocessing

Proton Beam provides efficient preprocessing capabilities to filter out invalid events before they are parsed. This can significantly improve conversion performance when dealing with datasets that contain many invalid events.

## Kind Value Filtering

By default, Proton Beam automatically filters out any JSON lines with `kind` values exceeding the valid u16 range (0-65535) before JSON parsing occurs. This ultra-fast preprocessing improves performance when dealing with invalid data.

### Why Use This Feature?

When converting large JSONL files, you may encounter events with invalid kind values that would normally fail during conversion. Without preprocessing:
- Each line is read from the file
- JSON is parsed into a Rust structure
- The event is validated (which fails for invalid kinds)
- An error is logged

With `--filter-invalid-kinds` enabled:
- Each line is read from the file
- A fast regex check examines the `kind` field value
- Invalid lines are skipped immediately (no JSON parsing)
- Only valid lines proceed to full parsing and validation

### Performance Benefits

The preprocessing uses a compiled regex pattern that matches `"kind": <number>` in the raw JSON string. This is much faster than:
1. Parsing the entire JSON object
2. Deserializing into Rust structures
3. Validating the kind field

For datasets with many invalid kinds, this can save significant CPU time and reduce error logging overhead.

### Usage

```bash
# Filtering is enabled by default
proton-beam convert events.jsonl

# Disable filtering if you want to see all invalid kind errors
proton-beam convert events.jsonl --no-filter-kinds

# Explicitly enable filtering (redundant, but allowed)
proton-beam convert events.jsonl --filter-invalid-kinds

# Combine with other options for maximum performance
proton-beam convert events.jsonl \
  --no-validate \
  --parallel 8 \
  --output-dir ./output

# Works with both single and multi-threaded conversion
proton-beam convert events.jsonl -j 1
```

### How It Works

The preprocessor uses a regex pattern to extract the `kind` value from each JSON line:

```regex
"kind"\s*:\s*(\d+)
```

This matches patterns like:
- `"kind": 1`
- `"kind":65535`
- `"kind": 999999` (would be filtered)

If a kind value is found and exceeds 65535, the line is skipped. If no kind field is found, the line passes through (it will be caught later in validation if it's truly invalid).

### Statistics

When filtering is enabled, the conversion summary will show how many events were pre-filtered:

```
INFO Pre-filtered 1234 events with invalid kind values

📊 Conversion Summary:
  Total lines processed: 10000
  ✅ Valid events:       8500
  ❌ Invalid events:     266
  Success rate:         85.0%
```

Note: "Total lines processed" reflects only the lines that passed the pre-filter. Pre-filtered lines are not included in the totals.

## Implementation Details

### Single-Threaded Mode

In single-threaded mode, the `InputReader` iterator checks each line before yielding it:

```rust
let reader = InputReader::with_options(input_path, filter_invalid_kinds)?;
for line in reader {
    // line has already been filtered if needed
}

// Check how many were filtered
let filtered_count = reader.filtered_count();
```

### Multi-Threaded Mode

In parallel mode, each thread performs the same preprocessing on its chunk of the file:

```rust
if filter_invalid_kinds && !InputReader::has_valid_kind(&line) {
    filtered_count += 1;
    continue;
}
```

The regex is compiled once and reused via `OnceLock`, making it extremely efficient even with millions of lines.

## Best Practices

1. **Enabled by default**: Preprocessing is on by default, so you get the benefits automatically without any configuration.

2. **Disable when debugging**: If you need to see all invalid kind errors for debugging, use `--no-filter-kinds`:
   ```bash
   proton-beam convert events.jsonl --no-filter-kinds
   ```

3. **Combine with `--no-validate`**: If you trust your data source, combine with `--no-validate` for maximum speed:
   ```bash
   proton-beam convert events.jsonl --no-validate
   ```

4. **Monitor the filtered count**: Check the logs to understand how many events are being filtered. If it's a large percentage, you may want to investigate your data source.

5. **Not a replacement for validation**: This preprocessing only checks the `kind` value. Other validation (ID, signature, pubkey format, etc.) still occurs during normal processing.

## Future Enhancements

Potential additions to preprocessing:
- Filter by timestamp range
- Filter by specific kind values or ranges
- Filter by pubkey patterns
- Pre-parse and cache JSON for parallel processing

