use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to get the path to the sample events file
fn sample_events_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("sample_events.jsonl")
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Convert Nostr events"))
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("proton-beam"));
}

#[test]
fn test_convert_help() {
    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Convert Nostr events"))
        .stdout(predicate::str::contains("--output-dir"))
        .stdout(predicate::str::contains("--no-validate"))
        .stdout(predicate::str::contains("--batch-size"));
}

#[test]
fn test_convert_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg("/nonexistent/file.jsonl")
        .arg("--output-dir")
        .arg(temp_dir.path());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_convert_sample_events() {
    let temp_dir = TempDir::new().unwrap();
    let sample_path = sample_events_path();

    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg(&sample_path)
        .arg("--output-dir")
        .arg(temp_dir.path())
        .arg("--no-progress");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Conversion Summary"))
        .stdout(predicate::str::contains("Valid events:"))
        .stdout(predicate::str::contains("Success rate:"));

    // Check that output files were created
    let pb_files: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.ends_with(".pb.gz"))
        })
        .collect();

    assert!(!pb_files.is_empty(), "No .pb.gz files were created");

    // Check that log file exists
    let log_file = temp_dir.path().join("proton-beam.log");
    assert!(log_file.exists(), "proton-beam.log was not created");
}

#[test]
fn test_convert_with_no_validation() {
    let temp_dir = TempDir::new().unwrap();
    let sample_path = sample_events_path();

    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg(&sample_path)
        .arg("--output-dir")
        .arg(temp_dir.path())
        .arg("--no-validate")
        .arg("--no-progress");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Conversion Summary"));
}

#[test]
fn test_convert_with_custom_batch_size() {
    let temp_dir = TempDir::new().unwrap();
    let sample_path = sample_events_path();

    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg(&sample_path)
        .arg("--output-dir")
        .arg(temp_dir.path())
        .arg("--batch-size")
        .arg("10")
        .arg("--no-progress");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Conversion Summary"));
}

#[test]
fn test_convert_from_stdin() {
    let temp_dir = TempDir::new().unwrap();
    let sample_path = sample_events_path();

    // Read first 10 lines as test input
    let content = fs::read_to_string(sample_path).unwrap();
    let input: String = content.lines().take(10).collect::<Vec<_>>().join("\n");

    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg("-")
        .arg("--output-dir")
        .arg(temp_dir.path())
        .arg("--no-progress")
        .write_stdin(input);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Conversion Summary"))
        .stdout(predicate::str::contains("Total lines processed: 10"));
}

#[test]
fn test_error_logging() {
    let temp_dir = TempDir::new().unwrap();

    // Create a test file with some invalid JSON
    let test_file = temp_dir.path().join("test_input.jsonl");
    fs::write(
        &test_file,
        r#"{"id": "invalid", "not": "complete"}
{"id": "859501854a0e2b63383db18f187f8d2a7f988651793687215a6549f2da380528", "sig": "d693cca65af7df2619be909042f5b11a4e4bbe32932d5aa6ac22eb20c6e0551ab6e34690eddcbc76d893d64e60b6bf1c9838b02dea0eb1c05b38b28a700061cf", "kind": 7, "tags": [["e", "43f5606a0ceff70c40800855ffc24f2690d04c99d28a76cbdfdfe0c16737d7b4"], ["p", "f9c8838736f5a0b611ed2c458a8ae7a480802e4ec38e52e96483986ca44ce612"]], "pubkey": "7776c32d4b1d1e8bf2a96babeb43ad9ade157bd363d89b87fb63e6f145558888", "content": "🤙", "created_at": 1758991030}
"#,
    ).unwrap();

    let output_dir = temp_dir.path().join("output");

    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg(&test_file)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--no-progress")
        .arg("--no-validate"); // Don't validate signatures in test

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Invalid events:").and(predicate::str::contains("1")));

    // Check that errors were logged to the log file (not JSONL)
    let log_file = output_dir.join("proton-beam.log");
    assert!(log_file.exists(), "proton-beam.log should exist");

    // Verify the log file has content
    let content = fs::read_to_string(&log_file).unwrap();
    assert!(!content.is_empty(), "Log file should not be empty");
    assert!(content.contains("parse_error"), "Log should contain parse error");
}

#[test]
fn test_date_based_organization() {
    let temp_dir = TempDir::new().unwrap();
    let sample_path = sample_events_path();

    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg(&sample_path)
        .arg("--output-dir")
        .arg(temp_dir.path())
        .arg("--no-progress");

    cmd.assert().success();

    // Look for .pb.gz files in date format (YYYY_MM_DD.pb.gz)
    let pb_files: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| {
                    // Check if filename matches YYYY_MM_DD.pb.gz pattern
                    if !name.ends_with(".pb.gz") {
                        return false;
                    }
                    // Strip the .pb.gz extension and check the date format
                    let stem = &name[..name.len() - 6]; // Remove ".pb.gz"
                    let parts: Vec<&str> = stem.split('_').collect();
                    parts.len() == 3
                        && parts[0].len() == 4
                        && parts[1].len() == 2
                        && parts[2].len() == 2
                })
        })
        .collect();

    assert!(
        !pb_files.is_empty(),
        "Should have at least one date-formatted .pb.gz file"
    );
}

#[test]
fn test_verbose_output() {
    let temp_dir = TempDir::new().unwrap();
    let sample_path = sample_events_path();

    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg(&sample_path)
        .arg("--output-dir")
        .arg(temp_dir.path())
        .arg("--verbose")
        .arg("--no-progress");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Starting Proton Beam CLI"))
        .stderr(predicate::str::contains("Batch size:"));
}

#[test]
fn test_empty_file() {
    let temp_dir = TempDir::new().unwrap();

    // Create an empty file
    let test_file = temp_dir.path().join("empty.jsonl");
    fs::write(&test_file, "").unwrap();

    let output_dir = temp_dir.path().join("output");

    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg(&test_file)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--no-progress");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Total lines processed: 0"));
}

#[test]
fn test_file_with_blank_lines() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file with blank lines
    let test_file = temp_dir.path().join("with_blanks.jsonl");
    fs::write(
        &test_file,
        r#"
{"id": "859501854a0e2b63383db18f187f8d2a7f988651793687215a6549f2da380528", "sig": "d693cca65af7df2619be909042f5b11a4e4bbe32932d5aa6ac22eb20c6e0551ab6e34690eddcbc76d893d64e60b6bf1c9838b02dea0eb1c05b38b28a700061cf", "kind": 7, "tags": [["e", "43f5606a0ceff70c40800855ffc24f2690d04c99d28a76cbdfdfe0c16737d7b4"], ["p", "f9c8838736f5a0b611ed2c458a8ae7a480802e4ec38e52e96483986ca44ce612"]], "pubkey": "7776c32d4b1d1e8bf2a96babeb43ad9ade157bd363d89b87fb63e6f145558888", "content": "🤙", "created_at": 1758991030}

{"id": "56aa4f81df193b084e2cb85fa1552e94f16246c6eba6db010891729b02f436b7", "sig": "8ffd678e0fb8ce574d132fb98f3c3af8ad9c3ff00f2eab64babc05e5a981c81da5a6acb699c80ac85474155f6df091785f745c56c2d19e743d97ae527e750390", "kind": 1, "tags": [["e", "99f4f259c390d31451d4ebdbdd50f6731abb17d2e3749b1d47b3bc2584937620", "", "root"], ["p", "99cefa645b00817373239aebb96d2d1990244994e5e565566c82c04b8dc65b54"]], "pubkey": "01d0bbf9537ef1fd0ddf815f41c1896738f6a3a0f600f51c782b7d8891130d4c", "content": "Test content", "created_at": 1758991030}

"#,
    ).unwrap();

    let output_dir = temp_dir.path().join("output");

    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg(&test_file)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--no-progress")
        .arg("--no-validate"); // Don't validate signatures in test

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Valid events:       2"))
        .stdout(predicate::str::contains("Skipped lines:      3")); // 3 because of empty lines at start/middle/end
}

#[test]
fn test_index_creation() {
    let temp_dir = TempDir::new().unwrap();
    let sample_path = sample_events_path();

    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg(&sample_path)
        .arg("--output-dir")
        .arg(temp_dir.path())
        .arg("--no-progress");

    cmd.assert().success();

    // Check that index was created
    let index_file = temp_dir.path().join("index.db");
    assert!(index_file.exists(), "Index database was not created");
}

#[test]
fn test_deduplication() {
    let temp_dir = TempDir::new().unwrap();

    // Create a test file with duplicate events
    let test_file = temp_dir.path().join("test.jsonl");
    let event_json = r#"{"id": "859501854a0e2b63383db18f187f8d2a7f988651793687215a6549f2da380528", "sig": "d693cca65af7df2619be909042f5b11a4e4bbe32932d5aa6ac22eb20c6e0551ab6e34690eddcbc76d893d64e60b6bf1c9838b02dea0eb1c05b38b28a700061cf", "kind": 7, "tags": [["e", "43f5606a0ceff70c40800855ffc24f2690d04c99d28a76cbdfdfe0c16737d7b4"], ["p", "f9c8838736f5a0b611ed2c458a8ae7a480802e4ec38e52e96483986ca44ce612"]], "pubkey": "7776c32d4b1d1e8bf2a96babeb43ad9ade157bd363d89b87fb63e6f145558888", "content": "🤙", "created_at": 1758991030}"#;

    fs::write(
        &test_file,
        format!("{}\n{}\n{}\n", event_json, event_json, event_json), // Same event 3 times
    )
    .unwrap();

    let output_dir = temp_dir.path().join("output");

    // First conversion
    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg(&test_file)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--no-progress")
        .arg("--no-validate");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Valid events:       1"))
        .stdout(predicate::str::contains("Duplicate events:   2"));

    // Second conversion - all should be duplicates
    let mut cmd2 = Command::cargo_bin("proton-beam").unwrap();
    cmd2.arg("convert")
        .arg(&test_file)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--no-progress")
        .arg("--no-validate");

    cmd2.assert()
        .success()
        .stdout(predicate::str::contains("Valid events:       0"))
        .stdout(predicate::str::contains("Duplicate events:   3"));
}

#[test]
fn test_custom_index_path() {
    let temp_dir = TempDir::new().unwrap();
    let sample_path = sample_events_path();
    let custom_index = temp_dir.path().join("custom_index.db");

    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg(&sample_path)
        .arg("--output-dir")
        .arg(temp_dir.path())
        .arg("--index-path")
        .arg(&custom_index)
        .arg("--no-progress");

    cmd.assert().success();

    // Check that custom index was created
    assert!(
        custom_index.exists(),
        "Custom index database was not created"
    );

    // Default index should not exist
    let default_index = temp_dir.path().join("index.db");
    assert!(
        !default_index.exists(),
        "Default index was created when custom path was specified"
    );
}

#[test]
fn test_deduplication_across_batches() {
    let temp_dir = TempDir::new().unwrap();

    // Create a test file with events
    let test_file = temp_dir.path().join("test.jsonl");
    let event1 = r#"{"id": "859501854a0e2b63383db18f187f8d2a7f988651793687215a6549f2da380528", "sig": "d693cca65af7df2619be909042f5b11a4e4bbe32932d5aa6ac22eb20c6e0551ab6e34690eddcbc76d893d64e60b6bf1c9838b02dea0eb1c05b38b28a700061cf", "kind": 7, "tags": [["e", "43f5606a0ceff70c40800855ffc24f2690d04c99d28a76cbdfdfe0c16737d7b4"], ["p", "f9c8838736f5a0b611ed2c458a8ae7a480802e4ec38e52e96483986ca44ce612"]], "pubkey": "7776c32d4b1d1e8bf2a96babeb43ad9ade157bd363d89b87fb63e6f145558888", "content": "🤙", "created_at": 1758991030}"#;
    let event2 = r#"{"id": "56aa4f81df193b084e2cb85fa1552e94f16246c6eba6db010891729b02f436b7", "sig": "8ffd678e0fb8ce574d132fb98f3c3af8ad9c3ff00f2eab64babc05e5a981c81da5a6acb699c80ac85474155f6df091785f745c56c2d19e743d97ae527e750390", "kind": 1, "tags": [["e", "99f4f259c390d31451d4ebdbdd50f6731abb17d2e3749b1d47b3bc2584937620", "", "root"], ["p", "99cefa645b00817373239aebb96d2d1990244994e5e565566c82c04b8dc65b54"]], "pubkey": "01d0bbf9537ef1fd0ddf815f41c1896738f6a3a0f600f51c782b7d8891130d4c", "content": "Test content", "created_at": 1758991030}"#;

    fs::write(&test_file, format!("{}\n{}\n", event1, event2)).unwrap();

    let output_dir = temp_dir.path().join("output");

    // First conversion with small batch size
    let mut cmd = Command::cargo_bin("proton-beam").unwrap();
    cmd.arg("convert")
        .arg(&test_file)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--batch-size")
        .arg("1") // Very small batch to test across batches
        .arg("--no-progress")
        .arg("--no-validate");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Valid events:       2"));

    // Second conversion - both should be duplicates
    let mut cmd2 = Command::cargo_bin("proton-beam").unwrap();
    cmd2.arg("convert")
        .arg(&test_file)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--no-progress")
        .arg("--no-validate");

    cmd2.assert()
        .success()
        .stdout(predicate::str::contains("Valid events:       0"))
        .stdout(predicate::str::contains("Duplicate events:   2"));
}
