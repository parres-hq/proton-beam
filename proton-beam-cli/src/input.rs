use anyhow::{Context, Result};
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines};
use std::path::Path;
use std::sync::OnceLock;

/// Regex to extract kind value from JSON
/// Matches: "kind": 123, "kind":456, etc.
static KIND_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_kind_regex() -> &'static Regex {
    KIND_REGEX
        .get_or_init(|| Regex::new(r#""kind"\s*:\s*(\d+)"#).expect("Failed to compile kind regex"))
}

/// Input reader for JSONL files with optional preprocessing
pub struct InputReader {
    reader: Lines<BufReader<File>>,
    filter_invalid_kinds: bool,
    filtered_count: usize,
}

impl InputReader {
    /// Create a new input reader from a file path
    #[cfg(test)]
    pub fn new(input: &str) -> Result<Self> {
        Self::with_options(input, false)
    }

    /// Create a new input reader with preprocessing options
    ///
    /// # Arguments
    /// * `input` - Path to the input file
    /// * `filter_invalid_kinds` - If true, filters out events with kind values > 65535
    pub fn with_options(input: &str, filter_invalid_kinds: bool) -> Result<Self> {
        let path = Path::new(input);
        if !path.exists() {
            anyhow::bail!("Input file does not exist: {}", input);
        }

        let file = File::open(path).context(format!("Failed to open input file: {}", input))?;
        let reader = BufReader::with_capacity(1024 * 1024, file); // 1MB buffer

        Ok(Self {
            reader: reader.lines(),
            filter_invalid_kinds,
            filtered_count: 0,
        })
    }

    /// Get the number of lines filtered out due to invalid kinds
    pub fn filtered_count(&self) -> usize {
        self.filtered_count
    }

    /// Check if a JSON line has a valid kind value (0-65535)
    pub fn has_valid_kind(line: &str) -> bool {
        let regex = get_kind_regex();

        // Extract kind value using regex
        regex
            .captures(line)
            .and_then(|captures| captures.get(1))
            .and_then(|kind_match| kind_match.as_str().parse::<u64>().ok())
            .is_none_or(|kind| kind <= 65535)

        // If no kind field found or parsing failed, assume valid
        // (will be caught later in validation)
    }
}

impl Iterator for InputReader {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line_result = self.reader.next()?;

            let line = match line_result {
                Ok(l) => l,
                Err(e) => return Some(Err(e).context("Failed to read line from file")),
            };

            // Apply kind filtering if enabled
            if self.filter_invalid_kinds && !Self::has_valid_kind(&line) {
                self.filtered_count += 1;
                continue; // Skip this line and read the next one
            }

            return Some(Ok(line));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_reader() {
        // Create a temporary file with test data
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line 1").unwrap();
        writeln!(file, "line 2").unwrap();
        writeln!(file, "line 3").unwrap();
        file.flush().unwrap();

        let reader = InputReader::new(file.path().to_str().unwrap()).unwrap();
        let lines: Vec<String> = reader.map(|r| r.unwrap()).collect();

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[1], "line 2");
        assert_eq!(lines[2], "line 3");
    }

    #[test]
    fn test_file_not_found() {
        let result = InputReader::new("/nonexistent/file.jsonl");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_valid_kind() {
        // Valid kinds
        assert!(InputReader::has_valid_kind(r#"{"kind": 1}"#));
        assert!(InputReader::has_valid_kind(r#"{"kind":0}"#));
        assert!(InputReader::has_valid_kind(r#"{"kind": 65535}"#));
        assert!(InputReader::has_valid_kind(
            r#"{"id":"abc","kind": 1000,"other":"field"}"#
        ));

        // Invalid kinds (over u16 max)
        assert!(!InputReader::has_valid_kind(r#"{"kind": 65536}"#));
        assert!(!InputReader::has_valid_kind(r#"{"kind":100000}"#));
        assert!(!InputReader::has_valid_kind(r#"{"kind": 999999999}"#));

        // Edge cases (no kind field, will be validated later)
        assert!(InputReader::has_valid_kind(r#"{"other":"field"}"#));
    }

    #[test]
    fn test_filter_invalid_kinds() {
        // Create a temporary file with mixed valid/invalid kinds
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"kind": 1, "content": "valid"}}"#).unwrap();
        writeln!(file, r#"{{"kind": 100000, "content": "invalid"}}"#).unwrap();
        writeln!(file, r#"{{"kind": 65535, "content": "valid max"}}"#).unwrap();
        writeln!(file, r#"{{"kind": 65536, "content": "invalid"}}"#).unwrap();
        writeln!(file, r#"{{"kind": 0, "content": "valid zero"}}"#).unwrap();
        file.flush().unwrap();

        let mut reader = InputReader::with_options(
            file.path().to_str().unwrap(),
            true, // enable filtering
        )
        .unwrap();

        let lines: Vec<String> = reader.by_ref().map(|r| r.unwrap()).collect();

        // Should only get 3 valid lines
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("valid"));
        assert!(lines[1].contains("valid max"));
        assert!(lines[2].contains("valid zero"));

        // Check that 2 lines were filtered
        assert_eq!(reader.filtered_count(), 2);
    }

    #[test]
    fn test_no_filtering_by_default() {
        // Create a temporary file with invalid kind
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"kind": 100000, "content": "invalid"}}"#).unwrap();
        file.flush().unwrap();

        // InputReader::new() uses with_options(input, false) so filtering is disabled
        let mut reader = InputReader::new(file.path().to_str().unwrap()).unwrap();
        let lines: Vec<String> = reader.by_ref().map(|r| r.unwrap()).collect();

        // The `new()` method explicitly disables filtering for backward compatibility
        assert_eq!(lines.len(), 1);
        assert_eq!(reader.filtered_count(), 0);
    }
}
