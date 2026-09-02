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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewed_question_authorship_is_bounded_and_ordered() {
        let ada = QuestionAuthor {
            display_name: QuestionAuthorDisplayName::new(" Ada Lovelace ".into())
                .expect("valid display name"),
        };
        let grace = QuestionAuthor {
            display_name: QuestionAuthorDisplayName::new("Grace Hopper".into())
                .expect("valid display name"),
        };
        assert_eq!(
            QuestionAuthorship::new(vec![ada.clone(), grace])
                .expect("valid authorship")
                .authors[0]
                .display_name
                .as_str(),
            "Ada Lovelace"
        );
        assert!(QuestionAuthorship::new(vec![ada.clone(), ada]).is_err());
        assert!(QuestionAuthorDisplayName::new("\n".into()).is_err());
        let sixteen_authors = (1..=16)
            .map(|position| QuestionAuthor {
                display_name: QuestionAuthorDisplayName::new(format!("Question Author {position}"))
                    .expect("bounded display name"),
            })
            .collect();
        assert!(QuestionAuthorship::new(sixteen_authors).is_ok());
        assert!(
            serde_json::from_value::<QuestionAuthorship>(serde_json::json!({"authors": []}))
                .is_err()
        );
    }
}
