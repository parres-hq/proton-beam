#!/bin/bash

set -euo pipefail

# Default to stable for fast local checks
version="${1:-stable}"

# Install toolchain
if [ "$version" != "stable" ]; then
    cargo +$version --version || rustup install $version
    cargo +$version clippy --version || rustup component add clippy --toolchain $version
else
    cargo +$version --version || rustup update "$version"
    cargo +$version clippy --version || rustup component add clippy --toolchain $version
fi

echo "Checking clippy with $version..."
echo ""

echo "→ Checking workspace with all features..."
cargo +$version clippy --workspace --all-targets --all-features --no-deps -- -D warnings

echo "→ Checking workspace with no default features..."
cargo +$version clippy --workspace --all-targets --no-default-features --no-deps -- -D warnings

echo ""
echo "✓ All clippy checks passed for $version!"
echo
