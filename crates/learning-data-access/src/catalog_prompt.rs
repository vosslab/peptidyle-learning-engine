//! One answer-free, deterministic prompt projection for catalog detail.

use domain::draft_preview::materialize_prompt;
use question_model::generation::{RandomizationDefinition, Seed};
use question_model::{CatalogPromptProjection, QuestionDefinition};

use crate::StoreError;

/// Stable product seed for the representative catalog example.
///
/// The seed stays server-side. Changing it would change a visible projection
/// of every existing generated publication, so it is part of the catalog
/// presentation contract even though it is not a browser field.
const CATALOG_GENERATED_EXAMPLE_SEED_VALUE: u64 = 0x504c_452d_4341_5431;

pub(crate) fn catalog_prompt_projection(
    question: &QuestionDefinition,
) -> Result<CatalogPromptProjection, StoreError> {
    match &question.randomization {
        RandomizationDefinition::Static => Ok(CatalogPromptProjection::Static {
            blocks: question.prompt.clone(),
        }),
        RandomizationDefinition::Seeded { .. } => materialize_prompt(
            &question.prompt,
            Seed::new(CATALOG_GENERATED_EXAMPLE_SEED_VALUE),
            &question.randomization,
        )
        .map(|blocks| CatalogPromptProjection::GeneratedExample { blocks })
        .map_err(|_| {
            StoreError::InvalidRecord("published catalog prompt cannot be materialized".to_string())
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use question_model::answer::NumericTolerance;
    use question_model::envelope::ContentBlock;
    use question_model::generation::{GeneratorReference, ParameterSpec};
    use question_model::run_policy::{AttemptPolicy, TimingPolicy};
    use question_model::taxonomy::{License, Tag};
    use question_model::{
        DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, QuestionMetadata,
        ResponseDefinition,
    };
    use uuid::Uuid;

    use super::*;

    fn question(randomization: RandomizationDefinition, markdown: &str) -> QuestionDefinition {
        let draft = DraftQuestionDefinition {
            workspace: question_model::WorkspaceId::from_uuid(Uuid::from_u128(1)),
            source: DraftQuestionSource::Native {
                family: "catalog-projection-fixture".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: markdown.to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.01 },
                unit: None,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Catalog projection fixture".to_string(),
                tags: vec![Tag::new("catalog")],
                taxonomy: Vec::new(),
                license: License::Cc0,
                language: "en".to_string(),
            },
        };
        QuestionDefinition::from_draft(
            draft,
            question_model::ProblemId::from_uuid(Uuid::from_u128(2)),
            question_model::VersionId::from_uuid(Uuid::from_u128(3)),
            question_model::QuestionSource::Native {
                family: "catalog-projection-fixture".to_string(),
            },
        )
    }

    #[test]
    fn static_prompt_preserves_authored_literal_content() {
        let projection = catalog_prompt_projection(&question(
            RandomizationDefinition::Static,
            "Literal {{text}} remains literal.",
        ))
        .expect("static prompt projects");
        assert_eq!(
            projection,
            CatalogPromptProjection::Static {
                blocks: vec![ContentBlock::Text {
                    markdown: "Literal {{text}} remains literal.".to_string(),
                }],
            }
        );
    }

    #[test]
    fn generated_example_is_deterministic_and_resolves_every_parameter() {
        let randomization = RandomizationDefinition::Seeded {
            generator: GeneratorReference {
                id: "catalog-projection-fixture".to_string(),
                version: "1".to_string(),
            },
            parameters: BTreeMap::from([(
                "residue".to_string(),
                ParameterSpec::Choice {
                    options: vec!["glycine".to_string()],
                },
            )]),
        };
        let question = question(randomization, "A {{residue}} example.");
        let first = catalog_prompt_projection(&question).expect("example projects");
        let second = catalog_prompt_projection(&question).expect("example replays");
        assert_eq!(first, second);
        assert_eq!(
            first,
            CatalogPromptProjection::GeneratedExample {
                blocks: vec![ContentBlock::Text {
                    markdown: "A glycine example.".to_string(),
                }],
            }
        );
    }

    #[test]
    fn invalid_generated_template_returns_one_safe_corruption_category() {
        let randomization = RandomizationDefinition::Seeded {
            generator: GeneratorReference {
                id: "catalog-projection-fixture".to_string(),
                version: "1".to_string(),
            },
            parameters: BTreeMap::new(),
        };
        assert_eq!(
            catalog_prompt_projection(&question(randomization, "{{missing}}")),
            Err(StoreError::InvalidRecord(
                "published catalog prompt cannot be materialized".to_string(),
            ))
        );
    }
}
