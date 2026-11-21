use super::{FormatError, PayloadCodec};

/// JSON codec.
///
/// Decoding pretty prints the document, encoding validates the user's input and
/// returns it unchanged (apart from surrounding whitespace) so that the exact
/// bytes the user typed are what gets published.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonCodec;

impl PayloadCodec for JsonCodec {
    fn decode(&self, data: &[u8]) -> Result<String, FormatError> {
        let text = std::str::from_utf8(data).map_err(|_| FormatError::InvalidUtf8)?;
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|error| json_error(&error))?;
        serde_json::to_string_pretty(&value).map_err(|error| json_error(&error))
    }

    fn encode(&self, input: &str) -> Result<Vec<u8>, FormatError> {
        let value: serde_json::Value =
            serde_json::from_str(input.trim()).map_err(|error| json_error(&error))?;
        serde_json::to_string(&value)
            .map(String::into_bytes)
            .map_err(|error| json_error(&error))
    }
}

/// Reformat a JSON document with the given indent width.
pub fn format_json(input: &str, indent: usize) -> Result<String, FormatError> {
    let value: serde_json::Value =
        serde_json::from_str(input.trim()).map_err(|error| json_error(&error))?;
    let indent_str = " ".repeat(indent);
    let formatter = serde_json::ser::PrettyFormatter::with_indent(indent_str.as_bytes());
    let mut buffer = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, formatter);
    serde::Serialize::serialize(&value, &mut serializer).map_err(|error| json_error(&error))?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

/// Compact a JSON document onto a single line.
pub fn minify_json(input: &str) -> Result<String, FormatError> {
    let value: serde_json::Value =
        serde_json::from_str(input.trim()).map_err(|error| json_error(&error))?;
    serde_json::to_string(&value).map_err(|error| json_error(&error))
}

/// Parse a JSON document into a tree the UI can walk.
pub fn parse_json(input: &str) -> Result<serde_json::Value, FormatError> {
    serde_json::from_str(input.trim()).map_err(|error| json_error(&error))
}

fn json_error(error: &serde_json::Error) -> FormatError {
    FormatError::InvalidJson {
        message: error.to_string(),
        line: error.line(),
        column: error.column(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_to_pretty_json() {
        let decoded = JsonCodec.decode(br#"{"b":2,"a":1}"#).unwrap();
        assert!(decoded.contains("\n"));
        assert!(decoded.contains("\"a\""));
    }

    #[test]
    fn rejects_invalid_json_with_position() {
        let error = JsonCodec.decode(b"{\"a\":").unwrap_err();
        match error {
            FormatError::InvalidJson { line, column, .. } => {
                assert!(line >= 1);
                assert!(column >= 1);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn encodes_valid_json() {
        let bytes = JsonCodec.encode("  {\"a\": 1}  ").unwrap();
        assert_eq!(bytes, br#"{"a":1}"#);
    }

    #[test]
    fn formats_and_minifies() {
        let formatted = format_json("{\"a\":1}", 4).unwrap();
        assert!(formatted.contains("    \"a\""));
        assert_eq!(minify_json("{\n  \"a\": 1\n}").unwrap(), "{\"a\":1}");
    }
}
