//! Attempt-presentation contracts for the compact student boundary.
//!
//! Durable source identifiers and grading parameters remain in the server
//! model. This module projects only the public objects needed to render one
//! issued question and route a student response back to that exact
//! presentation.

mod binding;
mod builder;
mod codec;
mod model;
mod response_translation;

pub use binding::PresentationBindingV1;
pub use builder::{
    NonceSourceV1, OsNonceSourceV1, PresentationBuildError, PresentationV1, RenderedItemBindingV1,
    RenderedItemRoleV1, build_presentation_v1, build_presentation_v1_with_nonce_source,
    rebuild_public_presentation_v1, reproduce_presentation_v1,
};
pub use codec::{
    DESCRIPTOR_VERSION_V1, PresentationDigestV1, descriptor_bytes_v1, verify_presentation_v1,
};
pub use model::{
    AssetBindingV1, PresentationDigestTokenV1, PresentationEnvelopeV1, PresentationNonceV1,
    PresentedBlankV1, PresentedChoiceV1, PresentedHotspotRegionV1, PresentedHotspotSurfaceV1,
    RenderedItemIdV1, ResponseSchemaV1, StudentAssignmentAttemptScreenAttemptV1,
    StudentAssignmentAttemptScreenScopeV1, StudentAssignmentAttemptScreenV1, StudentAttemptDescriptorV1,
};
pub use response_translation::{
    InspectedExternalToolStateV1, InspectedMatchPairV1, InspectedStudentArtifactStateV1,
    InspectedStudentResponseV1, InspectedTextEntryV1, RenderedResponseTranslationErrorV1,
    project_durable_response_to_rendered_v1, project_rendered_response_for_inspection_v1,
    translate_rendered_response_v1,
};

#[cfg(test)]
mod tests;
