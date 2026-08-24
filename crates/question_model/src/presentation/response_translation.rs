//! Server-side translation from rendered response identifiers to durable IDs.
//!
//! The presentation binding is server-only.  This operation is deliberately
//! pure: callers must reproduce and authenticate a [`PresentationV1`] before
//! translating a browser response, and validation of the response's bounded
//! public shape remains the caller's responsibility.

use crate::response::{ChoiceId, MatchPair, StudentResponse, TextEntryAnswer};

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
/// Only identifier-bearing response families are rewritten. Scalar response
/// families preserve their values exactly. The function intentionally exposes
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

fn durable_id_v1(
    id: &ChoiceId,
    expected_role: RenderedItemRoleV1,
    presentation: &PresentationV1,
) -> Result<ChoiceId, RenderedResponseTranslationErrorV1> {
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
    Ok(ChoiceId::new(binding.durable_id.clone()))
}
