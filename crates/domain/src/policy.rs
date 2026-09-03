//! Assignment and Question Backend capability validation.

use std::collections::BTreeSet;

use question_model::{Capability, QuestionBackendCapabilities, QuestionRevisionReference};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentQuestionConfig {
    pub question: QuestionRevisionReference,
    pub question_backend_capabilities: QuestionBackendCapabilities,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentConfig {
    pub questions: Vec<AssignmentQuestionConfig>,
    /// Assignment-owned requirements; Question Source does not derive these.
    pub required_capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Violation {
    pub question: QuestionRevisionReference,
    pub capability: Capability,
}

pub fn validate_assignment_config(config: &AssignmentConfig) -> Vec<Violation> {
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
                    question: selected.question.clone(),
                    capability,
                })
        })
        .collect()
}
