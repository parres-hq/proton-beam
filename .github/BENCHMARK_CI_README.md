# Benchmark CI Workflow

This directory contains the GitHub Actions workflow for automated performance benchmarking.

## Workflow: `benchmarks.yml`

### What It Does

1. **Runs benchmarks** on:
   - Pull requests (when performance-critical files change)
   - Pushes to `main` branch
   - Weekly schedule (Sundays at 2 AM UTC)
   - Manual dispatch

2. **Saves results** as artifacts (90-day retention)

3. **Maintains baseline** from main branch

4. **Compares PR results** against baseline and posts comment

### Triggered By

The workflow runs when:

- **Pull Requests** modify:
  - `proton-beam-core/**`
  - `proton-beam-cli/**`
  - `Cargo.toml` / `Cargo.lock`
  - `.github/workflows/benchmarks.yml`

- **Pushes to main** (same paths)

- **Schedule**: Every Sunday at 2:00 AM UTC

- **Manual**: Via GitHub Actions UI

### Artifacts

Each run produces:
- `benchmark-results-{SHA}` - Full benchmark output
- `benchmark-results-baseline` - Latest main branch baseline (updated on main pushes)

### Viewing Results

1. Go to **Actions** tab
2. Click on **Performance Benchmarks** workflow
3. Select a run
4. Download artifacts from the "Artifacts" section
5. Extract and view `.txt` files

### Comparing Results

The workflow automatically compares PR results against the main branch baseline:

- If baseline exists, generates comparison
- Posts comment on PR (requires `GITHUB_TOKEN`)
- Highlights differences in results

### CI Configuration

#### Cache Strategy

The workflow caches:
- Cargo registry
- Cargo git index
- Build artifacts (target directory)

This speeds up subsequent runs significantly.

#### Timeout

Maximum run time: 30 minutes

If benchmarks take longer, the workflow will fail. This helps catch performance issues or infinite loops.

#### Runner

Uses: `ubuntu-latest`

Note: Results may differ from:
- Local development (different OS/hardware)
- macOS (different syscalls/threading)
- Other Linux distributions

### Interpreting CI Results

#### Normal Variance
Benchmarks can vary ±5-10% between runs due to:
- CI runner load
- Shared resources
- Background processes
- Cache state

#### Significant Changes
- **>15% regression**: 🚨 Investigate immediately
- **10-15% regression**: ⚠️ Review and justify
- **5-10% change**: ℹ️ Probably noise, but monitor
- **>10% improvement**: 🎉 Document what changed!

### Customization

#### Change Trigger Paths

Edit the `paths` filter in `benchmarks.yml`:

```yaml
paths:
  - 'proton-beam-core/**'
  - 'proton-beam-cli/**'
  # Add more paths as needed
```

#### Change Schedule

Edit the `cron` schedule:

```yaml
schedule:
  - cron: '0 2 * * 0'  # Every Sunday at 2 AM UTC
```

Cron format: `minute hour day month weekday`

#### Change Retention

Edit artifact retention:

```yaml
retention-days: 90  # Change to desired number of days
```

### Disabling the Workflow

To temporarily disable:

1. Rename `benchmarks.yml` to `benchmarks.yml.disabled`
2. Or add at the top:
   ```yaml
   on:
     workflow_dispatch:  # Only manual triggers
   ```

### Troubleshooting

#### Workflow Doesn't Run

Check:
- File paths match the trigger paths
- You're pushing to a PR or main branch
- Workflow file is valid YAML
- Repository has Actions enabled

#### Artifacts Not Found

Check:
- Workflow completed successfully
- Artifact retention period (90 days)
- You have permission to access artifacts

#### Comparison Fails

Check:
- Baseline artifact exists (needs one main branch run first)
- `GITHUB_TOKEN` has correct permissions
- PR is from same repository (not fork)

#### Benchmarks Timeout

If benchmarks take >30 minutes:
- Check for infinite loops or hangs
- Consider splitting into multiple workflows
- Increase timeout in workflow file

### Best Practices

1. **Don't fail CI on small regressions** - Benchmarks have natural variance

2. **Review results manually** - Automated comparison is a guide, not gospel

3. **Keep history** - Download important milestone benchmarks locally

4. **Document significant changes** - When performance changes >10%, explain why

5. **Run locally first** - Don't rely solely on CI for performance work

### Local Development

For local development, use:

```bash
# Run benchmarks locally
just bench

# Save results
just bench-save

# Compare results
just bench-compare OLD.txt NEW.txt

# Analyze history
just bench-history
```

See `docs/BENCHMARK_PRACTICES.md` for detailed guidance.

### Related Documentation

- [BENCHMARK_PRACTICES.md](../docs/BENCHMARK_PRACTICES.md) - When/how to run benchmarks
- [BENCHMARKING.md](../docs/BENCHMARKING.md) - Detailed benchmarking guide
- [BENCHMARKS_README.md](../docs/BENCHMARKS_README.md) - Quick overview

### Questions?

- **Setup issues**: Check GitHub Actions logs
- **Results questions**: See `docs/BENCHMARK_PRACTICES.md`
- **Workflow changes**: Submit PR with explanation

---

**Remember**: Benchmarks inform decisions, they don't make them. Use judgment! 🚀

