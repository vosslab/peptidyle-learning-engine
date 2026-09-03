//! Bounded Question Revision substitutions for Blueprint-operation previews.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, de};

use super::bounded::{
    deserialize_question_revision_substitutions, deserialize_replacement_question_revisions,
};
use crate::{
    MAX_ASSIGNMENT_ORDERED_ENTRIES, MAX_ASSIGNMENT_QUESTION_POOL_ITEMS,
    MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY, QuestionRevisionReference,
};

/// Exact bounded position of one Question Revision in Blueprint Revision Content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", try_from = "BlueprintQuestionPositionParts")]
pub struct BlueprintQuestionPosition {
    module_index: Option<u16>,
    assignment_index: u16,
    entry_index: u16,
    question_pool_item_index: Option<u16>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct BlueprintQuestionPositionParts {
    module_index: Option<u16>,
    assignment_index: u16,
    entry_index: u16,
    question_pool_item_index: Option<u16>,
}

impl TryFrom<BlueprintQuestionPositionParts> for BlueprintQuestionPosition {
    type Error = BlueprintQuestionPositionError;

    fn try_from(value: BlueprintQuestionPositionParts) -> Result<Self, Self::Error> {
        Self::new(
            value.module_index,
            value.assignment_index,
            value.entry_index,
            value.question_pool_item_index,
        )
    }
}

impl BlueprintQuestionPosition {
    /// Validates zero-based module, assignment, entry, and optional Question Pool Item coordinates.
    pub fn new(
        module_index: Option<u16>,
        assignment_index: u16,
        entry_index: u16,
        question_pool_item_index: Option<u16>,
    ) -> Result<Self, BlueprintQuestionPositionError> {
        let bound = u16::try_from(MAX_ASSIGNMENT_ORDERED_ENTRIES)
            .expect("assignment position bound fits u16");
        let question_pool_item_bound = u16::try_from(MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY)
            .expect("Question Pool Item position bound fits u16");
        if assignment_index >= bound
            || entry_index >= bound
            || module_index.is_some_and(|index| index >= bound)
            || question_pool_item_index.is_some_and(|index| index >= question_pool_item_bound)
        {
            return Err(BlueprintQuestionPositionError);
        }
        Ok(Self {
            module_index,
            assignment_index,
            entry_index,
            question_pool_item_index,
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

    /// Returns the zero-based Assignment Entry position for either a Fixed Question or Question Pool.
    pub fn entry_index(self) -> u16 {
        self.entry_index
    }

    /// Returns the zero-based Question Pool Item position, or `None` for one fixed item.
    pub fn question_pool_item_index(self) -> Option<u16> {
        self.question_pool_item_index
    }
}

/// A Blueprint Question position exceeded a reusable ordering or Question Pool Item bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintQuestionPositionError;

impl std::fmt::Display for BlueprintQuestionPositionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Blueprint Question position is outside the reusable ordering bound")
    }
}

impl std::error::Error for BlueprintQuestionPositionError {}

/// One explicit Question Revision substitution for an exact Blueprint Question position.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QuestionRevisionSubstitution {
    /// Exact Blueprint Revision Content coordinate selected by the server preview.
    pub position: BlueprintQuestionPosition,
    /// Exact immutable Question Revision selected through the shared Question Picker.
    pub question_revision: QuestionRevisionReference,
}

/// Bounded unique Question Revision substitutions for one Blueprint-operation preview.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(into = "Vec<QuestionRevisionSubstitution>")]
pub struct QuestionRevisionSubstitutions(Vec<QuestionRevisionSubstitution>);

impl<'de> Deserialize<'de> for QuestionRevisionSubstitutions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = deserialize_question_revision_substitutions(deserializer)?;
        Self::new(values).map_err(de::Error::custom)
    }
}

impl QuestionRevisionSubstitutions {
    /// Validates unique Blueprint Question positions within the source-selection bound.
    pub fn new(
        mut values: Vec<QuestionRevisionSubstitution>,
    ) -> Result<Self, QuestionRevisionSubstitutionsError> {
        if values.len() > MAX_ASSIGNMENT_QUESTION_POOL_ITEMS {
            return Err(QuestionRevisionSubstitutionsError);
        }
        values.sort_unstable_by_key(|value| value.position);
        if values
            .windows(2)
            .any(|pair| pair[0].position == pair[1].position)
        {
            return Err(QuestionRevisionSubstitutionsError);
        }
        Ok(Self(values))
    }

    /// Returns substitutions in the Instructor-confirmed order echoed by preview.
    pub fn as_slice(&self) -> &[QuestionRevisionSubstitution] {
        &self.0
    }
}

impl TryFrom<Vec<QuestionRevisionSubstitution>> for QuestionRevisionSubstitutions {
    type Error = QuestionRevisionSubstitutionsError;

    fn try_from(value: Vec<QuestionRevisionSubstitution>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<QuestionRevisionSubstitutions> for Vec<QuestionRevisionSubstitution> {
    fn from(value: QuestionRevisionSubstitutions) -> Self {
        value.0
    }
}

/// Question Revision substitutions exceeded the bound or repeated one Blueprint Question position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionRevisionSubstitutionsError;

impl std::fmt::Display for QuestionRevisionSubstitutionsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Question Revision substitutions are invalid")
    }
}

impl std::error::Error for QuestionRevisionSubstitutionsError {}

/// Validated answer-free replacement Question Revisions for one replacement action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(into = "Vec<QuestionRevisionReference>")]
pub struct ReplacementQuestionRevisionChoices(Vec<QuestionRevisionReference>);

impl<'de> Deserialize<'de> for ReplacementQuestionRevisionChoices {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = deserialize_replacement_question_revisions(deserializer)?;
        Self::new(values).map_err(de::Error::custom)
    }
}

impl ReplacementQuestionRevisionChoices {
    /// Validates nonempty unique replacement Question Revisions within the pool bound.
    pub fn new(
        values: Vec<QuestionRevisionReference>,
    ) -> Result<Self, ReplacementQuestionRevisionChoicesError> {
        if values.is_empty() || values.len() > MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY {
            return Err(ReplacementQuestionRevisionChoicesError);
        }
        if values.iter().collect::<BTreeSet<_>>().len() != values.len() {
            return Err(ReplacementQuestionRevisionChoicesError);
        }
        Ok(Self(values))
    }

    /// Returns replacement Question Revisions in server-selected recovery order.
    pub fn as_slice(&self) -> &[QuestionRevisionReference] {
        &self.0
    }
}

impl TryFrom<Vec<QuestionRevisionReference>> for ReplacementQuestionRevisionChoices {
    type Error = ReplacementQuestionRevisionChoicesError;

    fn try_from(value: Vec<QuestionRevisionReference>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ReplacementQuestionRevisionChoices> for Vec<QuestionRevisionReference> {
    fn from(value: ReplacementQuestionRevisionChoices) -> Self {
        value.0
    }
}

/// Replacement Question Revisions were empty, duplicated, or above the pool bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplacementQuestionRevisionChoicesError;

impl std::fmt::Display for ReplacementQuestionRevisionChoicesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("replacement question choices are invalid")
    }
}

impl std::error::Error for ReplacementQuestionRevisionChoicesError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{QuestionId, QuestionRevisionNumber};

    #[test]
    fn pin_replacements_use_snake_case_and_refuse_unknown_fields() {
        let question_revision = QuestionRevisionReference {
            question_id: QuestionId::from_canonical_parts("7K3M9Q", 'X').expect("Question ID"),
            revision_number: QuestionRevisionNumber::new(2).expect("version"),
        };
        let replacement = QuestionRevisionSubstitution {
            position: BlueprintQuestionPosition::new(Some(1), 2, 3, Some(4)).expect("position"),
            question_revision,
        };
        let wire = serde_json::to_value(&replacement).expect("replacement serializes");
        assert_eq!(wire["position"]["module_index"], 1);
        assert_eq!(wire["position"]["assignment_index"], 2);
        assert_eq!(wire["position"]["question_pool_item_index"], 4);
        assert!(wire["position"].get("moduleIndex").is_none());
        assert!(serde_json::from_value::<QuestionRevisionSubstitution>(wire.clone()).is_ok());
        let mut forged = wire;
        forged["position"]["authority"] = serde_json::json!("instructor");
        assert!(serde_json::from_value::<QuestionRevisionSubstitution>(forged).is_err());
    }
}
