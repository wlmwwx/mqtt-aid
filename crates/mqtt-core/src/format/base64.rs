use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use super::{FormatError, PayloadCodec};

/// Base64 codec.
#[derive(Debug, Clone, Copy, Default)]
pub struct Base64Codec;

impl PayloadCodec for Base64Codec {
    fn decode(&self, data: &[u8]) -> Result<String, FormatError> {
        Ok(STANDARD.encode(data))
    }

    fn encode(&self, input: &str) -> Result<Vec<u8>, FormatError> {
        STANDARD
            .decode(input.trim())
            .map_err(|error| FormatError::InvalidBase64 {
                reason: error.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let bytes = vec![0x00, 0x01, 0xfe, 0xff];
        let text = Base64Codec.decode(&bytes).unwrap();
        assert_eq!(text, "AAH+/w==");
        assert_eq!(Base64Codec.encode(&text).unwrap(), bytes);
    }

    #[test]
    fn rejects_invalid_base64() {
        assert!(Base64Codec.encode("not base64!!").is_err());
    }
}
