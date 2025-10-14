use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use proton_beam_core::{ProtoEvent, validate_event};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, error, info, warn};

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
        /// Input file path (.jsonl) or '-' for stdin
        #[arg(value_name = "INPUT")]
        input: String,

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
        } => {
            // Create output directory first (needed for log file)
            std::fs::create_dir_all(&output_dir)
                .context("Failed to create output directory")?;

            // Initialize logging (creates log file in output_dir)
            init_logging(verbose, &output_dir);

            // Determine index path
            let index_path = index_path.unwrap_or_else(|| output_dir.join("index.db"));

            info!("Starting Proton Beam CLI");
            info!("Input: {}", input);
            info!("Output directory: {}", output_dir.display());
            info!("Index database: {}", index_path.display());
            info!(
                "Validation: {}",
                if no_validate { "disabled" } else { "enabled" }
            );
            info!("Batch size: {}", batch_size);

            // Run conversion
            convert_events(
                &input,
                &output_dir,
                &index_path,
                !no_validate,
                batch_size,
                !no_progress,
            )?;
        }
    }

    Ok(())
}

fn init_logging(verbose: bool, output_dir: &Path) {
    use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*, EnvFilter};
    use std::fs::OpenOptions;

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
    input: &str,
    output_dir: &Path,
    index_path: &Path,
    validate: bool,
    batch_size: usize,
    show_progress: bool,
) -> Result<()> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    // Initialize event index
    use proton_beam_core::EventIndex;
    let mut index = EventIndex::new(index_path).context("Failed to open event index")?;

    // Initialize storage manager
    let mut storage = StorageManager::new(output_dir, batch_size)?;

    // Initialize input reader
    let reader = InputReader::new(input)?;

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
    for (line_num, line_result) in reader.enumerate() {
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
                warn!("Failed to parse event on line {}: {}", line_num + 1, e);
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
            warn!("Failed to validate event on line {}: {}", line_num + 1, e);
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

    // Clean up progress bar
    if let Some(pb) = progress {
        pb.finish_with_message(format!(
            "Complete! Processed: {} | Valid: {} | Errors: {} | Dupes: {}",
            stats.total_lines, stats.valid_events, stats.invalid_events, stats.duplicates
        ));
    }

    info!("Conversion complete");
    stats.print_summary();

    // Exit code: 0 if any events succeeded or were duplicates, 1 if all failed
    // Duplicates are not considered failures since they were successfully processed before
    if stats.valid_events == 0 && stats.duplicates == 0 && stats.total_lines > 0 {
        std::process::exit(1);
    }

    Ok(())
}
