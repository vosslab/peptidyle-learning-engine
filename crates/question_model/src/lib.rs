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
mod catalog_facets;
/// Tenant-owned course and assignment browser projections.
pub mod course;
/// Closed, browser-safe course appearance and banner presentation contracts.
pub mod course_appearance;
/// Closed course-grade aggregation configuration.
pub mod course_grade;
/// Validated inclusive course-calendar bounds and authoritative IANA zone.
pub mod course_term;
/// Browser-safe collections, favorites, and saved-search contracts.
pub mod curation;
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
/// Browser-safe, no-store Instructor samples of saved assignment item pools.
pub mod pool_preview;
/// Browser-safe, attempt-presentation-scoped question contracts.
pub mod presentation;
/// Strict non-mutating preview-plane contracts, separate from T2 teaching operations.
pub mod preview_plane;
/// Human-facing route locators that resolve to internal identities under authorization.
pub mod public_route;
pub mod response;
/// Browser-safe reusable blueprint and public Alpha curriculum contracts.
pub mod reusable_curriculum;
pub mod run_policy;
/// Browser-safe anonymous-statistics projections and disclosure policy.
pub mod statistics;
pub mod taxonomy;
/// Internal global approval and target-bound co-instructor lifecycle facts.
pub mod teaching_authority;
/// Browser/server teaching-operations wire contracts.
pub mod teaching_operations;

// The crate's front door. These are the types a caller reaches for first, so
// they are re-exported to keep call sites short. Everything else stays
// available under its module.
pub use crate::activity::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentId, AssignmentItemId,
    AssignmentPolicyExceptionId, AssignmentRun, AssignmentRunItem, AssignmentSelectionGroupId,
    AttemptProvenance, AttemptResult, AttemptStatus, AttemptTimerRecord, CourseGroupId, CourseId,
    CourseMembershipId, EnrollmentId, EnrollmentStatus, ImplementationVersion,
    IssuedAttemptCapabilityV1, LearnerAssignmentProgress, LearnerScoreState, QuestionAttempt,
    QuestionAttemptId, RunCompletionStatus, RunId, RunMode, SourceArtifact,
    StudentAssignmentSummary, StudentId, TenantId,
};
pub use crate::assignment::{
    AssignmentDeadlineBehavior, AssignmentDeliveryState, AssignmentInstructions,
    AssignmentInstructionsError, AssignmentItem, AssignmentLifecycle, AssignmentScoringMode,
    AssignmentSelectionCandidate, AssignmentSelectionGroup, AssignmentTeachingSettings,
    AssignmentTeachingSettingsFailureCode, AssignmentTeachingSettingsFailureReason,
    AssignmentTeachingSettingsField, AssignmentTeachingSettingsLocalError,
    AssignmentTeachingSettingsValidationFailure, BaseAssignmentPolicy, CourseLocalDateTime,
    CourseLocalDateTimeError, InstructorAssignmentCurrentState,
    InstructorAssignmentTeachingSettingsLocal, LateSubmissionPolicy, MAX_ASSIGNMENT_ATTEMPT_LIMIT,
    MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP, MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS,
    MAX_ASSIGNMENT_ORDERED_ENTRIES, MAX_ASSIGNMENT_TIME_LIMIT_SECONDS,
    MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES, PointValue, PoolDrawAlgorithm, ScoringGeneration,
    ScoringStatus, SelectionOrdering, derive_instructor_assignment_current_state,
};
pub use crate::auth::{UserId, UserRole};
pub use crate::byline::{PublicAuthorName, PublicByline, PublicBylineError};
pub use crate::capability::{BackendCapabilities, Capability};
pub use crate::catalog::{
    CatalogAuthorship, CatalogBackendFacet, CatalogBylineFacet, CatalogCapabilityFacet,
    CatalogDiscoveryEvidence, CatalogDiscoveryItem, CatalogEvidenceAvailability,
    CatalogEvidenceFacet, CatalogLicenseFacet, CatalogLicenseValue, CatalogLifecycle,
    CatalogOwnCourseUsage, CatalogProblemDetail, CatalogProblemSummary, CatalogPromptProjection,
    CatalogResponseFamily, CatalogResponseFamilyFacet, CatalogSearchFacets, CatalogSearchPage,
    CatalogSearchQuery, CatalogSearchQueryError, CatalogTagFacet, CatalogTaxonomyFacet,
    CatalogTaxonomyFilter, CatalogUsageDetail, CatalogUsageSummary, CatalogUsedInMyCourses,
    CatalogUsedInMyCoursesFacet, MAX_CATALOG_BACKEND_FACETS, MAX_CATALOG_BYLINE_FACETS,
    MAX_CATALOG_BYLINE_FILTERS, MAX_CATALOG_OWN_COURSE_USAGES, MAX_CATALOG_RESPONSE_FAMILY_FACETS,
    MAX_CATALOG_RESPONSE_FAMILY_FILTERS, MAX_CATALOG_TAG_FACETS, MAX_CATALOG_TAG_FILTERS,
    MAX_CATALOG_TAXONOMY_FACETS, MAX_QUESTION_ID_COUNT, ProblemDisplayRef, ProblemVersionRef,
    PublicationScope, QUESTION_ID_ALPHABET, QUESTION_ID_COMPACT_LENGTH,
    QUESTION_ID_IDENTIFIER_LENGTH, QuestionBackend, QuestionId,
};
pub use crate::catalog_facets::CatalogSearchFilter;
pub use crate::course::{
    AssignmentItemSummary, AssignmentSelectionCandidateSummary, AssignmentSelectionGroupSummary,
    AssignmentSummary, CourseMembershipRole, CourseSummary, GradebookSummaryRow,
    LearnerAssignmentDelivery, LearnerAssignmentDetail, LearnerAssignmentSummary,
    LearnerLateStatus,
};
pub use crate::course_appearance::{
    CourseAppearance, CourseAppearanceRevision, CourseAppearanceUpdate, CourseBannerAltText,
    CourseBannerAlternativeText, CourseBannerCandidateId, CourseBannerCandidateReceipt,
    CourseBannerId, CourseBannerMutation, CourseBannerPresentation, CourseThemeId,
};
pub use crate::course_grade::{
    CourseGradeAssignmentSetting, CourseGradeAssignmentView, CourseGradeMode,
    CourseGradeOutcomeView, CourseGradeRoundingRule, CourseGradeScheme, CourseGradeSchemeError,
    CourseGradeSchemeUpdateView, CourseGradeSchemeView, CourseGradeUnavailableReasonView,
    CourseGradebookTotalViewRow, CourseGradebookTotalsView, GradeCategoryId, GradeCategoryTitle,
    LetterBand, LetterBandLabel, WeightedGradeCategory,
};
pub use crate::course_term::{
    CourseDate, CourseDateError, CourseTerm, CourseTermError, CourseTermFailureCode,
    CourseTermFailureReason, CourseTermField, CourseTermValidationFailure, IanaTimeZone,
    IanaTimeZoneError,
};
pub use crate::curation::{
    MAX_NAMED_PROBLEM_COLLECTIONS, MAX_PROBLEM_COLLECTION_MEMBERS,
    MAX_PROBLEM_CURATION_TITLE_UNICODE_SCALARS, MAX_SAVED_PROBLEM_SEARCHES,
    ProblemCollectionAccess, ProblemCollectionKind, ProblemCollectionMemberView,
    ProblemCollectionRevision, ProblemCollectionSelectionAvailability,
    ProblemCollectionSummaryView, ProblemCollectionVisibility, ProblemCurationTitleError,
    SavedProblemSearchRevision, SavedProblemSearchView, validate_problem_curation_title,
};
pub use crate::definition::{
    DraftQuestionDefinition, DraftQuestionSource, DraftSourcePublicationError, GradingDefinition,
    MAX_QUESTION_TITLE_UNICODE_SCALARS, QuestionDefinition, QuestionMetadata, QuestionSource,
    QuestionSourceValidationError, QuestionTitleError, WorkspaceDraftSummary,
    validate_question_title,
};
pub use crate::entitlement::{
    AssignmentAudience, AssignmentAudienceError, CourseGroupPurpose, CourseGroupPurposePolicy,
    EntitlementMaterialization, EntitlementPurpose, EvaluatorVersion, GroupPurposeCapabilities,
    MaterializationAuthority, MaterializationBasis, MaterializationDisposition,
    MaterializationRule, MultipleMembershipDisposition, MultipleMembershipPolicy,
    NonEmptyAudienceGroups,
};
pub use crate::envelope::QuestionEnvelope;
pub use crate::feedback::{DisclosedFeedback, FeedbackContent};
pub use crate::generation::GeneratorReference;
pub use crate::identity::{
    AssetId, ObjectId, ProblemId, VersionId, WorkspaceId, WorkspaceImportId,
};
pub use crate::lifecycle::{Lifecycle, LifecycleError, LifecycleEvent};
pub use crate::pool_preview::{PoolDrawPreview, PoolDrawPreviewQuestion, PoolDrawPreviewRequest};
pub use crate::presentation::{
    AssetBindingV1, LearnerAttemptDescriptorV1, LearnerRunScreenRunV1, LearnerRunScreenScopeV1,
    LearnerRunScreenV1, PresentationBindingV1, PresentationDigestTokenV1, PresentationDigestV1,
    PresentationEnvelopeV1, PresentationNonceV1, PresentedBlankV1, PresentedChoiceV1,
    PresentedHotspotRegionV1, PresentedHotspotSurfaceV1, RenderedItemIdV1, ResponseSchemaV1,
};
pub use crate::preview_plane::{
    DerivedPreviewSubjectRequest, InstructorPreviewSchedulePage, InstructorPreviewScheduleRow,
    MAX_PREVIEW_SUBJECT_GROUPS, PreviewAccommodationComparison, PreviewDeadlineBehaviorField,
    PreviewDeferredCapability, PreviewDenialReason, PreviewDisclosureFlags,
    PreviewDisclosureMoment, PreviewDisclosureProjection, PreviewDisclosureUnavailableReason,
    PreviewEntitlementDenialReason, PreviewEntitlementGrantReason, PreviewEntitlementOutcome,
    PreviewEvaluation, PreviewFutureSeam, PreviewGroupFact, PreviewGroupFacts, PreviewGroupRole,
    PreviewLateSubmissionField, PreviewLimitField, PreviewPlaneResponse, PreviewPolicySourceLayer,
    PreviewPriorRunCount, PreviewResolvedPolicy, PreviewScheduleProjection, PreviewSelectedMoment,
    PreviewSubject, PreviewSubjectKind, PreviewSyntheticGroupReferences, PreviewTimeField,
    SyntheticPreviewModifiers, SyntheticPreviewSubjectRequest, preview_group_role,
};
pub use crate::public_route::{
    AccountReference, AlphaCourseReference, AssignmentReference, BlueprintReference,
    CoInstructorInvitationReference, CourseGroupReference, CourseMembershipReference,
    CourseReference, MAX_PUBLIC_ROUTE_NUMBER, NavigationResolution, ProblemCollectionReference,
    RESERVED_REFERENCE_PREFIXES, RunReference, SavedProblemSearchReference, WorkspaceReference,
};
pub use crate::response::{ResponseDefinition, StudentResponse};
pub use crate::reusable_curriculum::{
    AlphaCourseAccess, AlphaCourseDefinitionInput, AlphaCourseModuleInput, AlphaCourseModuleView,
    AlphaCourseRevision, AlphaCourseSummaryView, AlphaCourseView, BlueprintAccess,
    BlueprintDefinitionInput, BlueprintRevision, BlueprintSummaryView, BlueprintView,
    LocalTimeOfDay, LocalTimeOfDayError, MAX_REUSABLE_CURRICULUM_TITLE_UNICODE_SCALARS,
    RelativeAssignmentSchedule, RelativeScheduleMoment, ReusableAssignmentDefaults,
    ReusableAssignmentDefinitionInput, ReusableAssignmentDefinitionView,
    ReusableAssignmentEntryInput, ReusableAssignmentEntryView, ReusableCurriculumTitleError,
    ReusableCurriculumValidationError, ReusableFixedQuestionInput, ReusablePoolCandidateView,
    ReusablePoolInput, ReusablePoolView, ReusableQuestionView, ReusableSelectionAvailability,
    validate_reusable_curriculum_title,
};
pub use crate::run_policy::{
    CompletionRequirement, ContinuedPractice, GradePolicy, LearnerDisclosurePolicy,
    LearnerDisclosureTiming, PoolDrawBasis, PoolDrawBasisError, PoolDrawPreviewNonce, RunPolicies,
    VariationPolicy,
};
pub use crate::statistics::{
    DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE, LearnerClassStatistics, QuestionStatisticsDisclosure,
    QuestionStatisticsView, StatisticsDisclosurePolicy, StatisticsDisclosurePolicyError,
};
pub use crate::teaching_authority::{
    CoInstructorInvitation, CoInstructorInvitationId, CoInstructorInvitationState,
    InstructorApproval,
};
pub use crate::teaching_operations::{
    AccountApprovalView, AssignmentPolicyPatchUpdateRequest, CoInstructorInvitationCreateRequest,
    CoInstructorInvitationStateView, CoInstructorInvitationTerminalAction,
    CoInstructorInvitationTerminalActionRequest, CoInstructorTargetSearchPage,
    CoInstructorTargetSearchQuery, CoInstructorTargetSearchRequest, CoInstructorTargetView,
    CourseCoInstructorInvitationView, CourseCoInstructorInvitationsPage, CourseGroupCreateRequest,
    CourseGroupDetailView, CourseGroupListPage, CourseGroupMemberView, CourseGroupMembers,
    CourseGroupMembershipWarningView, CourseGroupPurposePolicyUpdateRequest,
    CourseGroupPurposePolicyView, CourseGroupSummaryView, CourseGroupUpdateRequest,
    CourseStudentMembershipsPage, GroupScheduleOffsetSeconds, GroupScheduleOffsetUpdateRequest,
    IndividualPolicyPatchUpdateRequest, InstructorApprovalStateView,
    InstructorMembershipRemovalRequest, InstructorMembershipView, InstructorMembershipsPage,
    MembershipPageRequest, PendingCoInstructorInvitationView, PendingCoInstructorInvitationsPage,
    PolicyModificationModeView, PolicyPatchView, RetentionActionOutcomeView,
    RetentionActionResponse, RetentionAdditionalDays, RetentionArchiveRequest,
    RetentionDispositionView, RetentionExtendRequest, RetentionNotificationIntentView,
    RetentionNotificationView, RetentionReadView, RetentionStateView,
    SysadminInstructorApprovalStateView, SysadminInstructorApprovalView,
    SysadminInstructorCandidateSearchPage, SysadminInstructorCandidateSearchRequest,
    SysadminInstructorCandidateView, TeachingAccountView, TeachingAttemptLimit,
    TeachingAttemptLimitFieldPatch, TeachingDisplayLabel, TeachingLateVerdict,
    TeachingLimitFieldPatch, TeachingOperationRevision, TeachingOperationRevisionResponse,
    TeachingPageSize, TeachingPreviewDeadlineBehaviorField, TeachingPreviewDenialReason,
    TeachingPreviewFieldSource, TeachingPreviewGroupSource, TeachingPreviewGroupSources,
    TeachingPreviewLateSubmissionField, TeachingPreviewLimitField, TeachingPreviewTimeField,
    TeachingPreviewView, TeachingStartVerdict, TeachingTimeFieldPatch, TeachingTimeLimitSeconds,
    project_teaching_preview_time_field, resolve_teaching_local_time,
};
