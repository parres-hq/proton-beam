# GitHub Actions Workflows

This directory contains automated CI/CD workflows for Proton Beam.

## Workflows

### 🔍 `ci.yml` - Continuous Integration

**Purpose**: Comprehensive quality checks on every push and PR

**Runs on**:
- Push to `main` or `develop` branches
- Pull requests to `main` or `develop`
- Manual dispatch

**Jobs**:

1. **Format Check** (`format`)
   - Verifies code formatting with `rustfmt`
   - Fast fail - runs first
   - Command: `cargo fmt --all -- --check`

2. **Clippy Lints** (`clippy`)
   - Runs Rust linter
   - Checks both all-features and no-default-features
   - Treats warnings as errors (`-D warnings`)
   - Commands:
     - `cargo clippy --workspace --all-features --all-targets -- -D warnings`
     - `cargo clippy --workspace --no-default-features --all-targets -- -D warnings`

3. **Documentation** (`docs`)
   - Builds documentation
   - Checks for broken links and missing docs
   - Treats warnings as errors
   - Command: `cargo doc --workspace --all-features --no-deps --document-private-items`

4. **Tests (Stable)** (`test-stable`)
   - Runs on: Ubuntu, macOS, Windows
   - Tests all feature combinations
   - Includes doc tests
   - Commands:
     - `cargo test --workspace --all-features --all-targets`
     - `cargo test --workspace --all-features --doc`
     - `cargo test --workspace --no-default-features --all-targets`

5. **Tests (MSRV)** (`test-msrv`)
   - Tests on Minimum Supported Rust Version (1.70.0)
   - Ensures backward compatibility
   - Runs on Ubuntu only

6. **Build (Feature Combinations)** (`build-features`)
   - Builds with different feature flags
   - Validates each crate builds independently
   - Catches feature-related issues

7. **Security Audit** (`security-audit`)
   - Scans dependencies for known vulnerabilities
   - Uses `cargo-audit`
   - Fails on any security warnings

8. **CI Success** (`ci-success`)
   - Final job that requires all others to pass
   - Use this as branch protection requirement

**Status**: ✅ Ready to use

**Estimated runtime**:
- Format: ~30 seconds
- Clippy: ~2-3 minutes
- Docs: ~2-3 minutes
- Tests (per OS): ~5-10 minutes
- MSRV: ~5-10 minutes
- Build: ~3-5 minutes
- Audit: ~1 minute
- **Total**: ~20-30 minutes (jobs run in parallel)

---

### 📊 `benchmarks.yml` - Performance Benchmarks

**Purpose**: Track performance and catch regressions

**Runs on**:
- Pull requests (when performance-critical files change)
- Push to `main` branch
- Weekly schedule (Sundays at 2 AM UTC)
- Manual dispatch

**Triggers on file changes**:
- `proton-beam-core/**`
- `proton-beam-cli/**`
- `Cargo.toml` / `Cargo.lock`
- `.github/workflows/benchmarks.yml`

**Jobs**:

1. **Run Benchmarks** (`benchmark`)
   - Runs full benchmark suite in release mode
   - Saves results as artifacts (90-day retention)
   - Maintains baseline from main branch
   - Compares PR results against baseline
   - Posts comparison comment on PR

2. **Regression Check** (`regression-check`)
   - Analyzes benchmark differences
   - Flags significant regressions (>15%)
   - Runs only on pull requests

**Artifacts**:
- `benchmark-results-{SHA}` - Full output for each run
- `benchmark-results-baseline` - Latest main branch baseline

**Status**: ✅ Ready to use

**Estimated runtime**: ~10-15 minutes

**Note**: Does not fail CI on regressions - benchmarks are informational

---

## Workflow Configuration

### Branch Protection

Recommended branch protection rules for `main`:

1. **Require status checks to pass**:
   - ✅ `CI Success` (from `ci.yml`)
   - ℹ️ `Run Benchmarks` (optional - for visibility)

2. **Require branches to be up to date**: ✅

3. **Require pull request reviews**: ✅ (1+ reviewers)

4. **Dismiss stale reviews**: ✅

### Caching Strategy

All workflows use GitHub Actions cache for:
- Cargo registry (`~/.cargo/registry`)
- Cargo git index (`~/.cargo/git`)
- Build artifacts (`target/`)

**Benefits**:
- Faster CI runs (30-50% time reduction)
- Reduced network usage
- Lower costs

**Cache keys** include `Cargo.lock` hash, so cache automatically invalidates when dependencies change.

### Secrets Required

No secrets required for current workflows!

All workflows use:
- `${{ secrets.GITHUB_TOKEN }}` - Automatically provided by GitHub Actions

### Environment Variables

Configured in workflows:
- `RUST_BACKTRACE=1` - Show backtraces on panics
- `CARGO_TERM_COLOR=always` - Colorized output
- `MSRV=1.70.0` - Minimum Supported Rust Version

---

## Usage

### For Developers

**Local commands match CI checks**:

```bash
# Format check (matches CI)
cargo fmt --all -- --check

# Clippy (matches CI)
cargo clippy --workspace --all-features --all-targets -- -D warnings

# Docs (matches CI)
cargo doc --workspace --all-features --no-deps --document-private-items

# Tests (matches CI)
cargo test --workspace --all-features --all-targets
cargo test --workspace --all-features --doc

# Or use just commands
just fmt
just lint
just docs
just test
just precommit  # Runs all checks
```

**Before creating a PR**:
```bash
just precommit  # Ensures CI will pass
```

### For Maintainers

**Viewing CI results**:
1. Go to **Actions** tab on GitHub
2. Select a workflow run
3. Review job results and logs

**Handling failures**:

- **Format failures**: Run `cargo fmt --all`
- **Clippy failures**: Fix warnings in code
- **Doc failures**: Fix documentation issues
- **Test failures**: Fix failing tests
- **MSRV failures**: Update code or bump MSRV
- **Security audit failures**: Update vulnerable dependencies

**Re-running workflows**:
- Click "Re-run jobs" in GitHub Actions UI
- Or push a new commit

**Manual workflow dispatch**:
1. Go to **Actions** tab
2. Select workflow
3. Click "Run workflow"
4. Choose branch and click "Run"

---

## Customization

### Changing MSRV

Edit `ci.yml`:
```yaml
env:
  MSRV: "1.70.0"  # Change to desired version
```

### Adding/Removing Test Platforms

Edit `ci.yml` under `test-stable` job:
```yaml
matrix:
  os: [ubuntu-latest, macos-latest, windows-latest]
  # Add/remove platforms as needed
```

### Changing Benchmark Schedule

Edit `benchmarks.yml`:
```yaml
schedule:
  - cron: '0 2 * * 0'  # Change cron expression
```

### Disabling Security Audit

Remove or comment out the `security-audit` job in `ci.yml` and remove it from `ci-success` needs.

---

## Troubleshooting

### CI is slow

**Solutions**:
1. Check cache hit rate in workflow logs
2. Consider splitting into more granular workflows
3. Reduce matrix dimensions (fewer OS/versions)
4. Use faster GitHub-hosted runners

### Cache issues

**Clear cache**:
1. Go to **Settings** → **Actions** → **Caches**
2. Delete problematic caches
3. Re-run workflow

### Workflow not triggering

**Check**:
1. File paths in trigger filters
2. Branch names match
3. Workflow file is valid YAML
4. Actions are enabled in repository settings

### Tests pass locally but fail in CI

**Common causes**:
1. Platform differences (Windows vs Linux vs macOS)
2. Missing environment variables
3. File path assumptions
4. Timing issues in async code
5. Network dependencies (mock them!)

**Debug**:
- Check CI logs for error details
- Run on matching platform locally
- Use `act` to run GitHub Actions locally

---

## Monitoring

### CI Health Metrics

Track these over time:
- ✅ Success rate (aim for >95%)
- ⏱️ Average runtime (watch for increases)
- 📦 Cache hit rate (aim for >80%)
- 🔴 Mean time to fix failures

### Performance Trends

For benchmarks:
- Download artifacts regularly
- Compare against historical baselines
- Watch for gradual degradation
- Celebrate improvements!

---

## Best Practices

### ✅ DO

- Run `just precommit` before creating PRs
- Fix CI failures promptly
- Review benchmark results on performance PRs
- Update workflows when project structure changes
- Keep dependencies up to date (security!)

### ❌ DON'T

- Skip CI checks (use `[skip ci]` only for docs)
- Ignore security audit warnings
- Merge PRs with failing CI
- Add flaky tests without fixing them
- Commit directly to main (use PRs!)

---

## Related Documentation

- [BENCHMARK_PRACTICES.md](../../docs/BENCHMARK_PRACTICES.md) - Benchmark best practices
- [BENCHMARK_CI_README.md](../BENCHMARK_CI_README.md) - Benchmark CI details
- [DEVELOPER_GUIDE.md](../../docs/DEVELOPER_GUIDE.md) - Development guide
- [JUSTFILE_COMMANDS.md](../../docs/JUSTFILE_COMMANDS.md) - Local commands

---

## Questions?

- **CI failures**: Check logs and compare with local runs
- **Workflow changes**: Submit PR with clear explanation
- **New workflows**: Discuss in issue first

**Need help?** Open an issue with:
- Workflow name
- Run link
- Error message
- What you've tried

---

**Status**: ✅ All workflows operational and tested

**Last Updated**: 2025-10-14

