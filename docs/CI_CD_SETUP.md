# CI/CD Setup

This document provides an overview of the Continuous Integration and Continuous Deployment setup for Proton Beam.

## Overview

Proton Beam uses **GitHub Actions** for automated CI/CD with two main workflows:

1. **CI Workflow** (`ci.yml`) - Code quality and testing
2. **Benchmarks Workflow** (`benchmarks.yml`) - Performance tracking

Both workflows are configured to run automatically and provide fast feedback on code changes.

---

## CI Workflow

**File**: `.github/workflows/ci.yml`

**Purpose**: Ensure code quality, correctness, and compatibility

### When It Runs

- ✅ Push to `main` or `develop` branches
- ✅ Pull requests to `main` or `develop`
- ✅ Manual dispatch (via Actions UI)

### What It Checks

| Check | Purpose | Duration | Blocking? |
|-------|---------|----------|-----------|
| **Format** | Code formatting with `rustfmt` | ~30s | ✅ Yes |
| **Clippy** | Lint warnings and errors | ~3min | ✅ Yes |
| **Docs** | Documentation builds | ~3min | ✅ Yes |
| **Tests (Stable)** | Tests on Linux/macOS/Windows | ~10min | ✅ Yes |
| **Tests (MSRV)** | Rust 1.90.0 compatibility | ~10min | ✅ Yes |
| **Build (Features)** | Different feature combinations | ~5min | ✅ Yes |
| **Security Audit** | Dependency vulnerabilities | ~1min | ✅ Yes |

**Total time**: ~20-30 minutes (runs in parallel)

### Jobs Details

#### 1. Format Check
```bash
cargo fmt --all -- --check
```
- Verifies code follows Rust style guidelines
- Fast fail - runs first to catch obvious issues
- Fix locally: `cargo fmt --all`

#### 2. Clippy Lints
```bash
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo clippy --workspace --no-default-features --all-targets -- -D warnings
```
- Catches common mistakes and anti-patterns
- Tests both feature modes
- Treats warnings as errors
- Fix locally: `cargo clippy --fix`

#### 3. Documentation
```bash
cargo doc --workspace --all-features --no-deps --document-private-items
```
- Builds API documentation
- Catches broken links and missing docs
- Ensures doc examples compile
- Fix locally: `cargo doc --workspace`

#### 4. Tests (Stable Rust)
- **Platforms**: Ubuntu, macOS, Windows
- **Commands**:
  ```bash
  cargo test --workspace --all-features --lib --bins --tests --examples
  cargo test --workspace --all-features --doc
  cargo test --workspace --no-default-features --lib --bins --tests --examples
  ```
- **Coverage**: All tests, doc tests, all feature combinations
- **Note**: Excludes benchmarks (too slow for CI, run separately)
- **Current status**: 92/92 tests passing

#### 5. Tests (MSRV - Rust 1.90.0)
- Ensures backward compatibility
- Tests on minimum supported Rust version
- Ubuntu only (for speed)
- Prevents accidental use of newer Rust features

#### 6. Build (Feature Combinations)
- Tests different feature flag combinations
- Validates each crate builds independently
- Catches feature-related compilation issues

#### 7. Security Audit
```bash
cargo audit --deny warnings
```
- Scans dependencies for known vulnerabilities
- Uses RustSec Advisory Database
- Fails on any security issues
- Update vulnerable deps: `cargo update`

#### 8. CI Success
- Final job requiring all others to pass
- Use this as branch protection requirement
- Simplifies branch protection rules

---

## Benchmarks Workflow

**File**: `.github/workflows/benchmarks.yml`

**Purpose**: Track performance over time and catch regressions

### When It Runs

- ✅ Pull requests (when performance files change)
- ✅ Push to `main` branch
- ✅ Weekly schedule (Sundays at 2 AM UTC)
- ✅ Manual dispatch

### Triggers on Changes To

- `proton-beam-core/**`
- `proton-beam-cli/**`
- `Cargo.toml` / `Cargo.lock`
- `.github/workflows/benchmarks.yml`

### What It Does

1. **Runs full benchmark suite** (release mode)
2. **Saves results as artifacts** (90-day retention)
3. **Maintains baseline** from main branch
4. **Compares PR vs baseline**
5. **Posts comparison comment** on PR

### Jobs Details

#### 1. Run Benchmarks
```bash
cargo bench --workspace --no-fail-fast
```
- Runs all 6 benchmark suites
- Takes ~10-15 minutes
- Saves full output as artifact
- Compares against baseline (on PRs)

#### 2. Regression Check
- Downloads current and baseline results
- Analyzes performance differences
- Flags significant changes (>15%)
- **Does NOT fail CI** - informational only

### Artifacts

- `benchmark-results-{SHA}` - Full results for each run
- `benchmark-results-baseline` - Latest main branch baseline

**Retention**: 90 days

**Download**: Actions → Workflow Run → Artifacts section

---

## Local Development

### Matching CI Locally

Run the same checks CI runs:

```bash
# Format check
cargo fmt --all -- --check

# Clippy (all warnings)
cargo clippy --workspace --all-features --all-targets -- -D warnings

# Documentation
cargo doc --workspace --all-features --no-deps

# Tests (comprehensive, excludes benchmarks)
cargo test --workspace --all-features --lib --bins --tests --examples
cargo test --workspace --all-features --doc
cargo test --workspace --no-default-features --lib --bins --tests --examples

# Security audit
cargo install cargo-audit  # First time only
cargo audit
```

### Using Just Commands (Recommended)

```bash
# Individual checks
just fmt      # Format check
just lint     # Clippy
just docs     # Documentation
just test     # Tests

# Combined checks
just check     # fmt + lint + docs + test
just precommit # Comprehensive (includes MSRV)
just ci        # Full CI simulation

# Benchmarks
just bench       # Run benchmarks
just bench-save  # Save results
```

### Pre-commit Checklist

Before creating a PR, run:

```bash
just precommit
```

This runs:
1. ✅ Build all crates
2. ✅ Format check
3. ✅ Clippy (all features)
4. ✅ Documentation
5. ✅ MSRV check (Rust 1.90.0)
6. ✅ All tests

**Time**: ~5-10 minutes locally

If this passes, CI will pass! 🎉

---

## Branch Protection

### Recommended Settings

For the `main` branch:

#### Required Status Checks
- ✅ `CI Success` (from `ci.yml`)
- ℹ️ `Run Benchmarks` (optional, for visibility)

#### Other Settings
- ✅ Require branches to be up to date before merging
- ✅ Require pull request reviews (1+ approvers)
- ✅ Dismiss stale pull request approvals
- ✅ Require review from Code Owners
- ❌ Allow force pushes (never!)
- ❌ Allow deletions (never!)

### Setting Up

1. Go to **Settings** → **Branches**
2. Click **Add rule**
3. Branch name pattern: `main`
4. Enable:
   - "Require status checks to pass before merging"
   - Select: `CI Success`
   - "Require branches to be up to date"
   - "Require pull request reviews before merging"
5. Save changes

---

## Caching Strategy

All workflows use GitHub Actions caching to speed up builds:

### What's Cached

1. **Cargo Registry** (`~/.cargo/registry`)
   - Downloaded crate files
   - Key: Includes `Cargo.lock` hash

2. **Cargo Git Index** (`~/.cargo/git`)
   - Git dependencies
   - Key: Includes `Cargo.lock` hash

3. **Build Artifacts** (`target/`)
   - Compiled dependencies and artifacts
   - Key: Includes `Cargo.lock` hash + job name

### Cache Benefits

- **30-50% faster** CI runs
- **Reduced network** usage
- **Lower costs** (less compute time)

### Cache Invalidation

Cache automatically invalidates when:
- `Cargo.lock` changes (dependency updates)
- 7 days pass (GitHub's limit)
- Cache exceeds size limit (10 GB per repo)

### Managing Cache

**View caches**:
Settings → Actions → Caches

**Clear cache** (if issues):
1. Delete specific caches
2. Re-run workflow

---

## Performance

### CI Duration

Typical durations:

| Workflow | Jobs | Duration | Parallelism |
|----------|------|----------|-------------|
| **CI** | 8 jobs | ~20-30 min | High |
| **Benchmarks** | 2 jobs | ~10-15 min | Medium |

### Optimization Tips

1. **Cache hits**: Aim for >80% hit rate
2. **Selective triggers**: Limit benchmark triggers
3. **Fast failures**: Format/lint checks fail first
4. **Parallel jobs**: Most jobs run concurrently

---

## Troubleshooting

### CI Passes Locally But Fails in GitHub

**Common causes**:

1. **Platform differences**
   - Solution: Test on matching OS
   - CI runs on Linux (Ubuntu)

2. **Missing dependencies**
   - Solution: Check workflow `apt-get` installs
   - Add missing system dependencies

3. **Environment variables**
   - Solution: Check `env` in workflow file
   - Add needed vars to workflow

4. **File paths**
   - Solution: Use OS-agnostic paths
   - Test on multiple platforms locally

5. **Timing/race conditions**
   - Solution: Add retries or increase timeouts
   - Make tests deterministic

### Benchmark Results Noisy

**Normal variance**: ±5-10%

**Solutions**:
- Run multiple times and average
- Compare trends over time, not single runs
- Focus on large differences (>15%)

### Security Audit Failures

**When it fails**:
- Vulnerable dependency detected

**Solutions**:
1. Update dependency: `cargo update -p <crate>`
2. Check if fixed version available
3. If no fix: file issue with dependency
4. Last resort: Temporarily allow in `audit.toml`

### Workflow Not Triggering

**Check**:
1. File paths in trigger match changed files
2. Branch names are correct
3. Actions enabled in repo settings
4. YAML syntax is valid

---

## Monitoring & Metrics

### Track These Over Time

1. **Success rate**
   - Target: >95%
   - Alert if drops below 90%

2. **Average duration**
   - Watch for increases
   - Investigate if +20% change

3. **Cache hit rate**
   - Target: >80%
   - Improve if below 70%

4. **Test pass rate**
   - Target: 100%
   - Fix flaky tests immediately

### GitHub Insights

View metrics:
- **Insights** → **Actions**
- See workflow runs, success rates, durations
- Download workflow usage reports

---

## Best Practices

### ✅ DO

- Run `just precommit` before every PR
- Fix CI failures promptly (within 24h)
- Keep dependencies up to date
- Review benchmark results on performance PRs
- Write tests for new features
- Update docs when changing APIs

### ❌ DON'T

- Merge PRs with failing CI
- Skip CI with `[skip ci]` (except docs-only changes)
- Commit directly to main (use PRs!)
- Ignore security audit warnings
- Add flaky/non-deterministic tests
- Use `todo!()` or `unimplemented!()` in prod code

---

## Continuous Improvement

### Regular Tasks

**Weekly**:
- Review benchmark trends
- Check for new security advisories
- Update dependencies (minor versions)

**Monthly**:
- Review CI duration trends
- Optimize slow jobs if needed
- Update GitHub Actions versions

**Quarterly**:
- Review and update MSRV if needed
- Evaluate new CI tools/features
- Performance testing and optimization

---

## Cost Optimization

GitHub Actions is **free for public repositories**!

For private repos:
- 2,000 minutes/month on free plan
- Current usage: ~50 min/day typical
- ~1,500 min/month for this project

**Tips to reduce usage**:
1. Use cache effectively
2. Limit benchmark frequency
3. Use sparse matrix (fewer OS combinations)
4. Skip CI on docs-only changes

---

## Future Enhancements

### Potential Additions

1. **Code Coverage**
   - Use `cargo-tarpaulin` or `cargo-llvm-cov`
   - Upload to Codecov
   - Track coverage trends

2. **Release Automation**
   - Publish to crates.io on tags
   - Generate changelog automatically
   - Create GitHub releases

3. **Docker Images**
   - Build and publish Docker images
   - Multi-arch support (amd64, arm64)

4. **Nightly Builds**
   - Test against Rust nightly
   - Catch future breakage early

5. **Integration Tests**
   - End-to-end testing with real relay
   - Performance testing at scale

---

## Related Documentation

- [CI Workflows README](../.github/workflows/README.md) - Workflow details
- [BENCHMARK_PRACTICES.md](BENCHMARK_PRACTICES.md) - Benchmark best practices
- [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md) - Development guide
- [JUSTFILE_COMMANDS.md](JUSTFILE_COMMANDS.md) - Local commands

---

## Questions?

**CI failures**: Check workflow logs and compare with local runs

**Workflow changes**: Submit PR with clear explanation

**New features**: Discuss in issue first

**Need help?** Open an issue with:
- Workflow name and run link
- Error message
- What you've tried

---

**Status**: ✅ CI/CD fully configured and operational

**Workflows**:
- ✅ `ci.yml` - Comprehensive quality checks
- ✅ `benchmarks.yml` - Performance tracking

**Coverage**:
- ✅ Code quality (format, lint, docs)
- ✅ Testing (multi-platform, MSRV)
- ✅ Security (dependency audit)
- ✅ Performance (benchmarks)

**Ready for production use!** 🚀

