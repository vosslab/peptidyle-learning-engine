//! MOD-ADP-IMATHAS: isolated iMathAS/MyOpenMath adapter.
//!
//! A provider is deployment configuration selected by an opaque key. This
//! crate never accepts author-supplied endpoints or browser grading material.

#[path = "lib/adapter.rs"]
mod adapter;
pub mod broker_provider;
#[path = "lib/cache.rs"]
mod cache;
#[path = "lib/grade.rs"]
mod grade;
#[cfg(feature = "http-transport")]
pub mod http_transport;
#[path = "lib/provider.rs"]
mod provider;
pub mod scored_embed;
#[cfg(feature = "test-support")]
pub mod test_support;

/// Stable adapter identity persisted in provenance, independent of CalVer.
pub const ADAPTER_ID: &str = "imathas-adapter";
/// Current compatible adapter implementation.
pub const ADAPTER_VERSION: &str = "1";
/// Stable identity for server-verified provider grading.
pub const GRADING_ID: &str = "imathas-verified-grader";
/// Current compatible server verifier implementation.
pub const GRADING_VERSION: &str = "1";

pub use adapter::{ImathasAdapter, ImathasIssuedAttempt, ImathasSource, VerifiedGradeReceipt};
pub use grade::{
    CorrelationIssuer, GradeBinding, ImathasAdapterError, PersistedCorrelation, ProviderFailure,
    ServerCorrelation, VerifiedProviderGrade,
};
pub use provider::{
    DraftLocator, ImathasProvider, PreparedSnapshot, ProviderGradeRequest, ProviderRenderRequest,
    SafeProviderRender, SupportedProfile,
};

pub(crate) use cache::{constant_time_eq, hex, verify_binding};
pub(crate) use provider::sealed;

#[cfg(test)]
use cache::render_key;

#[cfg(test)]
#[path = "lib/tests.rs"]
mod tests;

#[cfg(test)]
use async_trait::async_trait;
#[cfg(test)]
use learning_data_access::PublishedSourceArtifact;
#[cfg(test)]
use objects::{ObjectKey, ObjectStore, PutObject};
#[cfg(test)]
use question_model::envelope::ContentBlock;
#[cfg(test)]
use question_model::generation::Seed;
#[cfg(test)]
use question_model::{
    ActivityTimestamp, AttemptResult, ObjectId, ProblemId, QuestionAttemptId, QuestionDefinition,
    QuestionSource, TenantId, VersionId,
};
#[cfg(test)]
use sha2::{Digest, Sha256};
#[cfg(test)]
use uuid::Uuid;
