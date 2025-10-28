//! Bulk importer for ClickHouse
//!
//! This tool reads `.pb.gz` files (protobuf + gzip compressed Nostr events)
//! and imports them into ClickHouse for efficient querying.
//!
//! # Usage
//!
//! ```bash
//! # Import a single file
//! proton-beam-clickhouse-import --input events.pb.gz
//!
//! # Import multiple files
//! proton-beam-clickhouse-import --input pb_data/*.pb.gz
//!
//! # Custom ClickHouse connection
//! proton-beam-clickhouse-import \
//!   --input events.pb.gz \
//!   --host clickhouse.example.com \
//!   --port 8123 \
//!   --user admin \
//!   --password secret \
//!   --database nostr
//!
//! # Batch size control (for memory management)
//! proton-beam-clickhouse-import --input events.pb.gz --batch-size 10000
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use proton_beam_core::{create_gzip_decoder, read_events_delimited};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Instant;
use tracing::info;

#[cfg(feature = "clickhouse")]
use proton_beam_cli::{
    clickhouse::{ClickHouseClient, ClickHouseConfig, EventRow},
};

#[derive(Parser, Debug)]
#[command(name = "proton-beam-clickhouse-import")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input .pb.gz file(s) to import
    #[arg(short, long, required = true)]
    input: Vec<PathBuf>,

    /// ClickHouse host
    #[arg(long, default_value = "localhost")]
    host: String,

    /// ClickHouse HTTP port
    #[arg(long, default_value = "8123")]
    port: u16,

    /// ClickHouse user
    #[arg(long, default_value = "default")]
    user: String,

    /// ClickHouse password
    #[arg(long, default_value = "")]
    password: String,

    /// ClickHouse database
    #[arg(long, default_value = "nostr")]
    database: String,

    /// ClickHouse table name
    #[arg(long, default_value = "events_local")]
    table: String,

    /// Batch size for inserts (events per batch)
    #[arg(long, default_value = "5000")]
    batch_size: usize,

    /// Skip connection test
    #[arg(long)]
    skip_test: bool,

    /// Dry run - parse files but don't insert into ClickHouse
    #[arg(long)]
    dry_run: bool,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .init();

    #[cfg(not(feature = "clickhouse"))]
    {
        error!("ClickHouse support not enabled!");
        error!("Please recompile with: cargo build --release --features clickhouse");
        std::process::exit(1);
    }

    #[cfg(feature = "clickhouse")]
    {
        run_import(args).await
    }
}

#[cfg(feature = "clickhouse")]
async fn run_import(args: Args) -> Result<()> {
    info!("Starting ClickHouse bulk import");

    // Build configuration
    let config = ClickHouseConfig {
        host: args.host.clone(),
        port: args.port,
        user: args.user.clone(),
        password: args.password.clone(),
        database: args.database.clone(),
        table: args.table.clone(),
    };

    info!("Configuration:");
    info!("  Host: {}:{}", config.host, config.port);
    info!("  Database: {}", config.database);
    info!("  Table: {}", config.table);
    info!("  Batch size: {}", args.batch_size);
    info!("  Input files: {}", args.input.len());

    // Create ClickHouse client
    let client = if !args.dry_run {
        let client = ClickHouseClient::new(config.clone())
            .context("Failed to create ClickHouse client")?;

        // Test connection
        if !args.skip_test {
            info!("Testing ClickHouse connection...");
            client.test_connection().await?;
            info!("✓ Connection successful");

            // Verify schema
            info!("Verifying database schema...");
            client.verify_schema().await?;
            info!("✓ Schema verified");

            // Get initial count
            let initial_count = client.get_event_count().await?;
            info!("Current event count: {}", initial_count);
        }

        Some(client)
    } else {
        info!("Dry run mode - skipping ClickHouse connection");
        None
    };

    // Process each input file
    let mut total_events = 0u64;
    let start_time = Instant::now();

    for input_path in &args.input {
        info!("Processing file: {}", input_path.display());

        let file_events = process_file(
            input_path,
            client.as_ref(),
            args.batch_size,
            args.dry_run,
        )
        .await?;

        total_events += file_events;

        info!(
            "✓ Processed {} events from {}",
            file_events,
            input_path.display()
        );
    }

    let elapsed = start_time.elapsed();
    let events_per_sec = total_events as f64 / elapsed.as_secs_f64();

    info!("");
    info!("Import complete!");
    info!("  Total events: {}", total_events);
    info!("  Total time: {:.2}s", elapsed.as_secs_f64());
    info!("  Speed: {:.0} events/sec", events_per_sec);

    if !args.dry_run {
        if let Some(client) = client {
            let final_count = client.get_event_count().await?;
            info!("  Final event count: {}", final_count);
        }
    }

    Ok(())
}

#[cfg(feature = "clickhouse")]
async fn process_file(
    path: &PathBuf,
    client: Option<&ClickHouseClient>,
    batch_size: usize,
    dry_run: bool,
) -> Result<u64> {
    // Open and decompress file
    let file = File::open(path).context(format!("Failed to open {}", path.display()))?;
    let buf_reader = BufReader::new(file);
    let decoder = create_gzip_decoder(buf_reader);

    // Create progress bar
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap(),
    );

    let mut event_batch = Vec::with_capacity(batch_size);
    let mut total_count = 0u64;
    let mut batch_count = 0u64;

    // Read events
    for result in read_events_delimited(decoder) {
        let event = result.context("Failed to read event from protobuf")?;

        if dry_run {
            // In dry run, just count
            total_count += 1;
        } else {
            // Convert to EventRow
            event_batch.push(EventRow::from(event));

            // Insert batch when full
            if event_batch.len() >= batch_size {
                if let Some(client) = client {
                    let inserted = client.insert_events(event_batch.clone()).await?;
                    total_count += inserted as u64;
                    batch_count += 1;
                }
                event_batch.clear();
            }
        }

        // Update progress every 1000 events
        if total_count % 1000 == 0 {
            pb.set_message(format!("Processed {} events", total_count));
        }
    }

    // Insert remaining events
    if !event_batch.is_empty() && !dry_run {
        if let Some(client) = client {
            let inserted = client.insert_events(event_batch).await?;
            total_count += inserted as u64;
            batch_count += 1;
        }
    }

    pb.finish_with_message(format!("Completed - {} events", total_count));

    if !dry_run {
        info!("Inserted {} batches", batch_count);
    }

    Ok(total_count)
}

#[cfg(not(feature = "clickhouse"))]
async fn run_import(_args: Args) -> Result<()> {
    unreachable!()
}

