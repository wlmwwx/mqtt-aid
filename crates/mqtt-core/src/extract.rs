//! Taking a number out of a message payload.
//!
//! A data series watches a topic and reads one value out of every message it
//! receives: a field of a JSON document, or the whole payload when it is a
//! number by itself. This module is the rule for both.

use serde_json::Value;

use crate::format::parse_json;

/// One step of a field path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    /// A field of an object.
    Key(String),
    /// An element of an array.
    Index(usize),
}

/// Read a number out of a payload.
///
/// The path is a dotted field path such as `data.temperature`; an array
/// element is written `[0]`, as in `sensors[0].value`. An empty path means
/// the whole payload, which is then read as a number.
///
/// Numbers are taken as they are, `true` and `false` become `1` and `0`, and
/// a string is read as a number when it holds one. A payload that is not JSON
/// has no fields, so only the empty path can find anything in it.
pub fn extract_value(payload: &[u8], path: &str) -> Option<f64> {
    let text = String::from_utf8_lossy(payload);
    let path = path.trim();

    if path.is_empty() {
        return text.trim().parse::<f64>().ok();
    }

    let segments = parse_path(path)?;
    let value = parse_json(&text).ok()?;
    let value = walk(&value, &segments)?;
    to_number(value)
}

/// Split a path into its steps.
fn parse_path(path: &str) -> Option<Vec<Segment>> {
    let mut segments = Vec::new();
    let mut key = String::new();
    let mut characters = path.chars().peekable();

    while let Some(character) = characters.next() {
        match character {
            '.' => push_key(&mut segments, &mut key),
            '[' => {
                push_key(&mut segments, &mut key);
                let mut digits = String::new();
                let mut closed = false;
                for character in characters.by_ref() {
                    if character == ']' {
                        closed = true;
                        break;
                    }
                    digits.push(character);
                }
                if !closed {
                    return None;
                }
                segments.push(Segment::Index(digits.trim().parse().ok()?));
            }
            _ => key.push(character),
        }
    }
    push_key(&mut segments, &mut key);

    if segments.is_empty() {
        None
    } else {
        Some(segments)
    }
}

/// Move a field name into the list of steps.
fn push_key(segments: &mut Vec<Segment>, key: &mut String) {
    if !key.is_empty() {
        segments.push(Segment::Key(std::mem::take(key)));
    }
}

/// Follow a path into a JSON document.
fn walk<'a>(value: &'a Value, segments: &[Segment]) -> Option<&'a Value> {
    let mut current = value;
    for segment in segments {
        current = match segment {
            Segment::Key(key) => current.get(key.as_str())?,
            Segment::Index(index) => current.get(*index)?,
        };
    }
    Some(current)
}

/// The number a JSON value stands for.
fn to_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::Bool(flag) => Some(if *flag { 1.0 } else { 0.0 }),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(payload: &str, path: &str) -> Option<f64> {
        extract_value(payload.as_bytes(), path)
    }

    #[test]
    fn reads_a_top_level_field() {
        assert_eq!(value(r#"{"temperature": 21.5}"#, "temperature"), Some(21.5));
    }

    #[test]
    fn reads_a_nested_field() {
        assert_eq!(
            value(
                r#"{"data": {"room": {"temperature": 21.5}}}"#,
                "data.room.temperature"
            ),
            Some(21.5)
        );
    }

    #[test]
    fn reads_an_array_element() {
        assert_eq!(value(r#"{"sensors": [1, 2, 3]}"#, "sensors[1]"), Some(2.0));
    }

    #[test]
    fn reads_a_field_of_an_array_element() {
        assert_eq!(
            value(r#"{"sensors": [{"value": 7.25}]}"#, "sensors[0].value"),
            Some(7.25)
        );
    }

    #[test]
    fn reads_a_whole_number_payload() {
        assert_eq!(value("21.5", ""), Some(21.5));
        assert_eq!(value("  -3 \n", ""), Some(-3.0));
    }

    #[test]
    fn reads_a_boolean_as_one_and_zero() {
        assert_eq!(value(r#"{"on": true}"#, "on"), Some(1.0));
        assert_eq!(value(r#"{"on": false}"#, "on"), Some(0.0));
    }

    #[test]
    fn reads_a_number_written_as_a_string() {
        assert_eq!(value(r#"{"value": "21.5"}"#, "value"), Some(21.5));
    }

    #[test]
    fn has_nothing_to_say_about_a_missing_field() {
        assert_eq!(value(r#"{"temperature": 21.5}"#, "humidity"), None);
        assert_eq!(value(r#"{"temperature": 21.5}"#, "data.temperature"), None);
    }

    #[test]
    fn has_nothing_to_say_about_a_field_that_is_not_a_number() {
        assert_eq!(value(r#"{"name": "kitchen"}"#, "name"), None);
        assert_eq!(value(r#"{"list": [1, 2]}"#, "list"), None);
    }

    #[test]
    fn a_payload_that_is_not_json_has_no_fields() {
        assert_eq!(value("hello", "temperature"), None);
        assert_eq!(value("21.5", "temperature"), None);
    }

    #[test]
    fn a_broken_path_finds_nothing() {
        assert_eq!(value(r#"{"sensors": [1]}"#, "sensors[x]"), None);
        assert_eq!(value(r#"{"sensors": [1]}"#, "sensors["), None);
    }

    #[test]
    fn a_broken_document_finds_nothing() {
        assert_eq!(value(r#"{"temperature": }"#, "temperature"), None);
    }
}
