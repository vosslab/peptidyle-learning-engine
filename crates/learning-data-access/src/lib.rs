//! Backend-neutral persistence contract (WP-C4, MOD-STO).
//!
//! Globally public content needs no tenant context. Institution-visible
//! catalog content goes through [`CatalogStore`], whose reads require
//! [`TenantContext`]. Every educational-record operation also requires that
//! non-defaultable context. Lists require a bounded [`PageRequest`]; the trait
//! has no unbounded or positional paging method. No SQL type appears in this
//! contract.

use async_trait::async_trait;
use base64::Engine;
use domain::run::RunModelError;
use objects::{ObjectRecord, Sha256Digest};
use question_model::FeedbackContent;
use question_model::run_policy::FeedbackDisclosure;
use question_model::taxonomy::TaxonomyTerm;
use question_model::{
    ActivityTimestamp, AssignmentDeliveryState, AssignmentEnrollment, AssignmentId, AssignmentItem,
    AssignmentItemId, AssignmentPolicyExceptionId, AssignmentRun, AssignmentRunItem,
    AssignmentSelectionGroup, AssignmentSummary, AssignmentTimingPolicy, AttemptProvenance,
    AttemptResult, AttemptStatus, BackendCapabilities, CatalogLifecycle, CatalogProblemDetail,
    CatalogProblemSummary, CatalogSearchPage, CatalogSearchQuery, CourseGroupId, CourseId,
    CourseMembership, CourseRole, CourseSummary, DraftQuestionDefinition, EnrollmentId,
    GradePolicy, GradebookSummaryRow, PresentationBindingV1, PresentationEnvelopeV1, ProblemId,
    PublicationScope, QuestionAttempt, QuestionAttemptId, QuestionBackend, QuestionDefinition,
    QuestionStatisticsDisclosure, RunId, RunPolicies, ScoringGeneration, ScoringStatus,
    SelectionOrdering, StudentAssignmentSummary, StudentId, StudentResponse, TenantId, UserId,
    VersionId, WorkspaceDraftSummary, WorkspaceId,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod account_identity;
mod activity_policy;
mod asset_delivery;
mod course_appearance;
mod course_roster;
mod external_tool;
mod feedback;
mod flat_import_provenance;
mod flat_question;
mod flat_question_assets;
mod gradebook_cursor;
pub mod in_memory;
mod item_analysis;
/// In-memory backend used by tests and lanes waiting for PostgreSQL.
pub mod jobs;
mod manual_grade_export;
mod manual_grading;
/// Cursor and bounded-page types shared by every list method.
pub mod pagination;
mod policy;
/// PostgreSQL health and future backend implementation.
pub mod postgres;
mod publication_validation;
mod qti;
mod qti_ingress;
/// Pure retention lifecycle policy; persistence and worker execution land in MOD-RETENTION R2+.
pub mod retention;
/// Explicit tenant context used by every educational-record operation.
pub mod rls;
mod run_summary_cursor;
mod score_precision;
/// Provider-neutral, replica-safe authentication session contract.
pub mod session;
mod statistics;

pub use crate::account_identity::{
    AccountCourseContext, AccountIdentityError, AccountIdentityStore, AccountRecord,
    AccountSessionLifetime, AccountSessionRecord, AccountSessionStore, AccountSessionTokenHash,
    AuthenticationEmail, AuthenticationRateLimitDecision, AuthenticationRateLimitKey,
    AuthenticationRateLimitPolicy, AuthenticationRateLimitScope, BeginEmailAuthentication,
    BeginWebauthnCeremony, BrowserBindingHash, CompleteEmailAuthentication,
    CompleteEmailAuthenticationAndCreateSession, CompletePasskeyAuthenticationAndCreateSession,
    CompletedAccountSession, CompletedEmailAuthentication, CompletedPasskeySession,
    ConsumeAuthenticationRateLimit, CredentialIdHash, EmailAuthenticationChallenge,
    EmailAuthenticationPurpose, EmailChallengeId, EmailChallengeLifetime, EmailChallengeSecretHash,
    EmailDomain, PasskeyId, PasskeyRecord, RegisterPasskey, WebauthnCeremony, WebauthnCeremonyId,
    WebauthnCeremonyKind, WebauthnCeremonyLifetime, WebauthnState, validated_account_display_name,
    validated_passkey_label,
};
pub use crate::asset_delivery::{
    AssetAccessEvent, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, AssetStore,
    AuthorizedAssetDelivery, CatalogAssetBinding,
};
pub use crate::course_appearance::{
    COURSE_BANNER_HEIGHT, COURSE_BANNER_WIDTH, CourseAppearanceStore, CourseBannerCleanupBatch,
    CourseBannerCleanupClaim, CourseBannerCleanupToken, CourseBannerPromotion,
    RegisterCourseBannerCandidate, SaveCourseAppearance,
};
pub use crate::course_roster::{
    AllowedEmailDomain, ClaimCourseInvitation, ClaimedCourseMembership, CommitCourseRosterImport,
    CommittedCourseRosterImport, CourseEnrollmentPolicy, CourseInvitation, CourseInvitationId,
    CourseInvitationLifetime, CourseInvitationSecretHash, CourseInvitationStatus, CourseMemberId,
    CourseMemberStatus, CourseRosterContact, CourseRosterEntry, CourseRosterError, CourseRosterId,
    CourseRosterImportId, CourseRosterImportLifetime, CourseRosterImportPreview,
    CourseRosterImportRow, CourseRosterImportRowInput, CourseRosterImportState, CourseRosterMember,
    CourseRosterPage, CourseRosterStore, CourseSignupPosture, CreateCourseInvitation,
    MAX_ROSTER_IMPORT_ROWS, ReplaceCourseEnrollmentPolicy, RevokeCourseInvitation,
    RevokeCourseMember, RosterIdempotencyKey, RosterImportInvitation, RosterImportRevision,
    RosterImportRowStatus, RosterRevision, StageCourseRosterImport, UpsertCourseMember,
};
pub(crate) use crate::external_tool::fresh_external_tool_launch_id;
pub use crate::external_tool::{
    BeginExternalToolGradeCommand, CommitExternalToolSubmissionCommand,
    CommitVerifiedExternalToolSubmissionCommand, CreateExternalToolLaunchSessionCommand,
    CreatedExternalToolLaunchSession, ExternalToolBegin, ExternalToolBinding,
    ExternalToolBrokerStore, ExternalToolLaunchProof, ExternalToolLaunchSessionStore,
    ExternalToolLaunchToken, ExternalToolLease, ExternalToolLeaseToken,
    ExternalToolVerifiedPending, PersistedCorrelation, ResolvedExternalToolLaunchSession,
    StageExternalToolVerificationCommand,
};
pub use crate::feedback::{
    AttemptFeedbackRecord, FeedbackReleaseRecord, ReleaseAttemptFeedbackCommand,
    RunSummaryOutcomeInput, RunSummaryPageInput, private_feedback_record,
};
pub use crate::flat_import_provenance::{
    FlatImportChoiceMapPayload, FlatImportConversionVersion, FlatImportIntegrityDigests,
    FlatImportProvenanceStore, FlatImportPublicationPromotion, PersistedFlatImportProfile,
    PublishedFlatImportOrigin, QTI_PROFILE_ARCHIVE_MEDIA_TYPE, QtiProfileFlatConversionCommand,
    QtiProfileImportEvidence, WorkspaceFlatImportOrigin, WorkspaceFlatImportOriginIdentity,
};
pub use crate::flat_question::{
    FlatQuestionGradingPayload, FlatQuestionGradingStore, FlatQuestionPublicationPromotion,
    FlatQuestionStore, IssuedFlatGradingContract, UpsertFlatQuestionCommand,
    WorkspaceFlatQuestionSource,
};
pub use crate::flat_question_assets::{
    FlatQuestionAssetStore, MAX_WORKSPACE_FLAT_QUESTION_ASSET_LABEL_CHARS,
    MAX_WORKSPACE_FLAT_QUESTION_ASSET_PROVENANCE_CHARS, WORKSPACE_FLAT_QUESTION_IMAGE_MEDIA_TYPES,
    WorkspaceFlatQuestionAsset, validate_workspace_flat_question_asset_record,
};
pub use crate::item_analysis::{
    CourseItemAnalysisCommitOutcome, CourseItemAnalysisStore, CourseItemAnalysisWorkerCommand,
    CourseItemAnalysisWorkerStore,
};
pub use crate::jobs::{
    ClaimedJob, CreateAssignmentExport, EnqueueJob, ExportArtifactKind, ExportArtifactRecord,
    ExportCommitDisposition, ExportId, ExportJobCommit, ExportJobStore, JobClaimFilter,
    JobFailureDisposition, JobFailureKind, JobId, JobKind, JobLeaseDuration, JobLeaseToken,
    JobPayload, JobState, JobStore, QueueDepth, StudentExportArtifactView, StudentExportJob,
    StudentExportState, StudentExportView, TenantJobView,
};
pub use crate::manual_grade_export::{
    CreateManualGradeExport, MAX_MANUAL_GRADE_EXPORT_ROWS, ManualGradeExport, ManualGradeExportId,
    ManualGradeExportRow, ManualGradeExportStore,
};
pub use crate::manual_grading::{
    EvaluationRevision, ManualCredit, ManualEvaluationRecord, ManualEvaluationStatus,
    ManualGradeActionId, ManualGradeReceipt, ManualGradingStore, SetManualGradeCommand,
    SubmitPendingManualQuestionAttemptCommand,
};
pub use crate::pagination::{Cursor, Page, PageRequest, PageSize, PaginationError};
pub use crate::policy::{
    AssignmentExceptionLimit, AssignmentExceptionTimestamp, AssignmentPolicyException,
    AssignmentPolicyExceptionTarget, CourseGroupRecord, CourseGroupRevision,
    DeleteAssignmentPolicyExceptionCommand, PutCourseGroupCommand, ResolvedAssignmentTiming,
    ResolvedAttemptTiming, SetAssignmentPolicyExceptionCommand, StoredAssignmentPolicyException,
    StoredCourseGroup,
};
pub(crate) use crate::policy::{
    ResolvedAssignmentTimingPolicy, resolve_assignment_policy, validate_assignment_policy_exception,
};
pub use crate::qti::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand,
    QtiGradingStore, QtiImportGradingPayload, QtiImportItem, QtiImportItemRegistration,
    QtiImportItemResult, QtiImportItemStatus, QtiImportProfileSummary, QtiImportRef,
    QtiImportRegistry, QtiImportStore, QtiPublicationPromotion, QtiUnsupportedFeature,
};
pub(crate) use crate::qti_ingress::validate_queue_qti_import;
pub use crate::qti_ingress::{
    QtiImportApiState, QtiImportApiStore, QtiImportApiView, QueueQtiImportCommand,
    qti_import_job_id,
};
pub use crate::retention::{
    AssignmentDefinitionDisposition, CourseRetentionRecord, CourseRetentionSnapshot,
    CourseRetentionState, CourseRetentionStatus, CourseRetentionView,
    DEFAULT_RETENTION_ARCHIVE_DAYS, DEFAULT_RETENTION_DELETE_DAYS, DEFAULT_RETENTION_NOTIFY_DAYS,
    InstitutionRetentionPolicy, MAX_RETENTION_DAYS, MAX_RETENTION_DISPATCH_BATCH,
    RETENTION_ARCHIVE_NOTIFICATION_COPY, RETENTION_JOB_MAX_ATTEMPTS, RetentionCleanupManifest,
    RetentionDays, RetentionDispatchBatch, RetentionNotificationIntent, RetentionNotificationView,
    RetentionPolicyError, RetentionRequestOutcome, RetentionRequestResult, RetentionRevision,
    RetentionStage, RetentionWork, RetentionWorkerCommand,
};
pub use crate::rls::TenantContext;
pub use crate::session::{
    SessionLifetime, SessionRecord, SessionStore, SessionSubject, SessionSubjectError,
    SessionTokenHash, SessionTokenHashParseError,
};
#[cfg(test)]
pub(crate) use activity_policy::CurrentRunQuestion;
pub(crate) use activity_policy::{
    completed_run_score, current_run_questions, ensure_tenant, grade_policy,
    project_enrollment_completion, summary_transition, validate_asset_delivery,
    validate_assignment, validate_assignment_timing, validate_attempt_result, validate_course,
    validate_course_group,
};
#[cfg(feature = "postgres")]
pub(crate) use publication_validation::validate_source_artifact_identity;
pub(crate) use publication_validation::{
    validate_draft, validate_flat_question_publication, validate_publication_source,
    validate_published, validate_qti_import, validate_qti_publication_promotion,
    validate_source_artifact_for_publication,
};

mod contracts;
pub use contracts::*;
pub(crate) use contracts::{
    ActivityStore, AssignmentPolicyStore, AuthoringStore, CourseAssignmentStore, CourseStore,
    FeedbackStore, RunStore, StatisticsStore, assignment_item_is_retired,
    assignment_scoring_changed, current_attempt_points, decode_catalog_search_cursor,
    decode_workspace_draft_cursor, delete_and_regrade_update, encode_catalog_search_cursor,
    encode_workspace_draft_cursor, recalculated_enrollment_projection, select_assignment_run_items,
};
