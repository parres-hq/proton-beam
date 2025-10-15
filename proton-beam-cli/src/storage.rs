use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use proton_beam_core::{
    EventIndex, ProtoEvent, create_gzip_encoder_with_level, write_event_delimited,
};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use tracing::error;

// Buffer size for storage writers (512KB for optimal compression)
const STORAGE_WRITER_BUFFER_SIZE: usize = 512 * 1024;

type GzipWriter = BufWriter<flate2::write::GzEncoder<File>>;

/// Manages storage of events into date-organized protobuf files
pub struct StorageManager {
    output_dir: PathBuf,
    batch_size: usize,
    compression_level: u32,
    index: Option<EventIndex>,

    // Optional prefix for temp file names (used for parallel processing)
    file_prefix: Option<String>,

    // Map of date string (YYYY_MM_DD) to buffered events
    buffers: HashMap<String, Vec<ProtoEvent>>,

    // Keep writers open for reuse (map of date -> writer)
    writers: HashMap<String, GzipWriter>,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new(output_dir: &Path, batch_size: usize, compression_level: u32) -> Result<Self> {
        // Create the output directory if it doesn't exist
        std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

        Ok(Self {
            output_dir: output_dir.to_path_buf(),
            batch_size,
            compression_level,
            index: None,
            file_prefix: None,
            buffers: HashMap::new(),
            writers: HashMap::new(),
        })
    }

    /// Create a new storage manager with a file prefix for parallel processing
    /// Files will be named: {prefix}_{date}.pb.gz.tmp
    pub fn new_with_prefix(
        output_dir: &Path,
        batch_size: usize,
        thread_id: usize,
        compression_level: u32,
    ) -> Result<Self> {
        // Create the output directory if it doesn't exist
        std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

        Ok(Self {
            output_dir: output_dir.to_path_buf(),
            batch_size,
            compression_level,
            index: None,
            file_prefix: Some(format!("thread_{}", thread_id)),
            buffers: HashMap::new(),
            writers: HashMap::new(),
        })
    }

    /// Store an event (buffers it until batch size is reached)
    pub fn store_event(&mut self, event: ProtoEvent) -> Result<()> {
        // Get the date string from the event's created_at timestamp
        let date_str = self.get_date_string(&event)?;

        // Add event to the appropriate buffer
        let buffer = self.buffers.entry(date_str.clone()).or_default();
        buffer.push(event);

        // Check if we should flush this buffer
        if buffer.len() >= self.batch_size {
            self.flush_buffer(&date_str)?;
        }

        Ok(())
    }

    /// Get the date string (YYYY_MM_DD) from an event's created_at timestamp
    fn get_date_string(&self, event: &ProtoEvent) -> Result<String> {
        let timestamp = event.created_at;

        // Convert Unix timestamp to DateTime
        let datetime =
            DateTime::<Utc>::from_timestamp(timestamp, 0).context("Invalid timestamp")?;

        // Format as YYYY_MM_DD
        Ok(datetime.format("%Y_%m_%d").to_string())
    }

    /// Flush a specific buffer to disk (reuses writer if possible)
    fn flush_buffer(&mut self, date_str: &str) -> Result<()> {
        let buffer = match self.buffers.remove(date_str) {
            Some(buf) if !buf.is_empty() => buf,
            _ => return Ok(()), // Nothing to flush
        };

        let (filename, index_target): (String, Option<String>) =
            if let Some(ref prefix) = self.file_prefix {
                (format!("{}_{}.pb.gz.tmp", prefix, date_str), None)
            } else {
                (
                    format!("{}.pb.gz", date_str),
                    Some(format!("{}.pb.gz", date_str)),
                )
            };

        // Get or create writer for this date
        let output_path = self.output_dir.join(&filename);
        if !self.writers.contains_key(date_str) {
            let writer = self.create_writer(&output_path)?;
            self.writers.insert(date_str.to_string(), writer);
        }

        let writer = self
            .writers
            .get_mut(date_str)
            .expect("Writer should exist after insert");
        let mut index_batch: Vec<(ProtoEvent, String)> = Vec::new();

        for event in buffer {
            write_event_delimited(writer, &event).context("Failed to write event")?;
            if let Some(ref file_name) = index_target {
                index_batch.push((event, file_name.clone()));
            }
        }

        // Flush writer periodically but keep it open
        writer.flush().context("Failed to flush writer")?;

        if let (Some(index), Some(_)) = (&mut self.index, index_target)
            && !index_batch.is_empty()
        {
            let batch_refs: Vec<_> = index_batch
                .iter()
                .map(|(event, path)| (event, path.as_str()))
                .collect();
            index.insert_batch(&batch_refs)?;
        }

        Ok(())
    }

    /// Flush all buffers to disk
    pub fn flush(&mut self) -> Result<()> {
        let date_strings: Vec<String> = self.buffers.keys().cloned().collect();

        for date_str in date_strings {
            self.flush_buffer(&date_str)?;
        }

        Ok(())
    }

    /// Log an error using tracing (compact format)
    pub fn log_error(&self, line_num: u64, error_reason: &str, event_id: Option<&str>) {
        // Truncate long error messages for compactness (keep first 100 chars)
        let compact_reason = if error_reason.len() > 100 {
            format!("{}...", &error_reason[..97])
        } else {
            error_reason.to_string()
        };

        if let Some(id) = event_id {
            // Truncate ID to first 8 chars for compactness
            let short_id = if id.len() > 8 { &id[..8] } else { id };
            error!(line = line_num, id = short_id, "{}", compact_reason);
        } else {
            error!(line = line_num, "{}", compact_reason);
        }
    }
}

impl Drop for StorageManager {
    fn drop(&mut self) {
        // Ensure all buffers are flushed when the manager is dropped
        if let Err(e) = self.flush() {
            tracing::error!("StorageManager drop flush error: {}", e);
        }

        // Close all writers (flush and drop them)
        for (date, mut writer) in self.writers.drain() {
            if let Err(e) = writer.flush() {
                tracing::error!("Failed to flush writer for {}: {}", date, e);
            }
            // Writer's Drop will finish the gzip encoding
        }
    }
}

impl StorageManager {
    fn create_writer(&self, output_path: &Path) -> Result<GzipWriter> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(output_path)
            .context(format!(
                "Failed to open output file: {}",
                output_path.display()
            ))?;
        Ok(BufWriter::with_capacity(
            STORAGE_WRITER_BUFFER_SIZE,
            create_gzip_encoder_with_level(file, self.compression_level),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proton_beam_core::ProtoEventBuilder;
    use tempfile::TempDir;

    #[test]
    fn test_date_string_generation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StorageManager::new(temp_dir.path(), 100, 6).unwrap();

        // Create an event with a known timestamp
        // 2025-09-27 00:00:00 UTC = 1758960000
        let event = ProtoEventBuilder::new()
            .id("0000000000000000000000000000000000000000000000000000000000000000")
            .pubkey("0000000000000000000000000000000000000000000000000000000000000000")
            .created_at(1758960000)
            .kind(1)
            .content("test")
            .sig("0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")
            .build();

        let date_str = manager.get_date_string(&event).unwrap();
        assert_eq!(date_str, "2025_09_27");
    }

    #[test]
    fn test_storage_and_flush() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = StorageManager::new(temp_dir.path(), 10, 6).unwrap();

        // Create and store some events
        for i in 0..5 {
            let event = ProtoEventBuilder::new()
                .id(format!("{:064x}", i))
                .pubkey("0000000000000000000000000000000000000000000000000000000000000000")
                .created_at(1758960000)
                .kind(1)
                .content(format!("test {}", i))
                .sig("0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")
                .build();

            manager.store_event(event).unwrap();
        }

        // Flush to disk
        manager.flush().unwrap();

        // Check that the compressed file was created
        let pb_file = temp_dir.path().join("2025_09_27.pb.gz");
        assert!(pb_file.exists());
    }

    #[test]
    fn test_error_logging() {
        // Initialize test logging
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();

        let temp_dir = TempDir::new().unwrap();
        let manager = StorageManager::new(temp_dir.path(), 10, 6).unwrap();

        // Test error logging with event ID
        manager.log_error(42, "parse_error: missing field 'id'", Some("abcd1234"));

        // Test error logging without event ID
        manager.log_error(43, "validation_error: invalid signature", None);

        // The errors are now logged via tracing, not to a file
        // This test just ensures the log_error method doesn't panic
    }
}
