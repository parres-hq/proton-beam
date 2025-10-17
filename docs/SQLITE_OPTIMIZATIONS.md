# SQLite Index Optimizations

## Overview

The event index uses SQLite with optimized PRAGMA settings for different use cases. These optimizations can significantly improve performance, especially for large-scale index operations (billions of events).

## Modes

### Normal Mode (Default)

Used for standard operations (queries, incremental updates).

**Settings:**
- `journal_mode = WAL`: Write-Ahead Logging for better concurrency
- `synchronous = NORMAL`: Balanced safety/performance (fsync at critical moments)
- `cache_size = -204800`: 200MB in-memory cache
- `temp_store = MEMORY`: Temp tables in RAM
- `mmap_size = 268435456`: 256MB memory-mapped I/O

**Performance:** Good for interactive workloads, safe for production use.

### Bulk Insert Mode

Used for initial index building (e.g., `index rebuild` command).

**Settings:**
- `journal_mode = WAL`: Write-Ahead Logging
- `synchronous = OFF`: ⚠️ Minimal durability (safe for rebuilds that can restart)
- `cache_size = -2097152`: 2GB in-memory cache
- `page_size = 32768`: Large pages for sequential writes
- `mmap_size = 2147483648`: 2GB memory-mapped I/O
- `auto_vacuum = NONE`: Disabled during bulk insert
- `temp_cache_size = -524288`: 512MB temp cache

**Performance:** 2-4x faster for bulk inserts. Safe for rebuilds but NOT for production writes.

**After bulk insert:**
- `synchronous = NORMAL`: Re-enabled for safety
- `ANALYZE`: Updates query planner statistics

## Performance Impact

### Index Rebuild Benchmarks (4-6 billion events)

| Configuration | Time | Throughput |
|--------------|------|------------|
| Without optimizations | ~6 hours | ~200k events/sec |
| With bulk mode | ~2-3 hours | ~500k events/sec |

**Speedup: 2-3x**

### Memory Usage

- **Normal mode:** ~200MB cache + SQLite overhead
- **Bulk mode:** ~2-3GB cache + larger page cache
- **System requirements:** 256GB RAM is more than sufficient

### Disk Space

For 4-6 billion events:
- **Final index size:** 100-200GB
- **During rebuild:** 2-3x (200-600GB) due to temp indices and WAL
- **Recommendation:** Ensure at least 600GB free space before rebuilding

## Usage

### In Code

```rust
// Normal mode (default)
let index = EventIndex::new(Path::new("index.db"))?;

// Bulk mode for rebuilds
let mut index = EventIndex::new_bulk_mode(Path::new("index.db"))?;
// ... insert millions of events ...
index.finalize_bulk_mode()?;
```

### CLI

The `index rebuild` command automatically uses bulk mode:

```bash
# Optimized for large-scale rebuilds
proton-beam index rebuild ./pb_data
```

## Safety Considerations

### Bulk Mode Risks

With `synchronous = OFF`:
- Power failure during rebuild = corrupted database
- OS crash during rebuild = corrupted database
- Application crash = incomplete but valid database

**Mitigation:**
- Only use for rebuilds that can be restarted
- The CLI automatically finalizes bulk mode at the end
- If rebuild is interrupted, simply delete index.db and restart

### Production Use

For production systems with incremental updates, always use normal mode (`EventIndex::new()`).

## Tuning for Different Hardware

### High-Memory Systems (512GB+)

Increase cache sizes in `configure_bulk_mode()`:
```rust
PRAGMA cache_size = -4194304;  -- 4GB
PRAGMA temp_cache_size = -1048576;  -- 1GB
PRAGMA mmap_size = 4294967296;  -- 4GB
```

### NVMe/Fast SSDs

Can reduce cache sizes since disk I/O is very fast:
```rust
PRAGMA cache_size = -1048576;  -- 1GB
```

### Network File Systems (NFS)

Increase cache to reduce network round-trips:
```rust
PRAGMA cache_size = -4194304;  -- 4GB
PRAGMA mmap_size = 8589934592;  -- 8GB (if latency is high)
```

## Monitoring

During bulk insert, monitor:
- **Disk I/O:** Should be write-heavy (sequential)
- **Memory:** cache_size + OS page cache
- **WAL size:** Check `index.db-wal` file size (should flush periodically)

## References

- [SQLite PRAGMA Documentation](https://www.sqlite.org/pragma.html)
- [SQLite Performance Tuning](https://www.sqlite.org/np1queryprob.html)
- [SQLite Write-Ahead Logging](https://www.sqlite.org/wal.html)

