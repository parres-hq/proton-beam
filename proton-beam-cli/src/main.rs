use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use proton_beam_core::{
    ProtoEvent, compute_event_hash, validate_basic_fields, validate_event_id_from_hash,
    validate_signature_from_hash,
};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tracing::{debug, error, info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

// Performance tuning constants
const LINE_COUNT_BUFFER_SIZE: usize = 1024 * 1024; // 1MB for line counting
const LINE_COUNT_READ_BUFFER: usize = 512 * 1024; // 512KB for read buffer
// const FILE_READER_BUFFER_SIZE: usize = 1024 * 1024; // 1MB for file reading
// const STORAGE_WRITER_BUFFER_SIZE: usize = 512 * 1024; // 512KB for writing
const PROGRESS_UPDATE_INTERVAL: u64 = 1000; // Update progress every N lines
const INDEX_BATCH_SIZE: usize = 5000; // Batch size for index operations

fn count_lines(path: &Path) -> Result<u64> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(LINE_COUNT_BUFFER_SIZE, file);
    let mut count = 0u64;
    let mut buffer = [0u8; LINE_COUNT_READ_BUFFER];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        count += bytecount::count(&buffer[..bytes_read], b'\n') as u64;
    }

    Ok(count)
}

mod input;
mod progress;
mod storage;

#[cfg(feature = "s3")]
mod s3;

use input::InputReader;
use storage::{ErrorStats, LogErrorContext, StorageManager};

#[derive(Parser, Debug)]
#[command(name = "proton-beam")]
#[command(about = "Convert Nostr events from JSON to Protocol Buffers", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Convert Nostr events from JSON to protobuf format
    Convert {
        /// Input file path (.jsonl)
        #[arg(value_name = "INPUT")]
        input: PathBuf,

        /// Output directory for protobuf files
        #[arg(short, long, default_value = "./pb_data")]
        output_dir: PathBuf,

        /// Validate Schnorr signatures (default: true)
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        validate_signatures: bool,

        /// Validate event IDs / hashes (default: true)
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        validate_event_ids: bool,

        /// Batch size for writing events
        #[arg(short, long, default_value = "1000")]
        batch_size: usize,

        /// Show detailed progress information
        #[arg(short, long)]
        verbose: bool,

        /// Disable progress bar
        #[arg(long)]
        no_progress: bool,

        /// Number of parallel threads (default: number of CPUs)
        #[arg(short = 'j', long)]
        parallel: Option<usize>,

        /// Prefilter events with invalid kind values (> 65535) before parsing (enabled by default)
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        filter_invalid_kinds: bool,

        /// Disable preprocessing filter for invalid kinds
        #[arg(long, conflicts_with = "filter_invalid_kinds")]
        no_filter_kinds: bool,

        /// Compression level (0-9, default: 6)
        #[arg(long, value_parser = clap::value_parser!(u32).range(0..=9), default_value_t = 6)]
        compression_level: u32,

        /// Upload output files to S3 (format: s3://bucket/prefix)
        #[arg(long)]
        s3_output: Option<String>,
    },

    /// Merge temporary protobuf files from a parallel conversion
    Merge {
        /// Output directory containing the tmp/ subdirectory
        #[arg(value_name = "OUTPUT_DIR")]
        output_dir: PathBuf,

        /// Compression level (0-9, default: 6)
        #[arg(long, value_parser = clap::value_parser!(u32).range(0..=9), default_value_t = 6)]
        compression_level: u32,

        /// Show detailed progress information
        #[arg(short, long)]
        verbose: bool,

        /// Delete temp directory after successful merge
        #[arg(long)]
        cleanup: bool,
    },

    /// Build or rebuild the event index from protobuf files
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },
}

#[derive(Parser, Debug)]
enum IndexAction {
    /// Rebuild the index from existing protobuf files
    Rebuild {
        /// Directory containing protobuf files
        #[arg(value_name = "PB_DIR", default_value = "./pb_data")]
        pb_dir: PathBuf,

        /// Path to SQLite index database (defaults to PB_DIR/index.db)
        #[arg(long)]
        index_path: Option<PathBuf>,

        /// Show detailed progress information
        #[arg(short, long)]
        verbose: bool,

        /// Upload index to S3 (format: s3://bucket/prefix)
        #[arg(long)]
        s3_output: Option<String>,
    },
}

#[derive(Debug)]
struct ConversionStats {
    total_lines: u64,
    valid_events: u64,
    invalid_events: u64,
    skipped_lines: u64,
}

impl ConversionStats {
    fn new() -> Self {
        Self {
            total_lines: 0,
            valid_events: 0,
            invalid_events: 0,
            skipped_lines: 0,
        }
    }

    fn print_summary(&self, error_stats: Option<&ErrorStats>) {
        println!("\n📊 Conversion Summary:");
        println!("  Total lines processed: {}", self.total_lines);
        println!("  ✅ Valid events:       {}", self.valid_events);
        println!("  ❌ Invalid events:     {}", self.invalid_events);
        if self.skipped_lines > 0 {
            println!("  ⏭️  Skipped lines:      {}", self.skipped_lines);
        }

        let success_rate = if self.total_lines > 0 {
            (self.valid_events as f64 / self.total_lines as f64) * 100.0
        } else {
            0.0
        };
        println!("  Success rate:         {:.1}%", success_rate);

        // Print error breakdown if available
        if let Some(stats) = error_stats {
            stats.print_summary();
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Merge {
            output_dir,
            compression_level,
            verbose,
            cleanup,
        } => {
            // Initialize logging
            init_logging(verbose, &output_dir);

            let temp_dir = output_dir.join("tmp");

            if !temp_dir.exists() {
                anyhow::bail!(
                    "Temp directory does not exist: {}\nExpected to find thread_*.pb.gz.tmp files there.",
                    temp_dir.display()
                );
            }

            println!("🔄 Proton Beam - Merge Temporary Files");
            println!("   Output: {}", output_dir.display());
            println!("   Temp dir: {}", temp_dir.display());
            println!("   Compression: {}", compression_level);
            println!();

            info!("Starting merge process...");
            info!("Output directory: {}", output_dir.display());
            info!("Temp directory: {}", temp_dir.display());

            // Merge temporary files
            merge_temp_files(&output_dir, &temp_dir, compression_level)?;

            info!("Merge complete!");
            println!("\n✅ Merge complete!");

            // Cleanup if requested
            if cleanup {
                println!("🧹 Cleaning up temp directory...");
                info!("Removing temp directory: {}", temp_dir.display());
                std::fs::remove_dir_all(&temp_dir).context("Failed to remove temp directory")?;
                println!("✅ Temp directory removed");
            } else {
                println!(
                    "\n💡 Tip: Run with --cleanup to remove temp files after successful merge"
                );
            }
        }

        Commands::Convert {
            input,
            output_dir,
            validate_signatures,
            validate_event_ids,
            batch_size,
            verbose,
            no_progress,
            parallel,
            filter_invalid_kinds,
            no_filter_kinds,
            compression_level,
            s3_output,
        } => {
            // Apply no_filter_kinds flag
            let filter_invalid_kinds = filter_invalid_kinds && !no_filter_kinds;
            // Create output directory first (needed for log file)
            std::fs::create_dir_all(&output_dir).context("Failed to create output directory")?;

            // Initialize logging (creates log file in output_dir)
            init_logging(verbose, &output_dir);

            // Check available disk space (warn if low)
            let file_size = std::fs::metadata(&input).map(|m| m.len()).unwrap_or(0);
            if file_size > 0 {
                // Estimate output size (conservative: 30-50% of input depending on compression)
                let estimated_output = file_size / 2;
                info!(
                    "Input file size: {:.2} GB, estimated output: {:.2} GB",
                    file_size as f64 / 1_000_000_000.0,
                    estimated_output as f64 / 1_000_000_000.0
                );

                // Note: Checking available space is platform-specific and requires additional deps
                // For now, we log the estimates so users can verify manually
            }

            // Determine number of threads
            let num_threads = parallel.unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(1)
            });

            // Log to file
            info!("Starting Proton Beam - Conversion");
            info!("Input: {}", input.display());
            info!("Output directory: {}", output_dir.display());
            info!(
                "Validation (signatures): {}",
                if validate_signatures {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            info!(
                "Validation (event IDs): {}",
                if validate_event_ids {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            info!("Batch size: {}", batch_size);
            info!("Parallel threads: {}", num_threads);
            info!(
                "Kind filtering: {}",
                if filter_invalid_kinds {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            info!("Compression level: {}", compression_level);

            // Print clean startup message to stdout
            if !no_progress {
                println!("🚀 Proton Beam - Converting Nostr events to Protobuf");
                println!("   Input: {}", input.display());
                println!("   Output: {}", output_dir.display());
                println!("   Threads: {}", num_threads);
                println!();
            }

            // Run conversion (always parallel if num_threads > 1)
            if num_threads > 1 {
                convert_events_parallel(
                    &input,
                    &output_dir,
                    validate_signatures,
                    validate_event_ids,
                    batch_size,
                    !no_progress,
                    num_threads,
                    filter_invalid_kinds,
                    compression_level,
                )?;
            } else {
                convert_events(
                    &input,
                    &output_dir,
                    validate_signatures,
                    validate_event_ids,
                    batch_size,
                    !no_progress,
                    filter_invalid_kinds,
                    compression_level,
                )?;
            }

            // Upload to S3 if requested
            #[cfg(feature = "s3")]
            if let Some(s3_uri) = s3_output {
                println!("\n☁️  Uploading to S3...");
                info!("Starting S3 upload to: {}", s3_uri);

                let (bucket, prefix) = s3::parse_s3_uri(&s3_uri)?;
                let uploader = s3::S3Uploader::new(bucket, prefix).await?;
                uploader.upload_all(&output_dir).await?;

                println!("✅ Upload to S3 complete!");
            }

            #[cfg(not(feature = "s3"))]
            if s3_output.is_some() {
                eprintln!("⚠️  Warning: S3 upload requested but S3 feature not enabled.");
                eprintln!("   Rebuild with: cargo build --release --features s3");
            }
        }

        Commands::Index { action } => match action {
            IndexAction::Rebuild {
                pb_dir,
                index_path,
                verbose,
                s3_output,
            } => {
                // Initialize logging
                init_logging(verbose, &pb_dir);

                // Determine index path
                let index_path = index_path.unwrap_or_else(|| pb_dir.join("index.db"));

                info!("Starting Proton Beam - Index Rebuild");
                info!("Protobuf directory: {}", pb_dir.display());
                info!("Index database: {}", index_path.display());

                println!("🔍 Proton Beam - Rebuilding Event Index");
                println!("   Source: {}", pb_dir.display());
                println!("   Index: {}", index_path.display());
                println!();

                rebuild_index(&pb_dir, &index_path)?;

                // Upload to S3 if requested
                #[cfg(feature = "s3")]
                if let Some(s3_uri) = s3_output {
                    println!("\n☁️  Uploading to S3...");
                    info!("Starting S3 upload to: {}", s3_uri);

                    let (bucket, prefix) = s3::parse_s3_uri(&s3_uri)?;
                    let uploader = s3::S3Uploader::new(bucket, prefix).await?;

                    // Upload index and protobuf files
                    uploader.upload_all(&pb_dir).await?;

                    println!("✅ Upload to S3 complete!");
                }

                #[cfg(not(feature = "s3"))]
                if s3_output.is_some() {
                    eprintln!("⚠️  Warning: S3 upload requested but S3 feature not enabled.");
                    eprintln!("   Rebuild with: cargo build --release --features s3");
                }
            }
        },
    }

    Ok(())
}

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

fn init_logging(verbose: bool, output_dir: &Path) {
    use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*};

    // Ensure output directory exists for log files
    if let Err(e) = fs::create_dir_all(output_dir) {
        eprintln!(
            "⚠️  Failed to create log directory {}: {}",
            output_dir.display(),
            e
        );
        // Continue without file logging (stderr layer will still initialize)
    }

    // Determine log level for file output
    let file_filter = if verbose {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    // Configure rolling file appender (daily rotation, keep 14 files)
    let (file_layer, guard) = if let Ok(appender) = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("proton-beam")
        .filename_suffix("log")
        .max_log_files(14)
        .build(output_dir)
    {
        let (writer, guard) = tracing_appender::non_blocking(appender);
        let layer = fmt::layer()
            .with_writer(writer)
            .event_format(
                fmt::format()
                    .with_timer(fmt::time::SystemTime)
                    .with_level(true)
                    .with_target(false)
                    .with_thread_ids(true)
                    .with_thread_names(true),
            )
            .with_filter(file_filter);
        (Some(layer), Some(guard))
    } else {
        (None, None)
    };

    if let Some(guard) = guard {
        let _ = LOG_GUARD.set(guard);
    }

    let stderr_layer = || {
        fmt::layer()
            .with_writer(std::io::stderr)
            .event_format(
                fmt::format()
                    .with_timer(fmt::time::SystemTime)
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .compact(),
            )
            .with_filter(LevelFilter::WARN)
    };

    // It's possible init_logging is called multiple times; ignore if already set
    match file_layer {
        Some(layer) => {
            let _ = tracing_subscriber::registry()
                .with(stderr_layer())
                .with(layer)
                .try_init();
        }
        None => {
            let _ = tracing_subscriber::registry()
                .with(stderr_layer())
                .try_init();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn convert_events(
    input: &Path,
    output_dir: &Path,
    validate_signatures: bool,
    validate_event_ids: bool,
    batch_size: usize,
    show_progress: bool,
    filter_invalid_kinds: bool,
    compression_level: u32,
) -> Result<()> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    // Initialize storage manager
    let mut storage = StorageManager::new(output_dir, batch_size, compression_level)?;

    // Initialize input reader with preprocessing options
    let mut reader = InputReader::with_options(input.to_str().unwrap(), filter_invalid_kinds)?;

    // Count total lines for progress bar
    let total_lines = if show_progress {
        count_lines(input).unwrap_or(0)
    } else {
        0
    };

    // Set up progress bar
    let progress = if show_progress && total_lines > 0 {
        let pb = ProgressBar::new(total_lines);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                .unwrap()
                .progress_chars("█▓▒░ ")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        Some(pb)
    } else if show_progress {
        // Fallback to spinner if we can't count lines
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        Some(pb)
    } else {
        None
    };

    let mut stats = ConversionStats::new();

    // Process each line
    for (line_num, line_result) in reader.by_ref().enumerate() {
        stats.total_lines += 1;

        let line = match line_result {
            Ok(line) => line,
            Err(e) => {
                error!("Failed to read line {}: {}", line_num + 1, e);
                stats.skipped_lines += 1;
                continue;
            }
        };

        // Skip empty lines
        if line.trim().is_empty() {
            stats.skipped_lines += 1;
            continue;
        }

        // Update progress
        if let Some(ref pb) = progress {
            pb.set_position(stats.total_lines);
            pb.set_message(format!(
                "Valid: {} | Errors: {}",
                stats.valid_events, stats.invalid_events
            ));
        }

        // Parse JSON to ProtoEvent
        let event = match ProtoEvent::try_from(line.as_str()) {
            Ok(event) => event,
            Err(e) => {
                storage.log_error(
                    LogErrorContext::from_line((line_num + 1) as u64),
                    &format!("parse_error: {}", e),
                    None,
                );
                stats.invalid_events += 1;
                continue;
            }
        };

        // Validate basic fields first (fast check)
        if let Err(e) = validate_basic_fields(&event) {
            storage.log_error(
                LogErrorContext::from_line((line_num + 1) as u64),
                &format!("validation_error: {}", e),
                Some(&event.id),
            );
            stats.invalid_events += 1;
            continue;
        }

        // Compute hash once and reuse for both validations if needed
        if validate_signatures || validate_event_ids {
            let hash = match compute_event_hash(&event) {
                Ok(h) => h,
                Err(e) => {
                    storage.log_error(
                        LogErrorContext::from_line((line_num + 1) as u64),
                        &format!("hash_error: {}", e),
                        Some(&event.id),
                    );
                    stats.invalid_events += 1;
                    continue;
                }
            };

            if validate_event_ids && let Err(e) = validate_event_id_from_hash(&event, &hash) {
                storage.log_error(
                    LogErrorContext::from_line((line_num + 1) as u64),
                    &format!("validation_error: {}", e),
                    Some(&event.id),
                );
                stats.invalid_events += 1;
                continue;
            }

            if validate_signatures && let Err(e) = validate_signature_from_hash(&event, &hash) {
                storage.log_error(
                    LogErrorContext::from_line((line_num + 1) as u64),
                    &format!("validation_error: {}", e),
                    Some(&event.id),
                );
                stats.invalid_events += 1;
                continue;
            }
        }

        // Store the event
        match storage.store_event(event) {
            Ok(_) => {
                stats.valid_events += 1;
                debug!("Successfully stored event from line {}", line_num + 1);
            }
            Err(e) => {
                error!("Failed to store event from line {}: {}", line_num + 1, e);
                storage.log_error(
                    LogErrorContext::from_line((line_num + 1) as u64),
                    &format!("storage_error: {}", e),
                    None,
                );
                stats.invalid_events += 1;
            }
        }
    }

    // Flush any remaining events
    storage.flush()?;

    // Get filtered count from reader
    let filtered_count = reader.filtered_count();

    // Clean up progress bar
    if let Some(pb) = progress {
        pb.finish_with_message(format!(
            "Complete! Processed: {} | Valid: {} | Errors: {}{}",
            stats.total_lines,
            stats.valid_events,
            stats.invalid_events,
            if filtered_count > 0 {
                format!(" | Filtered: {}", filtered_count)
            } else {
                String::new()
            }
        ));
    }

    // Get error statistics from storage manager
    let error_stats = storage.error_stats();

    info!("Conversion complete");
    if filtered_count > 0 {
        info!(
            "Pre-filtered {} events with invalid kind values",
            filtered_count
        );
    }
    stats.print_summary(Some(error_stats));

    // Exit code: 0 if any events succeeded, 1 if all failed
    if stats.valid_events == 0 && stats.total_lines > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Parallel version of convert_events using file chunking
#[allow(clippy::too_many_arguments)]
fn convert_events_parallel(
    input: &Path,
    output_dir: &Path,
    validate_signatures: bool,
    validate_event_ids: bool,
    batch_size: usize,
    show_progress: bool,
    num_threads: usize,
    filter_invalid_kinds: bool,
    compression_level: u32,
) -> Result<()> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    // Create temp directory for parallel writes
    let temp_dir = output_dir.join("tmp");
    std::fs::create_dir_all(&temp_dir).context("Failed to create temp directory")?;

    // Shared atomic counters for statistics (lock-free)
    let total_lines = Arc::new(AtomicU64::new(0));
    let valid_events = Arc::new(AtomicU64::new(0));
    let invalid_events = Arc::new(AtomicU64::new(0));
    let skipped_lines = Arc::new(AtomicU64::new(0));
    let bytes_processed = Arc::new(AtomicU64::new(0));

    // Get file size for progress bar
    let file_size = std::fs::metadata(input)?.len();

    // Find chunk boundaries
    info!(
        "Calculating chunk boundaries for {} threads...",
        num_threads
    );
    let chunks = find_chunk_boundaries(input, num_threads)?;
    info!("Processing {} chunks in parallel", chunks.len());

    // Progress bar (track by bytes processed for parallel mode)
    let progress = if show_progress {
        let pb = ProgressBar::new(file_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%) {msg}")
                .unwrap()
                .progress_chars("█▓▒░ ")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        Some(Arc::new(pb))
    } else {
        None
    };

    // Track both errors and which chunks failed (for better reporting)
    let parallel_errors: Arc<Mutex<Vec<(usize, anyhow::Error)>>> = Arc::new(Mutex::new(Vec::new()));
    let error_stats_list: Arc<Mutex<Vec<ErrorStats>>> = Arc::new(Mutex::new(Vec::new()));

    rayon::scope(|scope| {
        for (thread_id, (start, end)) in chunks.into_iter().enumerate() {
            let input = input.to_path_buf();
            let temp_dir = temp_dir.clone();
            let total_lines = Arc::clone(&total_lines);
            let valid_events = Arc::clone(&valid_events);
            let invalid_events = Arc::clone(&invalid_events);
            let skipped_lines = Arc::clone(&skipped_lines);
            let bytes_processed = Arc::clone(&bytes_processed);
            let progress = progress.as_ref().map(Arc::clone);
            let errors = Arc::clone(&parallel_errors);
            let error_stats_list = Arc::clone(&error_stats_list);

            scope.spawn(move |_| {
                match process_chunk(
                    thread_id,
                    &input,
                    start,
                    end,
                    temp_dir.as_path(),
                    total_lines,
                    valid_events,
                    invalid_events,
                    skipped_lines,
                    bytes_processed,
                    progress,
                    validate_signatures,
                    validate_event_ids,
                    batch_size,
                    filter_invalid_kinds,
                    compression_level,
                ) {
                    Ok(stats) => {
                        // Collect error stats from this thread
                        error_stats_list.lock().unwrap().push(stats);
                    }
                    Err(e) => {
                        error!(
                            "Thread {} (bytes {}-{}) error: {:?}",
                            thread_id, start, end, e
                        );
                        errors.lock().unwrap().push((thread_id, e));
                    }
                }
            });
        }
    });

    let errors = Arc::try_unwrap(parallel_errors)
        .unwrap()
        .into_inner()
        .unwrap();
    if !errors.is_empty() {
        // Log all errors for debugging
        eprintln!(
            "\n⚠️  WARNING: {} thread(s) failed during parallel processing:",
            errors.len()
        );
        eprintln!("   Partial data from these threads has been saved to temp files.");
        eprintln!("   However, events after the error point in each failed chunk are LOST.\n");

        for (thread_id, e) in &errors {
            error!("Thread {} failed: {:?}", thread_id, e);
            eprintln!("   Thread {}: {}", thread_id, e);
        }

        eprintln!("\n📝 Recovery options:");
        eprintln!(
            "   1. Use 'proton-beam merge {}' to salvage successfully processed data",
            output_dir.display()
        );
        eprintln!("      (Note: You will be missing data from the failed chunks)");
        eprintln!("   2. Fix the underlying issue and re-run the full conversion");
        eprintln!("      (Recommended for complete data integrity)\n");

        return Err(anyhow::anyhow!(
            "Parallel processing failed: {}/{} chunks encountered errors. See above for details.",
            errors.len(),
            num_threads
        ));
    }

    // Clean up progress bar
    if let Some(pb) = progress {
        pb.finish_with_message("Merging temporary files...");
    }

    info!("All chunks processed, merging temporary files...");

    // Merge temporary files
    if let Err(e) = merge_temp_files(output_dir, &temp_dir, compression_level) {
        error!("Failed to merge temp files: {:?}", e);
        return Err(e).context("Failed to merge temporary files");
    }

    // Clean up temp directory
    std::fs::remove_dir_all(&temp_dir).context("Failed to remove temp directory")?;

    info!("Merge complete");

    // Merge error statistics from all threads
    let error_stats_list = Arc::try_unwrap(error_stats_list)
        .unwrap()
        .into_inner()
        .unwrap();
    let mut merged_error_stats = ErrorStats::new();
    for stats in error_stats_list {
        merged_error_stats.merge(&stats);
    }

    // Print summary using atomic values
    let final_stats = ConversionStats {
        total_lines: total_lines.load(Ordering::Relaxed),
        valid_events: valid_events.load(Ordering::Relaxed),
        invalid_events: invalid_events.load(Ordering::Relaxed),
        skipped_lines: skipped_lines.load(Ordering::Relaxed),
    };
    final_stats.print_summary(Some(&merged_error_stats));

    // Exit code: 0 if any events succeeded, 1 if all failed
    if final_stats.valid_events == 0 && final_stats.total_lines > 0 {
        return Err(anyhow::anyhow!(
            "Conversion failed: no valid events processed"
        ));
    }

    Ok(())
}

/// Find chunk boundaries aligned to line breaks
fn find_chunk_boundaries(path: &Path, num_chunks: usize) -> Result<Vec<(u64, u64)>> {
    let file_size = std::fs::metadata(path)?.len();

    if file_size == 0 {
        return Ok(vec![]);
    }

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let chunk_size = file_size / num_chunks as u64;
    let mut boundaries = vec![];
    let mut current_start = 0u64;

    for i in 0..num_chunks {
        if i == num_chunks - 1 {
            // Last chunk goes to EOF
            boundaries.push((current_start, file_size));
            break;
        }

        // Seek to approximate boundary
        let target = chunk_size * (i + 1) as u64;
        reader.seek(SeekFrom::Start(target))?;

        // Read bytes until we find a newline, handling invalid UTF-8
        // We read raw bytes to avoid UTF-8 validation errors when seeking to arbitrary positions
        let mut byte_buf = vec![0u8; 8192]; // 8KB buffer
        let bytes_read = reader.read(&mut byte_buf)?;

        // Find the first newline in the buffer
        let newline_pos = byte_buf[..bytes_read]
            .iter()
            .position(|&b| b == b'\n')
            .unwrap_or(bytes_read - 1);

        // Seek to just after the newline
        let chunk_end = target + newline_pos as u64 + 1;
        reader.seek(SeekFrom::Start(chunk_end))?;

        boundaries.push((current_start, chunk_end));
        current_start = chunk_end; // Next chunk starts after this line
    }

    Ok(boundaries)
}

/// Process a single chunk of the input file
///
/// # Error Handling
///
/// If this function returns an error:
/// - Events processed BEFORE the error are saved to the temp file (via StorageManager::Drop)
/// - Events after the error point in this chunk are LOST
/// - Other threads continue processing their chunks
/// - The overall parallel conversion will fail, but temp files are preserved for recovery
///
/// Common failure scenarios:
/// - I/O errors reading from input file (disk issues, NFS timeouts)
/// - Corrupted/malformed JSON that crashes the parser
/// - Disk full while writing temp file
///
/// Returns the error statistics collected during processing.
#[allow(clippy::too_many_arguments)]
fn process_chunk(
    thread_id: usize,
    input_path: &Path,
    start: u64,
    end: u64,
    temp_dir: &Path,
    total_lines: Arc<AtomicU64>,
    valid_events: Arc<AtomicU64>,
    invalid_events: Arc<AtomicU64>,
    skipped_lines: Arc<AtomicU64>,
    bytes_processed: Arc<AtomicU64>,
    progress: Option<Arc<ProgressBar>>,
    validate_signatures: bool,
    validate_event_ids: bool,
    batch_size: usize,
    filter_invalid_kinds: bool,
    compression_level: u32,
) -> Result<ErrorStats> {
    // Open the file and seek to start position
    let file = File::open(input_path)?;
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(start))?;

    // Thread-local state
    let mut storage =
        StorageManager::new_with_prefix(temp_dir, batch_size, thread_id, compression_level)?;

    // Local stats for this chunk (for logging only)
    let mut local_total = 0u64;
    let mut local_valid = 0u64;
    let mut local_invalid = 0u64;
    let mut filtered_count = 0usize;

    let mut position = start;
    let mut line_num = 0u64;

    while position < end {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;

        if bytes_read == 0 {
            break; // EOF
        }

        position += bytes_read as u64;
        line_num += 1;
        local_total += 1;

        // Update atomic counters
        total_lines.fetch_add(1, Ordering::Relaxed);
        bytes_processed.fetch_add(bytes_read as u64, Ordering::Relaxed);

        // Skip empty lines
        if line.trim().is_empty() {
            skipped_lines.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        // Pre-filter invalid kinds if enabled
        if filter_invalid_kinds && !InputReader::has_valid_kind(&line) {
            filtered_count += 1;
            continue;
        }

        // Update progress periodically (every PROGRESS_UPDATE_INTERVAL lines)
        if line_num.is_multiple_of(PROGRESS_UPDATE_INTERVAL)
            && let Some(ref pb) = progress
        {
            let current_bytes = bytes_processed.load(Ordering::Relaxed);
            let current_lines = total_lines.load(Ordering::Relaxed);
            let current_valid = valid_events.load(Ordering::Relaxed);
            let current_invalid = invalid_events.load(Ordering::Relaxed);

            pb.set_position(current_bytes);
            pb.set_message(format!(
                "Lines: {} | Valid: {} | Errors: {}",
                current_lines, current_valid, current_invalid
            ));
        }

        // Parse JSON to ProtoEvent
        let event = match ProtoEvent::try_from(line.as_str()) {
            Ok(event) => event,
            Err(e) => {
                storage.log_error(
                    LogErrorContext::new(line_num, thread_id)
                        .with_chunk_offset(start)
                        .with_bytes_read(position - start),
                    &format!("parse_error: {}", e),
                    None,
                );
                local_invalid += 1;
                invalid_events.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        // Validate basic fields first (fast check)
        if let Err(e) = validate_basic_fields(&event) {
            storage.log_error(
                LogErrorContext::new(line_num, thread_id)
                    .with_chunk_offset(start)
                    .with_bytes_read(position - start),
                &format!("validation_error: {}", e),
                Some(&event.id),
            );
            local_invalid += 1;
            invalid_events.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        // Compute hash once and reuse for both validations if needed
        if validate_signatures || validate_event_ids {
            let hash = match compute_event_hash(&event) {
                Ok(h) => h,
                Err(e) => {
                    storage.log_error(
                        LogErrorContext::new(line_num, thread_id)
                            .with_chunk_offset(start)
                            .with_bytes_read(position - start),
                        &format!("hash_error: {}", e),
                        Some(&event.id),
                    );
                    local_invalid += 1;
                    invalid_events.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
            };

            if validate_event_ids && let Err(e) = validate_event_id_from_hash(&event, &hash) {
                storage.log_error(
                    LogErrorContext::new(line_num, thread_id)
                        .with_chunk_offset(start)
                        .with_bytes_read(position - start),
                    &format!("validation_error: {}", e),
                    Some(&event.id),
                );
                local_invalid += 1;
                invalid_events.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            if validate_signatures && let Err(e) = validate_signature_from_hash(&event, &hash) {
                storage.log_error(
                    LogErrorContext::new(line_num, thread_id)
                        .with_chunk_offset(start)
                        .with_bytes_read(position - start),
                    &format!("validation_error: {}", e),
                    Some(&event.id),
                );
                local_invalid += 1;
                invalid_events.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        }

        // Store the event
        match storage.store_event(event) {
            Ok(_) => {
                local_valid += 1;
                valid_events.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                error!(
                    "Thread {}: Failed to store event from line {}: {}",
                    thread_id, line_num, e
                );
                storage.log_error(
                    LogErrorContext::new(line_num, thread_id)
                        .with_chunk_offset(start)
                        .with_bytes_read(position - start),
                    &format!("storage_error: {}", e),
                    None,
                );
                local_invalid += 1;
                invalid_events.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    // Flush any remaining events
    storage.flush()?;

    info!(
        "Thread {} completed: {} lines, {} valid, {} errors{}",
        thread_id,
        local_total,
        local_valid,
        local_invalid,
        if filtered_count > 0 {
            format!(", {} filtered", filtered_count)
        } else {
            String::new()
        }
    );

    // Return error statistics from this thread
    Ok(storage.clone_error_stats())
}

/// Merge temporary files into final date-organized files
fn merge_temp_files(output_dir: &Path, temp_dir: &Path, compression_level: u32) -> Result<()> {
    // Group temp files by date
    let mut files_by_date: HashMap<String, Vec<PathBuf>> = HashMap::new();

    // List all files in temp directory for debugging
    let temp_files: Vec<PathBuf> = std::fs::read_dir(temp_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();

    info!("Found {} files in temp directory", temp_files.len());

    for path in temp_files {
        if path.extension().and_then(|s| s.to_str()) == Some("tmp") {
            match extract_date_from_temp_filename(&path) {
                Some(date) => {
                    debug!("Grouping temp file: {} -> date: {}", path.display(), date);
                    files_by_date.entry(date).or_default().push(path);
                }
                None => {
                    error!(
                        "Failed to extract date from temp filename: {}",
                        path.display()
                    );
                }
            }
        } else {
            debug!("Skipping non-tmp file: {}", path.display());
        }
    }

    if files_by_date.is_empty() {
        info!("No temp files to merge (no events were processed)");
        println!("⚠️  No temp files found - no events were processed");
        return Ok(());
    }

    info!("Merging {} dates...", files_by_date.len());

    // Merge each date's files
    for (date, temp_files) in files_by_date {
        info!("Merging {} files for date: {}", temp_files.len(), date);
        println!(
            "📦 Merging {} temp files for date: {}",
            temp_files.len(),
            date
        );

        match merge_protobuf_files_with_dedup(&temp_files, output_dir, &date, compression_level) {
            Ok(stats) => {
                info!(
                    "Merge summary for {}: {} events, {} duplicates, {} corrupted skipped",
                    date, stats.written_events, stats.duplicates, stats.corrupted
                );
                println!(
                    "   ✅ {} (events: {}, dupes: {}, corrupt: {})",
                    date, stats.written_events, stats.duplicates, stats.corrupted
                );
            }
            Err(e) => {
                error!("Failed to merge files for date {}: {:?}", date, e);
                println!(
                    "   ❌ Failed to merge {} (see log for details: {})",
                    date,
                    output_dir.join("proton-beam.log").display()
                );
                continue;
            }
        }
    }

    Ok(())
}

/// Extract date string from temp filename
/// Format: thread_{id}_{date}.pb.gz.tmp
fn extract_date_from_temp_filename(path: &Path) -> Option<String> {
    let filename = path.file_name()?.to_str()?;

    // Remove .tmp extension
    let without_tmp = filename.strip_suffix(".tmp")?;

    // Remove .pb.gz extension
    let without_pb_gz = without_tmp.strip_suffix(".pb.gz")?;

    // Split by underscore: thread_{id}_{date}
    let parts: Vec<&str> = without_pb_gz.split('_').collect();

    // We need at least ["thread", "{id}", "{year}", "{month}", "{day}"]
    if parts.len() >= 5 && parts[0] == "thread" {
        // Reconstruct date: {year}_{month}_{day}
        let date = format!("{}_{}_{}", parts[2], parts[3], parts[4]);
        Some(date)
    } else {
        None
    }
}

/// Merge multiple protobuf files with deduplication
struct MergeStats {
    written_events: u64,
    duplicates: u64,
    corrupted: u64,
}

fn merge_protobuf_files_with_dedup(
    sources: &[PathBuf],
    output_dir: &Path,
    date_str: &str,
    compression_level: u32,
) -> Result<MergeStats> {
    use proton_beam_core::{
        create_gzip_decoder, create_gzip_encoder_with_level, read_events_delimited,
        write_event_delimited,
    };
    use std::io::BufWriter;

    let final_file = output_dir.join(format!("{}.pb.gz", date_str));
    let temp_output = output_dir.join(format!("{}.pb.gz.tmp", date_str));

    debug!(
        "Merging {} source files into {}",
        sources.len(),
        final_file.display()
    );

    // If final file already exists, we need to include it in the merge
    let mut all_sources = sources.to_vec();
    if final_file.exists() {
        debug!(
            "Including existing final file in merge: {}",
            final_file.display()
        );
        all_sources.push(final_file.clone());
    }

    let output_file = File::create(&temp_output).context(format!(
        "Failed to create temp output file: {}",
        temp_output.display()
    ))?;
    let gz = create_gzip_encoder_with_level(output_file, compression_level);
    let mut writer = BufWriter::new(gz);

    // Deduplicate during merge (streaming)
    let mut seen_ids = HashSet::new();
    let mut event_count = 0u64;
    let mut duplicate_count = 0u64;
    let mut corrupted_events = 0u64;
    let mut source_errors = 0u64;

    for (idx, source) in all_sources.iter().enumerate() {
        debug!(
            "Processing source file {}/{}: {}",
            idx + 1,
            all_sources.len(),
            source.display()
        );

        let file = match File::open(source) {
            Ok(f) => f,
            Err(e) => {
                source_errors += 1;
                error!(
                    "Failed to open source {} (skipping): {}",
                    source.display(),
                    e
                );
                continue;
            }
        };
        let gz = create_gzip_decoder(file);

        let mut source_events = 0;
        for (event_idx, event_result) in read_events_delimited(gz).enumerate() {
            // IMPROVED: Handle corrupted events gracefully - continue merge instead of failing
            let event = match event_result {
                Ok(e) => e,
                Err(e) => {
                    corrupted_events += 1;
                    error!(
                        "Corrupted event {} in {} (skipping): {}",
                        event_idx + 1,
                        source.display(),
                        e
                    );
                    continue;
                }
            };

            if !seen_ids.insert(event.id.clone()) {
                duplicate_count += 1;
                continue;
            }

            write_event_delimited(&mut writer, &event).context(format!(
                "Failed to write event {} to output file: {}",
                event_count + 1,
                temp_output.display()
            ))?;
            event_count += 1;
            source_events += 1;
        }
        debug!(
            "Processed {} events from {}",
            source_events,
            source.display()
        );
    }

    writer.flush().context("Failed to flush writer")?;
    drop(writer);

    debug!(
        "Renaming {} to {}",
        temp_output.display(),
        final_file.display()
    );
    std::fs::rename(&temp_output, &final_file).context(format!(
        "Failed to rename {} to {}",
        temp_output.display(),
        final_file.display()
    ))?;

    // Log merge summary with all relevant stats
    if corrupted_events > 0 {
        println!(
            "⚠️  Warning: Skipped {} corrupted events during merge",
            corrupted_events
        );
    }
    if source_errors > 0 {
        warn!(
            "{} source files could not be opened and were skipped",
            source_errors
        );
        println!(
            "⚠️  Warning: {} source files failed to open and were skipped",
            source_errors
        );
    }

    Ok(MergeStats {
        written_events: event_count,
        duplicates: duplicate_count,
        corrupted: corrupted_events,
    })
}

/// Rebuild the event index from existing protobuf files
fn rebuild_index(pb_dir: &Path, index_path: &Path) -> Result<()> {
    use proton_beam_core::{EventIndex, create_gzip_decoder, read_events_delimited};
    use std::time::Instant;

    // Verify pb_dir exists
    if !pb_dir.exists() {
        anyhow::bail!("Protobuf directory does not exist: {}", pb_dir.display());
    }

    // Create or open the index (will truncate if exists)
    info!("Creating new index at: {}", index_path.display());

    // Delete existing index if it exists
    if index_path.exists() {
        std::fs::remove_file(index_path).context("Failed to remove existing index file")?;
        info!("Removed existing index");
    }

    // Use bulk mode for significantly faster initial index building
    let mut index =
        EventIndex::new_bulk_mode(index_path).context("Failed to create event index")?;
    info!("Using bulk insert mode with optimized SQLite settings");

    // Find all .pb.gz files in the directory
    let mut pb_files: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(pb_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file()
            && let Some(extension) = path.extension()
            && extension == "gz"
            && path.to_str().unwrap_or("").ends_with(".pb.gz")
        {
            pb_files.push(path);
        }
    }

    if pb_files.is_empty() {
        println!("⚠️  No protobuf files found in {}", pb_dir.display());
        return Ok(());
    }

    // Sort files for consistent ordering
    pb_files.sort();

    println!("📁 Found {} protobuf files", pb_files.len());
    println!();

    let start_time = Instant::now();
    let mut total_events = 0u64;
    let mut total_duplicates = 0u64;

    // Set up progress bar
    let progress = ProgressBar::new(pb_files.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files | {msg}")
            .unwrap()
            .progress_chars("█▓▒░ ")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    progress.enable_steady_tick(Duration::from_millis(100));

    // Process each file
    for (file_idx, pb_file) in pb_files.iter().enumerate() {
        let file_name = pb_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        progress.set_position(file_idx as u64);
        progress.set_message(format!(
            "Events: {} | Dupes: {}",
            total_events, total_duplicates
        ));

        // Open and decompress the file
        let file = File::open(pb_file).context(format!("Failed to open {}", file_name))?;
        let gz = create_gzip_decoder(file);

        // Stream events instead of loading all into memory
        let mut file_events = 0;
        let mut batch: Vec<(ProtoEvent, &str)> = Vec::with_capacity(INDEX_BATCH_SIZE);

        for (event_idx, event_result) in read_events_delimited(gz).enumerate() {
            let event = match event_result {
                Ok(ev) => ev,
                Err(e) => {
                    warn!(
                        "Corrupted event {} in {} during indexing: {}",
                        event_idx + 1,
                        file_name,
                        e
                    );
                    continue;
                }
            };
            batch.push((event, file_name));
            file_events += 1;

            // Insert in batches for performance
            if batch.len() >= INDEX_BATCH_SIZE {
                let batch_refs: Vec<_> = batch.iter().map(|(e, f)| (e, *f)).collect();

                // Count how many were actually inserted (duplicates are ignored)
                let (inserted, duplicates_in_batch) = index
                    .insert_batch(&batch_refs)
                    .context("Failed to insert batch into index")?;

                total_events += inserted as u64;
                total_duplicates += duplicates_in_batch as u64;
                batch.clear();
            }
        }

        // Insert remaining events
        if !batch.is_empty() {
            let batch_refs: Vec<_> = batch.iter().map(|(e, f)| (e, *f)).collect();

            let (inserted, duplicates_in_batch) = index
                .insert_batch(&batch_refs)
                .context("Failed to insert final batch into index")?;

            total_events += inserted as u64;
            total_duplicates += duplicates_in_batch as u64;
        }

        debug!("Indexed {} events from {}", file_events, file_name);
    }

    progress.finish_with_message(format!(
        "Complete! Indexed {} events ({} duplicates skipped)",
        total_events, total_duplicates
    ));

    let elapsed = start_time.elapsed();
    let events_per_sec = total_events as f64 / elapsed.as_secs_f64();

    // Finalize bulk mode (re-enable safety, run ANALYZE)
    info!("Finalizing index (running ANALYZE for query optimization)...");
    println!("\n🔧 Finalizing index...");
    index.finalize_bulk_mode()?;

    println!("\n✅ Index Rebuild Complete");
    println!("  Indexed events:      {}", total_events);
    println!("  Duplicates skipped:  {}", total_duplicates);
    println!("  Time elapsed:        {:.2}s", elapsed.as_secs_f64());
    println!("  Throughput:          {:.0} events/sec", events_per_sec);

    // Print index stats
    let stats = index.stats()?;
    println!("\n📊 Index Statistics:");
    println!("  Total events:        {}", stats.total_events);
    println!("  Unique files:        {}", stats.unique_files);
    println!("  Unique pubkeys:      {}", stats.unique_pubkeys);

    info!("Index rebuild complete: {} events indexed", total_events);

    Ok(())
}
