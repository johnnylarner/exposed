//! Source timestamps preserve their original spelling without inventing a timezone.
use serde::Serialize;

/// Original source timestamp, validated without assigning a timezone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct SourceTimestamp(String);

impl SourceTimestamp {
    /// Original spelling retained from the source.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SourceTimestamp {
    type Error = SourceTimestampError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if chrono::DateTime::parse_from_rfc3339(&value).is_err()
            && chrono::NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S%.f").is_err()
        {
            return Err(SourceTimestampError(value));
        }
        Ok(Self(value))
    }
}

/// Invalid timestamp supplied by an external source.
#[derive(Debug, thiserror::Error)]
#[error("invalid source timestamp {0:?}")]
pub struct SourceTimestampError(String);
