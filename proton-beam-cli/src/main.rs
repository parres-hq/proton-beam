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

        /// Skip event validation (faster but dangerous)
        #[arg(long)]
        no_validate: bool,

        /// Batch size for writing events
        #[arg(short, long, default_value = "500")]
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert {
            input,
            output_dir,
            no_validate,
            batch_size,
            verbose,
            no_progress,
        } => {
            // Initialize logging
            init_logging(verbose);

            info!("Starting Proton Beam CLI");
            info!("Input: {}", input);
            info!("Output directory: {}", output_dir.display());
            info!(
                "Validation: {}",
                if no_validate { "disabled" } else { "enabled" }
            );
            info!("Batch size: {}", batch_size);

            // Run conversion
            convert_events(&input, &output_dir, !no_validate, batch_size, !no_progress)?;
        }
    }

    Ok(())
}

fn init_logging(verbose: bool) {
    use tracing_subscriber::filter::LevelFilter;

    let filter = if verbose {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();
}

fn convert_events(
    input: &str,
    output_dir: &Path,
    validate: bool,
    batch_size: usize,
    show_progress: bool,
) -> Result<()> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

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
                "Processed: {} | Valid: {} | Errors: {}",
                stats.total_lines, stats.valid_events, stats.invalid_events
            ));
        }

        // Parse JSON to ProtoEvent
        let event = match ProtoEvent::try_from(line.as_str()) {
            Ok(event) => event,
            Err(e) => {
                warn!("Failed to parse event on line {}: {}", line_num + 1, e);
                storage.log_error((line_num + 1) as u64, &line, &format!("parse_error: {}", e))?;
                stats.invalid_events += 1;
                continue;
            }
        };

        // Validate if requested
        if validate && let Err(e) = validate_event(&event) {
            warn!("Failed to validate event on line {}: {}", line_num + 1, e);
            storage.log_error(
                (line_num + 1) as u64,
                &line,
                &format!("validation_error: {}", e),
            )?;
            stats.invalid_events += 1;
            continue;
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
                    &line,
                    &format!("storage_error: {}", e),
                )?;
                stats.invalid_events += 1;
            }
        }
    }

    // Flush any remaining events
    storage.flush()?;

    // Clean up progress bar
    if let Some(pb) = progress {
        pb.finish_with_message(format!(
            "Complete! Processed: {} | Valid: {} | Errors: {}",
            stats.total_lines, stats.valid_events, stats.invalid_events
        ));
    }

    info!("Conversion complete");
    stats.print_summary();

    // Exit code: 0 if any events succeeded, 1 if all failed
    if stats.valid_events == 0 && stats.total_lines > 0 {
        std::process::exit(1);
    }

    Ok(())
}
