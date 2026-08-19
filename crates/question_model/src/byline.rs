//! Immutable browser-safe publication attribution.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Failure to construct reviewed public attribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicBylineError {
    InvalidName,
    InvalidNames,
}
impl fmt::Display for PublicBylineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidName => {
                "public author name must not be blank, control-bearing, or over 120 characters"
            }
            Self::InvalidNames => "a public byline requires between one and sixteen distinct names",
        })
    }
}
impl std::error::Error for PublicBylineError {}

/// Reviewed display spelling. It deliberately carries no account identity or authority.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PublicAuthorName(String);

impl PublicAuthorName {
    pub fn new(value: String) -> Result<Self, PublicBylineError> {
        let trimmed = value.trim();
        if trimmed.is_empty()
            || trimmed.chars().count() > 120
            || trimmed.chars().any(char::is_control)
        {
            return Err(PublicBylineError::InvalidName);
        }
        Ok(Self(trimmed.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for PublicAuthorName {
    type Error = PublicBylineError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<PublicAuthorName> for String {
    fn from(value: PublicAuthorName) -> Self {
        value.0
    }
}

/// Ordered reviewed attribution snapshot persisted with a publication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    deny_unknown_fields,
    try_from = "PublicBylineWire",
    into = "PublicBylineWire"
)]
pub struct PublicByline {
    pub names: Vec<PublicAuthorName>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PublicBylineWire {
    names: Vec<PublicAuthorName>,
}
impl TryFrom<PublicBylineWire> for PublicByline {
    type Error = PublicBylineError;
    fn try_from(value: PublicBylineWire) -> Result<Self, Self::Error> {
        Self::new(value.names)
    }
}
impl From<PublicByline> for PublicBylineWire {
    fn from(value: PublicByline) -> Self {
        Self { names: value.names }
    }
}
impl PublicByline {
    pub fn new(names: Vec<PublicAuthorName>) -> Result<Self, PublicBylineError> {
        if names.is_empty()
            || names.len() > 16
            || names
                .iter()
                .map(PublicAuthorName::as_str)
                .collect::<BTreeSet<_>>()
                .len()
                != names.len()
        {
            return Err(PublicBylineError::InvalidNames);
        }
        Ok(Self { names })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reviewed_byline_is_bounded_and_ordered() {
        let a = PublicAuthorName::new(" Ada Lovelace ".into()).expect("valid");
        let b = PublicAuthorName::new("Grace Hopper".into()).expect("valid");
        assert_eq!(
            PublicByline::new(vec![a.clone(), b]).expect("valid").names[0].as_str(),
            "Ada Lovelace"
        );
        assert!(PublicByline::new(vec![a.clone(), a]).is_err());
        assert!(PublicAuthorName::new("\n".into()).is_err());
        assert!(serde_json::from_value::<PublicByline>(serde_json::json!({"names": []})).is_err());
    }
}
