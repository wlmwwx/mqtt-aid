//! The payload format engine.
//!
//! A payload is always a `Vec<u8>`. A codec only ever *views* those bytes
//! (decoding) or *re-creates* them from user input (encoding); nothing in this
//! module is allowed to rewrite the original bytes.

use thiserror::Error;

use crate::models::PayloadFormat;

mod base64;
mod binary;
mod hex;
mod json;
mod text;

pub use base64::Base64Codec;
pub use binary::BinaryCodec;
pub use hex::{HexCodec, decode_hex, encode_hex, hex_dump};
pub use json::{JsonCodec, format_json, minify_json, parse_json};
pub use text::TextCodec;

/// Everything that can go wrong while converting between bytes and text.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FormatError {
    #[error("invalid hex input at byte {position}: {reason}")]
    InvalidHex { position: usize, reason: String },
    #[error("payload is not valid UTF-8")]
    InvalidUtf8,
    #[error("invalid JSON at line {line}, column {column}: {message}")]
    InvalidJson {
        message: String,
        line: usize,
        column: usize,
    },
    #[error("invalid base64 input: {reason}")]
    InvalidBase64 { reason: String },
    #[error("input is empty")]
    Empty,
}

/// Converts between raw payload bytes and their textual representation.
pub trait PayloadCodec: Send + Sync {
    /// Render `data` as text.
    fn decode(&self, data: &[u8]) -> Result<String, FormatError>;

    /// Turn user input back into raw payload bytes.
    fn encode(&self, input: &str) -> Result<Vec<u8>, FormatError>;
}

/// Detect the most useful format for `data`.
///
/// The rules follow the product spec: valid JSON wins, otherwise readable
/// UTF-8 is shown as text, everything else falls back to hex.
pub fn detect_format(data: &[u8]) -> PayloadFormat {
    if data.is_empty() {
        return PayloadFormat::Text;
    }

    let Ok(text) = std::str::from_utf8(data) else {
        return PayloadFormat::Hex;
    };

    let trimmed = text.trim();
    if (trimmed.starts_with('{') || trimmed.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
    {
        return PayloadFormat::Json;
    }

    if text.chars().all(|c| !c.is_control() || c.is_whitespace()) {
        return PayloadFormat::Text;
    }

    PayloadFormat::Hex
}

/// The codec that belongs to a concrete payload format.
pub fn codec_for(format: PayloadFormat) -> Box<dyn PayloadCodec> {
    match format {
        PayloadFormat::Text => Box::new(TextCodec),
        PayloadFormat::Hex => Box::new(HexCodec),
        PayloadFormat::Json => Box::new(JsonCodec),
        PayloadFormat::Binary => Box::new(BinaryCodec),
        PayloadFormat::Base64 => Box::new(Base64Codec),
        PayloadFormat::Auto => Box::new(TextCodec),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_json() {
        assert_eq!(detect_format(br#"{"a":1}"#), PayloadFormat::Json);
        assert_eq!(detect_format(br#"[1,2,3]"#), PayloadFormat::Json);
    }

    #[test]
    fn detects_text() {
        assert_eq!(detect_format(b"hello world"), PayloadFormat::Text);
        assert_eq!(detect_format(b"hello\nworld"), PayloadFormat::Text);
        assert_eq!(detect_format(b"42"), PayloadFormat::Text);
        assert_eq!(detect_format(b""), PayloadFormat::Text);
    }

    #[test]
    fn detects_binary() {
        assert_eq!(detect_format(&[0x00, 0xff, 0x10]), PayloadFormat::Hex);
    }
}
