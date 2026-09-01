//! Optional browser-safe citation credit for one Question Revision.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Failure to construct a Question Citation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionCitationError {
    MissingCitation,
    InvalidCitationUrl,
    InvalidCitationText,
}

impl fmt::Display for QuestionCitationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingCitation => {
                "Question Citation requires Citation URL, Citation Text, or both"
            }
            Self::InvalidCitationUrl => {
                "Citation URL must be trimmed, control-free, and at most 2048 characters"
            }
            Self::InvalidCitationText => {
                "Citation Text must be trimmed, control-free, and at most 4000 characters"
            }
        })
    }
}

impl std::error::Error for QuestionCitationError {}

/// Optional citation for the source publication, textbook, website, or source Question.
///
/// Citation supplements Question Authorship and Question License. It never
/// represents Question Owner stewardship or a PLE fork relationship.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    deny_unknown_fields,
    try_from = "QuestionCitationWire",
    into = "QuestionCitationWire"
)]
pub struct QuestionCitation {
    pub citation_url: Option<String>,
    pub citation_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QuestionCitationWire {
    citation_url: Option<String>,
    citation_text: Option<String>,
}

impl QuestionCitation {
    pub fn new(
        citation_url: Option<String>,
        citation_text: Option<String>,
    ) -> Result<Self, QuestionCitationError> {
        let citation_url = validate_optional_text(
            citation_url,
            2048,
            QuestionCitationError::InvalidCitationUrl,
        )?;
        let citation_text = validate_optional_text(
            citation_text,
            4000,
            QuestionCitationError::InvalidCitationText,
        )?;
        if citation_url.is_none() && citation_text.is_none() {
            return Err(QuestionCitationError::MissingCitation);
        }
        Ok(Self {
            citation_url,
            citation_text,
        })
    }
}

impl TryFrom<QuestionCitationWire> for QuestionCitation {
    type Error = QuestionCitationError;

    fn try_from(value: QuestionCitationWire) -> Result<Self, Self::Error> {
        Self::new(value.citation_url, value.citation_text)
    }
}

impl From<QuestionCitation> for QuestionCitationWire {
    fn from(value: QuestionCitation) -> Self {
        Self {
            citation_url: value.citation_url,
            citation_text: value.citation_text,
        }
    }
}

fn validate_optional_text(
    value: Option<String>,
    maximum: usize,
    error: QuestionCitationError,
) -> Result<Option<String>, QuestionCitationError> {
    value
        .map(|text| {
            let trimmed = text.trim();
            if trimmed.is_empty()
                || trimmed.chars().count() > maximum
                || trimmed.chars().any(char::is_control)
            {
                return Err(error);
            }
            Ok(trimmed.to_owned())
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citation_requires_one_trimmed_bounded_credit_field() {
        let citation = QuestionCitation::new(
            Some(" https://example.test/source ".into()),
            Some(" Example et al. (2026). ".into()),
        )
        .expect("valid citation");
        assert_eq!(
            citation.citation_url.as_deref(),
            Some("https://example.test/source")
        );
        assert_eq!(
            citation.citation_text.as_deref(),
            Some("Example et al. (2026).")
        );
        assert!(QuestionCitation::new(None, None).is_err());
        assert!(QuestionCitation::new(Some("\n".into()), None).is_err());
    }
}
