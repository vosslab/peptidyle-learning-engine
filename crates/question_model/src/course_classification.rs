//! Current Course metadata, independent of reusable content Revisions.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Tag;

/// Exact selected global vocabulary identities and free-form Course Tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseClassification {
    /// Exactly one mandatory Discipline, including when Subject is absent.
    pub discipline_uuid: Uuid,
    /// Optional Subject associated with the selected Discipline.
    pub subject_uuid: Option<Uuid>,
    /// Optional Topic belonging to the selected Subject.
    pub topic_uuid: Option<Uuid>,
    /// Optional Subtopic belonging to the selected Topic.
    pub subtopic_uuid: Option<Uuid>,
    /// Complete Tag set; no product-level count ceiling.
    pub tags: Vec<Tag>,
}

impl CourseClassification {
    /// Checks structural input; PostgreSQL enforces actual vocabulary membership.
    /// ASVS 2.2.1/2.2.3: descendants require parents and Tags are canonical.
    pub fn validate(&self) -> Result<(), CourseClassificationError> {
        if (self.topic_uuid.is_some() && self.subject_uuid.is_none())
            || (self.subtopic_uuid.is_some() && self.topic_uuid.is_none())
            || self.tags.iter().any(|tag| {
                let text = tag.as_str();
                text != text.trim_matches(' ')
                    || !(1..=120).contains(&text.chars().count())
                    || text.chars().any(char::is_control)
            })
            || self
                .tags
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != self.tags.len()
        {
            return Err(CourseClassificationError);
        }
        Ok(())
    }
}

/// Course classification violates the structural metadata contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseClassificationError;

impl std::fmt::Display for CourseClassificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Course classification hierarchy or Tags are invalid")
    }
}

impl std::error::Error for CourseClassificationError {}

/// CAS validator for independently editable Instance classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseEditNumber(i64);

impl CourseEditNumber {
    /// Rebuilds a validator returned by trusted storage.
    pub fn from_edit_number(value: i64) -> Self {
        Self(value)
    }
    /// Storage representation, never a content Revision.
    pub fn as_i64(self) -> i64 {
        self.0
    }
}

impl std::str::FromStr for CourseEditNumber {
    type Err = CourseClassificationError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parsed = value
            .parse::<i64>()
            .map_err(|_| CourseClassificationError)?;
        (parsed > 0 && parsed.to_string() == value)
            .then_some(Self(parsed))
            .ok_or(CourseClassificationError)
    }
}

impl TryFrom<String> for CourseEditNumber {
    type Error = CourseClassificationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<CourseEditNumber> for String {
    fn from(value: CourseEditNumber) -> Self {
        value.0.to_string()
    }
}

impl std::fmt::Display for CourseEditNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}
