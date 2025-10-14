#!/usr/bin/env just --justfile

default:
    @just --list

# Run tests with all features (default) - excludes benchmarks
test:
    cargo test --workspace --all-features --lib --bins --tests --examples
    cargo test --workspace --all-features --doc

# Run tests for all feature combinations (excludes benchmarks - too slow)
test-all:
    @echo "Testing with all features..."
    cargo test --workspace --all-features --lib --bins --tests --examples
    cargo test --workspace --all-features --doc
    @echo ""
    @echo "Testing with no default features..."
    cargo test --workspace --no-default-features --lib --bins --tests --examples
    @echo ""
    @echo "Testing each crate independently..."
    cargo test -p proton-beam-core --all-features
    cargo test -p proton-beam-cli --all-features
    cargo test -p proton-beam-daemon --all-features

# Check clippy for all feature combinations (uses stable by default)
lint:
    @bash scripts/check-clippy.sh

# Check fmt (uses stable by default)
fmt:
    @bash scripts/check-fmt.sh

# Check docs (uses stable by default)
docs:
    @bash scripts/check-docs.sh

# Build all crates
build:
    cargo build --workspace --all-features
    cargo build --workspace --no-default-features

# Quick check with stable (fast for local development)
check:
    @bash scripts/check-all.sh
    @just test

# Pre-commit check: runs both stable and MSRV checks
precommit:
    @echo "=========================================="
    @echo "Running pre-commit checks (stable + MSRV)"
    @echo "=========================================="
    @echo ""
    @echo "→ Building all crates..."
    @just build
    @echo ""
    @echo "→ Checking with stable Rust..."
    @bash scripts/check-all.sh stable
    @echo ""
    @echo "→ Checking with MSRV (1.90.0)..."
    @bash scripts/check-msrv.sh
    @echo ""
    @echo "→ Running comprehensive tests..."
    @just test-all
    @echo ""
    @echo "=========================================="
    @echo "✓ All pre-commit checks passed!"
    @echo "=========================================="

# CI check: comprehensive test suite for continuous integration
ci:
    @echo "=========================================="
    @echo "Running CI checks"
    @echo "=========================================="
    @echo ""
    @just build
    @just precommit

# Run all benchmarks (cargo bench always uses release mode)
bench:
    @echo "=========================================="
    @echo "Running all benchmarks"
    @echo "=========================================="
    @echo ""
    cargo bench --workspace

# Run all benchmarks in debug mode (fast compilation, approximate results)
bench-debug:
    @echo "=========================================="
    @echo "Running all benchmarks (debug mode)"
    @echo "=========================================="
    @echo ""
    @echo "⚠️  Using debug mode - results are approximate"
    @echo ""
    cargo test --workspace --benches -- --nocapture

# Run only core library benchmarks
bench-core:
    @echo "Running core library benchmarks..."
    cargo bench --package proton-beam-core

# Run only CLI benchmarks
bench-cli:
    @echo "Running CLI benchmarks..."
    cargo bench --package proton-beam-cli

# Run conversion benchmarks
bench-conversion:
    @echo "Running conversion benchmarks..."
    cargo bench --package proton-beam-core --bench conversion_bench

# Run validation benchmarks
bench-validation:
    @echo "Running validation benchmarks..."
    cargo bench --package proton-beam-core --bench validation_bench

# Run storage benchmarks
bench-storage:
    @echo "Running storage benchmarks..."
    cargo bench --package proton-beam-core --bench storage_bench

# Run builder benchmarks
bench-builder:
    @echo "Running builder benchmarks..."
    cargo bench --package proton-beam-core --bench builder_bench

# Run index benchmarks
bench-index:
    @echo "Running index benchmarks..."
    cargo bench --package proton-beam-core --bench index_bench

# Run pipeline benchmarks
bench-pipeline:
    @echo "Running pipeline benchmarks..."
    cargo bench --package proton-beam-cli --bench pipeline_bench

# Run all benchmarks and save results to file
bench-save:
    @echo "Running benchmarks and saving results..."
    @mkdir -p benchmark-results
    cargo bench --workspace 2>&1 | tee benchmark-results/bench-$(date +%Y%m%d-%H%M%S).txt
    @echo ""
    @echo "✓ Results saved to benchmark-results/"

# Compare benchmark results (requires two saved benchmark files)
bench-compare BASELINE CURRENT:
    @echo "Comparing benchmarks:"
    @echo "  Baseline: {{BASELINE}}"
    @echo "  Current:  {{CURRENT}}"
    @echo ""
    @diff {{BASELINE}} {{CURRENT}} || echo "Differences found above"

# Quick benchmark smoke test (runs fast, approximate results)
bench-quick:
    @echo "Quick benchmark test (debug mode)..."
    cargo bench --package proton-beam-core --bench conversion_bench -- --test
    cargo bench --package proton-beam-core --bench validation_bench -- --test
    @echo "✓ Benchmarks compile and run"

# Analyze benchmark history and show trends
bench-history:
    @echo "Analyzing benchmark history..."
    @./scripts/analyze-benchmark-history.sh benchmark-results

