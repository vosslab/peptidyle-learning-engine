//! Key-free, deterministic workspace-draft presentation (MOD-GEN/MOD-WASM).
//!
//! This is intentionally an adapter-independent presentation transform.  It
//! applies the authored parameter map to fields which are visible to an
//! instructor, but never chooses an answer or evaluates a response.

use question_model::capability::Capability;
use question_model::envelope::QuestionContentBlock;
use question_model::generation::{QuestionSeed, QuestionVariationRule};
use question_model::question_library::QuestionBackend;
use question_model::{DraftQuestionBackendLocator, QuestionResponseFormat, WorkspaceId};
use serde::{Deserialize, Serialize};

use crate::generator::{GenerationError, QuestionVariationParameterValue, generate};

/// Browser-safe inputs needed to preview one editable workspace draft.
///
/// This deliberately does not reuse `DraftQuestionContent`: preview needs
/// neither grading policy nor publication-only validation data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPreviewRequest {
    /// Private workspace owning the unversioned draft.
    pub workspace: WorkspaceId,
    /// Draft adapter locator.
    pub backend_locator: DraftQuestionBackendLocator,
    /// Student-facing draft title.
    pub title: String,
    /// Authored prompt blocks.
    pub prompt: Vec<QuestionContentBlock>,
    /// Browser-safe response shape.
    pub response: QuestionResponseFormat,
    /// Deterministic authored parameter specification.
    pub question_variation_rule: QuestionVariationRule,
}

/// Identity-free prompt presentation for one draft and seed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftQuestionPreview {
    /// Private workspace owning the draft.
    pub workspace: WorkspaceId,
    /// Selected deterministic variant.
    pub seed: QuestionSeed,
    /// Student-facing title.
    pub title: String,
    /// Fully constructed prompt for the deterministic Question Variation.
    pub prompt: Vec<QuestionContentBlock>,
    /// Browser-safe response shape.
    pub response: QuestionResponseFormat,
}

/// The explicit result of a local draft-preview request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DraftPreviewResult {
    /// PLE Question presentation is ready locally.
    Ready { preview: DraftQuestionPreview },
    /// This source needs a backend path rather than a synthetic browser preview.
    Unavailable {
        /// Question Backend selected by the draft source.
        backend: QuestionBackend,
        /// Capability the source cannot provide locally.
        capability: Capability,
    },
}

/// Failure while materializing safe prompt presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentationError {
    /// Parameter generation itself was invalid.
    Generation(GenerationError),
    /// A placeholder begins but has no closing delimiter.
    UnclosedPlaceholder { field: &'static str },
    /// A placeholder has no visible parameter name.
    EmptyPlaceholder { field: &'static str },
    /// Prompt content references an unknown generated parameter.
    UnknownParameter {
        field: &'static str,
        parameter: String,
    },
}

impl std::fmt::Display for PresentationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Generation(error) => write!(formatter, "preview generation failed: {error}"),
            Self::UnclosedPlaceholder { field } => {
                write!(formatter, "unclosed placeholder in prompt {field}")
            }
            Self::EmptyPlaceholder { field } => {
                write!(formatter, "empty placeholder in prompt {field}")
            }
            Self::UnknownParameter { field, parameter } => {
                write!(formatter, "unknown parameter {parameter} in prompt {field}")
            }
        }
    }
}

impl std::error::Error for PresentationError {}

/// Produces a local preview when the Question Source is PLE.
///
/// All other draft Question Backends deliberately return an explicit capability result;
/// they do not fall back to an invented browser evaluator.
pub fn preview_ple_draft(
    request: &DraftPreviewRequest,
    seed: QuestionSeed,
) -> Result<DraftPreviewResult, PresentationError> {
    if !matches!(request.backend_locator, DraftQuestionBackendLocator::Ple) {
        return Ok(DraftPreviewResult::Unavailable {
            backend: QuestionBackend::from(&request.backend_locator),
            capability: Capability::OfflinePreview,
        });
    }
    let prompt = build_question_prompt(&request.prompt, seed, &request.question_variation_rule)?;
    Ok(DraftPreviewResult::Ready {
        preview: DraftQuestionPreview {
            workspace: request.workspace,
            seed,
            title: request.title.clone(),
            prompt,
            response: request.response.clone(),
        },
    })
}

/// Applies generated values to the explicitly safe prompt fields.
///
/// The allowed fields are data-driven by `QuestionContentBlock`, rather than an
/// question-content switch: prose, math, image descriptions, code source, and
/// table text.  Asset identifiers, checksums, code language labels, and all
/// response data remain literal.
pub fn build_question_prompt(
    prompt: &[QuestionContentBlock],
    seed: QuestionSeed,
    question_variation_rule: &QuestionVariationRule,
) -> Result<Vec<QuestionContentBlock>, PresentationError> {
    let generated =
        generate(seed, question_variation_rule).map_err(PresentationError::Generation)?;
    prompt
        .iter()
        .map(|block| build_question_content_block(block, &generated.parameters))
        .collect()
}

fn build_question_content_block(
    block: &QuestionContentBlock,
    parameters: &std::collections::BTreeMap<String, QuestionVariationParameterValue>,
) -> Result<QuestionContentBlock, PresentationError> {
    match block {
        QuestionContentBlock::Text { markdown } => Ok(QuestionContentBlock::Text {
            markdown: interpolate(markdown, parameters, "text.markdown")?,
        }),
        QuestionContentBlock::Math { latex, description } => Ok(QuestionContentBlock::Math {
            latex: interpolate(latex, parameters, "math.latex")?,
            description: interpolate(description, parameters, "math.description")?,
        }),
        QuestionContentBlock::Image { asset, description } => Ok(QuestionContentBlock::Image {
            asset: asset.clone(),
            description: interpolate(description, parameters, "image.description")?,
        }),
        QuestionContentBlock::Code { language, source } => Ok(QuestionContentBlock::Code {
            language: language.clone(),
            source: interpolate(source, parameters, "code.source")?,
        }),
        QuestionContentBlock::Table {
            headers,
            rows,
            description,
        } => Ok(QuestionContentBlock::Table {
            headers: headers
                .iter()
                .map(|value| interpolate(value, parameters, "table.headers"))
                .collect::<Result<_, _>>()?,
            rows: rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|value| interpolate(value, parameters, "table.rows"))
                        .collect::<Result<_, _>>()
                })
                .collect::<Result<_, _>>()?,
            description: interpolate(description, parameters, "table.description")?,
        }),
    }
}

fn interpolate(
    value: &str,
    parameters: &std::collections::BTreeMap<String, QuestionVariationParameterValue>,
    field: &'static str,
) -> Result<String, PresentationError> {
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(open) = remaining.find("{{") {
        output.push_str(&remaining[..open]);
        let after_open = &remaining[open + 2..];
        let Some(close) = after_open.find("}}") else {
            return Err(PresentationError::UnclosedPlaceholder { field });
        };
        let parameter = &after_open[..close];
        if parameter.is_empty() {
            return Err(PresentationError::EmptyPlaceholder { field });
        }
        let value =
            parameters
                .get(parameter)
                .ok_or_else(|| PresentationError::UnknownParameter {
                    field,
                    parameter: parameter.to_string(),
                })?;
        output.push_str(&generated_text(value));
        remaining = &after_open[close + 2..];
    }
    output.push_str(remaining);
    Ok(output)
}

fn generated_text(value: &QuestionVariationParameterValue) -> String {
    match value {
        QuestionVariationParameterValue::Integer { value } => value.to_string(),
        QuestionVariationParameterValue::Decimal { value }
        | QuestionVariationParameterValue::Choice { value }
        | QuestionVariationParameterValue::Fixed { value } => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use question_model::DraftQuestionBackendLocator;
    use question_model::answer::TextResponseMatchRule;
    use question_model::envelope::QuestionContentBlock;
    use question_model::generation::{QuestionGeneratorParameter, QuestionGeneratorReference};
    use question_model::response::QuestionResponseFormat;
    use uuid::Uuid;

    use super::*;

    fn request(backend_locator: DraftQuestionBackendLocator) -> DraftPreviewRequest {
        DraftPreviewRequest {
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(7)),
            backend_locator,
            title: "Preview".to_string(),
            prompt: vec![
                QuestionContentBlock::Text {
                    markdown: "Value {{count}} is {{residue}}.".to_string(),
                },
                QuestionContentBlock::Table {
                    headers: vec!["{{residue}}".to_string()],
                    rows: vec![vec!["{{count}}".to_string()]],
                    description: "{{residue}} table".to_string(),
                },
            ],
            response: QuestionResponseFormat::ShortText {
                match_mode: TextResponseMatchRule::Normalized,
                max_length: 20,
            },
            question_variation_rule: QuestionVariationRule::Seeded {
                generator: QuestionGeneratorReference {
                    id: "fixture".to_string(),
                    version: "1".to_string(),
                },
                parameters: BTreeMap::from([
                    (
                        "count".to_string(),
                        QuestionGeneratorParameter::Fixed {
                            value: "4".to_string(),
                        },
                    ),
                    (
                        "residue".to_string(),
                        QuestionGeneratorParameter::Choice {
                            options: vec!["glycine".to_string()],
                        },
                    ),
                ]),
            },
        }
    }

    #[test]
    fn ple_preview_is_key_free_and_builds_every_safe_text_field() {
        let result = preview_ple_draft(
            &request(DraftQuestionBackendLocator::Ple),
            QuestionSeed::new(19),
        )
        .expect("valid preview");
        let DraftPreviewResult::Ready { preview } = result else {
            panic!("PLE Question Source is ready")
        };
        assert_eq!(preview.seed, QuestionSeed::new(19));
        assert_eq!(
            preview.prompt[0],
            QuestionContentBlock::Text {
                markdown: "Value 4 is glycine.".to_string()
            }
        );
        let encoded = serde_json::to_string(&preview).expect("safe preview serializes");
        for forbidden in [
            "problem", "version", "answer", "key", "grading", "correct", "score",
        ] {
            assert!(!encoded.contains(forbidden), "preview leaked {forbidden}");
        }
    }

    #[test]
    fn non_ple_sources_are_explicitly_unavailable() {
        let result = preview_ple_draft(
            &request(DraftQuestionBackendLocator::Webwork {
                pg_path: "set/a.pg".to_string(),
            }),
            QuestionSeed::new(1),
        )
        .expect("unavailable is a valid result");
        assert_eq!(
            result,
            DraftPreviewResult::Unavailable {
                backend: QuestionBackend::Webwork,
                capability: Capability::OfflinePreview,
            }
        );
    }

    #[test]
    fn unresolved_or_unknown_placeholders_are_explicit() {
        let mut request = request(DraftQuestionBackendLocator::Ple);
        request.prompt = vec![QuestionContentBlock::Text {
            markdown: "{{missing}}".to_string(),
        }];
        assert!(matches!(
            preview_ple_draft(&request, QuestionSeed::new(1)),
            Err(PresentationError::UnknownParameter { parameter, .. }) if parameter == "missing"
        ));
        request.prompt = vec![QuestionContentBlock::Text {
            markdown: "{{missing".to_string(),
        }];
        assert!(matches!(
            preview_ple_draft(&request, QuestionSeed::new(1)),
            Err(PresentationError::UnclosedPlaceholder { .. })
        ));
    }
}
