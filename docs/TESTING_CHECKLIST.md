# Testing & Validation Checklist

This document outlines all the testing and validation checks performed on the Proton Beam workspace.

## Quick Reference

| Command | Purpose | Speed | Use Case |
|---------|---------|-------|----------|
| `just test` | Run all tests with all features | Fast | Local development |
| `just test-all` | Comprehensive test coverage | Slow | Pre-commit validation |
| `just check` | Quick checks + tests | Fast | During development |
| `just build` | Verify all crates compile | Fast | Build verification |
| `just precommit` | Full pre-commit suite | Slow | Before committing |
| `just ci` | CI/CD comprehensive checks | Slowest | Continuous integration |

## What Gets Checked

### 1. **Formatting** (`just fmt`)
- ✅ All workspace members
- ✅ Code in doc comments
- ✅ Stable and MSRV Rust versions

**Command:** `cargo fmt --all`

### 2. **Documentation** (`just docs`)
- ✅ All workspace members
- ✅ All features enabled
- ✅ Private items documented
- ✅ Warnings treated as errors
- ✅ Stable and MSRV Rust versions

**Commands:**
```bash
cargo doc --workspace --no-deps --all-features --document-private-items
```

### 3. **Linting** (`just lint`)
- ✅ All workspace members (proton-beam-core, proton-beam-cli, proton-beam-daemon)
- ✅ All targets (lib, bins, tests, examples, benches)
- ✅ With all features enabled
- ✅ With no default features
- ✅ Warnings treated as errors
- ✅ Stable and MSRV Rust versions

**Commands:**
```bash
cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings
cargo clippy --workspace --all-targets --no-default-features --no-deps -- -D warnings
```

### 4. **Testing** (`just test` or `just test-all`)

#### Basic Testing (`just test`)
- ✅ All workspace members
- ✅ All test targets (unit, integration, doc tests)
- ✅ With all features enabled

**Commands:**
```bash
cargo test --workspace --all-features --all-targets
cargo test --workspace --all-features --doc
```

#### Comprehensive Testing (`just test-all`)
- ✅ All workspace members
- ✅ All features enabled
- ✅ No default features
- ✅ Each crate independently:
  - `proton-beam-core`
  - `proton-beam-cli`
  - `proton-beam-daemon`

**Commands:**
```bash
cargo test --workspace --all-features --all-targets
cargo test --workspace --all-features --doc
cargo test --workspace --no-default-features --all-targets
cargo test -p proton-beam-core --all-features
cargo test -p proton-beam-cli --all-features
cargo test -p proton-beam-daemon --all-features
```

### 5. **Build Verification** (`just build`)
- ✅ All workspace members
- ✅ With all features enabled
- ✅ With no default features

**Commands:**
```bash
cargo build --workspace --all-features
cargo build --workspace --no-default-features
```

### 6. **MSRV Verification** (part of `just precommit`)
- ✅ Rust 1.90.0 compatibility
- ✅ All checks run with MSRV toolchain
- ✅ Format, docs, and clippy checks

## Pre-Commit Workflow

The `just precommit` command runs a comprehensive suite:

1. **Build** - Verify all crates compile with both feature configurations
2. **Format Check** - Ensure code is properly formatted (stable)
3. **Documentation Check** - Verify docs build without warnings (stable)
4. **Lint Check** - Run clippy with all feature combinations (stable)
5. **MSRV Checks** - Repeat format, docs, and lint with MSRV (1.90.0)
6. **Comprehensive Tests** - Run all tests with multiple feature configurations

## CI Workflow

The `just ci` command is designed for continuous integration:

1. Runs `just build`
2. Runs `just precommit` (which includes all the checks above)

## Feature Coverage

Each check validates multiple feature configurations:

- **All features enabled** (`--all-features`)
  - Tests maximum feature set
  - Ensures feature interactions work correctly

- **No default features** (`--no-default-features`)
  - Tests minimal feature set
  - Ensures optional dependencies are truly optional
  - Validates feature gates work correctly

## Per-Crate Validation

The `test-all` command explicitly tests each crate independently:

1. **proton-beam-core** - Core conversion library
2. **proton-beam-cli** - Command-line tool
3. **proton-beam-daemon** - Background daemon service

This ensures:
- Each crate's dependencies are correctly specified
- No implicit dependencies on workspace siblings
- Each crate can be used independently

## Toolchain Coverage

All checks run against two Rust versions:

1. **Stable** - Latest stable Rust (primary development target)
2. **MSRV (1.90.0)** - Minimum supported Rust version

This ensures backward compatibility and prevents accidental use of newer features.

## Improvements Made

### Before
- Missing `--workspace` flag in some checks
- No feature matrix testing (`--no-default-features`)
- No per-crate independence verification
- No explicit build verification
- Less structured pre-commit process

### After
- ✅ Explicit `--workspace` flag in all multi-crate checks
- ✅ Both `--all-features` and `--no-default-features` tested
- ✅ Each crate tested independently
- ✅ Explicit build verification step
- ✅ Structured pre-commit and CI workflows
- ✅ Better output formatting and progress indicators
- ✅ Comprehensive documentation of what gets checked

## Development Workflow Recommendations

### During Active Development
```bash
just check  # Fast feedback loop
```

### Before Creating a Commit
```bash
just precommit  # Comprehensive validation
```

### In CI/CD Pipeline
```bash
just ci  # Full validation suite
```

### Quick Format Fix
```bash
cargo fmt --all
```

### Quick Lint Check
```bash
just lint
```

## Notes

- All checks treat warnings as errors in CI mode
- The `--no-deps` flag in clippy/docs ensures we only check our code, not dependencies
- Format checks include code in documentation comments
- Documentation checks include private items to maintain internal documentation quality

