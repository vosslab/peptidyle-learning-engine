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
/// Stable Assignment Entries, Question Pools, and exact point values.
pub mod assignment;
/// Strict browser contracts and derived readiness for the Instructor assignment workspace.
pub mod assignment_workspace;
pub mod auth;
/// Immutable public publication attribution, separate from private author authority.
pub mod byline;
pub mod capability;
/// Shared catalog metadata, visibility, lineage, and browse projections.
pub mod question_library;
mod question_search;
/// Course and assignment browser projections.
pub mod course;
/// Closed, browser-safe course appearance and banner presentation contracts.
pub mod course_appearance;
/// Closed course-grade aggregation configuration.
pub mod course_grade;
/// Validated inclusive course-calendar bounds and authoritative IANA zone.
pub mod course_term;
/// Browser-safe private Question Folder and saved-search contracts.
pub mod curation;
/// Normalized B2 reusable meaning, semantic digests, and target-term schedule resolution.
pub mod curriculum_adoption;
pub mod definition;
/// Internal entitlement and materialization contracts. These types never
/// cross the browser boundary.
pub mod entitlement;
pub mod envelope;
/// Private teaching material and policy-redacted browser feedback.
pub mod feedback;
pub mod generation;
/// Browser-safe automated-grading operation status and safe explanation contracts.
pub mod grading_operations;
pub mod identity;
/// Browser-safe, no-store Instructor samples of saved Assignment Question Pools.
pub mod pool_preview;
/// Browser-safe, attempt-presentation-scoped question contracts.
pub mod presentation;
/// Strict non-mutating preview-plane contracts, separate from T2 teaching operations.
pub mod preview_plane;
/// Human-facing route locators that resolve to internal identities under authorization.
pub mod public_route;
pub mod response;
/// Browser-safe reusable BlueprintCourse contracts.
pub mod reusable_curriculum;
#[path = "run_policy.rs"]
pub mod assignment_activity_rules;
/// Browser-safe anonymous-statistics projections and disclosure policy.
pub mod statistics;
pub mod taxonomy;
/// Internal global approval and target-bound Course Invitation lifecycle facts.
pub mod teaching_authority;
/// Browser/server teaching-operations wire contracts.
pub mod teaching_operations;

// The crate's front door. These are the types a caller reaches for first, so
// they are re-exported to keep call sites short. Everything else stays
// available under its module.
pub use crate::activity::{
    ActivityTimestamp, AssignmentProgressRecord, AssignmentAttempt, AssignmentAttemptId,
    AccommodationId, AssignmentGrade, AssignmentId, AssignmentEntryId, AttemptProvenance,
    AttemptResult, AttemptStatus, QuestionPoolCandidateId,
    CourseId, CourseMembershipId, ImplementationVersion, IssuedAttemptCapabilityV1, IssuedQuestion,
    IssuedQuestionId, QuestionAttempt, QuestionAttemptId, QuestionAttemptTiming,
    AssignmentAttemptCompletion, SourceArtifact, AssignmentProgress, StudentRecordId,
    AssignmentProgressScoreState,
};
pub use crate::assignment::{
    AssignmentDeadlineBehavior, AssignmentDeliveryState, AssignmentInstructions,
    AssignmentEntry, AssignmentInstructionsError, FixedQuestionAssignmentEntry, AssignmentLifecycle, AssignmentRevision,
    AssignmentRevisionError, AssignmentScoringMode, QuestionPoolCandidate,
    QuestionPoolAssignmentEntry, AssignmentTeachingSettings, AssignmentTeachingSettingsFailureCode,
    AssignmentTeachingSettingsFailureReason, AssignmentTeachingSettingsField,
    AssignmentTeachingSettingsLocalError, AssignmentTeachingSettingsValidationFailure,
    BaseAssignmentPolicy, CourseLocalDateTime, CourseLocalDateTimeError,
    InstructorAssignmentCurrentState, InstructorAssignmentTeachingSettingsLocal,
    LateSubmissionPolicy, MAX_ASSIGNMENT_ATTEMPT_LIMIT,
    MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL, MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS,
    MAX_ASSIGNMENT_ORDERED_ENTRIES, MAX_ASSIGNMENT_TIME_LIMIT_SECONDS,
    MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES, PointValue, PoolDrawAlgorithm, ScoringGeneration,
    ScoringStatus, SelectionOrdering, derive_instructor_assignment_current_state,
};
pub use crate::assignment_workspace::{
    AssignmentContentIssuedWorkConflict, AssignmentContentIssuedWorkConflictKind,
    AssignmentEntryRequest, AssignmentPoliciesValidationFailure,
    AssignmentPoliciesValidationFailureCode, AssignmentPoliciesValidationIssue,
    AssignmentPolicyConfigurationReason, AssignmentPublicationBlockingIssue,
    AssignmentPublicationReadiness, CreateAssignmentDraftRequest, InstructorStudentView,
    InstructorStudentViewDelivery, ReplaceAssignmentContentRequest,
    ReplaceAssignmentFixedItemRequest, ReplaceAssignmentPoliciesRequest,
};
pub use crate::auth::{AccountId, AccountRole};
pub use crate::byline::{PublicAuthorName, PublicByline, PublicBylineError};
pub use crate::capability::{BackendCapabilities, Capability};
pub use crate::question_library::{
    QuestionSearchAuthorship, QuestionSearchBackendFacet, QuestionSearchBylineFacet,
    QuestionSearchCapabilityFacet, QuestionStatistics, QuestionStatisticsAvailability,
    QuestionStatisticsAvailabilityFacet, QuestionSearchLicenseFacet, QuestionSearchLicense,
    CourseQuestionUse, QuestionDetails, QuestionSummary, QuestionPromptProjection,
    QuestionSearchResult,
    QuestionTypeFacet, QuestionSearchFacets, QuestionSearchFilter,
    QuestionSearchPage, QuestionSearchRequest, QuestionSearchRequestError, QuestionSearchTagFacet,
    QuestionSearchTaxonomyFacet, QuestionSearchTaxonomyFilter, QuestionUseDetails, QuestionUseSummary,
    QuestionSearchCourseUse, QuestionSearchCourseUseFacet, MAX_QUESTION_SEARCH_BACKEND_FACETS,
    MAX_QUESTION_SEARCH_BYLINE_FACETS, MAX_QUESTION_SEARCH_BYLINE_FILTERS, MAX_QUESTION_SEARCH_OWN_COURSE_USAGES,
    MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS, MAX_QUESTION_SEARCH_QUESTION_TYPE_FILTERS,
    MAX_QUESTION_SEARCH_TAG_FACETS, MAX_QUESTION_SEARCH_TAG_FILTERS, MAX_QUESTION_SEARCH_TAXONOMY_FACETS,
    MAX_QUESTION_ID_COUNT, QuestionVersionAvailability, QuestionVersionReference, QUESTION_ID_ALPHABET,
    QUESTION_ID_COMPACT_LENGTH, QUESTION_ID_IDENTIFIER_LENGTH, QuestionBackend, QuestionId,
};
pub use crate::course::{
    AssignmentEntrySummary, FixedQuestionAssignmentEntrySummary, AssignmentLandingPresentation,
    QuestionPoolCandidateSummary, QuestionPoolAssignmentEntrySummary, AssignmentSummary,
    CourseMembershipRole, CourseSummary,
    GradebookSummaryRow, StudentAssignmentDelivery, StudentAssignmentDetail,
    StudentAssignmentLandingSummary, StudentLateStatus,
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
    MAX_NAMED_QUESTION_FOLDERS, MAX_QUESTION_FOLDER_MEMBERS,
    MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS, MAX_SAVED_QUESTION_SEARCHES,
    QuestionFolderEntryView,
    QuestionFolderEditNumber,
    QuestionFolderSummaryView, QuestionCurationTitleError,
    SavedQuestionSearchEditNumber, SavedQuestionSearchView, validate_question_curation_title,
};
pub use crate::curriculum_adoption::*;
pub use crate::definition::{
    DraftQuestionDefinition, DraftQuestionSource, DraftSourcePublicationError, GradingDefinition,
    MAX_QUESTION_TITLE_UNICODE_SCALARS, QuestionDefinition, QuestionFormat, QuestionMetadata,
    QuestionSource, QuestionSourceValidationError, QuestionTitleError, WorkspaceDraftSummary,
    validate_question_title,
};
pub use crate::entitlement::{
    EntitlementMaterialization, EntitlementPurpose, EvaluatorVersion, MaterializationAuthority,
    MaterializationBasis, MaterializationDisposition, MaterializationRule,
};
pub use crate::envelope::QuestionEnvelope;
pub use crate::feedback::{DisclosedFeedback, FeedbackContent, InspectedStudentScoreFeedbackV1};
pub use crate::generation::GeneratorReference;
pub use crate::grading_operations::{
    AutomatedGradingStatus, GradingOperationAction, GradingOperationReason, InstructorGradingOperationState,
    GradingOperationVisibleState, SubmissionEvaluationStatus,
};
pub use crate::identity::{
    AssetId, ObjectId, QuestionVersionNumber, WorkspaceId, WorkspaceImportId,
};
pub use crate::pool_preview::{PoolDrawPreview, PoolDrawPreviewQuestion, PoolDrawPreviewRequest};
pub use crate::presentation::{
    AssetBindingV1, PresentationBindingV1, PresentationDigestTokenV1, PresentationDigestV1,
    PresentationEnvelopeV1, PresentationNonceV1, PresentedBlankV1, PresentedChoiceV1,
    PresentedHotspotRegionV1, PresentedHotspotSurfaceV1, RenderedItemIdV1, IssuedQuestionResponseFormatV1,
    StudentAssignmentAttemptScreenAttemptV1, StudentAssignmentAttemptScreenScopeV1,
    StudentAssignmentAttemptScreenV1, StudentAttemptDescriptorV1,
};
pub use crate::preview_plane::{
    DerivedPreviewSubjectRequest, InstructorPreviewSchedulePage, InstructorPreviewScheduleRow,
    PreviewAccommodationComparison, PreviewDeadlineBehaviorField,
    PreviewDeferredCapability, PreviewDenialReason, PreviewDisclosureFlags,
    PreviewDisclosureMoment, PreviewDisclosureProjection, PreviewDisclosureUnavailableReason,
    PreviewEntitlementDenialReason, PreviewEntitlementGrantReason, PreviewEntitlementOutcome,
    PreviewEvaluation, PreviewFutureSeam, PreviewLateSubmissionField, PreviewLimitField,
    PreviewPlaneResponse, PreviewPolicySourceLayer,
    PreviewPriorRunCount, PreviewResolvedPolicy, PreviewScheduleProjection, PreviewSelectedMoment,
    PreviewSubject, PreviewSubjectKind, PreviewTimeField, SyntheticPreviewModifiers,
    SyntheticPreviewSubjectRequest,
};
pub use crate::public_route::{
    AccountReference, AssignmentAttemptReference, AssignmentReference, BlueprintCourseReference,
    CourseInvitationReference, CourseMembershipReference,
    CourseInstanceReference, InstructorGradingOperationReference, MAX_PUBLIC_ROUTE_NUMBER, NavigationResolution,
    QuestionFolderReference, RESERVED_REFERENCE_PREFIXES, SavedQuestionSearchReference,
    AuthoringWorkspaceReference,
};
pub use crate::response::{
    QuestionResponseControl, QuestionResponseFormat, QuestionType, StudentResponse,
};
pub use crate::reusable_curriculum::{
    BlueprintAssignmentEditHandle, BlueprintAssignmentId, BlueprintChildIdError,
    BlueprintCourseAccess, BlueprintCourseAssignmentDefinitionView,
    BlueprintCourseAssignmentReplacementInput, BlueprintCourseModuleReplacementInput,
    BlueprintCourseModuleView, BlueprintCourseSummaryView, BlueprintCourseView,
    BlueprintModuleEditHandle, BlueprintModuleId, BlueprintRevision,
    CreateBlueprintCourseDefinitionInput, CreateBlueprintCourseModuleInput, LocalTimeOfDay,
    LocalTimeOfDayError, MAX_REUSABLE_CURRICULUM_TITLE_UNICODE_SCALARS, RelativeAssignmentSchedule,
    RelativeScheduleMoment, ReplaceBlueprintCourseDefinitionInput, ReusableAssignmentDefaults,
    ReusableAssignmentDefinitionInput, ReusableAssignmentDefinitionView,
    ReusableAssignmentEntryInput, ReusableAssignmentEntryView, ReusableCurriculumTitleError,
    ReusableCurriculumValidationError, ReusableFixedQuestionInput, ReusablePoolCandidateView,
    ReusablePoolInput, ReusablePoolView, ReusableQuestionView, ReusableSelectionAvailability,
    validate_reusable_curriculum_title,
};
pub use crate::assignment_activity_rules::{
    CompletionRequirement, ContinuedPractice, GradePolicy, PoolDrawBasis, PoolDrawBasisError,
    PoolDrawPreviewNonce, AssignmentActivityRules, StudentDisclosurePolicy, StudentDisclosureTiming,
    VariationPolicy,
};
pub use crate::statistics::{
    DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE, QuestionStatisticsDisclosure, QuestionStatisticsView,
    StatisticsDisclosurePolicy, StatisticsDisclosurePolicyError, StudentClassStatistics,
};
pub use crate::teaching_authority::{
    CourseInvitation, CourseInvitationEvent, CourseInvitationEventKind,
    CourseInvitationId, CourseInvitationState, InstructorApprovalEvent,
    InstructorApprovalEventKind,
};
pub use crate::teaching_operations::{
    AccountApprovalView, AssignmentPolicyPatchUpdateRequest, InstructorCourseInvitationCreateRequest,
    CourseInvitationStateView, CourseInvitationTerminalAction,
    CourseInvitationTerminalActionRequest, CourseInvitationTargetSearchPage,
    CourseInvitationTargetSearchQuery, CourseInvitationTargetSearchRequest, CourseInvitationTargetView,
    InstructorCourseInvitationView, InstructorCourseInvitationsPage, CourseStudentMembershipsPage,
    StudentMembershipView,
    AccommodationPatchUpdateRequest, InstructorApprovalStateView,
    InstructorMembershipRemovalRequest, InstructorMembershipView, InstructorMembershipsPage,
    MembershipPageRequest, PendingCourseInvitationView, PendingCourseInvitationsPage,
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
    TeachingPreviewFieldSource,
    TeachingPreviewLateSubmissionField, TeachingPreviewLimitField, TeachingPreviewTimeField,
    TeachingPreviewView, TeachingStartVerdict, TeachingTimeFieldPatch, TeachingTimeLimitSeconds,
    project_teaching_preview_time_field, resolve_teaching_local_time,
};
