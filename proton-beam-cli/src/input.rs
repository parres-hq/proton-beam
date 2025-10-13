use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader, Lines, StdinLock};
use std::path::Path;

/// Enum to hold either file or stdin reader
enum ReaderType {
    File(Lines<BufReader<File>>),
    Stdin(Lines<StdinLock<'static>>),
}

/// Input reader that can handle both files and stdin
pub struct InputReader {
    reader: ReaderType,
}

impl InputReader {
    /// Create a new input reader
    /// - If input is "-", reads from stdin
    /// - Otherwise, reads from the specified file
    pub fn new(input: &str) -> Result<Self> {
        let reader = if input == "-" {
            // Read from stdin
            let stdin = Box::leak(Box::new(std::io::stdin()));
            let lines = stdin.lock().lines();
            ReaderType::Stdin(lines)
        } else {
            // Read from file
            let path = Path::new(input);
            if !path.exists() {
                anyhow::bail!("Input file does not exist: {}", input);
            }

            let file = File::open(path).context(format!("Failed to open input file: {}", input))?;
            let reader = BufReader::with_capacity(1024 * 1024, file); // 1MB buffer
            ReaderType::File(reader.lines())
        };

        Ok(Self { reader })
    }
}

impl Iterator for InputReader {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.reader {
            ReaderType::File(lines) => lines
                .next()
                .map(|result| result.context("Failed to read line from file")),
            ReaderType::Stdin(lines) => lines
                .next()
                .map(|result| result.context("Failed to read line from stdin")),
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
}
