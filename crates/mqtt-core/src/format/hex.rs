use super::{FormatError, PayloadCodec};

/// Hex codec: bytes as space separated uppercase hex digits.
///
/// The canonical hex *dump* (with offsets and an ASCII column) is a separate
/// rendering, see [`hex_dump`].
#[derive(Debug, Clone, Copy, Default)]
pub struct HexCodec;

impl PayloadCodec for HexCodec {
    fn decode(&self, data: &[u8]) -> Result<String, FormatError> {
        Ok(encode_hex(data))
    }

    fn encode(&self, input: &str) -> Result<Vec<u8>, FormatError> {
        decode_hex(input)
    }
}

/// Render bytes as space separated uppercase hex digits.
pub fn encode_hex(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 3);
    for (index, byte) in data.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(&format!("{byte:02X}"));
    }
    out
}

/// Parse hex input into bytes.
///
/// Accepts space separated and unspaced input, and tolerates `0x` prefixes and
/// separators such as `:` and `-`.
pub fn decode_hex(input: &str) -> Result<Vec<u8>, FormatError> {
    let mut digits = String::with_capacity(input.len());
    let mut byte_index = 0usize;

    let chars = input.chars().peekable();
    for ch in chars {
        if ch.is_whitespace() || ch == ':' || ch == '-' || ch == ',' {
            byte_index += 1;
            continue;
        }
        if (ch == 'x' || ch == 'X') && digits.ends_with('0') && digits.len() == 1 {
            // A `0x` prefix directly in front of the first byte.
            digits.pop();
            continue;
        }
        if !ch.is_ascii_hexdigit() {
            return Err(FormatError::InvalidHex {
                position: byte_index,
                reason: format!("{ch:?} is not a hex digit"),
            });
        }
        digits.push(ch);
    }

    if !digits.len().is_multiple_of(2) {
        return Err(FormatError::InvalidHex {
            position: digits.len() / 2,
            reason: "odd number of hex digits".to_string(),
        });
    }

    let mut bytes = Vec::with_capacity(digits.len() / 2);
    let raw = digits.as_bytes();
    for pair in raw.chunks(2) {
        let high = (pair[0] as char).to_digit(16).unwrap() as u8;
        let low = (pair[1] as char).to_digit(16).unwrap() as u8;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

/// Render bytes as a classic hex dump: offset, hex columns and ASCII.
pub fn hex_dump(data: &[u8], bytes_per_line: usize) -> String {
    let bytes_per_line = bytes_per_line.max(1);
    let mut out = String::new();

    for (line_index, chunk) in data.chunks(bytes_per_line).enumerate() {
        let offset = line_index * bytes_per_line;
        out.push_str(&format!("{offset:08x}  "));

        for index in 0..bytes_per_line {
            if index < chunk.len() {
                out.push_str(&format!("{:02x} ", chunk[index]));
            } else {
                out.push_str("   ");
            }
            if bytes_per_line >= 2 && index == bytes_per_line / 2 - 1 {
                out.push(' ');
            }
        }

        out.push_str(" |");
        for byte in chunk {
            let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '.'
            };
            out.push(ch);
        }
        out.push_str("|\n");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_bytes() {
        assert_eq!(encode_hex(&[0x01, 0x03, 0x00, 0x0a]), "01 03 00 0A");
        assert_eq!(encode_hex(&[]), "");
    }

    #[test]
    fn decodes_spaced_and_unspaced() {
        assert_eq!(
            decode_hex("01 03 00 0A").unwrap(),
            vec![0x01, 0x03, 0x00, 0x0a]
        );
        assert_eq!(
            decode_hex("0103000A").unwrap(),
            vec![0x01, 0x03, 0x00, 0x0a]
        );
        assert_eq!(
            decode_hex("01:03-00,0A").unwrap(),
            vec![0x01, 0x03, 0x00, 0x0a]
        );
        assert_eq!(decode_hex("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn rejects_bad_hex() {
        assert!(decode_hex("01030").is_err());
        assert!(decode_hex("zz").is_err());
    }

    #[test]
    fn round_trips_through_the_codec() {
        let codec = HexCodec;
        let bytes = vec![0xde, 0xad, 0xbe, 0xef];
        let text = codec.decode(&bytes).unwrap();
        assert_eq!(codec.encode(&text).unwrap(), bytes);
    }

    #[test]
    fn dumps_with_offset_and_ascii() {
        let dump = hex_dump(b"Hello MQTT", 8);
        assert!(dump.contains("00000000"));
        assert!(dump.contains("48 65 6c 6c"));
        assert!(dump.contains("6f 20 4d 51"));
        assert!(dump.contains("|Hello MQ|"));
        assert!(dump.contains("00000008"));
    }
}
