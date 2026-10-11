//! Immutable browser-safe Question Authorship display records.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Failure to construct reviewed Question Authorship.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionAuthorshipError {
    InvalidDisplayName,
    InvalidAuthors,
}

impl fmt::Display for QuestionAuthorshipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidDisplayName => {
                "a Question Author display name must not be blank, control-bearing, or over 120 characters"
            }
            Self::InvalidAuthors => {
                "Question Authorship requires between one and sixteen distinct Question Authors"
            }
        })
    }
}

impl std::error::Error for QuestionAuthorshipError {}

/// One reviewed public Question Author display record.
///
/// The durable database relationship may also bind this authored credit to an
/// Account. Browser contracts expose only the reviewed display name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct QuestionAuthorDisplayName(String);

impl QuestionAuthorDisplayName {
    pub fn new(value: String) -> Result<Self, QuestionAuthorshipError> {
        let trimmed = value.trim();
        if trimmed.is_empty()
            || trimmed.chars().count() > 120
            || trimmed.chars().any(char::is_control)
        {
            return Err(QuestionAuthorshipError::InvalidDisplayName);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for QuestionAuthorDisplayName {
    type Error = QuestionAuthorshipError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<QuestionAuthorDisplayName> for String {
    fn from(value: QuestionAuthorDisplayName) -> Self {
        value.0
    }
}

/// One browser-safe Question Author credit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionAuthor {
    pub display_name: QuestionAuthorDisplayName,
    /// The active PLE Instructor Profile that this immutable credit explicitly
    /// names, when the source supplied one. External attribution remains
    /// display-name-only.
    pub account_id: Option<crate::AccountId>,
}

/// Ordered immutable Question Authorship snapshot for one Published Question Revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    deny_unknown_fields,
    try_from = "QuestionAuthorshipWire",
    into = "QuestionAuthorshipWire"
)]
pub struct QuestionAuthorship {
    pub authors: Vec<QuestionAuthor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QuestionAuthorshipWire {
    authors: Vec<QuestionAuthor>,
}

impl TryFrom<QuestionAuthorshipWire> for QuestionAuthorship {
    type Error = QuestionAuthorshipError;

    fn try_from(value: QuestionAuthorshipWire) -> Result<Self, Self::Error> {
        Self::new(value.authors)
    }
}

impl From<QuestionAuthorship> for QuestionAuthorshipWire {
    fn from(value: QuestionAuthorship) -> Self {
        Self {
            authors: value.authors,
        }
    }
}

impl QuestionAuthorship {
    pub fn new(authors: Vec<QuestionAuthor>) -> Result<Self, QuestionAuthorshipError> {
        if authors.is_empty()
            || authors.len() > 16
            || authors
                .iter()
                .map(|author| author.display_name.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                != authors.len()
        {
            return Err(QuestionAuthorshipError::InvalidAuthors);
        }
        Ok(Self { authors })
    }
}
