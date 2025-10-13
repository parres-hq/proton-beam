#!/bin/bash

set -euo pipefail

# Default to stable for fast local checks
version="${1:-stable}"

# Install toolchain
if [ "$version" != "stable" ]; then
    cargo +$version --version || rustup install $version
else
    cargo +$version --version || rustup update stable
fi

# Ensure rustdoc is available (installed with the toolchain)
rustdoc +$version --version >/dev/null

echo "Checking docs with $version"
echo ""

echo "→ Documenting workspace with all features..."
RUSTDOCFLAGS="-D warnings" cargo +$version doc --workspace --no-deps --all-features --document-private-items

echo ""
echo "✓ Documentation check passed for $version!"
echo

