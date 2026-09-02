//! Assignment/backend capability validation (MOD-CAP).
//!
//! The editor and publish route call the same pure function. Each violation
//! names the immutable question revision and one missing capability, and the
//! complete list is returned in question order and capability declaration
//! order so an instructor can repair everything in one pass.

use std::collections::BTreeSet;

use question_model::assignment_activity_rules::QuestionAttemptTimeLimit;
use question_model::generation::QuestionVariationRule;
use question_model::{
    Capability, DraftQuestionContent, QuestionBackendCapabilities, QuestionGradingRule,
    QuestionRevision, QuestionRevisionReference,
};
use serde::{Deserialize, Serialize};

/// One selected question and its backend's honest capability declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentQuestionConfig {
    /// Browser-safe immutable Question Revision selected by the assignment.
    pub question: QuestionRevision,
    /// Capabilities declared by the adapter that owns this question.
    pub question_backend_capabilities: QuestionBackendCapabilities,
}

/// Complete input to assignment capability validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentConfig {
    /// Questions selected for the assignment, in instructor-visible order.
    pub questions: Vec<AssignmentQuestionConfig>,
    /// Assignment-wide features every selected backend must support.
    ///
    /// Client rendering, print export, and offline preview are requested here.
    /// Question-authored generation, grading, and timing requirements
    /// are derived directly from each definition and need no second flag.
    pub required_capabilities: Vec<Capability>,
}

/// One unsupported question/backend capability pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Violation {
    /// Immutable question revision whose backend lacks support.
    pub question: QuestionRevisionReference,
    /// Capability required by the assignment but absent from the backend.
    pub capability: Capability,
}

/// Capability diagnostic for unpublished workspace content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationViolation {
    /// Workspace-local label; a draft intentionally has no published version.
    pub workspace: question_model::WorkspaceId,
    /// Human-facing draft title to make the repair location clear.
    pub title: String,
    /// Capability required by the draft but absent from its trusted backend.
    pub capability: Capability,
}

/// Returns every unsupported capability in deterministic display order.
///
/// The function has no storage or adapter dependency. API routes resolve the
/// selected definitions and adapter declarations, then pass that browser-safe
/// data here. Duplicate assignment requirements collapse to one violation per
/// question and capability.
pub fn validate_assignment_config(config: &AssignmentConfig) -> Vec<Violation> {
    let assignment_requirements: BTreeSet<_> =
        config.required_capabilities.iter().copied().collect();
    let mut violations = Vec::new();

    for selected in &config.questions {
        let mut required = required_by_question(&selected.question);
        required.extend(assignment_requirements.iter().copied());

        for capability in Capability::ALL {
            if required.contains(&capability)
                && !selected.question_backend_capabilities.supports(capability)
            {
                violations.push(Violation {
                    question: QuestionRevisionReference {
                        question_id: selected.question.question_id.clone(),
                        revision_number: selected.question.revision_number,
                    },
                    capability,
                });
            }
        }
    }

    violations
}

/// Validates a draft at the publication boundary without inventing a version ID.
pub fn validate_draft_for_publication(
    question: &DraftQuestionContent,
    question_backend_capabilities: &QuestionBackendCapabilities,
) -> Vec<PublicationViolation> {
    required_by_content(question)
        .into_iter()
        .filter(|capability| !question_backend_capabilities.supports(*capability))
        .map(|capability| PublicationViolation {
            workspace: question.workspace,
            title: question.metadata.title.clone(),
            capability,
        })
        .collect()
}

fn required_by_question(question: &QuestionRevision) -> BTreeSet<Capability> {
    required_by_content(question)
}

trait QuestionContentView {
    fn question_variation_rule(&self) -> &question_model::generation::QuestionVariationRule;
    fn grading(&self) -> &QuestionGradingRule;
    fn question_attempt_time_limit(&self) -> &QuestionAttemptTimeLimit;
}

impl QuestionContentView for QuestionRevision {
    fn question_variation_rule(&self) -> &question_model::generation::QuestionVariationRule {
        &self.question_variation_rule
    }
    fn grading(&self) -> &QuestionGradingRule {
        &self.grading
    }
    fn question_attempt_time_limit(&self) -> &QuestionAttemptTimeLimit {
        &self.question_attempt_time_limit
    }
}

impl QuestionContentView for DraftQuestionContent {
    fn question_variation_rule(&self) -> &question_model::generation::QuestionVariationRule {
        &self.question_variation_rule
    }
    fn grading(&self) -> &QuestionGradingRule {
        &self.grading
    }
    fn question_attempt_time_limit(&self) -> &QuestionAttemptTimeLimit {
        &self.question_attempt_time_limit
    }
}

fn required_by_content(question: &impl QuestionContentView) -> BTreeSet<Capability> {
    let mut required = BTreeSet::new();

    if matches!(
        question.question_variation_rule(),
        QuestionVariationRule::Seeded { .. }
    ) {
        required.insert(Capability::AlgorithmicGeneration);
    }
    match question.grading() {
        QuestionGradingRule::AllOrNothing { .. } => {
            required.insert(Capability::ServerGrading);
        }
        QuestionGradingRule::PartialCredit { .. } => {
            required.insert(Capability::ServerGrading);
            required.insert(Capability::PartialCredit);
        }
        QuestionGradingRule::Ungraded => {}
    }
    if matches!(
        question.question_attempt_time_limit(),
        QuestionAttemptTimeLimit::Limited { .. }
    ) {
        required.insert(Capability::QuestionAttemptTimeLimit);
    }

    required
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use question_model::QuestionContentBlock;
    use question_model::answer::{NumericResponseTolerance, TextResponseMatchRule};
    use question_model::assignment_activity_rules::QuestionAttemptLimit;
    use question_model::classification::{QuestionLicense, Tag};
    use question_model::generation::{QuestionGeneratorParameter, QuestionGeneratorReference};
    use question_model::response::QuestionResponseFormat;
    use question_model::{
        QuestionBackendLocator, QuestionFormat, QuestionId, QuestionMetadata,
        QuestionRevisionNumber, QuestionRevisionReference, QuestionType, WorkspaceId,
    };
    use uuid::Uuid;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ViolationCase {
        name: String,
        features: Vec<CaseFeature>,
        required_capabilities: Vec<Capability>,
        supported_capabilities: Vec<Capability>,
        expected_violations: Vec<Capability>,
    }

    #[derive(Debug, Clone, Copy, Deserialize)]
    #[serde(rename_all = "camelCase")]
    enum CaseFeature {
        Seeded,
        AllOrNothing,
        PartialCredit,
        QuestionAttemptTimeLimit,
    }

    fn question_revision(revision_number: u32) -> QuestionRevisionReference {
        QuestionRevisionReference {
            question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
            revision_number: QuestionRevisionNumber::new(revision_number)
                .expect("positive version number"),
        }
    }

    fn base_question(question_revision: QuestionRevisionReference) -> QuestionRevision {
        QuestionRevision {
            question_id: question_revision.question_id,
            revision_number: question_revision.revision_number,
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(100)),
            backend_locator: QuestionBackendLocator::Ple,
            question_format: QuestionFormat::PleAlgorithmic,
            prompt: vec![QuestionContentBlock::Text {
                markdown: "Capability fixture".to_string(),
            }],
            response: QuestionResponseFormat::ShortText {
                match_mode: TextResponseMatchRule::Normalized,
                max_length: 20,
            },
            question_type: QuestionType::FillInBlank,
            question_attempt_limit: QuestionAttemptLimit {
                max_attempts: Some(1),
            },
            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            question_variation_rule: QuestionVariationRule::Static,
            grading: QuestionGradingRule::Ungraded,
            metadata: QuestionMetadata {
                title: "Capability fixture".to_string(),
                question_description: "Instructor-facing capability fixture summary.".to_string(),
                tags: vec![Tag::new("fixture")],
                classifications: Vec::new(),
                question_license: Some(QuestionLicense::CcBy4_0),
                question_citation: None,
                language: "en-US".to_string(),
            },
        }
    }

    fn apply_feature(question: &mut QuestionRevision, feature: CaseFeature) {
        match feature {
            CaseFeature::Seeded => {
                question.question_variation_rule = QuestionVariationRule::Seeded {
                    generator: QuestionGeneratorReference {
                        id: "capability-fixture".to_string(),
                        version: "1".to_string(),
                    },
                    parameters: BTreeMap::from([(
                        "mass".to_string(),
                        QuestionGeneratorParameter::IntegerRange { low: 1, high: 2 },
                    )]),
                };
            }
            CaseFeature::AllOrNothing => {
                question.grading = QuestionGradingRule::AllOrNothing { points: 1.0 };
            }
            CaseFeature::PartialCredit => {
                question.grading = QuestionGradingRule::PartialCredit { points: 1.0 };
                question.response = QuestionResponseFormat::Numeric {
                    tolerance: NumericResponseTolerance::Absolute { epsilon: 0.1 },
                    unit: None,
                };
            }
            CaseFeature::QuestionAttemptTimeLimit => {
                question.question_attempt_time_limit = QuestionAttemptTimeLimit::Limited {
                    seconds: 30,
                    grace_seconds: 2,
                };
            }
        }
    }

    #[test]
    fn committed_violation_table_covers_every_capability() {
        let cases: Vec<ViolationCase> =
            serde_json::from_str(include_str!("../tests/capability_violation_cases.json"))
                .expect("committed capability cases should parse");
        let mut covered = BTreeSet::new();

        for (index, case) in cases.into_iter().enumerate() {
            let question_revision =
                question_revision(u32::try_from(index + 1).expect("fixture index fits u32"));
            let mut question = base_question(question_revision.clone());
            for feature in case.features {
                apply_feature(&mut question, feature);
            }
            let config = AssignmentConfig {
                questions: vec![AssignmentQuestionConfig {
                    question,
                    question_backend_capabilities: case
                        .supported_capabilities
                        .into_iter()
                        .collect(),
                }],
                required_capabilities: case.required_capabilities,
            };
            let actual = validate_assignment_config(&config);
            let expected: Vec<_> = case
                .expected_violations
                .iter()
                .copied()
                .map(|capability| Violation {
                    question: question_revision.clone(),
                    capability,
                })
                .collect();

            assert_eq!(actual, expected, "{}", case.name);
            covered.extend(case.expected_violations);
        }

        assert_eq!(covered, Capability::ALL.into_iter().collect());
    }

    #[test]
    fn violations_preserve_question_order_and_deduplicate_requirements() {
        let question_revisions = [question_revision(1), question_revision(2)];
        let questions = question_revisions
            .iter()
            .cloned()
            .map(|question_revision| AssignmentQuestionConfig {
                question: base_question(question_revision),
                question_backend_capabilities: QuestionBackendCapabilities::none(),
            })
            .collect();
        let config = AssignmentConfig {
            questions,
            required_capabilities: vec![Capability::PrintExport, Capability::PrintExport],
        };

        assert_eq!(
            validate_assignment_config(&config),
            vec![
                Violation {
                    question: question_revisions[0].clone(),
                    capability: Capability::PrintExport,
                },
                Violation {
                    question: question_revisions[1].clone(),
                    capability: Capability::PrintExport,
                },
            ]
        );
    }

    #[test]
    fn violation_json_uses_the_lower_camel_wire_contract() {
        let violation = Violation {
            question: question_revision(1),
            capability: Capability::QuestionAttemptTimeLimit,
        };
        let json = serde_json::to_string(&violation).expect("violation should serialize");

        assert_eq!(
            json,
            r#"{"question":{"questionId":"ABC-DEFG","revisionNumber":1},"capability":"questionAttemptTimeLimit"}"#
        );
        assert!(json.contains(r#""capability":"questionAttemptTimeLimit""#));
    }
}
