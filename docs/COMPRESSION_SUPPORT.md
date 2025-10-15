# Compression Format Support

Proton Beam's AWS deployment script automatically handles compressed input files.

## Supported Formats

The deployment script automatically detects and decompresses:

| Format | Extension | Tool | Speed | Compression Ratio |
|--------|-----------|------|-------|-------------------|
| **Zstandard** | `.zst` | `zstd` | ⚡⚡⚡ Fast | Good (2-3x) |
| **Gzip** | `.gz` | `gunzip` / `pigz` | ⚡⚡ Medium | Good (2-3x) |
| **XZ** | `.xz` | `xz` | ⚡ Slow | Best (3-4x) |
| **Uncompressed** | `.jsonl` | - | ⚡⚡⚡ Instant | - |

## How It Works

The deployment script:

1. **Downloads** the compressed file
2. **Detects** compression format by extension
3. **Decompresses** to `/tmp/` directory
4. **Removes** compressed file to save space
5. **Processes** the decompressed JSONL

### Example Flow (Zstandard)

```
Download:    nostr-events.jsonl.zst (400GB)
             ↓
Decompress:  nostr-events.jsonl (1.2TB)
             ↓
Process:     Convert to protobuf
             ↓
Upload:      *.pb.gz files to S3 (450GB)
             ↓
Cleanup:     Delete local files
```

## Disk Space Requirements

Calculate disk space needed:

```
Required Space = Compressed Size + Decompressed Size + Output Size + 10% buffer

Example (1.2TB dataset):
  Compressed:     400GB  (.zst file)
  Decompressed: 1,200GB  (.jsonl file)
  Output:         450GB  (.pb.gz files)
  Buffer:         205GB  (10% safety margin)
  ─────────────────────
  Total:        2,255GB  (recommend 3TB minimum)
```

For your 1.2TB dataset, we use **5TB EBS** which provides plenty of headroom.

## Decompression Performance

### Zstandard (Recommended)

**Pros**:
- ✅ Very fast decompression (~500-800 MB/s)
- ✅ Good compression ratio
- ✅ Multi-threaded by default
- ✅ Modern, designed for this use case

**Decompression time** for 400GB file:
```
400GB ÷ 600 MB/s = ~11 minutes
```

### Gzip

**Pros**:
- ✅ Widely supported
- ✅ Fast with `pigz` (parallel gzip)

**Cons**:
- ❌ Slower than zstd (~100-200 MB/s single-threaded)
- ❌ Single-threaded by default (unless using `pigz`)

**Decompression time** for 400GB file:
```
With pigz: 400GB ÷ 400 MB/s = ~17 minutes
Without:   400GB ÷ 150 MB/s = ~45 minutes
```

### XZ

**Pros**:
- ✅ Best compression ratio

**Cons**:
- ❌ Very slow decompression (~50-80 MB/s)
- ❌ Single-threaded

**Decompression time** for 400GB file:
```
400GB ÷ 60 MB/s = ~110 minutes (1.8 hours)
```

## Tools Installed

The deployment script installs:

```bash
sudo apt-get install -y \
    zstd    # Zstandard compression
    pigz    # Parallel gzip
    pv      # Progress viewer
```

## Progress Monitoring

During decompression, you'll see:

```bash
# With pv (pipe viewer)
Decompressing Zstandard file...
 389GiB 0:10:23 [632MiB/s] [=========>          ] 45% ETA 0:11:42

# Without pv
Decompressing Zstandard file...
This may take several minutes for large files...
```

## Manual Decompression

If you need to decompress manually:

### Zstandard
```bash
# Basic
zstd -d input.jsonl.zst -o output.jsonl

# With progress
pv input.jsonl.zst | zstd -d -o output.jsonl

# Multi-threaded (if not default)
zstd -d -T0 input.jsonl.zst -o output.jsonl
```

### Gzip
```bash
# Basic
gunzip input.jsonl.gz

# Parallel (faster)
pigz -d input.jsonl.gz

# Keep original
gunzip -c input.jsonl.gz > output.jsonl
```

### XZ
```bash
# Basic
xz -d input.jsonl.xz

# With progress
pv input.jsonl.xz | xz -d > output.jsonl

# Multi-threaded
xz -d -T0 input.jsonl.xz
```

## Compression Commands (Creating Archives)

If you need to compress JSONL files:

### Zstandard (Recommended)
```bash
# Fast compression (level 3)
zstd -3 input.jsonl -o output.jsonl.zst

# Balanced (level 6)
zstd -6 input.jsonl -o output.jsonl.zst

# Best compression (level 19)
zstd -19 input.jsonl -o output.jsonl.zst

# Multi-threaded
zstd -T0 -6 input.jsonl -o output.jsonl.zst
```

### Gzip
```bash
# Parallel gzip (fastest)
pigz -6 input.jsonl

# Standard gzip
gzip -6 input.jsonl
```

### XZ (Best compression)
```bash
# Best compression
xz -9e input.jsonl

# Multi-threaded
xz -T0 -6e input.jsonl
```

## Your Dataset

For your Primal dataset:

```
URL:  https://dev.primal.net/_share/nostr-events-2025-09-27.jsonl.zst
Size: ~400GB compressed
      ~1.2TB decompressed
Format: Zstandard (.zst)

Decompression time: ~11 minutes @ 600 MB/s
Total processing:   ~10 minutes (conversion)
Total time:         ~21 minutes
```

The deployment script handles all of this automatically! 🚀

## Error Handling

If decompression fails:

```bash
# Check file integrity
zstd -t input.jsonl.zst

# Try with different memory settings
zstd -d --memory=2048MB input.jsonl.zst

# Manual decompression
zstd -d -v input.jsonl.zst -o output.jsonl
```

## Best Practices

1. **Use Zstandard** for best speed/compression balance
2. **Allocate enough disk space** (3-4x compressed size)
3. **Use SSD storage** (EBS gp3) for faster I/O
4. **Monitor disk space** during decompression
5. **Delete compressed file** after decompression (script does this)

## Comparison Table

| Metric | Zstandard | Gzip | XZ | Uncompressed |
|--------|-----------|------|----|--------------|
| Compression ratio | 2.5-3x | 2.5-3x | 3-4x | 1x |
| Compression speed | ⚡⚡⚡ | ⚡⚡ | ⚡ | N/A |
| Decompression speed | ⚡⚡⚡ | ⚡⚡ | ⚡ | N/A |
| CPU usage | Medium | Low-Med | High | None |
| Memory usage | Low | Low | High | None |
| Multi-threaded | ✅ Yes | ⚠️ pigz only | ⚠️ Limited | N/A |
| **Recommendation** | ✅ **Best choice** | Good | Space-constrained | Fast only |

## Summary

The AWS deployment script automatically:
- ✅ Detects compression format
- ✅ Installs required tools
- ✅ Decompresses efficiently
- ✅ Shows progress
- ✅ Cleans up temporary files

**You don't need to do anything!** Just provide the `.zst` URL and the script handles the rest. 🎉

