//! Bounded exact-pin replacement values for curriculum-adoption previews.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, de};

use super::bounded::{deserialize_pin_replacements, deserialize_replacement_questions};
use crate::{
    MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL, MAX_ASSIGNMENT_ORDERED_ENTRIES,
    MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES, QuestionId,
};

/// Exact bounded semantic position of one replaceable source pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", try_from = "CurriculumPinPositionParts")]
pub struct CurriculumPinPosition {
    module_index: Option<u16>,
    assignment_index: u16,
    entry_index: u16,
    candidate_index: Option<u16>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct CurriculumPinPositionParts {
    module_index: Option<u16>,
    assignment_index: u16,
    entry_index: u16,
    candidate_index: Option<u16>,
}

impl TryFrom<CurriculumPinPositionParts> for CurriculumPinPosition {
    type Error = CurriculumPinPositionError;

    fn try_from(value: CurriculumPinPositionParts) -> Result<Self, Self::Error> {
        Self::new(
            value.module_index,
            value.assignment_index,
            value.entry_index,
            value.candidate_index,
        )
    }
}

impl CurriculumPinPosition {
    /// Validates zero-based module, assignment, entry, and optional pool-candidate coordinates.
    pub fn new(
        module_index: Option<u16>,
        assignment_index: u16,
        entry_index: u16,
        candidate_index: Option<u16>,
    ) -> Result<Self, CurriculumPinPositionError> {
        let bound = u16::try_from(MAX_ASSIGNMENT_ORDERED_ENTRIES)
            .expect("assignment position bound fits u16");
        let candidate_bound = u16::try_from(MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL)
            .expect("candidate position bound fits u16");
        if assignment_index >= bound
            || entry_index >= bound
            || module_index.is_some_and(|index| index >= bound)
            || candidate_index.is_some_and(|index| index >= candidate_bound)
        {
            return Err(CurriculumPinPositionError);
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

/// A pin position exceeded a reusable ordering or pool-candidate bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurriculumPinPositionError;

impl std::fmt::Display for CurriculumPinPositionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum pin position is outside the reusable ordering bound")
    }
}

impl std::error::Error for CurriculumPinPositionError {}

/// One explicit public-question substitution for an exact semantic pin position.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CurriculumPinReplacement {
    /// Exact fixed-item or pool-candidate coordinate selected by the server preview.
    pub position: CurriculumPinPosition,
    /// Public Question ID selected through the shared Question Picker.
    pub question: QuestionId,
}

/// Bounded unique substitutions accumulated while correcting one adoption preview.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(into = "Vec<CurriculumPinReplacement>")]
pub struct CurriculumPinReplacements(Vec<CurriculumPinReplacement>);

impl<'de> Deserialize<'de> for CurriculumPinReplacements {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = deserialize_pin_replacements(deserializer)?;
        Self::new(values).map_err(de::Error::custom)
    }
}

impl CurriculumPinReplacements {
    /// Validates unique exact positions within the total source-selection bound.
    pub fn new(
        mut values: Vec<CurriculumPinReplacement>,
    ) -> Result<Self, CurriculumPinReplacementsError> {
        if values.len() > MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES {
            return Err(CurriculumPinReplacementsError);
        }
        values.sort_unstable_by_key(|value| value.position);
        if values
            .windows(2)
            .any(|pair| pair[0].position == pair[1].position)
        {
            return Err(CurriculumPinReplacementsError);
        }
        Ok(Self(values))
    }

    /// Returns substitutions in the Instructor-confirmed order echoed by preview.
    pub fn as_slice(&self) -> &[CurriculumPinReplacement] {
        &self.0
    }
}

impl TryFrom<Vec<CurriculumPinReplacement>> for CurriculumPinReplacements {
    type Error = CurriculumPinReplacementsError;

    fn try_from(value: Vec<CurriculumPinReplacement>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CurriculumPinReplacements> for Vec<CurriculumPinReplacement> {
    fn from(value: CurriculumPinReplacements) -> Self {
        value.0
    }
}

/// Pin substitutions exceeded the bound or repeated one exact semantic position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurriculumPinReplacementsError;

impl std::fmt::Display for CurriculumPinReplacementsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum pin replacements are invalid")
    }
}

impl std::error::Error for CurriculumPinReplacementsError {}

/// Validated answer-free candidate question IDs for one explicit replacement action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(into = "Vec<QuestionId>")]
pub struct ReplacementQuestionChoices(Vec<QuestionId>);

impl<'de> Deserialize<'de> for ReplacementQuestionChoices {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = deserialize_replacement_questions(deserializer)?;
        Self::new(values).map_err(de::Error::custom)
    }
}

impl ReplacementQuestionChoices {
    /// Validates nonempty unique public candidate IDs within the existing pool bound.
    pub fn new(values: Vec<QuestionId>) -> Result<Self, ReplacementQuestionChoicesError> {
        if values.is_empty() || values.len() > MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL {
            return Err(ReplacementQuestionChoicesError);
        }
        if values.iter().collect::<BTreeSet<_>>().len() != values.len() {
            return Err(ReplacementQuestionChoicesError);
        }
        Ok(Self(values))
    }

    /// Returns public candidate question IDs in server-selected recovery order.
    pub fn as_slice(&self) -> &[QuestionId] {
        &self.0
    }
}

impl TryFrom<Vec<QuestionId>> for ReplacementQuestionChoices {
    type Error = ReplacementQuestionChoicesError;

    fn try_from(value: Vec<QuestionId>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ReplacementQuestionChoices> for Vec<QuestionId> {
    fn from(value: ReplacementQuestionChoices) -> Self {
        value.0
    }
}

/// Replacement candidates were empty, duplicated, or above the pool bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplacementQuestionChoicesError;

impl std::fmt::Display for ReplacementQuestionChoicesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("replacement question choices are invalid")
    }
}

impl std::error::Error for ReplacementQuestionChoicesError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_replacements_use_snake_case_and_refuse_unknown_fields() {
        let replacement = CurriculumPinReplacement {
            position: CurriculumPinPosition::new(Some(1), 2, 3, Some(4)).expect("position"),
            question: "7K3-M9QX".parse().expect("QuestionId"),
        };
        let wire = serde_json::to_value(&replacement).expect("replacement serializes");
        assert_eq!(wire["position"]["module_index"], 1);
        assert_eq!(wire["position"]["assignment_index"], 2);
        assert_eq!(wire["position"]["candidate_index"], 4);
        assert!(wire["position"].get("moduleIndex").is_none());
        assert!(serde_json::from_value::<CurriculumPinReplacement>(wire.clone()).is_ok());
        let mut forged = wire;
        forged["position"]["authority"] = serde_json::json!("instructor");
        assert!(serde_json::from_value::<CurriculumPinReplacement>(forged).is_err());
    }
}
