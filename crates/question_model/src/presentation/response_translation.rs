//! Server-side translation from rendered response identifiers to durable IDs.
//!
//! The presentation binding is server-only.  This operation is deliberately
//! pure: callers must reproduce and authenticate a [`PresentationV1`] before
//! translating a browser response, and validation of the response's bounded
//! public shape remains the caller's responsibility.

use crate::response::{ChoiceId, MatchPair, StudentResponse, TextEntryAnswer};
use serde::{Deserialize, Serialize};

use super::{PresentationV1, RenderedItemIdV1, RenderedItemRoleV1};

/// Fail-closed reasons a browser-rendered identifier cannot be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderedResponseTranslationErrorV1 {
    /// The response identifier does not have the closed rendered-ID format.
    MalformedRenderedId,
    /// The rendered identifier is not present in the issued presentation.
    UnknownRenderedId,
    /// The issued presentation maps one rendered identifier more than once.
    DuplicateRenderedIdBinding,
    /// The identifier belongs to another response role in this presentation.
    WrongRenderedItemRole,
}

/// A solution-free rendering of one immutable submitted response.
///
/// This is deliberately a closed projection.  It contains only the Student's
/// submitted values and the rendered identifiers from the issued presentation;
/// answer keys, grader material, durable object keys, and provider payloads
/// have no representation here.  The server creates it after it has verified
/// the issued presentation witness.  ASVS 14.1.1 and 14.2.1: sensitive
/// educational-record data has one minimized response shape.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum InspectedStudentResponseV1 {
    /// A numeric value the Student submitted.
    Numeric {
        /// Submitted numeric value.
        value: f64,
    },
    /// Rendered choice identifiers the Student selected.
    MultipleChoice {
        /// Issued rendered choice identifiers, never durable choice IDs.
        selected: Vec<RenderedItemIdV1>,
    },
    /// A short text value the Student submitted.
    ShortText {
        /// Submitted text.
        text: String,
    },
    /// Text entries bound to their issued rendered blank identifiers.
    MultiBlank {
        /// Submitted blank entries.
        answers: Vec<InspectedTextEntryV1>,
    },
    /// Associations bound to issued rendered prompt and choice identifiers.
    Matching {
        /// Submitted associations.
        matches: Vec<InspectedMatchPairV1>,
    },
    /// Issued rendered order-item identifiers in Student-selected order.
    Ordering {
        /// Submitted ordering.
        order: Vec<RenderedItemIdV1>,
    },
    /// Submitted hotspot coordinates.
    Hotspot {
        /// Submitted points, without image storage or answer data.
        points: Vec<crate::response::HotspotPoint>,
    },
    /// Coarse file-upload submission state.
    FileUpload {
        /// Safe submitted-artifact state, without a download locator.
        artifact: InspectedStudentArtifactStateV1,
    },
    /// Coarse external-tool completion state.
    ExternalTool {
        /// Safe completion state, without provider data or launch authority.
        completion: InspectedExternalToolStateV1,
    },
}

/// One text entry bound to the rendered blank identifier visible in the issue.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspectedTextEntryV1 {
    /// Issued rendered blank identifier binding this entry.
    pub slot: RenderedItemIdV1,
    /// Text submitted for the rendered blank.
    pub text: String,
}

/// One association bound to rendered prompt and choice identifiers.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspectedMatchPairV1 {
    /// Issued rendered prompt identifier.
    pub prompt: RenderedItemIdV1,
    /// Issued rendered choice identifier.
    pub choice: RenderedItemIdV1,
}

/// Safe artifact fact. Storage location and download authority stay private.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InspectedStudentArtifactStateV1 {
    /// The Student submitted an artifact; its locator remains private.
    Submitted,
}

/// Safe external-tool fact. Provider data and launch authority stay private.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InspectedExternalToolStateV1 {
    /// The external-tool submission was recorded; provider details remain private.
    SubmissionRecorded,
}

impl std::fmt::Debug for InspectedStudentResponseV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            Self::Numeric { .. } => "numeric",
            Self::MultipleChoice { .. } => "multiple_choice",
            Self::ShortText { .. } => "short_text",
            Self::MultiBlank { .. } => "multi_blank",
            Self::Matching { .. } => "matching",
            Self::Ordering { .. } => "ordering",
            Self::Hotspot { .. } => "hotspot",
            Self::FileUpload { .. } => "file_upload",
            Self::ExternalTool { .. } => "external_tool",
        };
        formatter
            .debug_struct("InspectedStudentResponseV1")
            .field("kind", &kind)
            .finish()
    }
}

impl std::fmt::Debug for InspectedTextEntryV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("InspectedTextEntryV1([REDACTED])")
    }
}

impl std::fmt::Debug for InspectedMatchPairV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("InspectedMatchPairV1([REDACTED])")
    }
}

impl std::fmt::Display for RenderedResponseTranslationErrorV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedRenderedId => formatter.write_str("rendered response ID is malformed"),
            Self::UnknownRenderedId => formatter.write_str("rendered response ID is unknown"),
            Self::DuplicateRenderedIdBinding => {
                formatter.write_str("rendered response ID has duplicate issued bindings")
            }
            Self::WrongRenderedItemRole => {
                formatter.write_str("rendered response ID has the wrong issued role")
            }
        }
    }
}

impl std::error::Error for RenderedResponseTranslationErrorV1 {}

/// Converts browser-rendered item IDs into the durable IDs bound to one issue.
///
/// Only identifier-bearing Question Response Formats are rewritten. Scalar
/// response formats preserve their values exactly. The function intentionally exposes
/// no durable mapping or serializable wire type.
pub fn translate_rendered_response_v1(
    response: &StudentResponse,
    presentation: &PresentationV1,
) -> Result<StudentResponse, RenderedResponseTranslationErrorV1> {
    let durable_id = |id: &ChoiceId, role| durable_id_v1(id, role, presentation);

    match response {
        StudentResponse::MultipleChoice { selected } => Ok(StudentResponse::MultipleChoice {
            selected: selected
                .iter()
                .map(|id| durable_id(id, RenderedItemRoleV1::Choice))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::MultiBlank { answers } => Ok(StudentResponse::MultiBlank {
            answers: answers
                .iter()
                .map(|answer| {
                    Ok(TextEntryAnswer {
                        slot: durable_id(&answer.slot, RenderedItemRoleV1::Blank)?,
                        text: answer.text.clone(),
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Matching { matches } => Ok(StudentResponse::Matching {
            matches: matches
                .iter()
                .map(|pair| {
                    Ok(MatchPair {
                        prompt: durable_id(&pair.prompt, RenderedItemRoleV1::MatchPrompt)?,
                        choice: durable_id(&pair.choice, RenderedItemRoleV1::MatchChoice)?,
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Ordering { order } => Ok(StudentResponse::Ordering {
            order: order
                .iter()
                .map(|id| durable_id(id, RenderedItemRoleV1::OrderItem))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Numeric { value } => Ok(StudentResponse::Numeric { value: *value }),
        StudentResponse::ShortText { text } => {
            Ok(StudentResponse::ShortText { text: text.clone() })
        }
        StudentResponse::Hotspot { points } => Ok(StudentResponse::Hotspot {
            points: points.clone(),
        }),
        StudentResponse::FileUpload { object_key } => Ok(StudentResponse::FileUpload {
            object_key: object_key.clone(),
        }),
        StudentResponse::ExternalTool {} => Ok(StudentResponse::ExternalTool {}),
    }
}

/// Projects a durable submitted response into the exact rendered identifiers
/// of a verified issued presentation.
///
/// The inverse mapping is intentionally available only at the trusted
/// inspection boundary.  It is pure and does not reveal grading material.
pub fn project_durable_response_to_rendered_v1(
    response: &StudentResponse,
    presentation: &PresentationV1,
) -> Result<InspectedStudentResponseV1, RenderedResponseTranslationErrorV1> {
    let rendered_id = |id: &ChoiceId, role| rendered_id_v1(id, role, presentation);
    match response {
        StudentResponse::Numeric { value } => {
            Ok(InspectedStudentResponseV1::Numeric { value: *value })
        }
        StudentResponse::MultipleChoice { selected } => {
            Ok(InspectedStudentResponseV1::MultipleChoice {
                selected: selected
                    .iter()
                    .map(|id| rendered_id(id, RenderedItemRoleV1::Choice))
                    .collect::<Result<_, _>>()?,
            })
        }
        StudentResponse::ShortText { text } => {
            Ok(InspectedStudentResponseV1::ShortText { text: text.clone() })
        }
        StudentResponse::MultiBlank { answers } => Ok(InspectedStudentResponseV1::MultiBlank {
            answers: answers
                .iter()
                .map(|answer| {
                    Ok(InspectedTextEntryV1 {
                        slot: rendered_id(&answer.slot, RenderedItemRoleV1::Blank)?,
                        text: answer.text.clone(),
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Matching { matches } => Ok(InspectedStudentResponseV1::Matching {
            matches: matches
                .iter()
                .map(|pair| {
                    Ok(InspectedMatchPairV1 {
                        prompt: rendered_id(&pair.prompt, RenderedItemRoleV1::MatchPrompt)?,
                        choice: rendered_id(&pair.choice, RenderedItemRoleV1::MatchChoice)?,
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Ordering { order } => Ok(InspectedStudentResponseV1::Ordering {
            order: order
                .iter()
                .map(|id| rendered_id(id, RenderedItemRoleV1::OrderItem))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Hotspot { points } => Ok(InspectedStudentResponseV1::Hotspot {
            points: points.clone(),
        }),
        StudentResponse::FileUpload { .. } => Ok(InspectedStudentResponseV1::FileUpload {
            artifact: InspectedStudentArtifactStateV1::Submitted,
        }),
        StudentResponse::ExternalTool {} => Ok(InspectedStudentResponseV1::ExternalTool {
            completion: InspectedExternalToolStateV1::SubmissionRecorded,
        }),
    }
}

/// Validates an immutable browser-submitted response against its exact issued
/// presentation and returns the safe inspection projection for that issue.
///
/// Accepted-submission storage preserves the browser contract verbatim. The
/// inspection boundary validates each identifier against the reconstructed
/// public issue and retains that exact rendered identifier. Reconstructed
/// browser-safe presentations intentionally contain no durable identifiers.
pub fn project_rendered_response_for_inspection_v1(
    response: &StudentResponse,
    presentation: &PresentationV1,
) -> Result<InspectedStudentResponseV1, RenderedResponseTranslationErrorV1> {
    let rendered_id = |id: &ChoiceId, role| verified_rendered_id_v1(id, role, presentation);
    match response {
        StudentResponse::Numeric { value } => {
            Ok(InspectedStudentResponseV1::Numeric { value: *value })
        }
        StudentResponse::MultipleChoice { selected } => {
            Ok(InspectedStudentResponseV1::MultipleChoice {
                selected: selected
                    .iter()
                    .map(|id| rendered_id(id, RenderedItemRoleV1::Choice))
                    .collect::<Result<_, _>>()?,
            })
        }
        StudentResponse::ShortText { text } => {
            Ok(InspectedStudentResponseV1::ShortText { text: text.clone() })
        }
        StudentResponse::MultiBlank { answers } => Ok(InspectedStudentResponseV1::MultiBlank {
            answers: answers
                .iter()
                .map(|answer| {
                    Ok(InspectedTextEntryV1 {
                        slot: rendered_id(&answer.slot, RenderedItemRoleV1::Blank)?,
                        text: answer.text.clone(),
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Matching { matches } => Ok(InspectedStudentResponseV1::Matching {
            matches: matches
                .iter()
                .map(|pair| {
                    Ok(InspectedMatchPairV1 {
                        prompt: rendered_id(&pair.prompt, RenderedItemRoleV1::MatchPrompt)?,
                        choice: rendered_id(&pair.choice, RenderedItemRoleV1::MatchChoice)?,
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Ordering { order } => Ok(InspectedStudentResponseV1::Ordering {
            order: order
                .iter()
                .map(|id| rendered_id(id, RenderedItemRoleV1::OrderItem))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Hotspot { points } => Ok(InspectedStudentResponseV1::Hotspot {
            points: points.clone(),
        }),
        StudentResponse::FileUpload { .. } => Ok(InspectedStudentResponseV1::FileUpload {
            artifact: InspectedStudentArtifactStateV1::Submitted,
        }),
        StudentResponse::ExternalTool {} => Ok(InspectedStudentResponseV1::ExternalTool {
            completion: InspectedExternalToolStateV1::SubmissionRecorded,
        }),
    }
}

fn durable_id_v1(
    id: &ChoiceId,
    expected_role: RenderedItemRoleV1,
    presentation: &PresentationV1,
) -> Result<ChoiceId, RenderedResponseTranslationErrorV1> {
    Ok(ChoiceId::new(
        rendered_binding_v1(id, expected_role, presentation)?
            .durable_id
            .clone(),
    ))
}

fn verified_rendered_id_v1(
    id: &ChoiceId,
    expected_role: RenderedItemRoleV1,
    presentation: &PresentationV1,
) -> Result<RenderedItemIdV1, RenderedResponseTranslationErrorV1> {
    Ok(rendered_binding_v1(id, expected_role, presentation)?
        .rendered
        .clone())
}

fn rendered_binding_v1<'a>(
    id: &ChoiceId,
    expected_role: RenderedItemRoleV1,
    presentation: &'a PresentationV1,
) -> Result<&'a super::RenderedItemBindingV1, RenderedResponseTranslationErrorV1> {
    let rendered = RenderedItemIdV1::parse(id.as_str())
        .map_err(|_| RenderedResponseTranslationErrorV1::MalformedRenderedId)?;
    let mut bindings = presentation
        .item_bindings
        .iter()
        .filter(|binding| binding.rendered == rendered);
    let binding = bindings
        .next()
        .ok_or(RenderedResponseTranslationErrorV1::UnknownRenderedId)?;
    if bindings.next().is_some() {
        return Err(RenderedResponseTranslationErrorV1::DuplicateRenderedIdBinding);
    }
    if binding.role != expected_role {
        return Err(RenderedResponseTranslationErrorV1::WrongRenderedItemRole);
    }
    Ok(binding)
}

fn rendered_id_v1(
    durable: &ChoiceId,
    expected_role: RenderedItemRoleV1,
    presentation: &PresentationV1,
) -> Result<RenderedItemIdV1, RenderedResponseTranslationErrorV1> {
    let mut bindings = presentation
        .item_bindings
        .iter()
        .filter(|binding| binding.durable_id == durable.as_str());
    let binding = bindings
        .next()
        .ok_or(RenderedResponseTranslationErrorV1::UnknownRenderedId)?;
    if bindings.next().is_some() {
        return Err(RenderedResponseTranslationErrorV1::DuplicateRenderedIdBinding);
    }
    if binding.role != expected_role {
        return Err(RenderedResponseTranslationErrorV1::WrongRenderedItemRole);
    }
    Ok(binding.rendered.clone())
}
