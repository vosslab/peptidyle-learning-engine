//! MOD-QM: the root contract of the Peptidyle Learning Engine.
//!
//! Question types, activity records, backend capabilities, identity, and
//! taxonomy live here.
//! Every backend adapter maps its engine's questions into these types, and
//! everything downstream reads only these types, which is what lets one
//! attempt loop, gradebook, and export path serve every engine.
//!
//! Answer keys and correctness decisions live in `crates/grading`, which runs
//! server-side and sits outside the WebAssembly dependency closure. A type
//! belongs here when a browser may safely see it; a type that would reveal a
//! correct response belongs in `grading`.
//!
//! Contracts here are frozen in M1 and changed only alongside every consumer,
//! as recorded in `docs/CONTRACTS.md`.

pub mod activity;
pub mod answer;
pub mod capability;
pub mod definition;
pub mod envelope;
pub mod generation;
pub mod identity;
pub mod lifecycle;
pub mod response;
pub mod run_policy;
pub mod taxonomy;

// The crate's front door. These are the types a caller reaches for first, so
// they are re-exported to keep call sites short. Everything else stays
// available under its module.
pub use crate::activity::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentId, AssignmentRun, AttemptProvenance,
    AttemptResult, AttemptTimerRecord, EnrollmentId, EnrollmentStatus, ImplementationVersion,
    QuestionAttempt, QuestionAttemptId, RunId, RunMode, SourceArtifact, StudentAssignmentSummary,
    StudentId, TenantId,
};
pub use crate::capability::{BackendCapabilities, Capability};
pub use crate::definition::{
    GradingDefinition, QuestionDefinition, QuestionMetadata, QuestionSource,
};
pub use crate::envelope::QuestionEnvelope;
pub use crate::generation::GeneratorReference;
pub use crate::identity::{AssetId, ObjectId, ProblemId, VersionId, WorkspaceId};
pub use crate::lifecycle::{Lifecycle, LifecycleError, LifecycleEvent};
pub use crate::response::{ResponseDefinition, StudentResponse};
pub use crate::run_policy::{
    CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies, VariationPolicy,
};
