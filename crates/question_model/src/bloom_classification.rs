//! Closed Bloom classification values for Question Revisions and current Question Pools.
//!
//! `BloomClassificationView` is a display projection with two independently
//! nullable dimensions and no edit counter.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

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

/// Browser-safe display projection for ordinary nullable Bloom metadata.
///
/// A present view may contain either or both dimensions. Callers omit the view
/// only when both values are absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BloomClassificationView {
    pub cognitive_process: Option<BloomCognitiveProcess>,
    pub knowledge_dimension: Option<BloomKnowledgeDimension>,
}
