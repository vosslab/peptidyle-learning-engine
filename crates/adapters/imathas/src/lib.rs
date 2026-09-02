//! MOD-ADP-IMATHAS: isolated iMathAS/MyOpenMath adapter.
//!
//! An iMathAS Deployment Reference is deployment configuration selected by an
//! opaque key. This
//! crate never accepts author-supplied endpoints or browser Answer Keys,
//! Question Feedback, Question Answer Explanations, or Question Grading Input.

#[path = "lib/adapter.rs"]
mod adapter;
#[path = "lib/cache.rs"]
mod cache;
#[path = "lib/grade.rs"]
mod grade;
#[cfg(feature = "http-transport")]
pub mod http_transport;
pub mod imathas_question_backend;
#[path = "lib/question_backend.rs"]
mod question_backend;
pub mod result_verification;
#[cfg(feature = "test-support")]
pub mod test_support;

/// Stable adapter identity persisted in Question Attempt Reproduction Details, independent of CalVer.
pub const ADAPTER_ID: &str = "imathas-adapter";
/// Current compatible adapter implementation.
pub const ADAPTER_VERSION: &str = "1";
/// Stable identity for server-verified iMathAS grading.
pub const GRADING_ID: &str = "imathas-verified-grader";
/// Current compatible server verifier implementation.
pub const GRADING_VERSION: &str = "1";

pub use adapter::{ImathasAdapter, ImathasIssuedAttempt, ResolvedImathasQuestionSource};
pub use grade::{ImathasAdapterError, ImathasQuestionBackendFailure, VerifiedImathasResult};
pub use imathas_question_backend::ImathasSessionAuthenticationCodec;
pub use question_backend::{
    ImathasQuestionLocation, ImathasRenderRequest, ImathasResultRequest, PreparedSnapshot,
    QuestionBackend, SafeImathasQuestionRender, SupportedImathasProfile,
};
pub use question_model::{
    DraftImathasQuestionBackendBinding, ImathasDeploymentReference, ImathasItemReference,
    ImathasProfile, ImathasQuestionBackendBinding,
};

pub(crate) use cache::{constant_time_eq, hex, verify_binding};
pub(crate) use question_backend::sealed;

#[cfg(test)]
use cache::render_key;

#[cfg(test)]
#[path = "lib/tests.rs"]
mod tests;

#[cfg(test)]
use async_trait::async_trait;
#[cfg(test)]
use objects::{ObjectAddress, ObjectStore, PutObject};
#[cfg(test)]
use question_model::envelope::QuestionContentBlock;
#[cfg(test)]
use question_model::generation::QuestionSeed;
#[cfg(test)]
use question_model::{
    ObjectId, QuestionAttemptId, QuestionBackendLocator, QuestionId, QuestionRevisionNumber,
    QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference, Timestamp,
};
#[cfg(test)]
use uuid::Uuid;
