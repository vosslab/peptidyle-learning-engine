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

/// Opaque CAS validator for independently editable Instance classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseMetadataEtag(Uuid);

impl CourseMetadataEtag {
    /// Rebuilds a validator returned by trusted storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }
    /// Storage representation, never a content Revision.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::str::FromStr for CourseMetadataEtag {
    type Err = CourseClassificationError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parsed = Uuid::parse_str(value).map_err(|_| CourseClassificationError)?;
        (parsed.to_string() == value)
            .then_some(Self(parsed))
            .ok_or(CourseClassificationError)
    }
}

impl TryFrom<String> for CourseMetadataEtag {
    type Error = CourseClassificationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<CourseMetadataEtag> for String {
    fn from(value: CourseMetadataEtag) -> Self {
        value.0.to_string()
    }
}

impl std::fmt::Display for CourseMetadataEtag {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}
