//! MQTT topic filter rules (MQTT 3.1.1 §4.7).
//!
//! Two things live here: validation of a filter the user typed, and the
//! matching rule that decides whether a topic is covered by a filter.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TopicError {
    #[error("topic filter must not be empty")]
    Empty,
    #[error("topic filter must not contain a null character")]
    NullCharacter,
    #[error("topic level must not be empty")]
    EmptyLevel,
    #[error("the multi-level wildcard '#' must occupy an entire level and be the last level")]
    MisplacedHash,
    #[error("the single-level wildcard '+' must occupy an entire level")]
    MisplacedPlus,
    #[error("a publish topic must not contain wildcards")]
    WildcardInPublishTopic,
}

/// Validate a topic filter, e.g. `device/+/status` or `sensor/#`.
pub fn validate_filter(filter: &str) -> Result<(), TopicError> {
    if filter.is_empty() {
        return Err(TopicError::Empty);
    }
    if filter.contains('\0') {
        return Err(TopicError::NullCharacter);
    }

    let levels: Vec<&str> = filter.split('/').collect();
    for (index, level) in levels.iter().enumerate() {
        if level.is_empty() {
            return Err(TopicError::EmptyLevel);
        }
        if level.contains('#') {
            if *level != "#" {
                return Err(TopicError::MisplacedHash);
            }
            if index != levels.len() - 1 {
                return Err(TopicError::MisplacedHash);
            }
        }
        if level.contains('+') && *level != "+" {
            return Err(TopicError::MisplacedPlus);
        }
    }

    Ok(())
}

/// Validate a topic used for publishing, which may not contain wildcards.
pub fn is_valid_publish_topic(topic: &str) -> Result<(), TopicError> {
    if topic.is_empty() {
        return Err(TopicError::Empty);
    }
    if topic.contains('\0') {
        return Err(TopicError::NullCharacter);
    }
    if topic.contains('#') || topic.contains('+') {
        return Err(TopicError::WildcardInPublishTopic);
    }
    Ok(())
}

/// Whether `topic` is covered by `filter`.
///
/// Both arguments must already be valid; use [`validate_filter`] first.
pub fn matches(filter: &str, topic: &str) -> bool {
    let filter_levels: Vec<&str> = filter.split('/').collect();
    let topic_levels: Vec<&str> = topic.split('/').collect();

    let mut consumed = 0usize;
    for (index, level) in filter_levels.iter().enumerate() {
        if *level == "#" {
            // `#` also matches the parent level, so `sport/#` matches `sport`.
            return true;
        }
        if consumed >= topic_levels.len() {
            return false;
        }
        if *level != "+" && *level != topic_levels[consumed] {
            return false;
        }
        consumed += 1;
        let _ = index;
    }

    consumed == topic_levels.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_filters() {
        for filter in [
            "sport",
            "sport/tennis",
            "sport/+/player1",
            "sport/#",
            "#",
            "+",
            "+/tennis/#",
        ] {
            assert!(validate_filter(filter).is_ok(), "{filter} should be valid");
        }
    }

    #[test]
    fn rejects_invalid_filters() {
        for filter in [
            "",
            "sport/tennis#",
            "sport/tennis/#/ranking",
            "sport+",
            "sport//player1",
        ] {
            assert!(
                validate_filter(filter).is_err(),
                "{filter} should be invalid"
            );
        }
    }

    #[test]
    fn rejects_wildcards_in_publish_topic() {
        assert!(is_valid_publish_topic("sport/tennis").is_ok());
        assert!(is_valid_publish_topic("sport/#").is_err());
        assert!(is_valid_publish_topic("sport/+").is_err());
    }

    #[test]
    fn matches_exact_topic() {
        assert!(matches("sport/tennis", "sport/tennis"));
        assert!(!matches("sport/tennis", "sport/tennis/player1"));
        assert!(!matches("sport/tennis", "sport"));
    }

    #[test]
    fn matches_single_level_wildcard() {
        assert!(matches("sport/+/player1", "sport/tennis/player1"));
        assert!(matches("sport/+", "sport/tennis"));
        assert!(!matches("sport/+", "sport"));
        assert!(!matches("sport/+", "sport/tennis/player1"));
    }

    #[test]
    fn matches_multi_level_wildcard() {
        assert!(matches("sport/#", "sport/tennis/player1"));
        assert!(matches("sport/#", "sport"));
        assert!(matches("#", "anything/at/all"));
        assert!(!matches("sport/#", "weather/temperature"));
    }
}
