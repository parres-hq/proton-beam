#!/bin/bash
set -euo pipefail

# Proton Beam AWS Deployment Script
# This script runs on the EC2 instance created by the CloudFormation template
# This script sets up an EC2 instance, downloads data, converts it, and uploads to S3

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration - Set these via environment variables or modify here
INPUT_URL="${INPUT_URL:-}"
S3_OUTPUT="${S3_OUTPUT:-}"
OUTPUT_DIR="${OUTPUT_DIR:-/data/pb_data}"
VALIDATE_SIGNATURES="${VALIDATE_SIGNATURES:-true}"
VALIDATE_EVENT_IDS="${VALIDATE_EVENT_IDS:-true}"
PARALLEL_THREADS="${PARALLEL_THREADS:-}"
COMPRESSION_LEVEL="${COMPRESSION_LEVEL:-6}"
SHUTDOWN_WHEN_DONE="${SHUTDOWN_WHEN_DONE:-false}"
SKIP_INDEX="${SKIP_INDEX:-false}"

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "\n${BLUE}==>${NC} ${GREEN}$1${NC}\n"
}

# Error handler
error_exit() {
    log_error "$1"
    exit 1
}

# Trap errors
trap 'error_exit "Script failed at line $LINENO"' ERR

# Check required environment variables
check_requirements() {
    log_step "Checking requirements"

    if [[ -z "$INPUT_URL" ]]; then
        error_exit "INPUT_URL environment variable must be set"
    fi

    if [[ -z "$S3_OUTPUT" ]]; then
        error_exit "S3_OUTPUT environment variable must be set (format: s3://bucket/prefix)"
    fi

    log_info "INPUT_URL: $INPUT_URL"
    log_info "S3_OUTPUT: $S3_OUTPUT"
    log_info "OUTPUT_DIR: $OUTPUT_DIR"
    log_info "Validation (signatures): $VALIDATE_SIGNATURES"
    log_info "Validation (event IDs): $VALIDATE_EVENT_IDS"
    log_info "Compression level: $COMPRESSION_LEVEL"
    log_info "Shutdown when done: $SHUTDOWN_WHEN_DONE"
}

# Install system dependencies
install_dependencies() {
    log_step "Installing system dependencies"

    # Update package list
    log_info "Updating package list..."
    sudo apt-get update -qq

    # Install required packages
    log_info "Installing build tools..."
    sudo apt-get install -y -qq \
        build-essential \
        curl \
        wget \
        git \
        pkg-config \
        libssl-dev \
        protobuf-compiler \
        awscli \
        zstd \
        pigz \
        pv

    log_info "System dependencies installed"
}

# Install Rust
install_rust() {
    log_step "Installing Rust toolchain"

    if command -v rustc &> /dev/null; then
        log_info "Rust already installed: $(rustc --version)"
        return
    fi

    log_info "Downloading and installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable

    # Source Rust environment
    source "$HOME/.cargo/env"

    log_info "Rust installed: $(rustc --version)"
}

# Clone and build Proton Beam
build_proton_beam() {
    log_step "Building Proton Beam"

    # Create workspace directory
    WORKSPACE_DIR="/opt/proton-beam"
    sudo mkdir -p "$WORKSPACE_DIR"
    sudo chown "$(whoami):$(whoami)" "$WORKSPACE_DIR"

    cd "$WORKSPACE_DIR"

    # Clone repository if not exists
    if [[ ! -d ".git" ]]; then
        log_info "Cloning Proton Beam repository..."
        git clone https://github.com/parres-hq/proton-beam.git .
    else
        log_info "Repository already exists, pulling latest..."
        git pull
    fi

    # Build with S3 feature
    log_info "Building Proton Beam with S3 support (this may take a while)..."
    source "$HOME/.cargo/env"
    cargo build --release --features s3 -p proton-beam-cli

    # Make binary accessible
    sudo ln -sf "$WORKSPACE_DIR/target/release/proton-beam" /usr/local/bin/proton-beam

    log_info "Proton Beam built successfully: $(proton-beam --version)"
}

# Download input data
download_input() {
    log_step "Downloading input data"

    # Create data directory
    mkdir -p "$(dirname "$INPUT_FILE")"

    log_info "Downloading from: $INPUT_URL"
    log_info "Saving to: $DOWNLOAD_FILE"

    # Download with progress
    if [[ "$INPUT_URL" == s3://* ]]; then
        # S3 download
        aws s3 cp "$INPUT_URL" "$DOWNLOAD_FILE" --no-progress
    elif [[ "$INPUT_URL" == http://* ]] || [[ "$INPUT_URL" == https://* ]]; then
        # HTTP download
        wget -O "$DOWNLOAD_FILE" "$INPUT_URL" --progress=bar:force 2>&1
    else
        error_exit "Unsupported URL scheme: $INPUT_URL"
    fi

    # Get downloaded file size
    DOWNLOAD_SIZE=$(du -h "$DOWNLOAD_FILE" | cut -f1)
    log_info "Downloaded file size: $DOWNLOAD_SIZE"

    # Decompress if needed
    if [[ "$DOWNLOAD_FILE" == *.zst ]]; then
        log_info "Decompressing Zstandard file..."
        log_info "This may take several minutes for large files..."

        # Use pv for progress if file is large
        if command -v pv &> /dev/null; then
            pv "$DOWNLOAD_FILE" | zstd -d > "$INPUT_FILE"
        else
            zstd -d "$DOWNLOAD_FILE" -o "$INPUT_FILE"
        fi

        # Remove compressed file to save space
        rm "$DOWNLOAD_FILE"

        DECOMPRESSED_SIZE=$(du -h "$INPUT_FILE" | cut -f1)
        log_info "Decompressed size: $DECOMPRESSED_SIZE"
    elif [[ "$DOWNLOAD_FILE" == *.gz ]]; then
        log_info "Decompressing gzip file..."

        if command -v pv &> /dev/null && command -v pigz &> /dev/null; then
            pv "$DOWNLOAD_FILE" | pigz -d > "$INPUT_FILE"
        elif command -v pigz &> /dev/null; then
            pigz -d -c "$DOWNLOAD_FILE" > "$INPUT_FILE"
        else
            gunzip -c "$DOWNLOAD_FILE" > "$INPUT_FILE"
        fi

        rm "$DOWNLOAD_FILE"

        DECOMPRESSED_SIZE=$(du -h "$INPUT_FILE" | cut -f1)
        log_info "Decompressed size: $DECOMPRESSED_SIZE"
    elif [[ "$DOWNLOAD_FILE" == *.xz ]]; then
        log_info "Decompressing xz file..."

        if command -v pv &> /dev/null; then
            pv "$DOWNLOAD_FILE" | xz -d > "$INPUT_FILE"
        else
            xz -d -c "$DOWNLOAD_FILE" > "$INPUT_FILE"
        fi

        rm "$DOWNLOAD_FILE"

        DECOMPRESSED_SIZE=$(du -h "$INPUT_FILE" | cut -f1)
        log_info "Decompressed size: $DECOMPRESSED_SIZE"
    else
        # No decompression needed, file is already in final location
        log_info "No decompression needed"
    fi

    # Count lines for estimation
    log_info "Counting lines (this may take a moment)..."
    LINE_COUNT=$(wc -l < "$INPUT_FILE")
    log_info "Total lines: $LINE_COUNT"
}

# Run conversion
run_conversion() {
    log_step "Running conversion"

    # Build command arguments
    ARGS=(
        convert
        "$INPUT_FILE"
        --output-dir "$OUTPUT_DIR"
        --compression-level "$COMPRESSION_LEVEL"
        --s3-output "$S3_OUTPUT"
    )

    # Add validation flags
    if [[ "$VALIDATE_SIGNATURES" == "false" ]]; then
        ARGS+=(--validate-signatures false)
    fi

    if [[ "$VALIDATE_EVENT_IDS" == "false" ]]; then
        ARGS+=(--validate-event-ids false)
    fi

    # Add parallel threads if specified
    if [[ -n "$PARALLEL_THREADS" ]]; then
        ARGS+=(--parallel "$PARALLEL_THREADS")
    fi

    log_info "Starting conversion with command:"
    log_info "proton-beam ${ARGS[*]}"

    # Run conversion
    proton-beam "${ARGS[@]}"

    log_info "Conversion complete!"
}

# Build index
build_index() {
    if [[ "$SKIP_INDEX" == "true" ]]; then
        log_warn "Skipping index build (SKIP_INDEX=true)"
        return
    fi

    log_step "Building event index"

    log_info "Starting index rebuild..."
    proton-beam index rebuild "$OUTPUT_DIR" --s3-output "$S3_OUTPUT"

    log_info "Index build complete!"
}

# Cleanup and shutdown
cleanup_and_shutdown() {
    log_step "Cleanup"

    # Clean up input files to save space
    if [[ -f "$INPUT_FILE" ]]; then
        log_info "Removing input file: $INPUT_FILE"
        rm -f "$INPUT_FILE"
    fi

    if [[ -f "$DOWNLOAD_FILE" ]] && [[ "$DOWNLOAD_FILE" != "$INPUT_FILE" ]]; then
        log_info "Removing compressed file: $DOWNLOAD_FILE"
        rm -f "$DOWNLOAD_FILE"
    fi

    # Clean up output directory (already uploaded to S3)
    if [[ -d "$OUTPUT_DIR" ]]; then
        log_info "Removing output directory: $OUTPUT_DIR"
        rm -rf "$OUTPUT_DIR"
    fi

    log_info "Cleanup complete"

    # Shutdown if requested
    if [[ "$SHUTDOWN_WHEN_DONE" == "true" ]]; then
        log_warn "Shutting down instance in 60 seconds..."
        log_warn "Cancel with: sudo shutdown -c"
        sudo shutdown -h +1 "Proton Beam processing complete"
    fi
}

# Print summary
print_summary() {
    log_step "Summary"

    echo "┌──────────────────────────────────────────────────┐"
    echo "│           Proton Beam AWS Deployment             │"
    echo "└──────────────────────────────────────────────────┘"
    echo ""
    echo "✅ All tasks completed successfully!"
    echo ""
    echo "📊 Details:"
    echo "   Input:  $INPUT_URL"
    echo "   Output: $S3_OUTPUT"
    echo ""
    echo "📁 Check your S3 bucket for the results:"
    echo "   aws s3 ls $S3_OUTPUT"
    echo ""
}

# Main execution
main() {
    # Determine input filename from URL
    DOWNLOAD_FILE="/tmp/$(basename "$INPUT_URL")"

    # Determine decompressed filename (remove compression extension)
    if [[ "$DOWNLOAD_FILE" == *.zst ]]; then
        INPUT_FILE="${DOWNLOAD_FILE%.zst}"
    elif [[ "$DOWNLOAD_FILE" == *.gz ]]; then
        INPUT_FILE="${DOWNLOAD_FILE%.gz}"
    elif [[ "$DOWNLOAD_FILE" == *.xz ]]; then
        INPUT_FILE="${DOWNLOAD_FILE%.xz}"
    else
        INPUT_FILE="$DOWNLOAD_FILE"
    fi

    # Print banner
    echo "╔════════════════════════════════════════════════════════╗"
    echo "║     Proton Beam - AWS Deployment Script               ║"
    echo "║     Convert Nostr events to Protobuf at scale         ║"
    echo "╚════════════════════════════════════════════════════════╝"
    echo ""

    START_TIME=$(date +%s)

    # Execute steps
    check_requirements
    install_dependencies
    install_rust
    build_proton_beam
    download_input
    run_conversion
    build_index
    cleanup_and_shutdown

    # Calculate duration
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    DURATION_MIN=$((DURATION / 60))
    DURATION_SEC=$((DURATION % 60))

    print_summary

    log_info "Total time: ${DURATION_MIN}m ${DURATION_SEC}s"
}

# Run main function
main "$@"

