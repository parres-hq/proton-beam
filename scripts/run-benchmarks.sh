#!/bin/bash
# Benchmark runner script for Proton Beam
# Runs all benchmarks and generates a comprehensive performance report

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║              Proton Beam Performance Benchmark Suite          ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

# Parse arguments
RUN_ALL=true
BENCHMARKS=()

while [[ $# -gt 0 ]]; do
    case $1 in
        --core)
            RUN_ALL=false
            BENCHMARKS+=("core")
            shift
            ;;
        --cli)
            RUN_ALL=false
            BENCHMARKS+=("cli")
            shift
            ;;
        --conversion)
            RUN_ALL=false
            BENCHMARKS+=("conversion")
            shift
            ;;
        --validation)
            RUN_ALL=false
            BENCHMARKS+=("validation")
            shift
            ;;
        --storage)
            RUN_ALL=false
            BENCHMARKS+=("storage")
            shift
            ;;
        --builder)
            RUN_ALL=false
            BENCHMARKS+=("builder")
            shift
            ;;
        --index)
            RUN_ALL=false
            BENCHMARKS+=("index")
            shift
            ;;
        --pipeline)
            RUN_ALL=false
            BENCHMARKS+=("pipeline")
            shift
            ;;
        --release)
            BUILD_MODE="--release"
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --core        Run all core library benchmarks"
            echo "  --cli         Run all CLI benchmarks"
            echo "  --conversion  Run conversion benchmarks only"
            echo "  --validation  Run validation benchmarks only"
            echo "  --storage     Run storage benchmarks only"
            echo "  --builder     Run builder benchmarks only"
            echo "  --index       Run index benchmarks only"
            echo "  --pipeline    Run pipeline benchmarks only"
            echo "  --release     Build in release mode (slower build, accurate results)"
            echo "  --help        Show this help message"
            echo ""
            echo "If no options are specified, all benchmarks will run."
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Build configuration
BUILD_MODE=${BUILD_MODE:-""}

if [ -z "$BUILD_MODE" ]; then
    echo -e "${YELLOW}⚠️  Running in DEBUG mode for faster builds.${NC}"
    echo -e "${YELLOW}   Use --release flag for accurate benchmark results.${NC}"
    echo ""
fi

# Function to run a benchmark
run_benchmark() {
    local package=$1
    local bench_name=$2
    local description=$3

    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}▶ Running: $description${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""

    cargo bench --package "$package" --bench "$bench_name" $BUILD_MODE
}

# Determine which benchmarks to run
if [ "$RUN_ALL" = true ]; then
    BENCHMARKS=("conversion" "validation" "storage" "builder" "index" "pipeline")
fi

# Run Core Library Benchmarks
if [ "$RUN_ALL" = true ] || [[ " ${BENCHMARKS[@]} " =~ " core " ]] || [[ " ${BENCHMARKS[@]} " =~ " conversion " ]]; then
    run_benchmark "proton-beam-core" "conversion_bench" "Conversion Benchmarks (JSON ↔ Protobuf)"
fi

if [ "$RUN_ALL" = true ] || [[ " ${BENCHMARKS[@]} " =~ " core " ]] || [[ " ${BENCHMARKS[@]} " =~ " validation " ]]; then
    run_benchmark "proton-beam-core" "validation_bench" "Validation Benchmarks (Basic & Full)"
fi

if [ "$RUN_ALL" = true ] || [[ " ${BENCHMARKS[@]} " =~ " core " ]] || [[ " ${BENCHMARKS[@]} " =~ " storage " ]]; then
    run_benchmark "proton-beam-core" "storage_bench" "Storage Benchmarks (I/O & Compression)"
fi

if [ "$RUN_ALL" = true ] || [[ " ${BENCHMARKS[@]} " =~ " core " ]] || [[ " ${BENCHMARKS[@]} " =~ " builder " ]]; then
    run_benchmark "proton-beam-core" "builder_bench" "Builder Pattern Benchmarks"
fi

if [ "$RUN_ALL" = true ] || [[ " ${BENCHMARKS[@]} " =~ " core " ]] || [[ " ${BENCHMARKS[@]} " =~ " index " ]]; then
    run_benchmark "proton-beam-core" "index_bench" "Index Benchmarks (SQLite Operations)"
fi

# Run CLI Benchmarks
if [ "$RUN_ALL" = true ] || [[ " ${BENCHMARKS[@]} " =~ " cli " ]] || [[ " ${BENCHMARKS[@]} " =~ " pipeline " ]]; then
    run_benchmark "proton-beam-cli" "pipeline_bench" "CLI Pipeline Benchmarks (End-to-End)"
fi

echo ""
echo "╔════════════════════════════════════════════════════════════════╗"
echo "║                    Benchmarks Complete! ✅                      ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""
echo "📊 Benchmark results show current performance characteristics."
echo ""
echo "🎯 Key Performance Indicators:"
echo "   • JSON→Proto conversion: Target >50k events/sec"
echo "   • Basic validation: Target >500k validations/sec"
echo "   • Storage I/O: Target >30 MB/sec throughput"
echo "   • Index operations: Target >50k inserts/sec (batch)"
echo "   • End-to-end pipeline: Target >10k events/sec"
echo ""
echo "💡 Tips for better performance:"
echo "   • Use batch operations when possible"
echo "   • Skip validation (--validate-signatures=false --validate-event-ids=false) when processing trusted data"
echo "   • Use larger batch sizes (1000-5000) for bulk operations"
echo "   • Leverage streaming mode for large files"
echo ""
echo "📝 To save results: $0 --release 2>&1 | tee benchmark-results.txt"
echo ""

