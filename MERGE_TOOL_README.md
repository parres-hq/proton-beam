# Merge Command - Recovery Utility

The `proton-beam merge` command allows you to resume a failed parallel conversion by merging existing temporary files without re-processing the source JSONL data.

## When to Use This Command

Use this command when:
- A parallel conversion completed processing but failed during the merge step
- You still have the `tmp/` directory with `thread_*.pb.gz.tmp` files
- You don't want to spend hours re-processing 1.2TB of JSONL data

## Prerequisites

The output directory must contain a `tmp/` subdirectory with files in this format:
```
tmp/thread_0_2025_09_27.pb.gz.tmp
tmp/thread_1_2025_09_27.pb.gz.tmp
tmp/thread_2_2025_09_27.pb.gz.tmp
...
```

## Installation

Simply build the main CLI:
```bash
cargo build --release
```

The `merge` command is built into the main `proton-beam` binary.

## Usage

### Basic Usage
```bash
proton-beam merge ~/data/output
```

This will:
- Read all `thread_*.pb.gz.tmp` files from `~/data/output/tmp/`
- Group them by date (e.g., `2025_09_27`)
- Merge each date into a final file (e.g., `~/data/output/2025_09_27.pb.gz`)
- Apply deduplication during merge
- Keep the temp files (in case you need to retry)

### With Automatic Cleanup
```bash
proton-beam merge ~/data/output --cleanup
```

This will delete the `tmp/` directory after successful merge.

### With Verbose Logging
```bash
proton-beam merge ~/data/output --verbose
```

This provides detailed debug information about the merge process.

### Custom Compression Level
```bash
proton-beam merge ~/data/output --compression-level 9
```

Use compression level 0-9 (default is 6, which matches the convert command).

## Output

Example output:
```
🔄 Proton Beam - Merge Temporary Files
   Output: /home/ubuntu/data/output
   Temp dir: /home/ubuntu/data/output/tmp
   Compression: 6

📁 Found 1 date(s) to merge

[1/1] Merging 128 files for date: 2025_09_27
   📦 Processing 128 source files...
   ✅ Wrote /home/ubuntu/data/output/2025_09_27.pb.gz with 45230891 events (234 duplicates removed)

✅ Merge complete!

💡 Tip: Run with --cleanup to remove temp files after successful merge
```

## Error Handling

The tool provides detailed error messages if something goes wrong:
- Missing temp directory
- No valid temp files found
- Corrupted protobuf files
- Disk space issues
- Permission problems

All errors include context about which file and operation failed.

## What It Does

1. **Scans temp directory**: Finds all `thread_*.pb.gz.tmp` files
2. **Groups by date**: Extracts date from filename (`thread_{id}_{year}_{month}_{day}.pb.gz.tmp`)
3. **Merges with deduplication**: Streams events from all thread files, removes duplicates
4. **Creates final file**: Writes to `{date}.pb.gz` (e.g., `2025_09_27.pb.gz`)
5. **Atomic rename**: Uses temp file during write, renames at the end for safety

## Performance

- Streaming architecture: Low memory usage even with large files
- Parallel-friendly: Can handle output from 128+ threads
- Efficient deduplication: Uses HashSet for O(1) duplicate detection
- Fast I/O: Buffered reads and writes with configurable compression

## Safety Features

- **Existing file handling**: If a final file already exists, it's included in the merge for deduplication
- **Atomic writes**: Creates `.tmp` file first, then renames (prevents partial files)
- **Non-destructive by default**: Keeps temp files unless `--cleanup` is specified
- **Comprehensive error context**: Every failure includes details about what went wrong

## Troubleshooting

### "No valid temp files found"
Check that:
- The `tmp/` directory exists in your output directory
- Files have `.tmp` extension
- Filenames match pattern: `thread_{id}_{year}_{month}_{day}.pb.gz.tmp`

### "Failed to open source file"
Possible causes:
- Permission issues
- File was deleted or moved
- Disk errors

### "Failed to create temp output file"
Possible causes:
- Insufficient disk space
- Permission issues on output directory
- Output directory doesn't exist

## After Successful Merge

Once the merge completes successfully:
1. Verify the output file exists: `ls -lh ~/data/output/*.pb.gz`
2. If you used `--cleanup`, the temp directory is gone
3. If you didn't use `--cleanup`, you can manually remove it: `rm -rf ~/data/output/tmp`

## Full Command Reference

```
proton-beam merge [OPTIONS] <OUTPUT_DIR>

Arguments:
  <OUTPUT_DIR>  Output directory containing the tmp/ subdirectory

Options:
      --compression-level <COMPRESSION_LEVEL>
          Compression level (0-9, default: 6)
  -v, --verbose
          Show detailed progress information
      --cleanup
          Delete temp directory after successful merge
  -h, --help
          Print help
```

