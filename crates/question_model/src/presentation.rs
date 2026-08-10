//! Attempt-presentation contracts for the compact learner boundary.
//!
//! Durable source identifiers and grading parameters remain in the server
//! model. This module projects only the public objects needed to render one
//! issued question and route a learner response back to that exact
//! presentation.

mod binding;
mod builder;
mod codec;
mod model;

pub use binding::PresentationBindingV1;
pub use builder::{
    NonceSourceV1, OsNonceSourceV1, PresentationBuildError, PresentationV1, RenderedItemBindingV1,
    RenderedItemRoleV1, build_presentation_v1, build_presentation_v1_with_nonce_source,
    rebuild_public_presentation_v1,
};
pub use codec::{
    DESCRIPTOR_VERSION_V1, PresentationDigestV1, descriptor_bytes_v1, verify_presentation_v1,
};
pub use model::{
    AssetBindingV1, LearnerAttemptDescriptorV1, LearnerRunScreenRunV1, LearnerRunScreenScopeV1,
    LearnerRunScreenV1, PresentationDigestTokenV1, PresentationEnvelopeV1, PresentationNonceV1,
    PresentedBlankV1, PresentedChoiceV1, PresentedHotspotRegionV1, PresentedHotspotSurfaceV1,
    RenderedItemIdV1, ResponseSchemaV1,
};

#[cfg(test)]
mod tests;
