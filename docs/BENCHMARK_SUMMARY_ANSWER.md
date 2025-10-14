# Benchmark Best Practices - Quick Answer

## Your Questions Answered

### Q1: Should benchmarks run on pre-commit?

**NO** ❌

**Why not:**
- Benchmarks take 5-10 minutes to run properly (in release mode)
- Pre-commit hooks should be fast (<30 seconds) to not block developers
- Running on every commit creates noise in results
- Not every commit needs performance validation

**What should run on pre-commit:**
✅ Tests (fast, catches bugs)
✅ Clippy (linting)
✅ Rustfmt (formatting)
✅ MSRV check (compatibility)

Your current `just precommit` command already does this correctly!

### Q2: Should benchmarks run in CI?

**YES** ✅ - But strategically!

**When to run in CI:**
- ✅ On **pull requests** (to catch regressions before merge)
- ✅ On **pushes to main** (to track baseline performance)
- ✅ On **scheduled runs** (weekly/nightly for historical tracking)
- ❌ NOT on every push to feature branches (too expensive)

**What I've created for you:**
- `.github/workflows/benchmarks.yml` - Full CI workflow
- Automatically triggers on PRs that touch performance-critical code
- Saves baseline from main branch
- Compares PR results against baseline
- Stores results as artifacts for 90 days

### Q3: Are we storing past runs to track impact?

**Currently: NO** ❌ (but now you can!)

**What was missing:**
- No CI workflow for benchmarks
- No automated storage of results
- No baseline tracking
- No regression detection

**What I've added:**

1. **GitHub Actions Workflow** (`.github/workflows/benchmarks.yml`)
   - Runs benchmarks in CI
   - Saves results as artifacts (90-day retention)
   - Maintains baseline from main branch
   - Compares PRs against baseline

2. **Manual Tracking Commands** (already in your `justfile`)
   ```bash
   # Save results with timestamp
   just bench-save
   # Output: benchmark-results/bench-20251014-153045.txt

   # Compare two result files
   just bench-compare baseline.txt current.txt
   ```

3. **Best Practices Guide** (`docs/BENCHMARK_PRACTICES.md`)
   - Complete guide on when/how to run benchmarks
   - How to store history manually
   - How to interpret results
   - Optimization workflow

## Recommended Workflow

### For Regular Development
```bash
# Don't run benchmarks unless working on performance
git commit -m "Add feature X"  # No benchmarks needed
```

### For Performance Work
```bash
# 1. Save baseline before changes
just bench-save

# 2. Make your optimizations
# ... edit code ...

# 3. Run benchmarks again
just bench-save

# 4. Compare results
just bench-compare \
  benchmark-results/bench-OLD.txt \
  benchmark-results/bench-NEW.txt

# 5. Commit with performance info in PR description
```

### In CI (Automatic)
- CI runs benchmarks on your PR automatically
- Results saved as artifacts
- Compared against main branch baseline
- No action needed unless significant regression (>15%)

## Quick Commands Reference

```bash
# Run all benchmarks
just bench

# Run specific benchmark
just bench-conversion
just bench-storage
just bench-pipeline

# Save results with timestamp
just bench-save

# Compare two saved results
just bench-compare OLD.txt NEW.txt

# Quick smoke test (debug mode, fast)
just bench-quick
```

## Performance Impact Interpretation

| Change | Meaning | Action |
|--------|---------|--------|
| >15% slower | 🚨 Regression | Investigate immediately |
| 10-15% slower | ⚠️ Concern | Review and justify |
| 5-10% slower | ℹ️ Minor | Acceptable if justified |
| ±5% | ✅ Noise | Normal variance |
| 5-10% faster | ✅ Good | Document what improved |
| >10% faster | 🎉 Excellent | Celebrate and explain |

## Files Created/Modified

### New Files:
1. **`.github/workflows/benchmarks.yml`** - CI workflow for benchmarks
2. **`docs/BENCHMARK_PRACTICES.md`** - Comprehensive best practices guide
3. **`docs/BENCHMARK_SUMMARY_ANSWER.md`** (this file) - Quick reference

### Modified Files:
- **`docs/INDEX.md`** - Added links to new benchmark practices docs

### Already Existed (no changes needed):
- ✅ `justfile` - Already has great benchmark commands
- ✅ `docs/BENCHMARKING.md` - Already has detailed benchmarking guide
- ✅ `scripts/run-benchmarks.sh` - Already has benchmark runner
- ✅ Benchmark suite implementation - Already complete and working

## What Happens Next

### When you push these changes:

1. **On your next PR**, the benchmark CI workflow will run automatically
2. Results will be saved as artifacts
3. You can download and review them
4. Future PRs will compare against the baseline

### To track history manually:

```bash
# Store important milestones in git
mkdir -p benchmark-results/milestones
just bench 2>&1 | tee benchmark-results/milestones/v1.0-baseline.txt
git add benchmark-results/milestones/
git commit -m "Benchmark baseline for v1.0"
```

### To view CI results:

1. Go to **Actions** tab on GitHub
2. Click **Performance Benchmarks** workflow
3. Select a run
4. Download **benchmark-results** artifact
5. Extract and review the `.txt` file

## Bottom Line

### ✅ Best Practice Summary:

1. **Pre-commit**: NO benchmarks (too slow)
2. **CI**: YES benchmarks (on PRs and main)
3. **History**: Now tracked via CI artifacts + manual saves
4. **Workflow**: Run locally when optimizing, CI catches regressions

### 🎯 Your Setup Now:

- ✅ Comprehensive benchmark suite
- ✅ Easy local commands (`just bench`)
- ✅ CI integration (new!)
- ✅ History tracking (new!)
- ✅ Best practices documentation (new!)
- ✅ Baseline comparison (new!)

You're all set! 🚀

## See Also

- **[BENCHMARK_PRACTICES.md](BENCHMARK_PRACTICES.md)** - Full best practices guide
- **[BENCHMARKING.md](BENCHMARKING.md)** - How to run benchmarks
- **[BENCHMARKS_README.md](BENCHMARKS_README.md)** - Benchmark overview
- **[BENCHMARK_STATUS.md](BENCHMARK_STATUS.md)** - Implementation status

---

**Questions?** Open an issue or check the detailed guides above!

