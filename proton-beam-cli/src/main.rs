use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use proton_beam_core::{
    ProtoEvent, compute_event_hash, validate_basic_fields, validate_event_id_from_hash,
    validate_signature_from_hash,
};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, error, info};

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
use storage::StorageManager;

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

    fn print_summary(&self) {
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
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
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

fn init_logging(verbose: bool, output_dir: &Path) {
    use std::fs::OpenOptions;
    use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*};

    // Create log file in output directory
    let log_path = output_dir.join("proton-beam.log");
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("Failed to open log file");

    // Determine log level for file
    let file_filter = if verbose {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    // Create a formatter for the log file (all logs go here)
    let file_layer = fmt::layer()
        .with_writer(std::sync::Arc::new(log_file))
        .with_ansi(false)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_line_number(false)
        .with_file(false)
        .compact()
        .with_filter(file_filter);

    // Initialize subscriber with only file layer (no stderr output)
    tracing_subscriber::registry().with(file_layer).init();
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
                storage.log_error((line_num + 1) as u64, &format!("parse_error: {}", e), None);
                stats.invalid_events += 1;
                continue;
            }
        };

        // Validate basic fields first (fast check)
        if let Err(e) = validate_basic_fields(&event) {
            storage.log_error(
                (line_num + 1) as u64,
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
                        (line_num + 1) as u64,
                        &format!("hash_error: {}", e),
                        Some(&event.id),
                    );
                    stats.invalid_events += 1;
                    continue;
                }
            };

            if validate_event_ids && let Err(e) = validate_event_id_from_hash(&event, &hash) {
                storage.log_error(
                    (line_num + 1) as u64,
                    &format!("validation_error: {}", e),
                    Some(&event.id),
                );
                stats.invalid_events += 1;
                continue;
            }

            if validate_signatures && let Err(e) = validate_signature_from_hash(&event, &hash) {
                storage.log_error(
                    (line_num + 1) as u64,
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
                    (line_num + 1) as u64,
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

    info!("Conversion complete");
    if filtered_count > 0 {
        info!(
            "Pre-filtered {} events with invalid kind values",
            filtered_count
        );
    }
    stats.print_summary();

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

    let parallel_errors: Arc<Mutex<Vec<anyhow::Error>>> = Arc::new(Mutex::new(Vec::new()));

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

            scope.spawn(move |_| {
                if let Err(e) = process_chunk(
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
                    error!("Thread {} error: {:?}", thread_id, e);
                    errors.lock().unwrap().push(e);
                }
            });
        }
    });

    let errors = Arc::try_unwrap(parallel_errors)
        .unwrap()
        .into_inner()
        .unwrap();
    if !errors.is_empty() {
        return Err(anyhow::anyhow!(
            "Parallel processing encountered errors ({} threads)",
            errors.len()
        ));
    }

    // Clean up progress bar
    if let Some(pb) = progress {
        pb.finish_with_message("Merging temporary files...");
    }

    info!("All chunks processed, merging temporary files...");

    // Merge temporary files
    merge_temp_files(output_dir, &temp_dir, compression_level)?;

    // Clean up temp directory
    std::fs::remove_dir_all(&temp_dir).context("Failed to remove temp directory")?;

    info!("Merge complete");

    // Print summary using atomic values
    let final_stats = ConversionStats {
        total_lines: total_lines.load(Ordering::Relaxed),
        valid_events: valid_events.load(Ordering::Relaxed),
        invalid_events: invalid_events.load(Ordering::Relaxed),
        skipped_lines: skipped_lines.load(Ordering::Relaxed),
    };
    final_stats.print_summary();

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

        // Read to next newline (this line belongs to current chunk)
        let mut buf = String::new();
        reader.read_line(&mut buf)?;

        let chunk_end = reader.stream_position()?;

        boundaries.push((current_start, chunk_end));
        current_start = chunk_end; // Next chunk starts after this line
    }

    Ok(boundaries)
}

/// Process a single chunk of the input file
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
) -> Result<()> {
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
                storage.log_error(line_num, &format!("parse_error: {}", e), None);
                local_invalid += 1;
                invalid_events.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        // Validate basic fields first (fast check)
        if let Err(e) = validate_basic_fields(&event) {
            storage.log_error(
                line_num,
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
                    storage.log_error(line_num, &format!("hash_error: {}", e), Some(&event.id));
                    local_invalid += 1;
                    invalid_events.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
            };

            if validate_event_ids && let Err(e) = validate_event_id_from_hash(&event, &hash) {
                storage.log_error(
                    line_num,
                    &format!("validation_error: {}", e),
                    Some(&event.id),
                );
                local_invalid += 1;
                invalid_events.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            if validate_signatures && let Err(e) = validate_signature_from_hash(&event, &hash) {
                storage.log_error(
                    line_num,
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
                storage.log_error(line_num, &format!("storage_error: {}", e), None);
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

    Ok(())
}

/// Merge temporary files into final date-organized files
fn merge_temp_files(output_dir: &Path, temp_dir: &Path, compression_level: u32) -> Result<()> {
    // Group temp files by date
    let mut files_by_date: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for entry in std::fs::read_dir(temp_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("tmp")
            && let Some(date) = extract_date_from_temp_filename(&path)
        {
            files_by_date.entry(date).or_default().push(path);
        }
    }

    info!("Merging {} dates...", files_by_date.len());

    // Merge each date's files
    for (date, temp_files) in files_by_date {
        merge_protobuf_files_with_dedup(&temp_files, output_dir, &date, compression_level)?;
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
fn merge_protobuf_files_with_dedup(
    sources: &[PathBuf],
    output_dir: &Path,
    date_str: &str,
    compression_level: u32,
) -> Result<()> {
    use proton_beam_core::{
        create_gzip_decoder, create_gzip_encoder_with_level, read_events_delimited,
        write_event_delimited,
    };
    use std::io::BufWriter;

    let final_file = output_dir.join(format!("{}.pb.gz", date_str));
    let temp_output = output_dir.join(format!("{}.pb.gz.tmp", date_str));

    // If final file already exists, we need to include it in the merge
    let mut all_sources = sources.to_vec();
    if final_file.exists() {
        all_sources.push(final_file.clone());
    }

    let output_file = File::create(&temp_output)?;
    let gz = create_gzip_encoder_with_level(output_file, compression_level);
    let mut writer = BufWriter::new(gz);

    // Deduplicate during merge (streaming)
    let mut seen_ids = HashSet::new();
    let mut event_count = 0;
    let mut duplicate_count = 0;

    for source in &all_sources {
        let file = File::open(source)?;
        let gz = create_gzip_decoder(file);
        for event in read_events_delimited(gz) {
            let event = event?;
            if !seen_ids.insert(event.id.clone()) {
                duplicate_count += 1;
                continue;
            }

            write_event_delimited(&mut writer, &event)?;
            event_count += 1;
        }
    }

    writer.flush()?;
    drop(writer);
    std::fs::rename(&temp_output, &final_file)?;

    if duplicate_count > 0 {
        debug!(
            "Merged {} with {} events ({} duplicates removed)",
            final_file.display(),
            event_count,
            duplicate_count
        );
    } else {
        debug!(
            "Merged {} with {} events",
            final_file.display(),
            event_count
        );
    }

    Ok(())
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

    let mut index = EventIndex::new(index_path).context("Failed to create event index")?;

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

        for event_result in read_events_delimited(gz) {
            let event = event_result.context(format!("Failed to read event from {}", file_name))?;
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
