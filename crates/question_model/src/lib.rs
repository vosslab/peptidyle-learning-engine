//! Browser-safe domain contracts for the Peptidyle Learning Engine.
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
//! Contracts here change only alongside every consumer, as recorded in
//! `docs/CONTRACTS.md`.

pub mod activity;
pub mod answer;
/// Stable assignment items, selection groups, and exact point values.
pub mod assignment;
pub mod auth;
/// Immutable public publication attribution, separate from private author authority.
pub mod byline;
pub mod capability;
/// Shared catalog metadata, visibility, lineage, and browse projections.
pub mod catalog;
/// Tenant-owned course and assignment browser projections.
pub mod course;
/// Closed, browser-safe course appearance and banner presentation contracts.
pub mod course_appearance;
/// Validated inclusive course-calendar bounds and authoritative IANA zone.
pub mod course_term;
pub mod definition;
/// Internal entitlement and materialization contracts. These types never
/// cross the browser boundary.
pub mod entitlement;
pub mod envelope;
/// Private teaching material and policy-redacted browser feedback.
pub mod feedback;
pub mod generation;
pub mod identity;
pub mod lifecycle;
/// Browser-safe, attempt-presentation-scoped question contracts.
pub mod presentation;
/// Human-facing route locators that resolve to internal identities under authorization.
pub mod public_route;
pub mod response;
pub mod run_policy;
/// Browser-safe anonymous-statistics projections and disclosure policy.
pub mod statistics;
pub mod taxonomy;

// The crate's front door. These are the types a caller reaches for first, so
// they are re-exported to keep call sites short. Everything else stays
// available under its module.
pub use crate::activity::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentId, AssignmentItemId,
    AssignmentPolicyExceptionId, AssignmentRun, AssignmentRunItem, AssignmentSelectionGroupId,
    AttemptProvenance, AttemptResult, AttemptStatus, AttemptTimerRecord, CourseGroupId, CourseId,
    CourseMembershipId, EnrollmentId, EnrollmentStatus, ImplementationVersion,
    IssuedAttemptCapabilityV1, LearnerAssignmentProgress, LearnerScoreState, QuestionAttempt,
    QuestionAttemptId, RunId, RunMode, SourceArtifact, StudentAssignmentSummary, StudentId,
    TenantId,
};
pub use crate::assignment::{
    AssignmentDeadlineBehavior, AssignmentDeliveryState, AssignmentItem, AssignmentRunTiming,
    AssignmentScoringMode, AssignmentSelectionCandidate, AssignmentSelectionGroup,
    DEFAULT_MASTERY_TIME_LIMIT_SECONDS, LateSubmissionPolicy, MAX_ASSIGNMENT_TIME_LIMIT_SECONDS,
    PointValue, ScoringGeneration, ScoringStatus, SelectionOrdering,
};
pub use crate::auth::{UserId, UserRole};
pub use crate::byline::{PublicAuthorName, PublicByline, PublicBylineError};
pub use crate::capability::{BackendCapabilities, Capability};
pub use crate::catalog::{
    CatalogCapabilityFacet, CatalogLicenseFacet, CatalogLicenseValue, CatalogLifecycle,
    CatalogProblemDetail, CatalogProblemSummary, CatalogSearchFacets, CatalogSearchPage,
    CatalogSearchQuery, CatalogSearchQueryError, CatalogStatisticsAvailability,
    CatalogStatisticsFacet, CatalogStatisticsStatus, CatalogTaxonomyFacet, CatalogTaxonomyFilter,
    MAX_CATALOG_TAXONOMY_FACETS, MAX_QUESTION_ID_COUNT, ProblemDisplayRef, ProblemVersionRef,
    PublicationScope, QUESTION_ID_ALPHABET, QUESTION_ID_COMPACT_LENGTH,
    QUESTION_ID_IDENTIFIER_LENGTH, QuestionBackend, QuestionId,
};
pub use crate::course::{
    AssignmentItemSummary, AssignmentSelectionCandidateSummary, AssignmentSelectionGroupSummary,
    AssignmentSummary, CourseMembershipRole, CourseSummary, GradebookSummaryRow,
    LearnerAssignmentSummary,
};
pub use crate::course_appearance::{
    CourseAppearance, CourseAppearanceRevision, CourseAppearanceUpdate, CourseBannerAltText,
    CourseBannerAlternativeText, CourseBannerCandidateId, CourseBannerCandidateReceipt,
    CourseBannerId, CourseBannerMutation, CourseBannerPresentation, CourseThemeId,
};
pub use crate::course_term::{
    CourseDate, CourseDateError, CourseTerm, CourseTermError, CourseTermFailureCode,
    CourseTermFailureReason, CourseTermField, CourseTermValidationFailure, IanaTimeZone,
    IanaTimeZoneError,
};
pub use crate::definition::{
    DraftQuestionDefinition, DraftQuestionSource, DraftSourcePublicationError, GradingDefinition,
    MAX_QUESTION_TITLE_UNICODE_SCALARS, QuestionDefinition, QuestionMetadata, QuestionSource,
    QuestionSourceValidationError, QuestionTitleError, WorkspaceDraftSummary,
    validate_question_title,
};
pub use crate::entitlement::{
    AssignmentAudience, AssignmentAudienceError, CourseGroupPurpose, EntitlementMaterialization,
    EntitlementPurpose, EvaluatorVersion, GroupPurposeCapabilities, MaterializationAuthority,
    MaterializationBasis, MaterializationDisposition, MaterializationRule, NonEmptyAudienceGroups,
};
pub use crate::envelope::QuestionEnvelope;
pub use crate::feedback::{DisclosedFeedback, FeedbackContent};
pub use crate::generation::GeneratorReference;
pub use crate::identity::{
    AssetId, ObjectId, ProblemId, VersionId, WorkspaceId, WorkspaceImportId,
};
pub use crate::lifecycle::{Lifecycle, LifecycleError, LifecycleEvent};
pub use crate::presentation::{
    AssetBindingV1, LearnerAttemptDescriptorV1, LearnerRunScreenRunV1, LearnerRunScreenScopeV1,
    LearnerRunScreenV1, PresentationBindingV1, PresentationDigestTokenV1, PresentationDigestV1,
    PresentationEnvelopeV1, PresentationNonceV1, PresentedBlankV1, PresentedChoiceV1,
    PresentedHotspotRegionV1, PresentedHotspotSurfaceV1, RenderedItemIdV1, ResponseSchemaV1,
};
pub use crate::public_route::{
    AssignmentReference, CourseGroupReference, CourseReference, MAX_PUBLIC_ROUTE_NUMBER,
    NavigationResolution, RESERVED_REFERENCE_PREFIXES, RunReference, WorkspaceReference,
};
pub use crate::response::{ResponseDefinition, StudentResponse};
pub use crate::run_policy::{
    CompletionRequirement, ContinuedPractice, GradePolicy, LearnerDisclosurePolicy,
    LearnerDisclosureTiming, RunPolicies, VariationPolicy,
};
pub use crate::statistics::{
    DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE, LearnerClassStatistics, QuestionStatisticsDisclosure,
    QuestionStatisticsView, StatisticsDisclosurePolicy, StatisticsDisclosurePolicyError,
};
