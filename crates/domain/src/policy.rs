//! Assignment/backend capability validation (MOD-CAP).
//!
//! The editor and publish route call the same pure function. Each violation
//! names the immutable question version and one missing capability, and the
//! complete list is returned in question order and capability declaration
//! order so an instructor can repair everything in one pass.

use std::collections::BTreeSet;

use question_model::generation::RandomizationDefinition;
use question_model::run_policy::TimingPolicy;
use question_model::{
    BackendCapabilities, Capability, DraftQuestionDefinition, GradingDefinition,
    QuestionDefinition, VersionId,
};
use serde::{Deserialize, Serialize};

/// One selected question and its backend's honest capability declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentQuestionConfig {
    /// Browser-safe immutable question definition selected by the assignment.
    pub question: QuestionDefinition,
    /// Capabilities declared by the adapter that owns this question.
    pub backend_capabilities: BackendCapabilities,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Violation {
    /// Immutable question version whose backend lacks support.
    pub question: VersionId,
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
            if required.contains(&capability) && !selected.backend_capabilities.supports(capability)
            {
                violations.push(Violation {
                    question: selected.question.version,
                    capability,
                });
            }
        }
    }

    violations
}

/// Validates a draft at the publication boundary without inventing a version ID.
pub fn validate_draft_for_publication(
    question: &DraftQuestionDefinition,
    backend_capabilities: &BackendCapabilities,
) -> Vec<PublicationViolation> {
    required_by_content(question)
        .into_iter()
        .filter(|capability| !backend_capabilities.supports(*capability))
        .map(|capability| PublicationViolation {
            workspace: question.workspace,
            title: question.metadata.title.clone(),
            capability,
        })
        .collect()
}

fn required_by_question(question: &QuestionDefinition) -> BTreeSet<Capability> {
    required_by_content(question)
}

trait QuestionContentView {
    fn randomization(&self) -> &question_model::generation::RandomizationDefinition;
    fn grading(&self) -> &GradingDefinition;
    fn timing_policy(&self) -> &TimingPolicy;
}

impl QuestionContentView for QuestionDefinition {
    fn randomization(&self) -> &question_model::generation::RandomizationDefinition {
        &self.randomization
    }
    fn grading(&self) -> &GradingDefinition {
        &self.grading
    }
    fn timing_policy(&self) -> &TimingPolicy {
        &self.timing_policy
    }
}

impl QuestionContentView for DraftQuestionDefinition {
    fn randomization(&self) -> &question_model::generation::RandomizationDefinition {
        &self.randomization
    }
    fn grading(&self) -> &GradingDefinition {
        &self.grading
    }
    fn timing_policy(&self) -> &TimingPolicy {
        &self.timing_policy
    }
}

fn required_by_content(question: &impl QuestionContentView) -> BTreeSet<Capability> {
    let mut required = BTreeSet::new();

    if matches!(
        question.randomization(),
        RandomizationDefinition::Seeded { .. }
    ) {
        required.insert(Capability::AlgorithmicGeneration);
    }
    match question.grading() {
        GradingDefinition::AllOrNothing { .. } => {
            required.insert(Capability::ServerGrading);
        }
        GradingDefinition::PartialCredit { .. } => {
            required.insert(Capability::ServerGrading);
            required.insert(Capability::PartialCredit);
        }
        GradingDefinition::Ungraded => {}
    }
    if matches!(question.timing_policy(), TimingPolicy::PerQuestion { .. }) {
        required.insert(Capability::PerQuestionTiming);
    }

    required
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use question_model::answer::{NumericTolerance, TextMatchMode};
    use question_model::envelope::ContentBlock;
    use question_model::generation::{GeneratorReference, ParameterSpec};
    use question_model::response::ResponseDefinition;
    use question_model::run_policy::AttemptPolicy;
    use question_model::taxonomy::{License, Tag};
    use question_model::{ProblemId, QuestionMetadata, QuestionSource, WorkspaceId};
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
        PerQuestionTiming,
    }

    fn base_question(version: VersionId) -> QuestionDefinition {
        QuestionDefinition {
            version,
            problem: ProblemId::from_uuid(Uuid::from_u128(99)),
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(100)),
            source: QuestionSource::Native {
                family: "capability-fixture".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Capability fixture".to_string(),
            }],
            response: ResponseDefinition::ShortText {
                match_mode: TextMatchMode::Normalized,
                max_length: 20,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: Some(1),
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::Ungraded,
            metadata: QuestionMetadata {
                title: "Capability fixture".to_string(),
                tags: vec![Tag::new("fixture")],
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        }
    }

    fn apply_feature(question: &mut QuestionDefinition, feature: CaseFeature) {
        match feature {
            CaseFeature::Seeded => {
                question.randomization = RandomizationDefinition::Seeded {
                    generator: GeneratorReference {
                        id: "capability-fixture".to_string(),
                        version: "1".to_string(),
                    },
                    parameters: BTreeMap::from([(
                        "mass".to_string(),
                        ParameterSpec::IntegerRange { low: 1, high: 2 },
                    )]),
                };
            }
            CaseFeature::AllOrNothing => {
                question.grading = GradingDefinition::AllOrNothing { points: 1.0 };
            }
            CaseFeature::PartialCredit => {
                question.grading = GradingDefinition::PartialCredit { points: 1.0 };
                question.response = ResponseDefinition::Numeric {
                    tolerance: NumericTolerance::Absolute { epsilon: 0.1 },
                    unit: None,
                };
            }
            CaseFeature::PerQuestionTiming => {
                question.timing_policy = TimingPolicy::PerQuestion {
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
            let version = VersionId::from_uuid(Uuid::from_u128(
                u128::try_from(index + 1).expect("fixture index fits u128"),
            ));
            let mut question = base_question(version);
            for feature in case.features {
                apply_feature(&mut question, feature);
            }
            let config = AssignmentConfig {
                questions: vec![AssignmentQuestionConfig {
                    question,
                    backend_capabilities: case.supported_capabilities.into_iter().collect(),
                }],
                required_capabilities: case.required_capabilities,
            };
            let actual = validate_assignment_config(&config);
            let expected: Vec<_> = case
                .expected_violations
                .iter()
                .copied()
                .map(|capability| Violation {
                    question: version,
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
        let versions = [
            VersionId::from_uuid(Uuid::from_u128(1)),
            VersionId::from_uuid(Uuid::from_u128(2)),
        ];
        let questions = versions
            .into_iter()
            .map(|version| AssignmentQuestionConfig {
                question: base_question(version),
                backend_capabilities: BackendCapabilities::none(),
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
                    question: versions[0],
                    capability: Capability::PrintExport,
                },
                Violation {
                    question: versions[1],
                    capability: Capability::PrintExport,
                },
            ]
        );
    }

    #[test]
    fn violation_json_uses_the_lower_camel_wire_contract() {
        let violation = Violation {
            question: VersionId::from_uuid(Uuid::from_u128(1)),
            capability: Capability::PerQuestionTiming,
        };
        let json = serde_json::to_string(&violation).expect("violation should serialize");

        assert!(json.contains(r#""question":"00000000-0000-0000-0000-000000000001""#));
        assert!(json.contains(r#""capability":"perQuestionTiming""#));
    }
}
