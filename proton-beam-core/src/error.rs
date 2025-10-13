//! Error types for proton-beam-core

use thiserror::Error;

/// Result type alias for proton-beam-core operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for the library
#[derive(Error, Debug)]
pub enum Error {
    /// JSON parsing error
    #[error("JSON parsing failed: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Protobuf encoding error
    #[error("Protobuf encoding failed: {0}")]
    ProtobufEncode(#[from] prost::EncodeError),

    /// Protobuf decoding error
    #[error("Protobuf decoding failed: {0}")]
    ProtobufDecode(#[from] prost::DecodeError),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Validation error
    #[error("Validation failed: {0}")]
    Validation(#[from] ValidationError),

    /// Invalid event structure
    #[error("Invalid event: {0}")]
    InvalidEvent(String),

    /// Conversion error
    #[error("Conversion failed: {0}")]
    Conversion(String),
}

/// Validation-specific errors
#[derive(Error, Debug)]
pub enum ValidationError {
    /// Event ID does not match computed hash
    #[error("Event ID mismatch: expected {expected}, got {actual}")]
    EventIdMismatch { expected: String, actual: String },

    /// Signature verification failed
    #[error("Signature verification failed: {0}")]
    InvalidSignature(String),

    /// Invalid hex encoding
    #[error("Invalid hex encoding: {0}")]
    InvalidHex(String),

    /// Invalid timestamp
    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(i64),

    /// Invalid kind
    #[error("Invalid kind: {0}")]
    InvalidKind(i32),

    /// Nostr SDK error
    #[error("Nostr SDK error: {0}")]
    NostrSdk(String),
}

impl From<nostr_sdk::event::Error> for ValidationError {
    fn from(err: nostr_sdk::event::Error) -> Self {
        ValidationError::NostrSdk(err.to_string())
    }
}

impl From<nostr_sdk::key::Error> for ValidationError {
    fn from(err: nostr_sdk::key::Error) -> Self {
        ValidationError::NostrSdk(err.to_string())
    }
}
