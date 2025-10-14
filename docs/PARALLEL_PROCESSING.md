# Parallel Processing Implementation

## Overview

The Proton Beam CLI now supports parallel processing of JSONL files, allowing for significantly faster conversion of large datasets. The implementation uses a chunking strategy that splits the input file into equal-sized segments and processes them concurrently.

## Usage

```bash
# Use default number of threads (number of CPU cores)
proton-beam convert input.jsonl --output-dir ./pb_data

# Specify number of threads
proton-beam convert input.jsonl --output-dir ./pb_data --parallel 8
proton-beam convert input.jsonl --output-dir ./pb_data -j 8

# Force single-threaded processing
proton-beam convert input.jsonl --output-dir ./pb_data --parallel 1
```

## Architecture

### 1. File Chunking
- Input file is divided into N equal-sized byte chunks (one per thread)
- Boundaries are adjusted to align with line breaks to avoid splitting JSON objects
- Each thread processes its assigned chunk independently

### 2. Deduplication Strategy
- **Thread-local HashSet**: Fast in-memory duplicate checking within each thread
- **Global SQLite index**: Shared across all threads (mutex-protected)
- Each thread checks both local HashSet and global index for duplicates
- Index insertions are batched for performance (default: 1000 events per batch)

### 3. Parallel Storage
- Each thread writes to temporary files: `thread_{id}_{date}.pb.gz.tmp`
- No contention between threads during processing
- Temp files are stored in `{output_dir}/tmp/`

### 4. Merge Phase
- After all threads complete, temporary files are merged by date
- Deduplication occurs during merge using per-date HashSets
- Final files are written to: `{output_dir}/{date}.pb.gz`
- Temp directory is cleaned up automatically

### 5. Index Consistency
- Index stores final filenames (e.g., `2025_09_27.pb.gz`), not temp filenames
- `INSERT OR IGNORE` handles race conditions between threads
- Final state is guaranteed consistent: index points to merged files

## Performance Characteristics

### Memory Usage
For a file with M events processed by N threads:
- Each thread: ~96 bytes per event ID in HashSet
- For 8 threads processing 50M events: ~4.8GB total (600MB per thread)
- Merge phase: ~96MB per date with 1M events

### Expected Speedup
On an 8-core machine:
- Sequential: ~50-100K events/second
- Parallel (8 threads): ~300-600K events/second (5-7x speedup)

Actual speedup depends on:
- I/O vs CPU bound operations
- SQLite index contention
- Storage device performance (SSD vs HDD)

## Technical Details

### Duplicate Handling
Three levels of duplicate detection:

1. **Thread-local HashSet** (fastest)
   - Checked first for every event
   - Prevents redundant global index queries

2. **Global SQLite Index** (synchronized)
   - Checked for events not in local HashSet
   - Uses `INSERT OR IGNORE` for thread-safe insertions
   - Flushed in batches to minimize lock contention

3. **Merge-time Deduplication** (final safety net)
   - Per-date HashSet during file merge
   - Catches any boundary-condition duplicates
   - Ensures final files are 100% deduplicated

### Edge Cases Handled

#### Boundary Overlaps
If chunk boundaries cause the same event to be processed by multiple threads:
- First thread to flush to SQLite wins
- Other threads skip as duplicate
- Merge phase catches any remaining duplicates

#### Small Files
For files smaller than thread count:
- Some threads process empty chunks (handled gracefully)
- Still benefits from parallelized I/O and validation

#### Empty Chunks
Threads that process empty regions complete immediately with 0 events

## Changes from Sequential Version

### Removed Features
- **Stdin support**: Removed to enable file seeking and chunking
  - Sequential processing used `InputReader` that supported `-` for stdin
  - Parallel processing requires seekable files

### Modified Behavior
- Default: Uses all available CPU cores (instead of single-threaded)
- Can force sequential mode with `--parallel 1`
- Progress updates are aggregated from all threads

### API Changes
- `StorageManager::new_with_prefix()`: Creates temp files with thread ID prefix
- New functions: `convert_events_parallel()`, `find_chunk_boundaries()`, `process_chunk()`, `merge_temp_files()`

## Testing

Comprehensive test suite includes:
- `test_parallel_processing`: Verifies parallel mode works correctly
- `test_parallel_vs_sequential_consistency`: Ensures parallel and sequential produce identical results
- All existing tests updated to work with both modes
- Deduplication tests use `--parallel 1` for deterministic behavior

## Limitations

1. **File-only**: No stdin support (required for chunking)
2. **Memory**: Large numbers of unique events can consume significant RAM in HashSets
3. **Small files**: Overhead of parallelization may not benefit tiny files
4. **SQLite contention**: Index operations are serialized (mutex-protected)

## Future Improvements

Potential enhancements:
1. **Adaptive threading**: Automatically use sequential mode for small files
2. **Bloom filters**: Reduce memory usage for duplicate checking
3. **Parallel merge**: Merge different dates simultaneously
4. **Lock-free index**: Reduce contention on global index
5. **Async I/O**: Use tokio for improved throughput

## Benchmarks

See [BENCHMARK_SUMMARY.md](BENCHMARK_SUMMARY.md) for detailed performance comparisons.

