# Benchmark Best Practices

This document outlines best practices for running and managing performance benchmarks in Proton Beam.

## When to Run Benchmarks

### ✅ DO Run Benchmarks:

1. **Before and After Optimization Work**
   ```bash
   # Save baseline before changes
   just bench-save
   # Make your optimizations
   # ...
   # Compare results
   just bench-save
   just bench-compare benchmark-results/bench-OLD.txt benchmark-results/bench-NEW.txt
   ```

2. **When Reviewing Performance-Critical PRs**
   - Run benchmarks locally before creating PR
   - Include results in PR description
   - CI will run automatically on PR

3. **Periodically for Baseline Tracking**
   - Weekly scheduled CI runs track performance over time
   - Helps catch gradual regressions

4. **After Major Dependency Updates**
   - Verify no unexpected regressions
   - Document any improvements

5. **When Investigating Performance Issues**
   - Use benchmarks to identify bottlenecks
   - Profile with specific benchmark targets

### ❌ DON'T Run Benchmarks:

1. **On Pre-commit Hooks**
   - Too slow (5-10 minutes)
   - Blocks development workflow
   - Not necessary for every commit
   - Note: `just precommit` and `just test` exclude benchmarks by design

2. **On Every Push to Feature Branches**
   - Wastes CI resources
   - Creates noise in results
   - Wait until PR stage

3. **In Debug Mode for Accurate Results**
   - Debug builds are 10-100x slower
   - Use `just bench` (always release mode) or `--release` flag
   - Debug mode is only for quick smoke tests

## CI Integration

### Automatic Triggers

Benchmarks run automatically in CI when:

1. **Pull Requests** - On files that affect performance:
   - `proton-beam-core/**`
   - `proton-beam-cli/**`
   - `Cargo.toml` / `Cargo.lock`

2. **Main Branch** - After merges to track baseline

3. **Weekly Schedule** - Every Sunday at 2 AM UTC for long-term tracking

4. **Manual Dispatch** - When you need to run benchmarks on demand

### Viewing CI Results

1. Go to **Actions** tab in GitHub
2. Find the **Performance Benchmarks** workflow
3. Download the **benchmark-results** artifact
4. Review the detailed text output

### Baseline Comparison

On PRs, the CI workflow will:
- Download the baseline from the main branch
- Compare current results
- Post a comment with differences (if significant)

## Storing Benchmark History

### Automated Storage (via CI)

CI automatically stores benchmark results:
- **Artifacts**: Kept for 90 days per run
- **Baseline**: Latest main branch results always available
- **Historical**: Download artifacts from past runs for trending

### Manual Storage (Local Development)

For local tracking:

```bash
# Option 1: Use just commands
just bench-save
# Results saved to: benchmark-results/bench-YYYYMMDD-HHMMSS.txt

# Option 2: Store in git (recommended for important milestones)
git checkout -b benchmark-history
just bench-save
git add benchmark-results/
git commit -m "Benchmark results for feature X"
git push origin benchmark-history
```

### Long-term History Tracking

For project milestones, manually save key results:

```bash
# Create benchmark history directory
mkdir -p benchmark-results/milestones

# Run benchmarks and tag with version/milestone
just bench 2>&1 | tee benchmark-results/milestones/v1.0.0-baseline.txt
just bench 2>&1 | tee benchmark-results/milestones/phase3-index-complete.txt

# Commit these to version control
git add benchmark-results/milestones/
git commit -m "Milestone benchmark: Phase 3 completion"
```

## Interpreting Results

### Normal Variance

Benchmarks naturally vary by **5-10%** due to:
- CPU temperature and throttling
- Background processes
- System load
- Memory availability
- Cache state

**Action**: Don't worry about small fluctuations

### Significant Changes

| Change | Action |
|--------|--------|
| **>15% slower** | 🚨 Investigate immediately - likely regression |
| **10-15% slower** | ⚠️ Review code changes, may need optimization |
| **5-10% slower** | ℹ️ Monitor, might be variance or acceptable tradeoff |
| **5-10% faster** | ✅ Good! Document what improved |
| **>10% faster** | 🎉 Excellent! Explain in PR/commit |

### Looking at Impact

When reviewing benchmark results, focus on:

1. **Real-world scenarios** - Pipeline benchmarks matter most to users
2. **Bottleneck operations** - Fix the slowest parts first
3. **Common use cases** - JSON→Proto, validation, storage
4. **Tradeoffs** - Sometimes small slowdowns are acceptable for features

## Performance Regression Prevention

### During Development

1. **Test performance-critical changes locally**
   ```bash
   just bench-conversion  # Test specific area
   ```

2. **Profile before optimizing**
   ```bash
   cargo flamegraph --bench conversion_bench
   ```

3. **Document performance implications**
   - Add comments for hot path code
   - Note in PR description if performance affected

### Code Review

When reviewing PRs:

1. **Check benchmark results** in CI artifacts
2. **Question design** if >10% regression
3. **Verify justification** for any performance loss
4. **Celebrate wins** when performance improves

### Post-merge

1. **Monitor baseline** from weekly runs
2. **Track trends** over multiple weeks
3. **Investigate gradual degradation** if detected

## Performance Targets

### Current Targets (see BENCHMARKING.md for details)

- **JSON → Proto**: >50,000 events/sec
- **Proto → JSON**: >100,000 events/sec
- **Basic validation**: >500,000 validations/sec
- **Storage write**: >30 MB/sec
- **Index batch insert**: >50,000 events/sec
- **Pipeline (validated)**: >10,000 events/sec

### When Targets Are Not Met

If benchmarks fall below targets:

1. **Don't panic** - Targets are guidelines, not hard requirements
2. **Check if real-world impact exists** - Users might not notice
3. **Investigate root cause** - Profile the slow operations
4. **Consider optimization** - But don't over-optimize
5. **Update targets if needed** - Targets should be realistic

## Optimization Workflow

### Standard Process

1. **Establish baseline**
   ```bash
   just bench-save
   ```

2. **Make targeted changes**
   - Profile to find hot spots
   - Optimize one thing at a time
   - Keep changes small and testable

3. **Measure impact**
   ```bash
   just bench-save
   just bench-compare baseline.txt current.txt
   ```

4. **Iterate or revert**
   - If improvement <5%, may not be worth complexity
   - If regression, revert and try different approach
   - If good improvement (>10%), keep and document

5. **Document in PR**
   ```markdown
   ## Performance Impact

   ### Benchmark Results
   Before: 50,000 ops/sec
   After:  75,000 ops/sec
   Improvement: +50%

   ### Changes Made
   - Switched from Vec to SmallVec for tags
   - Avoided allocation in hot path

   ### Tradeoffs
   - Added 1 dependency (smallvec)
   - Slightly more complex code
   ```

## Tools and Commands

### Quick Reference

```bash
# Run all benchmarks (release mode)
just bench

# Run specific benchmark
just bench-conversion
just bench-validation
just bench-storage
just bench-builder
just bench-index
just bench-pipeline

# Run and save results
just bench-save

# Compare two result files
just bench-compare OLD.txt NEW.txt

# Quick smoke test (debug mode, fast)
just bench-quick

# Run in CI mode (with extra analysis)
.github/workflows/benchmarks.yml
```

### Advanced Usage

```bash
# Profile with flamegraph
cargo install flamegraph
cargo flamegraph --bench conversion_bench

# Run with custom iterations
cargo bench --bench conversion_bench -- --sample-size 1000

# Run specific test within benchmark
cargo bench --bench conversion_bench -- json_to_proto

# Time a specific operation
cargo bench --bench storage_bench -- write_single
```

## Continuous Monitoring

### GitHub Actions Dashboard

Monitor benchmark trends:
1. Go to **Actions** → **Performance Benchmarks**
2. Review weekly scheduled runs
3. Download artifacts to compare over time

### Creating Performance Reports

For quarterly reviews or releases:

```bash
# Collect recent benchmark history
mkdir -p reports
ls -t benchmark-results/*.txt | head -5 | xargs -I {} cp {} reports/

# Create comparison report
cd reports
echo "# Performance Trends - Q1 2025" > report.md
for file in *.txt; do
  echo "## $(basename $file .txt)" >> report.md
  echo '```' >> report.md
  cat "$file" >> report.md
  echo '```' >> report.md
  echo "" >> report.md
done
```

## Contributing Performance Improvements

When submitting performance PRs:

### Required Information

1. **Benchmark results** (before/after)
2. **Explanation** of what improved
3. **Tradeoffs** (if any)
4. **Impact scope** (which operations benefit)

### Example PR Description

```markdown
## Performance: Optimize tag conversion

### Summary
Reduced allocations in tag value conversion by 60%, improving JSON→Proto
conversion by 25% for events with many tags.

### Benchmark Results

| Operation | Before | After | Change |
|-----------|--------|-------|--------|
| JSON→Proto (small) | 195k/s | 200k/s | +2.6% |
| JSON→Proto (many tags) | 120k/s | 150k/s | +25% |
| Pipeline | 155k/s | 175k/s | +13% |

Full results: See CI artifacts

### Changes
- Use `SmallVec<[String; 4]>` for tag values (most tags have <4 values)
- Preallocate string buffer for hex conversion
- Skip validation in trusted internal conversions

### Tradeoffs
- Added `smallvec` dependency (14kb, no other deps)
- Slightly more complex tag handling code
- Memory usage unchanged (SmallVec only heap-allocates when needed)

### Testing
- All existing tests pass
- Added benchmark for many-tag scenario
- Validated correctness with large real-world dataset
```

## FAQs

### Q: Should benchmarks block merging?

**A:** No, but they should inform decisions:
- Small regressions (<10%) are usually acceptable
- Large regressions (>15%) should be justified
- Use judgment based on the change purpose

### Q: How often should I run benchmarks locally?

**A:**
- Before starting performance work
- After making performance changes
- When curious about impact
- Not needed for most bug fixes or features

### Q: What if CI benchmarks are noisy?

**A:**
- Some variance is normal (5-10%)
- Run benchmarks locally for definitive results
- Consider running multiple CI iterations for important comparisons
- Focus on consistent trends, not single-run differences

### Q: Can I disable benchmark CI for my PR?

**A:**
- CI only runs on paths that affect performance
- If it runs, results are valuable even if not reviewed immediately
- Artifacts help future investigation
- No need to disable; it runs in parallel

### Q: How do I benchmark my specific use case?

**A:**
- Add a new benchmark in `benches/`
- Follow existing patterns
- Register in `Cargo.toml`
- See BENCHMARKING.md for detailed guide

## See Also

- [BENCHMARKING.md](BENCHMARKING.md) - Detailed benchmarking guide
- [BENCHMARKS_README.md](BENCHMARKS_README.md) - Quick overview
- [BENCHMARK_STATUS.md](BENCHMARK_STATUS.md) - Implementation status
- [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md) - General development practices

---

**Remember**: Benchmarks are tools to inform decisions, not gatekeepers to block progress. Use them wisely! ⚡

