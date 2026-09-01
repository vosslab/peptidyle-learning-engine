//! Server-side translation from rendered response identifiers to durable IDs.
//!
//! The presentation binding is server-only.  This operation is deliberately
//! pure: callers must reproduce and authenticate an [`IssuedQuestionPresentation`] before
//! translating a browser response, and validation of the response's bounded
//! public shape remains the caller's responsibility.

use crate::response::{
    ResponseItemReference, StudentHotspotSelection, StudentMatch, StudentResponse, StudentTextEntry,
};
use serde::{Deserialize, Serialize};

use super::{IssuedQuestionPresentation, PresentationResponseItemReference, ResponseItemRole};

/// Fail-closed reasons a browser-rendered identifier cannot be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderedResponseTranslationError {
    /// The response identifier does not have the closed rendered-ID format.
    MalformedRenderedId,
    /// The rendered identifier is not present in the issued presentation.
    UnknownRenderedId,
    /// The issued presentation maps one rendered identifier more than once.
    DuplicateRenderedIdBinding,
    /// The identifier belongs to another response role in this presentation.
    WrongRenderedItemRole,
}

/// A closed rendering of one immutable submitted Student Response.
///
/// This is deliberately a closed projection.  It contains only the Student's
/// submitted values and the rendered identifiers from the issued presentation;
/// answer keys, grader material, durable Object Addresses, and provider payloads
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
pub enum StudentResponseInspection {
    /// A numeric value the Student submitted.
    Numeric {
        /// Submitted numeric value.
        value: f64,
    },
    /// Rendered choice identifiers the Student selected.
    MultipleChoice {
        /// Issued rendered choice identifiers, never durable choice IDs.
        selected: Vec<PresentationResponseItemReference>,
    },
    /// A short text value the Student submitted.
    ShortText {
        /// Submitted text.
        text: String,
    },
    /// Text entries bound to their issued rendered blank identifiers.
    MultiBlank {
        /// Submitted blank entries.
        answers: Vec<InspectedTextEntry>,
    },
    /// Associations bound to issued rendered prompt and choice identifiers.
    Matching {
        /// Submitted associations.
        matches: Vec<InspectedMatchPair>,
    },
    /// Issued rendered order-item identifiers in Student-selected order.
    Ordering {
        /// Submitted ordering.
        order: Vec<PresentationResponseItemReference>,
    },
    /// Submitted Hotspot Region selections.
    Hotspot {
        /// Issued rendered Hotspot Region identifiers selected by the Student.
        selected_regions: Vec<PresentationResponseItemReference>,
    },
    /// Coarse external-tool completion state.
    ExternalTool {
        /// Safe completion state, without provider data or launch authority.
        completion: InspectedExternalToolState,
    },
}

/// One text entry bound to the rendered blank identifier visible in the issue.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspectedTextEntry {
    /// Issued rendered blank identifier binding this entry.
    pub slot: PresentationResponseItemReference,
    /// Text submitted for the rendered blank.
    pub text: String,
}

/// One association bound to rendered prompt and choice identifiers.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspectedMatchPair {
    /// Issued rendered prompt identifier.
    pub prompt: PresentationResponseItemReference,
    /// Issued rendered choice identifier.
    pub choice: PresentationResponseItemReference,
}

/// Safe external-tool fact. Provider data and launch authority stay private.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InspectedExternalToolState {
    /// The external-tool submission was recorded; provider details remain private.
    SubmissionRecorded,
}

impl std::fmt::Debug for StudentResponseInspection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            Self::Numeric { .. } => "numeric",
            Self::MultipleChoice { .. } => "multiple_choice",
            Self::ShortText { .. } => "short_text",
            Self::MultiBlank { .. } => "multi_blank",
            Self::Matching { .. } => "matching",
            Self::Ordering { .. } => "ordering",
            Self::Hotspot { .. } => "hotspot",
            Self::ExternalTool { .. } => "external_tool",
        };
        formatter
            .debug_struct("StudentResponseInspection")
            .field("kind", &kind)
            .finish()
    }
}

impl std::fmt::Debug for InspectedTextEntry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("InspectedTextEntry([REDACTED])")
    }
}

impl std::fmt::Debug for InspectedMatchPair {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("InspectedMatchPair([REDACTED])")
    }
}

impl std::fmt::Display for RenderedResponseTranslationError {
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

impl std::error::Error for RenderedResponseTranslationError {}

/// Converts browser-rendered item IDs into the durable IDs bound to one issue.
///
/// Only identifier-bearing Question Response Formats are rewritten. Scalar
/// response formats preserve their values exactly. The function intentionally exposes
/// no durable mapping or serializable wire type.
pub fn translate_rendered_response(
    response: &StudentResponse,
    presentation: &IssuedQuestionPresentation,
) -> Result<StudentResponse, RenderedResponseTranslationError> {
    let response_item_reference = |id: &ResponseItemReference, role| {
        translated_response_item_reference(id, role, presentation)
    };

    match response {
        StudentResponse::MultipleChoice { selected } => Ok(StudentResponse::MultipleChoice {
            selected: selected
                .iter()
                .map(|id| response_item_reference(id, ResponseItemRole::QuestionChoice))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::MultiBlank { answers } => Ok(StudentResponse::MultiBlank {
            answers: answers
                .iter()
                .map(|answer| {
                    Ok(StudentTextEntry {
                        slot: response_item_reference(
                            &answer.slot,
                            ResponseItemRole::TextEntrySlot,
                        )?,
                        text: answer.text.clone(),
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Matching { matches } => Ok(StudentResponse::Matching {
            matches: matches
                .iter()
                .map(|pair| {
                    Ok(StudentMatch {
                        prompt: response_item_reference(
                            &pair.prompt,
                            ResponseItemRole::MatchingPrompt,
                        )?,
                        choice: response_item_reference(
                            &pair.choice,
                            ResponseItemRole::MatchingChoice,
                        )?,
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Ordering { order } => Ok(StudentResponse::Ordering {
            order: order
                .iter()
                .map(|id| response_item_reference(id, ResponseItemRole::OrderingItem))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Numeric { value } => Ok(StudentResponse::Numeric { value: *value }),
        StudentResponse::ShortText { text } => {
            Ok(StudentResponse::ShortText { text: text.clone() })
        }
        StudentResponse::Hotspot { selections } => Ok(StudentResponse::Hotspot {
            selections: selections
                .iter()
                .map(|selection| {
                    Ok(StudentHotspotSelection {
                        region: response_item_reference(
                            &selection.region,
                            ResponseItemRole::HotspotRegion,
                        )?,
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::ExternalTool {} => Ok(StudentResponse::ExternalTool {}),
    }
}

/// Projects a durable submitted response into the exact rendered identifiers
/// of a verified issued presentation.
///
/// The inverse mapping is intentionally available only at the trusted
/// inspection boundary. It is pure and does not reveal Answer Keys, Question
/// Feedback, Question Answer Explanations, or Question Grading Input.
pub fn project_durable_response_to_rendered(
    response: &StudentResponse,
    presentation: &IssuedQuestionPresentation,
) -> Result<StudentResponseInspection, RenderedResponseTranslationError> {
    let rendered_id = |id: &ResponseItemReference, role| rendered_id(id, role, presentation);
    match response {
        StudentResponse::Numeric { value } => {
            Ok(StudentResponseInspection::Numeric { value: *value })
        }
        StudentResponse::MultipleChoice { selected } => {
            Ok(StudentResponseInspection::MultipleChoice {
                selected: selected
                    .iter()
                    .map(|id| rendered_id(id, ResponseItemRole::QuestionChoice))
                    .collect::<Result<_, _>>()?,
            })
        }
        StudentResponse::ShortText { text } => {
            Ok(StudentResponseInspection::ShortText { text: text.clone() })
        }
        StudentResponse::MultiBlank { answers } => Ok(StudentResponseInspection::MultiBlank {
            answers: answers
                .iter()
                .map(|answer| {
                    Ok(InspectedTextEntry {
                        slot: rendered_id(&answer.slot, ResponseItemRole::TextEntrySlot)?,
                        text: answer.text.clone(),
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Matching { matches } => Ok(StudentResponseInspection::Matching {
            matches: matches
                .iter()
                .map(|pair| {
                    Ok(InspectedMatchPair {
                        prompt: rendered_id(&pair.prompt, ResponseItemRole::MatchingPrompt)?,
                        choice: rendered_id(&pair.choice, ResponseItemRole::MatchingChoice)?,
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Ordering { order } => Ok(StudentResponseInspection::Ordering {
            order: order
                .iter()
                .map(|id| rendered_id(id, ResponseItemRole::OrderingItem))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Hotspot { selections } => Ok(StudentResponseInspection::Hotspot {
            selected_regions: selections
                .iter()
                .map(|selection| rendered_id(&selection.region, ResponseItemRole::HotspotRegion))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::ExternalTool {} => Ok(StudentResponseInspection::ExternalTool {
            completion: InspectedExternalToolState::SubmissionRecorded,
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
pub fn project_rendered_response_for_inspection(
    response: &StudentResponse,
    presentation: &IssuedQuestionPresentation,
) -> Result<StudentResponseInspection, RenderedResponseTranslationError> {
    let rendered_id =
        |id: &ResponseItemReference, role| verified_rendered_id(id, role, presentation);
    match response {
        StudentResponse::Numeric { value } => {
            Ok(StudentResponseInspection::Numeric { value: *value })
        }
        StudentResponse::MultipleChoice { selected } => {
            Ok(StudentResponseInspection::MultipleChoice {
                selected: selected
                    .iter()
                    .map(|id| rendered_id(id, ResponseItemRole::QuestionChoice))
                    .collect::<Result<_, _>>()?,
            })
        }
        StudentResponse::ShortText { text } => {
            Ok(StudentResponseInspection::ShortText { text: text.clone() })
        }
        StudentResponse::MultiBlank { answers } => Ok(StudentResponseInspection::MultiBlank {
            answers: answers
                .iter()
                .map(|answer| {
                    Ok(InspectedTextEntry {
                        slot: rendered_id(&answer.slot, ResponseItemRole::TextEntrySlot)?,
                        text: answer.text.clone(),
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Matching { matches } => Ok(StudentResponseInspection::Matching {
            matches: matches
                .iter()
                .map(|pair| {
                    Ok(InspectedMatchPair {
                        prompt: rendered_id(&pair.prompt, ResponseItemRole::MatchingPrompt)?,
                        choice: rendered_id(&pair.choice, ResponseItemRole::MatchingChoice)?,
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Ordering { order } => Ok(StudentResponseInspection::Ordering {
            order: order
                .iter()
                .map(|id| rendered_id(id, ResponseItemRole::OrderingItem))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Hotspot { selections } => Ok(StudentResponseInspection::Hotspot {
            selected_regions: selections
                .iter()
                .map(|selection| rendered_id(&selection.region, ResponseItemRole::HotspotRegion))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::ExternalTool {} => Ok(StudentResponseInspection::ExternalTool {
            completion: InspectedExternalToolState::SubmissionRecorded,
        }),
    }
}

fn translated_response_item_reference(
    id: &ResponseItemReference,
    expected_role: ResponseItemRole,
    presentation: &IssuedQuestionPresentation,
) -> Result<ResponseItemReference, RenderedResponseTranslationError> {
    rendered_binding(id, expected_role, presentation)?
        .response_item_reference
        .clone()
        .ok_or(RenderedResponseTranslationError::UnknownRenderedId)
}

fn verified_rendered_id(
    id: &ResponseItemReference,
    expected_role: ResponseItemRole,
    presentation: &IssuedQuestionPresentation,
) -> Result<PresentationResponseItemReference, RenderedResponseTranslationError> {
    Ok(rendered_binding(id, expected_role, presentation)?
        .rendered
        .clone())
}

fn rendered_binding<'a>(
    id: &ResponseItemReference,
    expected_role: ResponseItemRole,
    presentation: &'a IssuedQuestionPresentation,
) -> Result<&'a super::ResponseItemBinding, RenderedResponseTranslationError> {
    let rendered = PresentationResponseItemReference::parse(id.as_str())
        .map_err(|_| RenderedResponseTranslationError::MalformedRenderedId)?;
    let mut bindings = presentation
        .item_bindings
        .iter()
        .filter(|binding| binding.rendered == rendered);
    let binding = bindings
        .next()
        .ok_or(RenderedResponseTranslationError::UnknownRenderedId)?;
    if bindings.next().is_some() {
        return Err(RenderedResponseTranslationError::DuplicateRenderedIdBinding);
    }
    if binding.role != expected_role {
        return Err(RenderedResponseTranslationError::WrongRenderedItemRole);
    }
    Ok(binding)
}

fn rendered_id(
    durable: &ResponseItemReference,
    expected_role: ResponseItemRole,
    presentation: &IssuedQuestionPresentation,
) -> Result<PresentationResponseItemReference, RenderedResponseTranslationError> {
    let mut bindings = presentation
        .item_bindings
        .iter()
        .filter(|binding| binding.response_item_reference.as_ref() == Some(durable));
    let binding = bindings
        .next()
        .ok_or(RenderedResponseTranslationError::UnknownRenderedId)?;
    if bindings.next().is_some() {
        return Err(RenderedResponseTranslationError::DuplicateRenderedIdBinding);
    }
    if binding.role != expected_role {
        return Err(RenderedResponseTranslationError::WrongRenderedItemRole);
    }
    Ok(binding.rendered.clone())
}
