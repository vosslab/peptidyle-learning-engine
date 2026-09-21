//! Assessment and Question Backend capability validation.

use std::collections::BTreeSet;

use question_model::{Capability, PublishedQuestionRevisionTuple, QuestionBackendCapabilities};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentQuestionConfig {
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    pub question_backend_capabilities: QuestionBackendCapabilities,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentConfig {
    pub questions: Vec<AssessmentQuestionConfig>,
    /// Assessment-owned requirements; Question Source does not derive these.
    pub required_capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Violation {
    pub published_question_revision_tuple: PublishedQuestionRevisionTuple,
    pub capability: Capability,
}

pub fn validate_assessment_config(config: &AssessmentConfig) -> Vec<Violation> {
    let required: BTreeSet<_> = config.required_capabilities.iter().copied().collect();
    config
        .questions
        .iter()
        .flat_map(|selected| {
            Capability::ALL
                .into_iter()
                .filter(|capability| {
                    required.contains(capability)
                        && !selected.question_backend_capabilities.supports(*capability)
                })
                .map(|capability| Violation {
                    published_question_revision_tuple: selected
                        .published_question_revision_tuple
                        .clone(),
                    capability,
                })
        })
        .collect()
}
