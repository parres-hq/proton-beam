use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use proton_beam_core::{ProtoEvent, write_event_delimited};
use serde_json::json;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Manages storage of events into date-organized protobuf files
pub struct StorageManager {
    output_dir: PathBuf,
    batch_size: usize,

    // Map of date string (YYYY_MM_DD) to buffered events
    buffers: HashMap<String, Vec<ProtoEvent>>,

    // Error log file
    error_writer: BufWriter<File>,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new(output_dir: &Path, batch_size: usize) -> Result<Self> {
        // Create the output directory if it doesn't exist
        std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

        // Open or create the errors.jsonl file
        let error_path = output_dir.join("errors.jsonl");
        let error_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&error_path)
            .context("Failed to open errors.jsonl")?;

        Ok(Self {
            output_dir: output_dir.to_path_buf(),
            batch_size,
            buffers: HashMap::new(),
            error_writer: BufWriter::new(error_file),
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

    /// Public method to get the date string for an event (used for indexing)
    pub fn get_date_string_for_event(&self, event: &ProtoEvent) -> Result<String> {
        self.get_date_string(event)
    }

    /// Flush a specific buffer to disk
    fn flush_buffer(&mut self, date_str: &str) -> Result<()> {
        let buffer = match self.buffers.remove(date_str) {
            Some(buf) if !buf.is_empty() => buf,
            _ => return Ok(()), // Nothing to flush
        };

        // Open the output file for this date (append mode)
        let filename = format!("{}.pb", date_str);
        let output_path = self.output_dir.join(&filename);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&output_path)
            .context(format!("Failed to open output file: {}", filename))?;

        let mut writer = BufWriter::new(file);

        // Write all events in the buffer
        for event in buffer {
            write_event_delimited(&mut writer, &event).context("Failed to write event")?;
        }

        // Ensure all data is written
        writer.flush().context("Failed to flush writer")?;

        Ok(())
    }

    /// Flush all buffers to disk
    pub fn flush(&mut self) -> Result<()> {
        let date_strings: Vec<String> = self.buffers.keys().cloned().collect();

        for date_str in date_strings {
            self.flush_buffer(&date_str)?;
        }

        // Flush error log
        self.error_writer
            .flush()
            .context("Failed to flush error log")?;

        Ok(())
    }

    /// Log an error to errors.jsonl
    pub fn log_error(
        &mut self,
        line_num: u64,
        original_json: &str,
        error_reason: &str,
    ) -> Result<()> {
        let error_entry = json!({
            "line": line_num,
            "error": error_reason,
            "original": original_json,
        });

        writeln!(self.error_writer, "{}", error_entry)
            .context("Failed to write error log entry")?;

        Ok(())
    }
}

impl Drop for StorageManager {
    fn drop(&mut self) {
        // Ensure all buffers are flushed when the manager is dropped
        let _ = self.flush();
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
        let manager = StorageManager::new(temp_dir.path(), 100).unwrap();

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
        let mut manager = StorageManager::new(temp_dir.path(), 10).unwrap();

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

        // Check that the file was created
        let pb_file = temp_dir.path().join("2025_09_27.pb");
        assert!(pb_file.exists());
    }

    #[test]
    fn test_error_logging() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = StorageManager::new(temp_dir.path(), 10).unwrap();

        manager
            .log_error(
                42,
                r#"{"invalid": "json"}"#,
                "parse_error: missing field 'id'",
            )
            .unwrap();
        manager.flush().unwrap();

        let error_file = temp_dir.path().join("errors.jsonl");
        assert!(error_file.exists());

        let content = std::fs::read_to_string(error_file).unwrap();
        assert!(content.contains("\"line\":42"));
        assert!(content.contains("parse_error"));
    }
}
