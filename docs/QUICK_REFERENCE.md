# Quick Reference Card

Quick reference for common CI/CD and benchmark commands.

## 🚀 Before Every Commit

```bash
just precommit  # Runs format, lint, docs, tests, MSRV check
```

## 📊 Benchmarks

### When to Run

| Scenario | Command | Why |
|----------|---------|-----|
| **Before optimization** | `just bench-save` | Establish baseline |
| **After optimization** | `just bench-save` | Measure improvement |
| **Compare results** | `just bench-history` | See the difference |
| **Quick smoke test** | `just bench-quick` | Verify benchmarks work |

### DON'T Run On
- ❌ Pre-commit hooks (too slow)
- ❌ Every single commit (wasteful)
- ❌ Non-performance changes (unnecessary)

### DO Run On
- ✅ Before/after performance work
- ✅ When creating performance PRs
- ✅ To establish project baselines

## 🔍 CI Checks

### Local Commands (Match CI)

```bash
# Format
cargo fmt --all -- --check

# Lint
cargo clippy --workspace --all-features --all-targets -- -D warnings

# Docs
cargo doc --workspace --all-features --no-deps

# Tests
cargo test --workspace --all-features --all-targets
cargo test --workspace --all-features --doc

# Security
cargo audit

# Or use just commands
just fmt
just lint
just docs
just test
just check      # All of the above
just precommit  # Everything including MSRV
```

## 📈 Performance Impact Guide

| Change | Meaning | Action |
|--------|---------|--------|
| >15% slower | 🚨 **Regression** | Investigate immediately |
| 10-15% slower | ⚠️ **Concern** | Review and justify |
| 5-10% change | ℹ️ **Variance** | Probably noise |
| 5-10% faster | ✅ **Good** | Document |
| >10% faster | 🎉 **Excellent** | Celebrate! |

## 🔧 Common Tasks

### Fixing CI Failures

```bash
# Format failure
cargo fmt --all

# Clippy warnings
cargo clippy --fix --workspace --all-features

# Test failures
cargo test --workspace -- --nocapture  # See output

# Doc failures
cargo doc --workspace --all-features
```

### Viewing CI Results

1. GitHub PR → Checks section → Details
2. Or: Actions tab → Select run → View logs
3. Download artifacts: Actions → Run → Artifacts section

### Benchmark Workflow

```bash
# 1. Save baseline
just bench-save

# 2. Make changes
# ... edit code ...

# 3. Run benchmarks
just bench-save

# 4. Compare
just bench-history

# 5. Include results in PR description
```

## 📁 Key Files

```
Workflows:
  .github/workflows/ci.yml          # CI checks
  .github/workflows/benchmarks.yml  # Performance tracking

Documentation:
  docs/BENCHMARK_SUMMARY_ANSWER.md  # Your questions answered ⭐
  docs/BENCHMARK_PRACTICES.md       # Complete guide
  docs/CI_CD_SETUP.md               # CI/CD documentation
  SETUP_SUMMARY.md                  # Setup summary

Scripts:
  scripts/analyze-benchmark-history.sh  # Compare benchmarks
```

## 🎯 What CI Checks

| Check | Time | Blocks Merge? |
|-------|------|---------------|
| Format | ~30s | ✅ Yes |
| Clippy | ~3min | ✅ Yes |
| Docs | ~3min | ✅ Yes |
| Tests (Linux/Mac/Win) | ~10min | ✅ Yes |
| MSRV (Rust 1.70) | ~10min | ✅ Yes |
| Security Audit | ~1min | ✅ Yes |
| Benchmarks | ~15min | ❌ No |

**Total**: ~20-30 min (parallel)

## 💡 Tips

- **Fast feedback**: Format/lint checks fail fast
- **Local first**: Run `just precommit` before pushing
- **Benchmark history**: Save results regularly for trends
- **CI artifacts**: Download benchmark results from Actions
- **Weekly benchmarks**: Run automatically on Sundays

## 🆘 Help

| Issue | Solution |
|-------|----------|
| CI fails but passes locally | Check platform (Linux vs Mac) |
| Benchmarks too noisy | Compare trends, not single runs |
| Security audit fails | `cargo update` vulnerable deps |
| Slow CI | Check cache hit rate in logs |

## 📚 Full Documentation

See `docs/INDEX.md` for complete documentation index.

---

**Quick Links**:
- Start here: `docs/BENCHMARK_SUMMARY_ANSWER.md`
- Best practices: `docs/BENCHMARK_PRACTICES.md`
- CI/CD guide: `docs/CI_CD_SETUP.md`
- All commands: `docs/JUSTFILE_COMMANDS.md`

