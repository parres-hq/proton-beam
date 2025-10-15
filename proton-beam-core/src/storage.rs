//! Storage I/O for length-delimited protobuf events with optional gzip compression

use crate::{ProtoEvent, error::Result};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use prost::Message;
use std::io::{Read, Write};

/// Write a single event in length-delimited format
///
/// Format: [varint length][protobuf binary data]
///
/// This format allows multiple events to be written to the same file
/// and read back independently without loading the entire file.
///
/// # Example
///
/// ```no_run
/// use proton_beam_core::{ProtoEvent, write_event_delimited};
/// use std::fs::File;
///
/// let event = ProtoEvent {
///     id: "test".to_string(),
///     pubkey: "test".to_string(),
///     created_at: 123,
///     kind: 1,
///     tags: vec![],
///     content: "Hello".to_string(),
///     sig: "test".to_string(),
/// };
///
/// let mut file = File::create("events.pb")?;
/// write_event_delimited(&mut file, &event)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn write_event_delimited<W: Write>(writer: &mut W, event: &ProtoEvent) -> Result<()> {
    write_event_delimited_with_buf(writer, event, &mut DelimitedBuffer::default())
}

/// Write multiple events in length-delimited format
///
/// This is a convenience function that writes multiple events in one call.
pub fn write_events_delimited<W: Write>(writer: &mut W, events: &[ProtoEvent]) -> Result<()> {
    let mut buffer = DelimitedBuffer::default();
    for event in events {
        write_event_delimited_with_buf(writer, event, &mut buffer)?;
    }
    Ok(())
}

/// Read events from a length-delimited protobuf stream
///
/// Returns an iterator that yields events one at a time, allowing
/// memory-efficient processing of large files.
///
/// # Example
///
/// ```no_run
/// use proton_beam_core::read_events_delimited;
/// use std::fs::File;
///
/// let file = File::open("events.pb")?;
/// for result in read_events_delimited(file) {
///     let event = result?;
///     println!("Event ID: {}", event.id);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn read_events_delimited<R: Read>(reader: R) -> EventIterator<R> {
    EventIterator::new(reader)
}

/// Iterator over events in a length-delimited stream
pub struct EventIterator<R: Read> {
    reader: R,
    buffer: Vec<u8>,
}

impl<R: Read> EventIterator<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
        }
    }
}

impl<R: Read> Iterator for EventIterator<R> {
    type Item = Result<ProtoEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        // Read varint length
        let length = match read_varint(&mut self.reader) {
            Ok(len) => len as usize,
            Err(e) => {
                // Check if this is EOF (expected end of iteration)
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    return None;
                }
                return Some(Err(e.into()));
            }
        };

        // Prepare buffer
        self.buffer.clear();
        self.buffer.resize(length, 0);

        // Read message bytes
        if let Err(e) = self.reader.read_exact(&mut self.buffer) {
            return Some(Err(e.into()));
        }

        // Decode event
        match ProtoEvent::decode(&self.buffer[..]) {
            Ok(event) => Some(Ok(event)),
            Err(e) => Some(Err(e.into())),
        }
    }
}

/// Create a gzip encoder wrapper for writing compressed protobuf files
///
/// This wraps any writer with gzip compression. Use default compression level (6).
///
/// # Example
///
/// ```no_run
/// use proton_beam_core::{ProtoEvent, create_gzip_encoder, write_event_delimited};
/// use std::fs::File;
/// use std::io::BufWriter;
///
/// let file = File::create("events.pb.gz")?;
/// let mut gz = create_gzip_encoder(file);
/// let mut writer = BufWriter::new(gz);
///
/// let event = ProtoEvent {
///     id: "test".to_string(),
///     pubkey: "test".to_string(),
///     created_at: 123,
///     kind: 1,
///     tags: vec![],
///     content: "Hello".to_string(),
///     sig: "test".to_string(),
/// };
///
/// write_event_delimited(&mut writer, &event)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn create_gzip_encoder<W: Write>(writer: W) -> GzEncoder<W> {
    create_gzip_encoder_with_level(writer, 6)
}

/// Create a gzip decoder wrapper for reading compressed protobuf files
///
/// This wraps any reader with gzip decompression.
///
/// # Example
///
/// ```no_run
/// use proton_beam_core::{create_gzip_decoder, read_events_delimited};
/// use std::fs::File;
///
/// let file = File::open("events.pb.gz")?;
/// let gz = create_gzip_decoder(file);
///
/// for result in read_events_delimited(gz) {
///     let event = result?;
///     println!("Event ID: {}", event.id);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn create_gzip_decoder<R: Read>(reader: R) -> GzDecoder<R> {
    GzDecoder::new(reader)
}

pub fn create_gzip_encoder_with_level<W: Write>(writer: W, level: u32) -> GzEncoder<W> {
    GzEncoder::new(writer, Compression::new(level))
}

#[derive(Default)]
struct DelimitedBuffer {
    len_buf: Vec<u8>,
    event_buf: Vec<u8>,
}

fn write_event_delimited_with_buf<W: Write>(
    writer: &mut W,
    event: &ProtoEvent,
    buf: &mut DelimitedBuffer,
) -> Result<()> {
    buf.event_buf.clear();
    event.encode(&mut buf.event_buf)?;

    buf.len_buf.clear();
    prost::encoding::encode_varint(buf.event_buf.len() as u64, &mut buf.len_buf);
    writer.write_all(&buf.len_buf)?;
    writer.write_all(&buf.event_buf)?;

    Ok(())
}

/// Read a varint from a reader
fn read_varint<R: Read>(reader: &mut R) -> std::io::Result<u64> {
    let mut result = 0u64;
    let mut shift = 0;
    let mut buf = [0u8; 1];

    loop {
        reader.read_exact(&mut buf)?;
        let byte = buf[0];

        // Add the lower 7 bits to result
        result |= ((byte & 0x7F) as u64) << shift;

        // If the high bit is not set, we're done
        if byte & 0x80 == 0 {
            break;
        }

        shift += 7;

        // Prevent overflow
        if shift >= 64 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Varint too large",
            ));
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tag;
    use std::io::Cursor;

    fn create_test_event(id: &str) -> ProtoEvent {
        ProtoEvent {
            id: id.to_string(),
            pubkey: "test_pubkey".to_string(),
            created_at: 1234567890,
            kind: 1,
            tags: vec![Tag {
                values: vec!["p".to_string(), "test".to_string()],
            }],
            content: format!("Test content for event {}", id),
            sig: "test_signature".to_string(),
        }
    }

    #[test]
    fn test_write_and_read_single_event() {
        let event = create_test_event("event1");

        // Write to buffer
        let mut buffer = Vec::new();
        write_event_delimited(&mut buffer, &event).unwrap();

        // Read back
        let cursor = Cursor::new(buffer);
        let mut events: Vec<ProtoEvent> = read_events_delimited(cursor)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(events.len(), 1);
        let read_event = events.pop().unwrap();

        assert_eq!(read_event.id, event.id);
        assert_eq!(read_event.pubkey, event.pubkey);
        assert_eq!(read_event.created_at, event.created_at);
        assert_eq!(read_event.kind, event.kind);
        assert_eq!(read_event.content, event.content);
    }

    #[test]
    fn test_write_and_read_multiple_events() {
        let events = vec![
            create_test_event("event1"),
            create_test_event("event2"),
            create_test_event("event3"),
        ];

        // Write to buffer
        let mut buffer = Vec::new();
        write_events_delimited(&mut buffer, &events).unwrap();

        // Read back
        let cursor = Cursor::new(buffer);
        let read_events: Vec<ProtoEvent> = read_events_delimited(cursor)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(read_events.len(), 3);

        for (original, read) in events.iter().zip(read_events.iter()) {
            assert_eq!(original.id, read.id);
            assert_eq!(original.content, read.content);
        }
    }

    #[test]
    fn test_read_empty_stream() {
        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let events: Vec<ProtoEvent> = read_events_delimited(cursor)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_iterator_lazy_evaluation() {
        let events = vec![
            create_test_event("event1"),
            create_test_event("event2"),
            create_test_event("event3"),
        ];

        let mut buffer = Vec::new();
        write_events_delimited(&mut buffer, &events).unwrap();

        let cursor = Cursor::new(buffer);
        let mut iter = read_events_delimited(cursor);

        // Read only first event
        let first = iter.next().unwrap().unwrap();
        assert_eq!(first.id, "event1");

        // Read second event
        let second = iter.next().unwrap().unwrap();
        assert_eq!(second.id, "event2");

        // We can stop here without reading all events
    }

    #[test]
    fn test_varint_encoding() {
        // Test small value
        let mut buf = Vec::new();
        prost::encoding::encode_varint(42, &mut buf);
        let mut cursor = Cursor::new(buf);
        assert_eq!(read_varint(&mut cursor).unwrap(), 42);

        // Test larger value
        let mut buf = Vec::new();
        prost::encoding::encode_varint(300, &mut buf);
        let mut cursor = Cursor::new(buf);
        assert_eq!(read_varint(&mut cursor).unwrap(), 300);
    }

    #[test]
    fn test_corrupted_stream() {
        // Write valid event
        let event = create_test_event("event1");
        let mut buffer = Vec::new();
        write_event_delimited(&mut buffer, &event).unwrap();

        // Corrupt the data
        if buffer.len() > 5 {
            buffer[5] = 0xFF;
        }

        // Try to read - should get an error
        let cursor = Cursor::new(buffer);
        let result: Result<Vec<ProtoEvent>> = read_events_delimited(cursor).collect();

        assert!(result.is_err());
    }

    #[test]
    fn test_empty_event() {
        let event = ProtoEvent {
            id: String::new(),
            pubkey: String::new(),
            created_at: 0,
            kind: 0,
            tags: vec![],
            content: String::new(),
            sig: String::new(),
        };

        let mut buffer = Vec::new();
        write_event_delimited(&mut buffer, &event).unwrap();

        let cursor = Cursor::new(buffer);
        let events: Vec<ProtoEvent> = read_events_delimited(cursor)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "");
        assert_eq!(events[0].content, "");
    }

    #[test]
    fn test_gzip_compression_single_event() {
        let event = create_test_event("event1");

        // Write with compression
        let mut compressed = Vec::new();
        {
            let gz = create_gzip_encoder(&mut compressed);
            let mut writer = std::io::BufWriter::new(gz);
            write_event_delimited(&mut writer, &event).unwrap();
        } // GzEncoder gets dropped and finishes here

        // Read back with decompression to verify it works
        let cursor = Cursor::new(&compressed);
        let gz = create_gzip_decoder(cursor);
        let mut events: Vec<ProtoEvent> = read_events_delimited(gz)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(events.len(), 1);
        let read_event = events.pop().unwrap();
        assert_eq!(read_event.id, event.id);
        assert_eq!(read_event.content, event.content);

        // Note: For very small data, gzip overhead may make compressed data larger.
        // The compression benefit shows with larger datasets (tested in test_compression_ratio).
    }

    #[test]
    fn test_gzip_compression_multiple_events() {
        let events = vec![
            create_test_event("event1"),
            create_test_event("event2"),
            create_test_event("event3"),
        ];

        // Write with compression
        let mut compressed = Vec::new();
        {
            let gz = create_gzip_encoder(&mut compressed);
            let mut writer = std::io::BufWriter::new(gz);
            write_events_delimited(&mut writer, &events).unwrap();
        } // GzEncoder gets dropped and finishes here

        // Read back with decompression
        let cursor = Cursor::new(compressed);
        let gz = create_gzip_decoder(cursor);
        let read_events: Vec<ProtoEvent> = read_events_delimited(gz)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(read_events.len(), 3);

        for (original, read) in events.iter().zip(read_events.iter()) {
            assert_eq!(original.id, read.id);
            assert_eq!(original.content, read.content);
        }
    }

    #[test]
    fn test_compression_ratio() {
        // Create a more realistic event with repeated patterns
        let event = ProtoEvent {
            id: "a".repeat(64),
            pubkey: "b".repeat(64),
            created_at: 1234567890,
            kind: 1,
            tags: vec![
                Tag {
                    values: vec!["e".to_string(), "c".repeat(64)],
                },
                Tag {
                    values: vec!["p".to_string(), "d".repeat(64)],
                },
            ],
            content: "Hello, Nostr! ".repeat(10),
            sig: "e".repeat(128),
        };

        // Write uncompressed
        let mut uncompressed = Vec::new();
        write_event_delimited(&mut uncompressed, &event).unwrap();

        // Write compressed
        let mut compressed = Vec::new();
        {
            let gz = create_gzip_encoder(&mut compressed);
            let mut writer = std::io::BufWriter::new(gz);
            write_event_delimited(&mut writer, &event).unwrap();
        }

        let ratio = uncompressed.len() as f64 / compressed.len() as f64;
        println!(
            "Compression ratio: {:.2}x (uncompressed: {} bytes, compressed: {} bytes)",
            ratio,
            uncompressed.len(),
            compressed.len()
        );

        // Gzip should provide meaningful compression on repetitive data
        assert!(
            ratio > 1.5,
            "Expected compression ratio > 1.5x, got {:.2}x",
            ratio
        );
    }
}
