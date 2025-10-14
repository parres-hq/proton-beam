# Justfile Commands Reference

Proton Beam uses [just](https://github.com/casey/just) as a command runner for common development tasks.

## Installation

If you don't have `just` installed:

```bash
# macOS
brew install just

# Linux
cargo install just

# Or use your system package manager
```

## Quick Reference

Run `just` or `just --list` to see all available commands.

## Benchmark Commands

### Run All Benchmarks

```bash
# Run all benchmarks (release mode, accurate results)
just bench

# Run all benchmarks in debug mode (fast compilation, approximate results)
just bench-debug

# Quick smoke test (ensures benchmarks compile and run)
just bench-quick
```

### Run Specific Benchmark Categories

```bash
# Core library benchmarks only
just bench-core

# CLI benchmarks only
just bench-cli
```

### Run Individual Benchmarks

```bash
# Conversion benchmarks (JSON ↔ Protobuf)
just bench-conversion

# Validation benchmarks (basic & cryptographic)
just bench-validation

# Storage benchmarks (I/O & compression)
just bench-storage

# Builder pattern benchmarks
just bench-builder

# Index benchmarks (SQLite operations)
just bench-index

# Pipeline benchmarks (end-to-end)
just bench-pipeline
```

### Save and Compare Results

```bash
# Save benchmark results with timestamp
just bench-save
# Results saved to: benchmark-results/bench-YYYYMMDD-HHMMSS.txt

# Compare two benchmark runs
just bench-compare benchmark-results/bench-20250113-100000.txt benchmark-results/bench-20250113-110000.txt

# Analyze benchmark history (compare oldest vs newest)
just bench-history
```

## Test Commands

```bash
# Run all tests
just test

# Run tests for all feature combinations
just test-all
```

## Code Quality Commands

```bash
# Check code formatting
just fmt

# Run clippy lints
just lint

# Check documentation
just docs

# Run all checks (fmt, lint, docs, test)
just check
```

## Build Commands

```bash
# Build all crates
just build
```

## Pre-commit and CI Commands

```bash
# Pre-commit check (comprehensive)
just precommit
# Runs: build, checks (fmt, lint, docs), and tests

# CI check (for continuous integration)
just ci
# Runs: build, precommit
```

## Example Workflows

### Development Workflow

```bash
# 1. Make changes to code
# 2. Run quick checks
just check

# 3. If adding performance-sensitive code, run benchmarks
just bench-conversion  # or other specific benchmark

# 4. Before committing
just precommit
```

### Performance Optimization Workflow

```bash
# 1. Save baseline benchmark
just bench-save
# Note the filename: benchmark-results/bench-YYYYMMDD-HHMMSS.txt

# 2. Make optimization changes

# 3. Run benchmarks again
just bench-save

# 4. Compare results
just bench-compare benchmark-results/bench-BASELINE.txt benchmark-results/bench-CURRENT.txt

# 5. Verify no regressions
just bench  # Run full suite to check all areas
```

### Quick Benchmark Check

```bash
# Just want to verify benchmarks still work?
just bench-quick
```

### Focus on Specific Area

```bash
# Working on JSON conversion?
just bench-conversion

# Working on validation?
just bench-validation

# Working on CLI pipeline?
just bench-pipeline
```

## Tips

### Parallel Execution

Some commands can be run in parallel:

```bash
# Run tests and benchmarks in parallel (in separate terminals)
just test &
just bench &
```

### Watching for Changes

Combine with `watchexec` or `cargo-watch`:

```bash
# Re-run tests on file changes
watchexec -e rs just test

# Re-run specific benchmark on changes
watchexec -e rs just bench-conversion
```

### Custom Aliases

Add to your shell config (~/.zshrc or ~/.bashrc):

```bash
alias jt='just test'
alias jb='just bench'
alias jc='just check'
alias jpc='just precommit'
```

## Command Categories

### 🧪 Testing
- `test` - Run all tests
- `test-all` - Run tests for all feature combinations

### 📊 Benchmarking
- `bench` - Run all benchmarks
- `bench-core` - Core library benchmarks
- `bench-cli` - CLI benchmarks
- `bench-conversion` - Conversion benchmarks
- `bench-validation` - Validation benchmarks
- `bench-storage` - Storage benchmarks
- `bench-builder` - Builder benchmarks
- `bench-index` - Index benchmarks
- `bench-pipeline` - Pipeline benchmarks
- `bench-save` - Save results with timestamp
- `bench-compare` - Compare two benchmark runs
- `bench-history` - Analyze all saved benchmark history
- `bench-quick` - Quick smoke test

### 🔍 Code Quality
- `fmt` - Check formatting
- `lint` - Run clippy
- `docs` - Check documentation
- `check` - Run all checks + tests

### 🏗️ Building
- `build` - Build all crates

### ✅ Pre-commit & CI
- `precommit` - Comprehensive pre-commit checks
- `ci` - CI checks (for continuous integration)

## See Also

- [BENCHMARKING.md](BENCHMARKING.md) - Detailed benchmarking guide
- [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md) - Development guide
- [justfile](../justfile) - The actual justfile (see all commands)

---

**Pro Tip**: Run `just --list` anytime to see all available commands with descriptions!

