use super::{FormatError, PayloadCodec};

/// Plain UTF-8 text codec.
///
/// Decoding is strict: bytes that are not valid UTF-8 are reported as an error
/// instead of being silently replaced, so the user can fall back to hex.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextCodec;

impl PayloadCodec for TextCodec {
    fn decode(&self, data: &[u8]) -> Result<String, FormatError> {
        String::from_utf8(data.to_vec()).map_err(|_| FormatError::InvalidUtf8)
    }

    fn encode(&self, input: &str) -> Result<Vec<u8>, FormatError> {
        Ok(input.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_text() {
        assert_eq!(TextCodec.decode("héllo".as_bytes()).unwrap(), "héllo");
        assert_eq!(TextCodec.encode("héllo").unwrap(), "héllo".as_bytes());
    }

    #[test]
    fn rejects_invalid_utf8() {
        assert_eq!(
            TextCodec.decode(&[0xff, 0xfe]).unwrap_err(),
            FormatError::InvalidUtf8
        );
    }
}
