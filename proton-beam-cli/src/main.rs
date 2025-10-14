use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use proton_beam_core::{ProtoEvent, validate_event};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, error, info};

mod input;
mod progress;
mod storage;

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

        /// Path to SQLite index database (defaults to OUTPUT_DIR/index.db)
        #[arg(long)]
        index_path: Option<PathBuf>,

        /// Skip event validation (faster but dangerous)
        #[arg(long)]
        no_validate: bool,

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
    },
}

#[derive(Debug)]
struct ConversionStats {
    total_lines: u64,
    valid_events: u64,
    invalid_events: u64,
    skipped_lines: u64,
    duplicates: u64,
}

impl ConversionStats {
    fn new() -> Self {
        Self {
            total_lines: 0,
            valid_events: 0,
            invalid_events: 0,
            skipped_lines: 0,
            duplicates: 0,
        }
    }

    fn print_summary(&self) {
        println!("\n📊 Conversion Summary:");
        println!("  Total lines processed: {}", self.total_lines);
        println!("  ✅ Valid events:       {}", self.valid_events);
        println!("  ❌ Invalid events:     {}", self.invalid_events);
        if self.duplicates > 0 {
            println!("  🔄 Duplicate events:   {}", self.duplicates);
        }
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert {
            input,
            output_dir,
            index_path,
            no_validate,
            batch_size,
            verbose,
            no_progress,
            parallel,
            filter_invalid_kinds,
            no_filter_kinds,
        } => {
            // Apply no_filter_kinds flag
            let filter_invalid_kinds = filter_invalid_kinds && !no_filter_kinds;
            // Create output directory first (needed for log file)
            std::fs::create_dir_all(&output_dir).context("Failed to create output directory")?;

            // Initialize logging (creates log file in output_dir)
            init_logging(verbose, &output_dir);

            // Determine index path
            let index_path = index_path.unwrap_or_else(|| output_dir.join("index.db"));

            // Determine number of threads
            let num_threads = parallel.unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(1)
            });

            info!("Starting Proton Beam CLI");
            info!("Input: {}", input.display());
            info!("Output directory: {}", output_dir.display());
            info!("Index database: {}", index_path.display());
            info!(
                "Validation: {}",
                if no_validate { "disabled" } else { "enabled" }
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

            // Run conversion
            if num_threads > 1 {
                convert_events_parallel(
                    &input,
                    &output_dir,
                    &index_path,
                    !no_validate,
                    batch_size,
                    !no_progress,
                    num_threads,
                    filter_invalid_kinds,
                )?;
            } else {
                convert_events(
                    &input,
                    &output_dir,
                    &index_path,
                    !no_validate,
                    batch_size,
                    !no_progress,
                    filter_invalid_kinds,
                )?;
            }
        }
    }

    Ok(())
}

fn init_logging(verbose: bool, output_dir: &Path) {
    use std::fs::OpenOptions;
    use tracing_subscriber::{EnvFilter, filter::LevelFilter, fmt, prelude::*};

    let filter = if verbose {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    // Create log file in output directory
    let log_path = output_dir.join("proton-beam.log");
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("Failed to open log file");

    // Create a compact formatter for the log file (only errors and warnings)
    let file_layer = fmt::layer()
        .with_writer(std::sync::Arc::new(log_file))
        .with_ansi(false)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_line_number(false)
        .with_file(false)
        .compact()
        .with_filter(EnvFilter::new("warn"));

    // Create a layer for stderr (info and debug messages)
    let stderr_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_filter(filter);

    // Combine both layers
    tracing_subscriber::registry()
        .with(file_layer)
        .with(stderr_layer)
        .init();
}

fn convert_events(
    input: &Path,
    output_dir: &Path,
    index_path: &Path,
    validate: bool,
    batch_size: usize,
    show_progress: bool,
    filter_invalid_kinds: bool,
) -> Result<()> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    // Initialize event index
    use proton_beam_core::EventIndex;
    let mut index = EventIndex::new(index_path).context("Failed to open event index")?;

    // Initialize storage manager
    let mut storage = StorageManager::new(output_dir, batch_size)?;

    // Initialize input reader with preprocessing options
    let mut reader = InputReader::with_options(input.to_str().unwrap(), filter_invalid_kinds)?;

    // Set up progress bar
    let progress = if show_progress {
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

    // Buffer for batch index insertions
    let mut index_buffer: Vec<(ProtoEvent, String)> = Vec::with_capacity(batch_size);

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
            pb.set_message(format!(
                "Processed: {} | Valid: {} | Errors: {} | Dupes: {}",
                stats.total_lines, stats.valid_events, stats.invalid_events, stats.duplicates
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

        // Check for duplicates (check both index and buffer)
        let in_index = index.contains(&event.id).context("Failed to check index")?;
        let in_buffer = index_buffer.iter().any(|(e, _)| e.id == event.id);

        if in_index || in_buffer {
            info!(
                "Skipping duplicate event: {} (line {})",
                event.id,
                line_num + 1
            );
            stats.duplicates += 1;
            continue;
        }

        // Validate if requested
        if validate && let Err(e) = validate_event(&event) {
            storage.log_error(
                (line_num + 1) as u64,
                &format!("validation_error: {}", e),
                Some(&event.id),
            );
            stats.invalid_events += 1;
            continue;
        }

        // Get the date string for file path
        let date_str = storage.get_date_string_for_event(&event)?;
        let file_name = format!("{}.pb.gz", date_str);

        // Store the event
        match storage.store_event(event.clone()) {
            Ok(_) => {
                // Buffer the event for batch index insertion
                index_buffer.push((event, file_name));
                stats.valid_events += 1;
                debug!("Successfully stored event from line {}", line_num + 1);

                // Flush index buffer when it reaches batch size
                if index_buffer.len() >= batch_size {
                    let batch_refs: Vec<_> =
                        index_buffer.iter().map(|(e, f)| (e, f.as_str())).collect();
                    index
                        .insert_batch(&batch_refs)
                        .context("Failed to batch insert to index")?;
                    index_buffer.clear();
                }
            }
            Err(e) => {
                error!("Failed to store event from line {}: {}", line_num + 1, e);
                storage.log_error(
                    (line_num + 1) as u64,
                    &format!("storage_error: {}", e),
                    Some(&event.id),
                );
                stats.invalid_events += 1;
            }
        }
    }

    // Flush any remaining events
    storage.flush()?;

    // Flush any remaining index entries
    if !index_buffer.is_empty() {
        let batch_refs: Vec<_> = index_buffer.iter().map(|(e, f)| (e, f.as_str())).collect();
        index
            .insert_batch(&batch_refs)
            .context("Failed to flush final index batch")?;
    }

    // Get filtered count from reader
    let filtered_count = reader.filtered_count();

    // Clean up progress bar
    if let Some(pb) = progress {
        pb.finish_with_message(format!(
            "Complete! Processed: {} | Valid: {} | Errors: {} | Dupes: {}{}",
            stats.total_lines,
            stats.valid_events,
            stats.invalid_events,
            stats.duplicates,
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

    // Exit code: 0 if any events succeeded or were duplicates, 1 if all failed
    // Duplicates are not considered failures since they were successfully processed before
    if stats.valid_events == 0 && stats.duplicates == 0 && stats.total_lines > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Parallel version of convert_events using file chunking
#[allow(clippy::too_many_arguments)]
fn convert_events_parallel(
    input: &Path,
    output_dir: &Path,
    index_path: &Path,
    validate: bool,
    batch_size: usize,
    show_progress: bool,
    num_threads: usize,
    filter_invalid_kinds: bool,
) -> Result<()> {
    use proton_beam_core::EventIndex;

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    // Create temp directory for parallel writes
    let temp_dir = output_dir.join("tmp");
    std::fs::create_dir_all(&temp_dir).context("Failed to create temp directory")?;

    // Initialize shared event index
    let index = Arc::new(Mutex::new(
        EventIndex::new(index_path).context("Failed to open event index")?,
    ));

    // Shared stats
    let stats = Arc::new(Mutex::new(ConversionStats::new()));

    // Find chunk boundaries
    info!(
        "Calculating chunk boundaries for {} threads...",
        num_threads
    );
    let chunks = find_chunk_boundaries(input, num_threads)?;
    info!("Processing {} chunks in parallel", chunks.len());

    // Progress bar
    let progress = if show_progress {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        Some(Arc::new(pb))
    } else {
        None
    };

    // Process chunks in parallel
    let handles: Vec<_> = chunks
        .into_iter()
        .enumerate()
        .map(|(thread_id, (start, end))| {
            let input = input.to_path_buf();
            let temp_dir = temp_dir.clone();
            let index = Arc::clone(&index);
            let stats = Arc::clone(&stats);
            let progress = progress.as_ref().map(Arc::clone);

            std::thread::spawn(move || {
                process_chunk(
                    thread_id,
                    &input,
                    start,
                    end,
                    &temp_dir,
                    index,
                    stats,
                    progress,
                    validate,
                    batch_size,
                    filter_invalid_kinds,
                )
            })
        })
        .collect();

    // Wait for all threads to complete
    for (i, handle) in handles.into_iter().enumerate() {
        handle
            .join()
            .unwrap_or_else(|_| Err(anyhow::anyhow!("Thread {} panicked", i)))?;
    }

    // Clean up progress bar
    if let Some(pb) = progress {
        pb.finish_with_message("Merging temporary files...");
    }

    info!("All chunks processed, merging temporary files...");

    // Merge temporary files
    merge_temp_files(output_dir, &temp_dir)?;

    // Clean up temp directory
    std::fs::remove_dir_all(&temp_dir).context("Failed to remove temp directory")?;

    info!("Merge complete");

    // Print summary
    let stats = stats.lock().unwrap();
    stats.print_summary();

    // Exit code: 0 if any events succeeded or were duplicates, 1 if all failed
    if stats.valid_events == 0 && stats.duplicates == 0 && stats.total_lines > 0 {
        std::process::exit(1);
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
    global_index: Arc<Mutex<proton_beam_core::EventIndex>>,
    global_stats: Arc<Mutex<ConversionStats>>,
    progress: Option<Arc<ProgressBar>>,
    validate: bool,
    batch_size: usize,
    filter_invalid_kinds: bool,
) -> Result<()> {
    // Open the file and seek to start position
    let file = File::open(input_path)?;
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(start))?;

    // Thread-local state
    let mut storage = StorageManager::new_with_prefix(temp_dir, batch_size, thread_id)?;
    let mut local_seen = HashSet::new();
    let mut index_buffer: Vec<(ProtoEvent, String)> = Vec::with_capacity(batch_size);

    // Local stats
    let mut local_stats = ConversionStats::new();
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
        local_stats.total_lines += 1;

        // Skip empty lines
        if line.trim().is_empty() {
            local_stats.skipped_lines += 1;
            continue;
        }

        // Pre-filter invalid kinds if enabled
        if filter_invalid_kinds && !InputReader::has_valid_kind(&line) {
            filtered_count += 1;
            continue;
        }

        // Update progress periodically
        if line_num.is_multiple_of(100)
            && let Some(ref pb) = progress
        {
            let stats = global_stats.lock().unwrap();
            pb.set_message(format!(
                "Processed: {} | Valid: {} | Errors: {} | Dupes: {}",
                stats.total_lines, stats.valid_events, stats.invalid_events, stats.duplicates
            ));
        }

        // Parse JSON to ProtoEvent
        let event = match ProtoEvent::try_from(line.as_str()) {
            Ok(event) => event,
            Err(e) => {
                storage.log_error(line_num, &format!("parse_error: {}", e), None);
                local_stats.invalid_events += 1;
                continue;
            }
        };

        // Check for duplicates (local HashSet first, then global index)
        if local_seen.contains(&event.id) {
            local_stats.duplicates += 1;
            continue;
        }

        let in_global_index = {
            let index = global_index.lock().unwrap();
            index.contains(&event.id).context("Failed to check index")?
        };

        if in_global_index {
            local_seen.insert(event.id.clone());
            local_stats.duplicates += 1;
            continue;
        }

        // Check against pending buffer
        if index_buffer.iter().any(|(e, _)| e.id == event.id) {
            local_stats.duplicates += 1;
            continue;
        }

        // Validate if requested
        if validate && let Err(e) = validate_event(&event) {
            storage.log_error(
                line_num,
                &format!("validation_error: {}", e),
                Some(&event.id),
            );
            local_stats.invalid_events += 1;
            continue;
        }

        // Get the date string for file path
        let date_str = storage.get_date_string_for_event(&event)?;
        let file_name = format!("{}.pb.gz", date_str);

        // Store the event
        match storage.store_event(event.clone()) {
            Ok(_) => {
                local_seen.insert(event.id.clone());
                index_buffer.push((event, file_name));
                local_stats.valid_events += 1;

                // Flush index buffer when it reaches batch size
                if index_buffer.len() >= batch_size {
                    let batch_refs: Vec<_> =
                        index_buffer.iter().map(|(e, f)| (e, f.as_str())).collect();
                    let mut index = global_index.lock().unwrap();
                    index
                        .insert_batch(&batch_refs)
                        .context("Failed to batch insert to index")?;
                    index_buffer.clear();
                }
            }
            Err(e) => {
                error!(
                    "Thread {}: Failed to store event from line {}: {}",
                    thread_id, line_num, e
                );
                storage.log_error(line_num, &format!("storage_error: {}", e), Some(&event.id));
                local_stats.invalid_events += 1;
            }
        }
    }

    // Flush any remaining events
    storage.flush()?;

    // Flush any remaining index entries
    if !index_buffer.is_empty() {
        let batch_refs: Vec<_> = index_buffer.iter().map(|(e, f)| (e, f.as_str())).collect();
        let mut index = global_index.lock().unwrap();
        index
            .insert_batch(&batch_refs)
            .context("Failed to flush final index batch")?;
    }

    // Update global stats
    {
        let mut stats = global_stats.lock().unwrap();
        stats.total_lines += local_stats.total_lines;
        stats.valid_events += local_stats.valid_events;
        stats.invalid_events += local_stats.invalid_events;
        stats.skipped_lines += local_stats.skipped_lines;
        stats.duplicates += local_stats.duplicates;
    }

    info!(
        "Thread {} completed: {} lines, {} valid, {} errors, {} dupes{}",
        thread_id,
        local_stats.total_lines,
        local_stats.valid_events,
        local_stats.invalid_events,
        local_stats.duplicates,
        if filtered_count > 0 {
            format!(", {} filtered", filtered_count)
        } else {
            String::new()
        }
    );

    Ok(())
}

/// Merge temporary files into final date-organized files
fn merge_temp_files(output_dir: &Path, temp_dir: &Path) -> Result<()> {
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
        merge_protobuf_files_with_dedup(&temp_files, output_dir, &date)?;
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

/// Read events from a gzipped protobuf file
fn read_events_from_gzip_file(path: &Path) -> Result<Vec<ProtoEvent>> {
    use proton_beam_core::{create_gzip_decoder, read_events_delimited};

    let file = File::open(path)?;
    let gz = create_gzip_decoder(file);
    let events: Result<Vec<_>> = read_events_delimited(gz)
        .map(|result| result.context("Failed to read event"))
        .collect();
    events
}

/// Merge multiple protobuf files with deduplication
fn merge_protobuf_files_with_dedup(
    sources: &[PathBuf],
    output_dir: &Path,
    date_str: &str,
) -> Result<()> {
    use proton_beam_core::{create_gzip_encoder, write_event_delimited};
    use std::io::BufWriter;

    let final_file = output_dir.join(format!("{}.pb.gz", date_str));

    // If final file already exists, we need to include it in the merge
    let mut all_sources = sources.to_vec();
    if final_file.exists() {
        all_sources.push(final_file.clone());
    }

    let output_file = File::create(&final_file)?;
    let gz = create_gzip_encoder(output_file);
    let mut writer = BufWriter::new(gz);

    // Deduplicate during merge
    let mut seen_ids = HashSet::new();
    let mut event_count = 0;
    let mut duplicate_count = 0;

    for source in &all_sources {
        for event in read_events_from_gzip_file(source)? {
            if !seen_ids.insert(event.id.clone()) {
                duplicate_count += 1;
                continue;
            }

            write_event_delimited(&mut writer, &event)?;
            event_count += 1;
        }
    }

    writer.flush()?;

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
