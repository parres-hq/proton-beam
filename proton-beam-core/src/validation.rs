//! Event validation functionality

use crate::{
    ProtoEvent,
    error::{Result, ValidationError},
};
use hex::FromHex;
use secp256k1::schnorr::Signature;
use secp256k1::{Message, Secp256k1, XOnlyPublicKey};
use serde_json::json;
use sha2::{Digest, Sha256};

/// Validate a Protobuf ProtoEvent
///
/// Performs comprehensive validation including:
/// - Event ID verification (SHA-256 hash)
/// - Schnorr signature verification
/// - Basic field validation
///
/// # Example
///
/// ```no_run
/// use proton_beam_core::{json_to_proto, validate_event};
///
/// let json = r#"{"id":"...","pubkey":"...","created_at":123,"kind":1,"tags":[],"content":"Hello","sig":"..."}"#;
/// let event = json_to_proto(json)?;
/// validate_event(&event)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn validate_event(event: &ProtoEvent) -> Result<()> {
    validate_basic_fields(event)?;

    let hash = compute_event_hash(event)?;
    validate_event_id_from_hash(event, &hash)?;
    validate_signature_from_hash(event, &hash)?;

    Ok(())
}

/// Validate basic fields without cryptographic verification
///
/// This is faster than full validation and useful for filtering
/// obviously invalid events before expensive crypto operations.
pub fn validate_basic_fields(event: &ProtoEvent) -> Result<()> {
    // Check ID format (64 hex characters)
    if event.id.len() != 64 || !is_hex(&event.id) {
        return Err(ValidationError::InvalidHex(format!(
            "Event ID must be 64 hex characters, got: {}",
            event.id
        ))
        .into());
    }

    // Check pubkey format (64 hex characters)
    if event.pubkey.len() != 64 || !is_hex(&event.pubkey) {
        return Err(ValidationError::InvalidHex(format!(
            "Pubkey must be 64 hex characters, got: {}",
            event.pubkey
        ))
        .into());
    }

    // Check signature format (128 hex characters)
    if event.sig.len() != 128 || !is_hex(&event.sig) {
        return Err(ValidationError::InvalidHex(format!(
            "Signature must be 128 hex characters, got: {}",
            event.sig
        ))
        .into());
    }

    // Check timestamp is reasonable (not negative)
    if event.created_at < 0 {
        return Err(ValidationError::InvalidTimestamp(event.created_at).into());
    }

    // Check kind is in valid range (0-65535)
    if event.kind < 0 || event.kind > 65535 {
        return Err(ValidationError::InvalidKind(event.kind).into());
    }

    Ok(())
}

/// Validate that the event ID matches the computed hash
/// Check if a string is valid hexadecimal
fn is_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn validate_signature_only(event: &ProtoEvent) -> Result<()> {
    validate_basic_fields(event)?;
    let hash = compute_event_hash(event)?;
    validate_signature_from_hash(event, &hash)
}

pub fn validate_event_id_only(event: &ProtoEvent) -> Result<()> {
    let hash = compute_event_hash(event)?;
    validate_event_id_from_hash(event, &hash)
}

/// Compute the SHA-256 hash of an event following Nostr's canonical format
///
/// This hash is used for both event ID verification and signature verification.
/// Exposing this allows callers to compute the hash once and reuse it for both validations.
pub fn compute_event_hash(event: &ProtoEvent) -> Result<[u8; 32]> {
    let tags: Vec<Vec<&str>> = event
        .tags
        .iter()
        .map(|tag| tag.values.iter().map(|v| v.as_str()).collect())
        .collect();

    let canonical = json!([
        0,
        event.pubkey,
        event.created_at,
        event.kind,
        tags,
        event.content,
    ]);

    let bytes = serde_json::to_vec(&canonical)?;
    let digest = Sha256::digest(&bytes);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&digest);
    Ok(hash)
}

/// Validate event ID against a pre-computed hash
///
/// This is useful when you've already computed the hash for signature verification
/// and want to avoid recomputing it for ID verification.
pub fn validate_event_id_from_hash(event: &ProtoEvent, hash: &[u8; 32]) -> Result<()> {
    let computed_id = hex::encode(hash);

    if computed_id != event.id {
        return Err(ValidationError::EventIdMismatch {
            expected: event.id.clone(),
            actual: computed_id,
        }
        .into());
    }

    Ok(())
}

/// Validate signature against a pre-computed hash
///
/// This is useful when you've already computed the hash for ID verification
/// and want to avoid recomputing it for signature verification.
pub fn validate_signature_from_hash(event: &ProtoEvent, hash: &[u8; 32]) -> Result<()> {
    let signature = parse_signature(&event.sig)?;
    let pubkey = parse_pubkey(&event.pubkey)?;
    let message = Message::from_digest_slice(hash).expect("hash length is 32 bytes");

    let secp = Secp256k1::verification_only();
    secp.verify_schnorr(&signature, &message, &pubkey)
        .map_err(|_| {
            ValidationError::InvalidSignature("Signature verification failed".to_string())
        })?;

    Ok(())
}

fn parse_signature(sig_hex: &str) -> Result<Signature> {
    let bytes =
        Vec::from_hex(sig_hex).map_err(|e| ValidationError::SignatureParse(e.to_string()))?;
    let signature = Signature::from_slice(&bytes)
        .map_err(|e| ValidationError::SignatureParse(e.to_string()))?;
    Ok(signature)
}

fn parse_pubkey(pubkey_hex: &str) -> Result<XOnlyPublicKey> {
    let bytes =
        Vec::from_hex(pubkey_hex).map_err(|e| ValidationError::PubkeyParse(e.to_string()))?;
    let pubkey = XOnlyPublicKey::from_slice(&bytes)
        .map_err(|e| ValidationError::PubkeyParse(e.to_string()))?;
    Ok(pubkey)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These are example hex values, not real cryptographic signatures
    // In real tests with actual Nostr events, validation would work correctly

    #[test]
    fn test_validate_basic_fields_valid() {
        let event = ProtoEvent {
            id: "a".repeat(64),
            pubkey: "b".repeat(64),
            created_at: 1234567890,
            kind: 1,
            tags: vec![],
            content: "test".to_string(),
            sig: "c".repeat(128),
        };

        assert!(validate_basic_fields(&event).is_ok());
    }

    #[test]
    fn test_validate_basic_fields_invalid_id_length() {
        let event = ProtoEvent {
            id: "short".to_string(),
            pubkey: "b".repeat(64),
            created_at: 1234567890,
            kind: 1,
            tags: vec![],
            content: "test".to_string(),
            sig: "c".repeat(128),
        };

        assert!(validate_basic_fields(&event).is_err());
    }

    #[test]
    fn test_validate_basic_fields_invalid_id_hex() {
        let event = ProtoEvent {
            id: "g".repeat(64), // 'g' is not a hex character
            pubkey: "b".repeat(64),
            created_at: 1234567890,
            kind: 1,
            tags: vec![],
            content: "test".to_string(),
            sig: "c".repeat(128),
        };

        assert!(validate_basic_fields(&event).is_err());
    }

    #[test]
    fn test_validate_basic_fields_negative_timestamp() {
        let event = ProtoEvent {
            id: "a".repeat(64),
            pubkey: "b".repeat(64),
            created_at: -1,
            kind: 1,
            tags: vec![],
            content: "test".to_string(),
            sig: "c".repeat(128),
        };

        assert!(validate_basic_fields(&event).is_err());
    }

    #[test]
    fn test_validate_basic_fields_invalid_kind() {
        let event = ProtoEvent {
            id: "a".repeat(64),
            pubkey: "b".repeat(64),
            created_at: 1234567890,
            kind: 70000, // exceeds max of 65535
            tags: vec![],
            content: "test".to_string(),
            sig: "c".repeat(128),
        };

        assert!(validate_basic_fields(&event).is_err());
    }

    #[test]
    fn test_is_hex() {
        assert!(is_hex("0123456789abcdef"));
        assert!(is_hex("ABCDEF"));
        assert!(is_hex(""));
        assert!(!is_hex("ghij"));
        assert!(!is_hex("0x123"));
        assert!(!is_hex("hello"));
    }

    #[test]
    fn test_validate_basic_fields_comprehensive() {
        // Test that basic field validation works without crypto
        let event = ProtoEvent {
            id: "a".repeat(64),
            pubkey: "b".repeat(64),
            created_at: 1671217411,
            kind: 1,
            tags: vec![],
            content: "test".to_string(),
            sig: "c".repeat(128),
        };

        // Basic fields should pass
        assert!(validate_basic_fields(&event).is_ok());

        // Note: Full validation with validate_event() requires proper nostr-sdk Event
        // which requires valid signatures. For testing invalid signatures, we would need
        // to generate real keypairs and create properly signed events, then modify them.
        // This is more appropriate for integration tests.
    }

    // Note: To test with real valid Nostr events, you would need to:
    // 1. Generate a real keypair
    // 2. Create and sign an event properly
    // 3. Then validate it
    //
    // For now, we're testing the validation logic with mock data
    // Integration tests will use real Nostr events from sample_events.jsonl
}
