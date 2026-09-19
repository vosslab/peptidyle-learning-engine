//! Closed Bloom classification values for Question Revisions and current Question Pools.
//!
//! The classification is one ordered pair of independent dimensions. It is
//! projected to authorized browsers with its independent correction precondition.

use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::QuestionRevisionTuple;

/// Cognitive work required for full credit on the exact classified Revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BloomCognitiveProcess {
    #[serde(rename = "Remember")]
    Remember,
    #[serde(rename = "Understand")]
    Understand,
    #[serde(rename = "Apply")]
    Apply,
    #[serde(rename = "Analyze")]
    Analyze,
    #[serde(rename = "Evaluate")]
    Evaluate,
    #[serde(rename = "Create")]
    Create,
}

impl BloomCognitiveProcess {
    /// Every accepted value in teaching-guide order.
    pub const ALL: [Self; 6] = [
        Self::Remember,
        Self::Understand,
        Self::Apply,
        Self::Analyze,
        Self::Evaluate,
        Self::Create,
    ];

    /// Returns the exact teaching-guide and PostgreSQL value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Remember => "Remember",
            Self::Understand => "Understand",
            Self::Apply => "Apply",
            Self::Analyze => "Analyze",
            Self::Evaluate => "Evaluate",
            Self::Create => "Create",
        }
    }
}

impl std::fmt::Display for BloomCognitiveProcess {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for BloomCognitiveProcess {
    type Err = BloomCognitiveProcessParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == value)
            .ok_or(BloomCognitiveProcessParseError)
    }
}

/// A value was not one exact Bloom Cognitive Process spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BloomCognitiveProcessParseError;

impl std::fmt::Display for BloomCognitiveProcessParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("unknown Bloom Cognitive Process")
    }
}

impl std::error::Error for BloomCognitiveProcessParseError {}

/// Primary kind of knowledge used by the exact classified Revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BloomKnowledgeDimension {
    #[serde(rename = "Factual Knowledge")]
    FactualKnowledge,
    #[serde(rename = "Conceptual Knowledge")]
    ConceptualKnowledge,
    #[serde(rename = "Procedural Knowledge")]
    ProceduralKnowledge,
    #[serde(rename = "Metacognitive Knowledge")]
    MetacognitiveKnowledge,
}

impl BloomKnowledgeDimension {
    /// Every accepted value in teaching-guide order.
    pub const ALL: [Self; 4] = [
        Self::FactualKnowledge,
        Self::ConceptualKnowledge,
        Self::ProceduralKnowledge,
        Self::MetacognitiveKnowledge,
    ];

    /// Returns the exact teaching-guide and PostgreSQL value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FactualKnowledge => "Factual Knowledge",
            Self::ConceptualKnowledge => "Conceptual Knowledge",
            Self::ProceduralKnowledge => "Procedural Knowledge",
            Self::MetacognitiveKnowledge => "Metacognitive Knowledge",
        }
    }
}

impl std::fmt::Display for BloomKnowledgeDimension {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for BloomKnowledgeDimension {
    type Err = BloomKnowledgeDimensionParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == value)
            .ok_or(BloomKnowledgeDimensionParseError)
    }
}

/// A value was not one exact Bloom Knowledge Dimension spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BloomKnowledgeDimensionParseError;

impl std::fmt::Display for BloomKnowledgeDimensionParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("unknown Bloom Knowledge Dimension")
    }
}

impl std::error::Error for BloomKnowledgeDimensionParseError {}

/// Complete Bloom Classification as two independent required values.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BloomClassification {
    /// Required cognitive work for full credit.
    pub cognitive_process: BloomCognitiveProcess,
    /// Primary kind of knowledge used by that work.
    pub knowledge_dimension: BloomKnowledgeDimension,
}

/// Positive compare-and-swap number for one exact Revision's Bloom Classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BloomClassificationEditNumber(NonZeroU64);

impl BloomClassificationEditNumber {
    /// First Edit Number for an attached Bloom Classification.
    pub const INITIAL: Self = Self(NonZeroU64::MIN);

    /// Rebuilds a positive Edit Number that fits PostgreSQL `BIGINT`.
    pub fn new(value: u64) -> Option<Self> {
        (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
    }

    /// Returns the exact positive persistence value.
    pub const fn value(self) -> u64 {
        self.0.get()
    }
}

impl FromStr for BloomClassificationEditNumber {
    type Err = BloomClassificationEditNumberError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(BloomClassificationEditNumberError);
        }
        value
            .parse::<u64>()
            .ok()
            .and_then(Self::new)
            .ok_or(BloomClassificationEditNumberError)
    }
}

impl TryFrom<String> for BloomClassificationEditNumber {
    type Error = BloomClassificationEditNumberError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<BloomClassificationEditNumber> for String {
    fn from(value: BloomClassificationEditNumber) -> Self {
        value.to_string()
    }
}

impl std::fmt::Display for BloomClassificationEditNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(formatter)
    }
}

/// A Bloom Classification Edit Number was not one canonical positive PostgreSQL-`BIGINT` decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BloomClassificationEditNumberError;

impl std::fmt::Display for BloomClassificationEditNumberError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Bloom Classification Edit Number must be a canonical positive decimal")
    }
}

impl std::error::Error for BloomClassificationEditNumberError {}

/// Browser-safe Bloom Classification and its exact correction precondition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BloomClassificationView {
    /// Required cognitive work for full credit.
    pub cognitive_process: BloomCognitiveProcess,
    /// Primary kind of knowledge used by that work.
    pub knowledge_dimension: BloomKnowledgeDimension,
    /// Independent compare-and-swap number for this exact Revision's classification.
    pub classification_edit_number: BloomClassificationEditNumber,
}

/// Complete optimistic-concurrency command for one exact Revision's classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BloomClassificationCorrectionRequest {
    /// Replacement cognitive dimension.
    pub cognitive_process: BloomCognitiveProcess,
    /// Replacement knowledge dimension.
    pub knowledge_dimension: BloomKnowledgeDimension,
    /// Exact independent classification state observed by the Instructor.
    pub expected_classification_edit_number: BloomClassificationEditNumber,
}

/// Committed Question classification and the exact immutable target it belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionBloomCorrectionReceipt {
    pub question_revision_tuple: QuestionRevisionTuple,
    pub bloom: BloomClassificationView,
}

/// Committed Pool classification for the current Question Pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionPoolBloomCorrectionReceipt {
    pub question_pool_id: crate::QuestionId,
    pub bloom: BloomClassificationView,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cognitive_process_values_parse_and_serialize_with_exact_sql_spellings() {
        assert_eq!(
            BloomCognitiveProcess::ALL.map(BloomCognitiveProcess::as_str),
            [
                "Remember",
                "Understand",
                "Apply",
                "Analyze",
                "Evaluate",
                "Create",
            ]
        );
        for process in BloomCognitiveProcess::ALL {
            let value = process.as_str();
            assert_eq!(value.parse::<BloomCognitiveProcess>(), Ok(process));
            assert_eq!(
                serde_json::to_string(&process).expect("serialize"),
                format!("\"{value}\"")
            );
        }
        assert!("remember".parse::<BloomCognitiveProcess>().is_err());
        assert!(serde_json::from_str::<BloomCognitiveProcess>("\"Synthesize\"").is_err());
    }

    #[test]
    fn knowledge_dimension_values_parse_and_serialize_with_exact_sql_spellings() {
        assert_eq!(
            BloomKnowledgeDimension::ALL.map(BloomKnowledgeDimension::as_str),
            [
                "Factual Knowledge",
                "Conceptual Knowledge",
                "Procedural Knowledge",
                "Metacognitive Knowledge",
            ]
        );
        for dimension in BloomKnowledgeDimension::ALL {
            let value = dimension.as_str();
            assert_eq!(value.parse::<BloomKnowledgeDimension>(), Ok(dimension));
            assert_eq!(
                serde_json::to_string(&dimension).expect("serialize"),
                format!("\"{value}\"")
            );
        }
        assert!("Conceptual".parse::<BloomKnowledgeDimension>().is_err());
        assert!(
            serde_json::from_str::<BloomKnowledgeDimension>("\"Strategic Knowledge\"").is_err()
        );
    }

    #[test]
    fn classification_round_trips_as_only_the_two_independent_dimensions() {
        let classification = BloomClassification {
            cognitive_process: BloomCognitiveProcess::Analyze,
            knowledge_dimension: BloomKnowledgeDimension::ConceptualKnowledge,
        };
        let encoded = serde_json::to_string(&classification).expect("serialize classification");

        assert_eq!(
            serde_json::from_str::<BloomClassification>(&encoded)
                .expect("deserialize classification"),
            classification
        );
        assert!(
            serde_json::from_str::<BloomClassification>(
                r#"{"cognitive_process":"Analyze","knowledge_dimension":"Conceptual Knowledge","level":4}"#
            )
            .is_err()
        );
    }

    #[test]
    fn browser_view_round_trips_with_a_precision_safe_edit_number() {
        let view = BloomClassificationView {
            cognitive_process: BloomCognitiveProcess::Evaluate,
            knowledge_dimension: BloomKnowledgeDimension::ProceduralKnowledge,
            classification_edit_number: BloomClassificationEditNumber::new(i64::MAX as u64)
                .expect("PostgreSQL BIGINT maximum is valid"),
        };
        let encoded = serde_json::to_value(view).expect("serialize browser view");
        assert_eq!(encoded["classificationEditNumber"], i64::MAX.to_string());
        assert_eq!(
            serde_json::from_value::<BloomClassificationView>(encoded)
                .expect("deserialize browser view"),
            view
        );
        for value in ["0", "01", "9223372036854775808"] {
            assert!(value.parse::<BloomClassificationEditNumber>().is_err());
        }
    }

    #[test]
    fn correction_request_is_closed_and_keeps_the_precision_safe_precondition() {
        let json = r#"{
            "cognitiveProcess":"Analyze",
            "knowledgeDimension":"Conceptual Knowledge",
            "expectedClassificationEditNumber":"9223372036854775807"
        }"#;
        let request = serde_json::from_str::<BloomClassificationCorrectionRequest>(json)
            .expect("closed correction request");
        assert_eq!(
            request.expected_classification_edit_number,
            BloomClassificationEditNumber::new(i64::MAX as u64).expect("BIGINT maximum")
        );
        assert!(
            serde_json::from_str::<BloomClassificationCorrectionRequest>(
                r#"{
                    "cognitiveProcess":"Analyze",
                    "knowledgeDimension":"Conceptual Knowledge",
                    "expectedClassificationEditNumber":"1",
                    "questionId":"caller-selected-authority"
                }"#,
            )
            .is_err()
        );
    }
}
