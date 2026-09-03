//! Attempt-presentation contracts for the compact student boundary.
//!
//! Durable source identifiers and grading parameters remain in the server
//! model. This module projects only the public objects needed to render one
//! issued question and route a student response back to that exact
//! presentation.

mod assets;
mod binding;
mod builder;
mod codec;
mod model;
mod response_translation;

pub use binding::QuestionPresentationBinding;
pub use builder::{
    IssuedQuestionPresentation, OperatingSystemQuestionPresentationNonceSource,
    PresentationBuildError, QuestionPresentationNonceSource, ResponseItemBinding, ResponseItemRole,
    build_question_presentation, build_question_presentation_with_nonce_source,
    rebuild_public_question_presentation, reproduce_question_presentation,
};
pub use codec::{
    CURRENT_DESCRIPTOR_VERSION, QuestionPresentationChecksum, descriptor_bytes,
    verify_question_presentation,
};
pub use model::{
    PresentationResponseItemReference, PresentedHotspotRegion, PresentedHotspotSurface,
    PresentedMatchingChoice, PresentedMatchingPrompt, PresentedOrderingItem,
    PresentedQuestionChoice, PresentedResponseItemContent, PresentedTextEntrySlot,
    QuestionAssetRendition, QuestionPresentation, QuestionPresentationNonce,
    QuestionPresentationResponseFormat, QuestionPresentationToken, StudentAssignmentAttemptScreen,
    StudentAssignmentAttemptScreenAttempt, StudentAssignmentAttemptScreenScope,
    StudentAttemptDescriptor,
};
pub use response_translation::{
    InspectedImathasQuestionBackendState, InspectedMatchPair, InspectedTextEntry,
    PresentationResponseItemTranslationError, StudentResponseInspection,
    project_durable_response_to_presentation_response_item_references,
    project_presentation_response_item_references_for_inspection,
    translate_presentation_response_item_references,
};

#[cfg(test)]
mod tests;
