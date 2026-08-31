//! Bounded Question Version substitutions for Blueprint-operation previews.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, de};

use super::bounded::{
    deserialize_question_version_substitutions, deserialize_replacement_question_versions,
};
use crate::{
    MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL, MAX_ASSIGNMENT_ORDERED_ENTRIES,
    MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES, QuestionVersionReference,
};

/// Exact bounded position of one Question Version in Blueprint Revision Content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", try_from = "BlueprintQuestionPositionParts")]
pub struct BlueprintQuestionPosition {
    module_index: Option<u16>,
    assignment_index: u16,
    entry_index: u16,
    candidate_index: Option<u16>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct BlueprintQuestionPositionParts {
    module_index: Option<u16>,
    assignment_index: u16,
    entry_index: u16,
    candidate_index: Option<u16>,
}

impl TryFrom<BlueprintQuestionPositionParts> for BlueprintQuestionPosition {
    type Error = BlueprintQuestionPositionError;

    fn try_from(value: BlueprintQuestionPositionParts) -> Result<Self, Self::Error> {
        Self::new(
            value.module_index,
            value.assignment_index,
            value.entry_index,
            value.candidate_index,
        )
    }
}

impl BlueprintQuestionPosition {
    /// Validates zero-based module, assignment, entry, and optional pool-candidate coordinates.
    pub fn new(
        module_index: Option<u16>,
        assignment_index: u16,
        entry_index: u16,
        candidate_index: Option<u16>,
    ) -> Result<Self, BlueprintQuestionPositionError> {
        let bound = u16::try_from(MAX_ASSIGNMENT_ORDERED_ENTRIES)
            .expect("assignment position bound fits u16");
        let candidate_bound = u16::try_from(MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL)
            .expect("candidate position bound fits u16");
        if assignment_index >= bound
            || entry_index >= bound
            || module_index.is_some_and(|index| index >= bound)
            || candidate_index.is_some_and(|index| index >= candidate_bound)
        {
            return Err(BlueprintQuestionPositionError);
        }
        Ok(Self {
            module_index,
            assignment_index,
            entry_index,
            candidate_index,
        })
    }

    /// Returns the optional zero-based BlueprintCourse module position.
    pub fn module_index(self) -> Option<u16> {
        self.module_index
    }

    /// Returns the zero-based assignment position within its source scope.
    pub fn assignment_index(self) -> u16 {
        self.assignment_index
    }

    /// Returns the zero-based fixed-item or pool entry position.
    pub fn entry_index(self) -> u16 {
        self.entry_index
    }

    /// Returns the zero-based pool candidate position, or `None` for one fixed item.
    pub fn candidate_index(self) -> Option<u16> {
        self.candidate_index
    }
}

/// A Blueprint Question position exceeded a reusable ordering or pool-candidate bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintQuestionPositionError;

impl std::fmt::Display for BlueprintQuestionPositionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Blueprint Question position is outside the reusable ordering bound")
    }
}

impl std::error::Error for BlueprintQuestionPositionError {}

/// One explicit Question Version substitution for an exact Blueprint Question position.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QuestionVersionSubstitution {
    /// Exact Blueprint Revision Content coordinate selected by the server preview.
    pub position: BlueprintQuestionPosition,
    /// Exact immutable Question Version selected through the shared Question Picker.
    pub question_version: QuestionVersionReference,
}

/// Bounded unique Question Version substitutions for one Blueprint-operation preview.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(into = "Vec<QuestionVersionSubstitution>")]
pub struct QuestionVersionSubstitutions(Vec<QuestionVersionSubstitution>);

impl<'de> Deserialize<'de> for QuestionVersionSubstitutions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = deserialize_question_version_substitutions(deserializer)?;
        Self::new(values).map_err(de::Error::custom)
    }
}

impl QuestionVersionSubstitutions {
    /// Validates unique Blueprint Question positions within the source-selection bound.
    pub fn new(
        mut values: Vec<QuestionVersionSubstitution>,
    ) -> Result<Self, QuestionVersionSubstitutionsError> {
        if values.len() > MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES {
            return Err(QuestionVersionSubstitutionsError);
        }
        values.sort_unstable_by_key(|value| value.position);
        if values
            .windows(2)
            .any(|pair| pair[0].position == pair[1].position)
        {
            return Err(QuestionVersionSubstitutionsError);
        }
        Ok(Self(values))
    }

    /// Returns substitutions in the Instructor-confirmed order echoed by preview.
    pub fn as_slice(&self) -> &[QuestionVersionSubstitution] {
        &self.0
    }
}

impl TryFrom<Vec<QuestionVersionSubstitution>> for QuestionVersionSubstitutions {
    type Error = QuestionVersionSubstitutionsError;

    fn try_from(value: Vec<QuestionVersionSubstitution>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<QuestionVersionSubstitutions> for Vec<QuestionVersionSubstitution> {
    fn from(value: QuestionVersionSubstitutions) -> Self {
        value.0
    }
}

/// Question Version substitutions exceeded the bound or repeated one Blueprint Question position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionVersionSubstitutionsError;

impl std::fmt::Display for QuestionVersionSubstitutionsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Question Version substitutions are invalid")
    }
}

impl std::error::Error for QuestionVersionSubstitutionsError {}

/// Validated answer-free candidate Question Versions for one replacement action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(into = "Vec<QuestionVersionReference>")]
pub struct ReplacementQuestionVersionChoices(Vec<QuestionVersionReference>);

impl<'de> Deserialize<'de> for ReplacementQuestionVersionChoices {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = deserialize_replacement_question_versions(deserializer)?;
        Self::new(values).map_err(de::Error::custom)
    }
}

impl ReplacementQuestionVersionChoices {
    /// Validates nonempty unique candidate Question Versions within the pool bound.
    pub fn new(
        values: Vec<QuestionVersionReference>,
    ) -> Result<Self, ReplacementQuestionVersionChoicesError> {
        if values.is_empty() || values.len() > MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL {
            return Err(ReplacementQuestionVersionChoicesError);
        }
        if values.iter().collect::<BTreeSet<_>>().len() != values.len() {
            return Err(ReplacementQuestionVersionChoicesError);
        }
        Ok(Self(values))
    }

    /// Returns candidate Question Versions in server-selected recovery order.
    pub fn as_slice(&self) -> &[QuestionVersionReference] {
        &self.0
    }
}

impl TryFrom<Vec<QuestionVersionReference>> for ReplacementQuestionVersionChoices {
    type Error = ReplacementQuestionVersionChoicesError;

    fn try_from(value: Vec<QuestionVersionReference>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ReplacementQuestionVersionChoices> for Vec<QuestionVersionReference> {
    fn from(value: ReplacementQuestionVersionChoices) -> Self {
        value.0
    }
}

/// Replacement candidates were empty, duplicated, or above the pool bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplacementQuestionVersionChoicesError;

impl std::fmt::Display for ReplacementQuestionVersionChoicesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("replacement question choices are invalid")
    }
}

impl std::error::Error for ReplacementQuestionVersionChoicesError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{QuestionId, QuestionVersionNumber};

    #[test]
    fn pin_replacements_use_snake_case_and_refuse_unknown_fields() {
        let question_version = QuestionVersionReference {
            question_id: QuestionId::from_canonical_parts("7K3M9Q", 'X').expect("Question ID"),
            version_number: QuestionVersionNumber::new(2).expect("version"),
        };
        let replacement = QuestionVersionSubstitution {
            position: BlueprintQuestionPosition::new(Some(1), 2, 3, Some(4)).expect("position"),
            question_version,
        };
        let wire = serde_json::to_value(&replacement).expect("replacement serializes");
        assert_eq!(wire["position"]["module_index"], 1);
        assert_eq!(wire["position"]["assignment_index"], 2);
        assert_eq!(wire["position"]["candidate_index"], 4);
        assert!(wire["position"].get("moduleIndex").is_none());
        assert!(serde_json::from_value::<QuestionVersionSubstitution>(wire.clone()).is_ok());
        let mut forged = wire;
        forged["position"]["authority"] = serde_json::json!("instructor");
        assert!(serde_json::from_value::<QuestionVersionSubstitution>(forged).is_err());
    }
}
