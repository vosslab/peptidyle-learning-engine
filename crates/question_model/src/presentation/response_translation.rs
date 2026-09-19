//! Server-side translation from Presentation Response Item IDs to durable IDs.
//!
//! The presentation binding is server-only.  This operation is deliberately
//! pure: callers must reproduce and authenticate an [`IssuedQuestionPresentation`] before
//! translating a browser response, and validation of the response's bounded
//! public shape remains the caller's responsibility.

use crate::response::{
    ResponseItemId, StudentHotspotSelection, StudentMatch, StudentResponse, StudentTextEntry,
};
use serde::{Deserialize, Serialize};

use super::{IssuedQuestionPresentation, PresentationResponseItemId, ResponseItemRole};

/// Fail-closed reasons a Presentation Response Item ID cannot be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationResponseItemTranslationError {
    /// The submitted Presentation Response Item ID has an invalid closed format.
    MalformedPresentationResponseItemId,
    /// The Presentation Response Item ID is not present in the issued presentation.
    UnknownPresentationResponseItemId,
    /// The issued presentation maps one Presentation Response Item ID more than once.
    DuplicatePresentationResponseItemIdBinding,
    /// The Presentation Response Item ID belongs to another Response Item Role in this presentation.
    WrongResponseItemRole,
}

/// A closed rendering of one immutable submitted Student Response.
///
/// This deliberately closed Student Response Inspection contains only the Student's
/// submitted values and the Presentation Response Item IDs from the issued presentation.
/// Backend-Owned submissions retain only their bounded opaque bytes for the
/// server-side response-restoration boundary; no consumer may decode or interpret them here.
/// Answer Keys, Question Grading Input, and durable Object Addresses have no representation here. The server creates it after verifying
/// the Issued Question Presentation. ASVS 14.1.1 and 14.2.1: sensitive
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
    /// Presentation Response Item IDs the Student selected.
    MultipleChoice {
        /// Issued Presentation Response Item IDs, never durable choice IDs.
        selected: Vec<PresentationResponseItemId>,
    },
    /// A short text value the Student submitted.
    ShortText {
        /// Submitted text.
        text: String,
    },
    /// Text entries bound to their issued Presentation Response Item IDs.
    MultiBlank {
        /// Submitted blank entries.
        answers: Vec<InspectedTextEntry>,
    },
    /// Associations bound to issued Presentation Response Item IDs.
    Matching {
        /// Submitted associations.
        matches: Vec<InspectedMatchPair>,
    },
    /// Issued Presentation Response Item IDs in Student-selected order.
    Ordering {
        /// Submitted ordering.
        order: Vec<PresentationResponseItemId>,
    },
    /// Submitted Hotspot Region selections.
    Hotspot {
        /// Issued Presentation Response Item IDs for Hotspot Regions selected by the Student.
        selected_regions: Vec<PresentationResponseItemId>,
    },
    /// Coarse iMathAS Question Backend completion state.
    ImathasQuestionBackend {
        /// Safe completion state, without iMathAS Question Backend data or launch authority.
        completion: InspectedImathasQuestionBackendState,
    },
    /// A Backend-Owned submission retained for server-side opaque restoration.
    BackendOwned {
        /// Bounded opaque bytes; serialization uses canonical base64.
        #[serde(with = "crate::response::backend_owned_payload")]
        payload: Vec<u8>,
    },
}

/// One text entry bound to the Presentation Response Item ID visible in the issue.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspectedTextEntry {
    /// Issued Presentation Response Item ID binding this entry.
    pub slot: PresentationResponseItemId,
    /// Text submitted for the referenced blank.
    pub text: String,
}

/// One association bound to Presentation Response Item IDs.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspectedMatchPair {
    /// Issued Presentation Response Item ID for the prompt.
    pub prompt: PresentationResponseItemId,
    /// Issued Presentation Response Item ID for the choice.
    pub choice: PresentationResponseItemId,
}

/// Safe iMathAS Question Backend fact. Backend data and launch authority stay private.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InspectedImathasQuestionBackendState {
    /// The iMathAS Question Backend submission was recorded; backend details remain private.
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
            Self::ImathasQuestionBackend { .. } => "imathas_question_backend",
            Self::BackendOwned { .. } => "backend_owned",
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

impl std::fmt::Display for PresentationResponseItemTranslationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedPresentationResponseItemId => {
                formatter.write_str("Presentation Response Item ID is malformed")
            }
            Self::UnknownPresentationResponseItemId => {
                formatter.write_str("Presentation Response Item ID is unknown")
            }
            Self::DuplicatePresentationResponseItemIdBinding => {
                formatter.write_str("Presentation Response Item ID has duplicate issued bindings")
            }
            Self::WrongResponseItemRole => formatter
                .write_str("Presentation Response Item ID has the wrong issued Response Item Role"),
        }
    }
}

impl std::error::Error for PresentationResponseItemTranslationError {}

/// Converts Presentation Response Item IDs into the durable IDs bound to one issue.
///
/// Only identifier-bearing Question Response Formats are rewritten. Scalar
/// response formats preserve their values exactly. The function intentionally exposes
/// no durable mapping or serializable wire type.
pub fn translate_presentation_response_item_ids(
    response: &StudentResponse,
    presentation: &IssuedQuestionPresentation,
) -> Result<StudentResponse, PresentationResponseItemTranslationError> {
    let response_item_id =
        |id: &ResponseItemId, role| translated_response_item_id(id, role, presentation);

    match response {
        StudentResponse::MultipleChoice { selected } => Ok(StudentResponse::MultipleChoice {
            selected: selected
                .iter()
                .map(|id| response_item_id(id, ResponseItemRole::QuestionChoice))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::MultiBlank { answers } => Ok(StudentResponse::MultiBlank {
            answers: answers
                .iter()
                .map(|answer| {
                    Ok(StudentTextEntry {
                        slot: response_item_id(&answer.slot, ResponseItemRole::TextEntrySlot)?,
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
                        prompt: response_item_id(&pair.prompt, ResponseItemRole::MatchingPrompt)?,
                        choice: response_item_id(&pair.choice, ResponseItemRole::MatchingChoice)?,
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Ordering { order } => Ok(StudentResponse::Ordering {
            order: order
                .iter()
                .map(|id| response_item_id(id, ResponseItemRole::OrderingItem))
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
                        region: response_item_id(
                            &selection.region,
                            ResponseItemRole::HotspotRegion,
                        )?,
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::ImathasQuestionBackend {} => {
            Ok(StudentResponse::ImathasQuestionBackend {})
        }
        StudentResponse::BackendOwned { payload } => Ok(StudentResponse::BackendOwned {
            payload: payload.clone(),
        }),
    }
}

/// Projects a durable submitted response into the exact Presentation Response Item IDs
/// of a verified issued presentation.
///
/// The inverse mapping is intentionally available only at the trusted
/// inspection boundary. It is pure and does not reveal Answer Keys, Question
/// Feedback, Question Answer Explanations, or Question Grading Input.
pub fn project_durable_response_to_presentation_response_item_ids(
    response: &StudentResponse,
    presentation: &IssuedQuestionPresentation,
) -> Result<StudentResponseInspection, PresentationResponseItemTranslationError> {
    let presentation_response_item_id =
        |id: &ResponseItemId, role| presentation_response_item_id(id, role, presentation);
    match response {
        StudentResponse::Numeric { value } => {
            Ok(StudentResponseInspection::Numeric { value: *value })
        }
        StudentResponse::MultipleChoice { selected } => {
            Ok(StudentResponseInspection::MultipleChoice {
                selected: selected
                    .iter()
                    .map(|id| presentation_response_item_id(id, ResponseItemRole::QuestionChoice))
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
                        slot: presentation_response_item_id(
                            &answer.slot,
                            ResponseItemRole::TextEntrySlot,
                        )?,
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
                        prompt: presentation_response_item_id(
                            &pair.prompt,
                            ResponseItemRole::MatchingPrompt,
                        )?,
                        choice: presentation_response_item_id(
                            &pair.choice,
                            ResponseItemRole::MatchingChoice,
                        )?,
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Ordering { order } => Ok(StudentResponseInspection::Ordering {
            order: order
                .iter()
                .map(|id| presentation_response_item_id(id, ResponseItemRole::OrderingItem))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Hotspot { selections } => Ok(StudentResponseInspection::Hotspot {
            selected_regions: selections
                .iter()
                .map(|selection| {
                    presentation_response_item_id(
                        &selection.region,
                        ResponseItemRole::HotspotRegion,
                    )
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::ImathasQuestionBackend {} => {
            Ok(StudentResponseInspection::ImathasQuestionBackend {
                completion: InspectedImathasQuestionBackendState::SubmissionRecorded,
            })
        }
        StudentResponse::BackendOwned { payload } => Ok(StudentResponseInspection::BackendOwned {
            payload: payload.clone(),
        }),
    }
}

/// Validates an immutable browser-submitted response against its exact issued
/// presentation and returns the safe Student Response Inspection for that issue.
///
/// Accepted-submission storage preserves the browser contract verbatim. The
/// inspection boundary validates each identifier against the reconstructed
/// public issue and retains that exact Presentation Response Item ID. Reconstructed
/// browser-safe presentations intentionally contain no durable identifiers.
pub fn project_presentation_response_item_ids_for_inspection(
    response: &StudentResponse,
    presentation: &IssuedQuestionPresentation,
) -> Result<StudentResponseInspection, PresentationResponseItemTranslationError> {
    let presentation_response_item_id =
        |id: &ResponseItemId, role| verified_presentation_response_item_id(id, role, presentation);
    match response {
        StudentResponse::Numeric { value } => {
            Ok(StudentResponseInspection::Numeric { value: *value })
        }
        StudentResponse::MultipleChoice { selected } => {
            Ok(StudentResponseInspection::MultipleChoice {
                selected: selected
                    .iter()
                    .map(|id| presentation_response_item_id(id, ResponseItemRole::QuestionChoice))
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
                        slot: presentation_response_item_id(
                            &answer.slot,
                            ResponseItemRole::TextEntrySlot,
                        )?,
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
                        prompt: presentation_response_item_id(
                            &pair.prompt,
                            ResponseItemRole::MatchingPrompt,
                        )?,
                        choice: presentation_response_item_id(
                            &pair.choice,
                            ResponseItemRole::MatchingChoice,
                        )?,
                    })
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Ordering { order } => Ok(StudentResponseInspection::Ordering {
            order: order
                .iter()
                .map(|id| presentation_response_item_id(id, ResponseItemRole::OrderingItem))
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::Hotspot { selections } => Ok(StudentResponseInspection::Hotspot {
            selected_regions: selections
                .iter()
                .map(|selection| {
                    presentation_response_item_id(
                        &selection.region,
                        ResponseItemRole::HotspotRegion,
                    )
                })
                .collect::<Result<_, _>>()?,
        }),
        StudentResponse::ImathasQuestionBackend {} => {
            Ok(StudentResponseInspection::ImathasQuestionBackend {
                completion: InspectedImathasQuestionBackendState::SubmissionRecorded,
            })
        }
        StudentResponse::BackendOwned { payload } => Ok(StudentResponseInspection::BackendOwned {
            payload: payload.clone(),
        }),
    }
}

fn translated_response_item_id(
    id: &ResponseItemId,
    expected_role: ResponseItemRole,
    presentation: &IssuedQuestionPresentation,
) -> Result<ResponseItemId, PresentationResponseItemTranslationError> {
    presentation_response_item_binding(id, expected_role, presentation)?
        .response_item_id
        .clone()
        .ok_or(PresentationResponseItemTranslationError::UnknownPresentationResponseItemId)
}

fn verified_presentation_response_item_id(
    id: &ResponseItemId,
    expected_role: ResponseItemRole,
    presentation: &IssuedQuestionPresentation,
) -> Result<PresentationResponseItemId, PresentationResponseItemTranslationError> {
    Ok(
        presentation_response_item_binding(id, expected_role, presentation)?
            .presentation_response_item_id
            .clone(),
    )
}

fn presentation_response_item_binding<'a>(
    id: &ResponseItemId,
    expected_role: ResponseItemRole,
    presentation: &'a IssuedQuestionPresentation,
) -> Result<&'a super::ResponseItemBinding, PresentationResponseItemTranslationError> {
    let presentation_response_item_id =
        PresentationResponseItemId::parse(id.as_str()).map_err(|_| {
            PresentationResponseItemTranslationError::MalformedPresentationResponseItemId
        })?;
    let mut bindings = presentation
        .item_bindings
        .iter()
        .filter(|binding| binding.presentation_response_item_id == presentation_response_item_id);
    let binding = bindings
        .next()
        .ok_or(PresentationResponseItemTranslationError::UnknownPresentationResponseItemId)?;
    if bindings.next().is_some() {
        return Err(
            PresentationResponseItemTranslationError::DuplicatePresentationResponseItemIdBinding,
        );
    }
    if binding.role != expected_role {
        return Err(PresentationResponseItemTranslationError::WrongResponseItemRole);
    }
    Ok(binding)
}

fn presentation_response_item_id(
    durable: &ResponseItemId,
    expected_role: ResponseItemRole,
    presentation: &IssuedQuestionPresentation,
) -> Result<PresentationResponseItemId, PresentationResponseItemTranslationError> {
    let mut bindings = presentation
        .item_bindings
        .iter()
        .filter(|binding| binding.response_item_id.as_ref() == Some(durable));
    let binding = bindings
        .next()
        .ok_or(PresentationResponseItemTranslationError::UnknownPresentationResponseItemId)?;
    if bindings.next().is_some() {
        return Err(
            PresentationResponseItemTranslationError::DuplicatePresentationResponseItemIdBinding,
        );
    }
    if binding.role != expected_role {
        return Err(PresentationResponseItemTranslationError::WrongResponseItemRole);
    }
    Ok(binding.presentation_response_item_id.clone())
}
