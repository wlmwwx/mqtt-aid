use super::{FormatError, PayloadCodec};

/// Binary codec.
///
/// A payload is raw bytes, so the only faithful textual rendering of "binary"
/// is the list of the byte values themselves. This codec therefore shows
/// space separated decimal byte values, and encodes them back.
#[derive(Debug, Clone, Copy, Default)]
pub struct BinaryCodec;

impl PayloadCodec for BinaryCodec {
    fn decode(&self, data: &[u8]) -> Result<String, FormatError> {
        Ok(data
            .iter()
            .map(|byte| byte.to_string())
            .collect::<Vec<_>>()
            .join(" "))
    }

    fn encode(&self, input: &str) -> Result<Vec<u8>, FormatError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let mut bytes = Vec::new();
        for (index, token) in trimmed.split_whitespace().enumerate() {
            let value = token.parse::<u16>().map_err(|_| FormatError::InvalidHex {
                position: index,
                reason: format!("{token:?} is not a byte value"),
            })?;
            if value > 255 {
                return Err(FormatError::InvalidHex {
                    position: index,
                    reason: format!("{value} is out of range for a byte"),
                });
            }
            bytes.push(value as u8);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_byte_values() {
        let bytes = vec![0u8, 1, 127, 255];
        let text = BinaryCodec.decode(&bytes).unwrap();
        assert_eq!(text, "0 1 127 255");
        assert_eq!(BinaryCodec.encode(&text).unwrap(), bytes);
    }

    #[test]
    fn rejects_out_of_range_values() {
        assert!(BinaryCodec.encode("256").is_err());
        assert!(BinaryCodec.encode("nope").is_err());
    }
}
