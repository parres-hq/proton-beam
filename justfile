#!/usr/bin/env just --justfile

default:
    @just --list

# Run tests with all features (default)
test:
    cargo test --workspace --all-features --all-targets
    cargo test --workspace --all-features --doc

# Run tests for all feature combinations
test-all:
    @echo "Testing with all features..."
    cargo test --workspace --all-features --all-targets
    cargo test --workspace --all-features --doc
    @echo ""
    @echo "Testing with no default features..."
    cargo test --workspace --no-default-features --all-targets
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

