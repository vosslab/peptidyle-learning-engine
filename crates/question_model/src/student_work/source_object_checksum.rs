use serde::{Deserialize, Serialize, de};

/// SHA-256 integrity evidence for bytes named by a Source Object Reference.
///
/// Construction and deserialization accept only canonical lowercase hexadecimal
/// SHA-256 values (ASVS 1.5.2, 2.2.1).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceObjectChecksum(String);

/// Failure to construct a canonical Source Object Checksum.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceObjectChecksumError;

impl std::fmt::Display for SourceObjectChecksumError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a lowercase 64-character hexadecimal SHA-256 checksum is required")
    }
}

impl std::error::Error for SourceObjectChecksumError {}

impl SourceObjectChecksum {
    /// Parses canonical SHA-256 integrity evidence.
    pub fn parse(value: impl Into<String>) -> Result<Self, SourceObjectChecksumError> {
        let value = value.into();
        if value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            Ok(Self(value))
        } else {
            Err(SourceObjectChecksumError)
        }
    }

    /// Returns the canonical lowercase hexadecimal checksum.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SourceObjectChecksum {
    type Error = SourceObjectChecksumError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl std::fmt::Display for SourceObjectChecksum {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for SourceObjectChecksum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SourceObjectChecksum {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?)
            .map_err(|error| de::Error::custom(error.to_string()))
    }
}
