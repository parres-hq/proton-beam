# Error Handling Review - Proton Beam

**Date:** 2025-10-17
**Context:** Review for 1.2TB file processing (several hours on 128-core machine)

## Executive Summary

✅ **Overall Assessment: GOOD** - Error handling is well-designed for graceful degradation. Individual event failures don't crash the process, and all errors are logged with context.

⚠️ **Critical Issue Found**: Thread failures in parallel processing result in partial data loss for that chunk. However, improvements have been made to make this more transparent and recoverable.

---

## ✅ What's Working Well

### 1. **Individual Event Error Handling** (Excellent)
- **Location**: Lines 510-606 (single-threaded), 925-997 (parallel)
- **Behavior**: Invalid events are logged and skipped, processing continues
- **Error Types Handled**:
  - JSON parse errors
  - Validation errors (basic fields, signatures, event IDs)
  - Storage errors
- **Logging**: All errors include line number, error type, and event ID (when available)
- ✅ **Result**: Bad events don't crash the entire conversion

### 2. **Storage Manager Reliability** (Good)
- **Drop Handler**: Ensures buffered events are flushed even on panic/error
- **Explicit Flush**: Called before Drop as primary mechanism (line 610, 1001)
- **Batching**: Events buffered and written in batches for efficiency
- ✅ **Improvement Made**: Added stderr warnings for Drop flush failures

### 3. **Preprocessing Filter** (Excellent for performance)
- **Kind Validation**: Pre-filters invalid kind values (>65535) before parsing
- **Regex-based**: Fast check avoids expensive JSON parsing for bad data
- **Stats Tracking**: Reports how many events were pre-filtered
- ✅ **Result**: Significant speedup for datasets with invalid kinds

### 4. **Merge Tool Robustness** (Now Improved)
- ✅ **Improvement Made**: Now handles corrupted events in temp files gracefully
- **Previous Behavior**: One corrupted event failed entire merge
- **New Behavior**: Corrupted events are skipped, merge continues, user is warned
- **Use Case**: Perfect for recovering from failed conversions

---

## ⚠️ Critical Issues Found & Fixed

### 1. **Parallel Thread Failures → Data Loss** ⚠️ HIGH IMPACT

**Problem:**
```
Thread 64 processing bytes 500GB-508GB encounters I/O error at 504GB
→ Events from 500GB-504GB are saved (via Drop handler)
→ Events from 504GB-508GB are LOST (never processed)
→ Entire conversion fails even though 127/128 threads succeeded
```

**Impact on 1.2TB File:**
- One failed thread = ~9.4GB of lost data (1/128th of file)
- After hours of processing, this is catastrophic

**What Happens:**
1. Thread encounters error (I/O error, disk full, corrupted data)
2. `StorageManager::Drop` flushes buffered events (saves partial progress)
3. Error is captured in `parallel_errors` vector
4. Other threads continue and complete successfully
5. After all threads finish: if ANY error exists → entire job fails
6. Temp directory is preserved (cleanup never runs)
7. **Data from failed chunk after error point is UNRECOVERABLE**

**Improvements Made:**
```rust
// NOW: Better error reporting with recovery guidance
✅ Reports which specific threads/chunks failed
✅ Shows byte ranges for failed chunks
✅ Provides clear recovery options:
   1. Use merge tool to salvage partial data (with data loss warning)
   2. Fix issue and re-run (recommended for integrity)
```

**Recovery Process:**
```bash
# After a failed parallel conversion:
./proton-beam merge ./pb_data

# This will:
# - Merge all successfully completed chunks
# - Include partial data from failed chunks (up to error point)
# - Report if corrupted events are encountered
# - Warn about missing data
```

### 2. **Merge Process - Single Corrupted Event Failed Everything** ✅ FIXED

**Previous Behavior:**
- One corrupted event in ANY temp file → entire merge fails
- After hours of processing, one bad protobuf message kills recovery

**Fix Applied:**
```rust
// Lines 1142-1155: Now handles corrupted events gracefully
let event = match event_result {
    Ok(e) => e,
    Err(e) => {
        corrupted_events += 1;
        error!("Corrupted event in {} (skipping): {}", source, e);
        continue;  // Skip and continue merge
    }
};
```

**New Behavior:**
- Corrupted events are logged and skipped
- Merge continues with remaining valid events
- Final summary reports: `X events (Y duplicates, Z corrupted events skipped)`
- User is warned but merge succeeds

### 3. **Storage Drop Handler - Silent Failures** ✅ IMPROVED

**Previous Behavior:**
- Flush errors in Drop only logged via tracing
- Disk full during Drop could lose data silently

**Fix Applied:**
```rust
// Lines 186-204: Now uses both tracing AND stderr
if let Err(e) = self.flush() {
    tracing::error!("❌ CRITICAL: StorageManager drop flush error: {}", e);
    eprintln!("❌ CRITICAL: StorageManager drop flush error: {}", e);
    eprintln!("   Some events may not have been written to disk!");
    eprintln!("   Check disk space and file permissions.");
}
```

**New Behavior:**
- Errors visible on console immediately
- Clear actionable guidance (check disk space, permissions)
- Still logs to file for post-mortem analysis

---

## 📊 Error Logging Analysis

### Log File: `pb_data/proton-beam.log`

**What's Logged:**
- Thread start/stop with chunk boundaries
- Individual event errors (parse, validation, storage)
- Merge process details
- File operations (open, write, rename)
- Statistics (events processed, duplicates, errors)

**Format:**
```
2025-10-17T10:30:45 INFO Starting Proton Beam - Conversion
2025-10-17T10:30:45 INFO Input: /data/events.jsonl
2025-10-17T10:30:45 INFO Parallel threads: 128
2025-10-17T10:30:45 INFO Calculating chunk boundaries...
...
2025-10-17T12:45:30 ERROR line=123456 id=abcd1234 parse_error: invalid JSON
2025-10-17T12:45:31 ERROR line=123789 id=def56789 validation_error: signature verification failed
...
2025-10-17T14:20:15 INFO Thread 0 completed: 45230 lines, 45102 valid, 128 errors
2025-10-17T14:20:16 ERROR Thread 64 (bytes 500000000-508000000) error: I/O error: read error
```

**Error Compaction:**
- Event IDs truncated to 8 chars for readability
- Error messages truncated to 100 chars
- Reduces log bloat on large conversions

---

## 🔧 Additional Improvements Made

### 1. **Disk Space Estimation**
- Logs input file size and estimated output size
- Helps users verify sufficient space before multi-hour runs
- **Note**: Actual space check requires platform-specific deps (not added to avoid complexity)

### 2. **Better Context Messages**
- File open errors now mention: "check disk space and permissions"
- Merge errors include source file name and event index
- Thread errors include byte ranges

### 3. **Merge Process Visibility**
- Added progress messages: `📦 Merging X temp files for date: Y`
- Success confirmation: `✅ Successfully merged date: Y`
- Clear warnings for corrupted events

---

## 🚨 Known Limitations

### 1. **No Automatic Retry for Failed Chunks**
When a thread fails:
- ❌ The chunk is not automatically retried
- ❌ Data after the error point is lost
- ✅ User must manually re-run or use merge tool

**Potential Future Enhancement:**
- Implement chunk-level retry with exponential backoff
- Identify specific line causing failure and skip just that line
- Continue processing rest of chunk

### 2. **No Disk Space Pre-Check**
- Currently only logs estimated output size
- Doesn't fail-fast if insufficient space
- Could run for hours then fail on disk full

**Why Not Implemented:**
- Platform-specific (requires different syscalls for macOS/Linux/Windows)
- Would need additional dependency (e.g., `fs2` crate)
- User should check manually for now

### 3. **No Progress Checkpointing**
- If conversion fails at 90%, must restart from 0% (or use merge tool with partial data)
- No way to resume from last successful batch

**Why This Matters for 1.2TB:**
- A failure near the end wastes hours of processing
- Merge tool helps but may have data gaps

---

## 📝 Recommendations for Your 1.2TB Conversion

### Before Starting:

1. **Check Disk Space:**
   ```bash
   df -h /path/to/output/directory
   # Ensure you have at least 600GB free (conservative estimate)
   ```

2. **Monitor the Log File:**
   ```bash
   tail -f pb_data/proton-beam.log
   # Watch for thread failures in real-time
   ```

3. **Use Screen/Tmux:**
   ```bash
   screen -S proton-beam
   ./proton-beam convert huge_file.jsonl --output-dir pb_data -j 128
   # Detach with Ctrl-A D
   ```

### During Conversion:

4. **Watch for Thread Errors:**
   - If you see "Thread X error" in logs, the conversion will fail
   - But temp files are preserved for recovery

5. **Monitor System Resources:**
   - Check if disk I/O is saturating
   - Monitor memory usage (should be low due to streaming)

### If Conversion Fails:

6. **Try the Merge Tool First:**
   ```bash
   ./proton-beam merge pb_data --verbose

   # This will:
   # - Merge all successfully completed chunks
   # - Include partial data from failed chunks
   # - Skip corrupted events if encountered
   # - Report what was salvaged
   ```

7. **Assess Data Loss:**
   - Check how many events were processed: see log file
   - Failed chunks = missing data (roughly 1/128th per failed thread)
   - Decide if acceptable or need full re-run

8. **For Complete Data Integrity:**
   ```bash
   # Remove partial results
   rm -rf pb_data/tmp pb_data/*.pb.gz

   # Re-run with any fixes
   ./proton-beam convert huge_file.jsonl --output-dir pb_data -j 128
   ```

---

## 🎯 Summary for Your Use Case

**For a 1.2TB file on 128 cores:**

✅ **Good:**
- Individual bad events won't crash anything
- Comprehensive error logging
- Parallel processing is efficient
- Merge tool can recover partial data

⚠️ **Be Aware:**
- Thread failures = partial data loss for that chunk
- No automatic retry
- Must re-run full conversion for 100% integrity
- Merge tool is for emergency recovery, not primary workflow

**Best Practice:**
1. Run full conversion first (trust the process)
2. If it fails, check logs to understand why
3. Fix underlying issue (disk space, file corruption, network timeout)
4. Re-run full conversion for complete data
5. Use merge tool only if re-running is not feasible (time-critical)

**Your merge tool is now much more robust** - it will handle corrupted events gracefully and give you clear reporting on what was salvaged vs. lost.

