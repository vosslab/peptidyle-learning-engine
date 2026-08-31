//! Browser-safe domain contracts for the Peptidyle Learning Engine.
//!
//! Question types, Student Work Records, backend capabilities, identity, and
//! Question Classifications live here.
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

pub mod answer;
/// Stable Assignment Entries, Question Pools, and exact point values.
pub mod assignment;
pub mod assignment_activity_rules;
/// Strict browser contracts and derived readiness for the Instructor assignment workspace.
pub mod assignment_workspace;
pub mod auth;
/// Browser-safe reusable BlueprintCourse contracts.
pub mod blueprint_course;
/// Immutable public publication attribution, separate from private author authority.
pub mod byline;
pub mod capability;
pub mod classification;
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
/// Shared Question Library metadata, visibility, lineage, and browse projections.
pub mod question_library;
mod question_search;
pub mod response;
/// Browser-safe anonymous-statistics projections and disclosure policy.
pub mod statistics;
pub mod student_work;
/// Internal global approval and target-bound Course Invitation lifecycle facts.
pub mod teaching_authority;
/// Browser/server teaching-operations wire contracts.
pub mod teaching_operations;

// The crate's front door. These are the types a caller reaches for first, so
// they are re-exported to keep call sites short. Everything else stays
// available under its module.
pub use crate::assignment::{
    AssignmentDeadlineRule, AssignmentEditNumber, AssignmentEntry, AssignmentEntryAvailability,
    AssignmentEntryScoringRule, AssignmentInstructions, AssignmentInstructionsError,
    AssignmentPointValue, AssignmentRevisionNumber, AssignmentRevisionNumberError,
    AssignmentScoringState, AssignmentStatus, AssignmentTitle, AssignmentTitleError,
    AssignmentWorkingCopyDefinition, AssignmentWorkingCopyDefinitionFailureCode,
    AssignmentWorkingCopyDefinitionFailureReason, AssignmentWorkingCopyDefinitionField,
    AssignmentWorkingCopyDefinitionLocalError, AssignmentWorkingCopyDefinitionValidationFailure,
    BaseAssignmentPolicy, CourseLocalDateAndTime, CourseLocalDateAndTimeError,
    FixedQuestionAssignmentEntry, InstructorAssignmentCurrentState,
    InstructorAssignmentWorkingCopyDefinitionLocal, LateWorkRule, MAX_ASSIGNMENT_ATTEMPT_LIMIT,
    MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS, MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL,
    MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS, MAX_ASSIGNMENT_ORDERED_ENTRIES,
    MAX_ASSIGNMENT_TITLE_UNICODE_SCALARS, MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES,
    QuestionPoolAssignmentEntry, QuestionPoolCandidate, QuestionPoolCandidateAvailability,
    QuestionPoolSelectedQuestionOrder, QuestionPoolSelectionRule, ScoringGeneration,
    derive_instructor_assignment_current_state,
};
pub use crate::assignment_activity_rules::{
    AssignmentActivityRules, AssignmentAttemptContinuationRule, AssignmentAttemptGradeRule,
    AssignmentAttemptResumeRule, AssignmentCompletionRule, AssignmentNavigationRule,
    AssignmentQuestionDisplayRule, AssignmentQuestionOrderRule, QuestionAttemptLimit,
    QuestionPoolPreviewNonce, QuestionPoolReuseRule, QuestionPoolSelectionInputs,
    QuestionVariationRule, StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming,
};
pub use crate::assignment_workspace::{
    AssignmentEntryRequest, AssignmentPoliciesValidationFailure,
    AssignmentPoliciesValidationFailureCode, AssignmentPoliciesValidationIssue,
    AssignmentReleaseIssue, AssignmentReleaseValidation, CreateAssignmentRequest,
    InstructorStudentView, InstructorStudentViewDelivery, ReplaceAssignmentContentRequest,
    ReplaceAssignmentFixedItemRequest, ReplaceAssignmentPoliciesRequest,
    SuccessorAssignmentRevisionRequired,
};
pub use crate::auth::{AccountId, AccountRole};
pub use crate::blueprint_course::{
    BlueprintAssignmentEditHandle, BlueprintAssignmentId, BlueprintChildIdError,
    BlueprintCourseAccess, BlueprintCourseAssignmentDefinitionView,
    BlueprintCourseAssignmentReplacementInput, BlueprintCourseModuleReplacementInput,
    BlueprintCourseModuleView, BlueprintCourseSummaryView, BlueprintCourseTitleError,
    BlueprintCourseValidationError, BlueprintCourseView, BlueprintModuleEditHandle,
    BlueprintModuleId, BlueprintRevision, CreateBlueprintCourseDefinitionInput,
    CreateBlueprintCourseModuleInput, LocalTimeOfDay, LocalTimeOfDayError,
    MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS, RelativeAssignmentSchedule,
    RelativeAssignmentScheduleMoment, ReplaceBlueprintCourseDefinitionInput,
    ReusableAssignmentDefaults, ReusableAssignmentDefinitionInput,
    ReusableAssignmentDefinitionView, ReusableAssignmentEntryInput, ReusableAssignmentEntryView,
    ReusableFixedQuestionInput, ReusablePoolCandidateView, ReusablePoolInput, ReusablePoolView,
    ReusableQuestionView, ReusableSelectionAvailability, validate_blueprint_course_title,
};
pub use crate::byline::{PublicAuthorName, PublicByline, PublicBylineError};
pub use crate::capability::{Capability, QuestionBackendCapabilities};
pub use crate::course::{
    AssignmentEntrySummary, AssignmentOverview, AssignmentSummary, CourseMembershipRole,
    CourseSummary, FixedQuestionAssignmentEntrySummary, GradebookSummaryRow,
    QuestionPoolAssignmentEntrySummary, QuestionPoolCandidateSummary, StudentAssignmentDelivery,
    StudentAssignmentDetail, StudentAssignmentLandingSummary, StudentLateWorkStatus,
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
    CourseGradebookTotalViewRow, CourseGradebookTotalsView, GradeCategory, GradeCategoryId,
    GradeCategoryTitle, LetterBand, LetterBandLabel,
};
pub use crate::course_term::{
    CourseDate, CourseDateError, CourseTerm, CourseTermError, CourseTermFailureCode,
    CourseTermFailureReason, CourseTermField, CourseTermValidationFailure, CourseTimeZone,
    CourseTimeZoneError,
};
pub use crate::curation::{
    MAX_NAMED_QUESTION_FOLDERS, MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS,
    MAX_QUESTION_FOLDER_MEMBERS, MAX_SAVED_QUESTION_SEARCHES, QuestionCurationTitleError,
    QuestionFolderEditNumber, QuestionFolderEntryView, QuestionFolderSummaryView,
    SavedQuestionSearchEditNumber, SavedQuestionSearchView, validate_question_curation_title,
};
pub use crate::curriculum_adoption::*;
pub use crate::definition::{
    DraftQuestionDefinition, DraftQuestionSource, DraftSourcePublicationError,
    MAX_QUESTION_TITLE_UNICODE_SCALARS, QuestionFormat, QuestionGradingRule, QuestionMetadata,
    QuestionRevision, QuestionSource, QuestionSourceValidationError, QuestionTitleError,
    WorkspaceDraftSummary, validate_question_title,
};
pub use crate::envelope::{QuestionPresentation, QuestionVariation};
pub use crate::feedback::{
    QuestionAnswer, QuestionAnswerExplanation, QuestionFeedback, QuestionHint,
    QuestionPostGradingContent, StudentFeedback, StudentResponseInspectionFeedback,
};
pub use crate::generation::QuestionGeneratorReference;
pub use crate::grading_operations::{
    GradingOperationAction, GradingOperationReason, GradingOperationVisibleState,
    InstructorGradingOperationState, QuestionSubmissionGradingState,
    StudentQuestionSubmissionGradingState,
};
pub use crate::identity::{
    AssetId, ObjectId, QuestionRevisionNumber, WorkspaceId, WorkspaceImportId,
};
pub use crate::pool_preview::{
    QuestionPoolPreview, QuestionPoolPreviewQuestion, QuestionPoolPreviewRequest,
};
pub use crate::presentation::{
    IssuedQuestionResponseFormatV1, PresentationEnvelopeV1, PresentationResponseItemReference,
    PresentedBlankV1, PresentedChoiceV1, PresentedHotspotRegionV1, PresentedHotspotSurfaceV1,
    PresentedQuestionAsset, QuestionPresentationBinding, QuestionPresentationDigest,
    QuestionPresentationNonce, QuestionPresentationToken, StudentAssignmentAttemptScreenAttemptV1,
    StudentAssignmentAttemptScreenScopeV1, StudentAssignmentAttemptScreenV1,
    StudentAttemptDescriptorV1,
};
pub use crate::preview_plane::{
    ActiveStudentCourseMembershipDenialReason, ActiveStudentCourseMembershipGrantReason,
    ActiveStudentCourseMembershipOutcome, AssignmentPolicySourceKind, DerivedPreviewSubjectRequest,
    EffectiveAssignmentPolicyView, InstructorPreviewSchedulePage, InstructorPreviewScheduleRow,
    PreviewAccommodationComparison, PreviewAssignmentDeadlineRuleField, PreviewDeferredCapability,
    PreviewDenialReason, PreviewDisclosureFlags, PreviewDisclosureMoment,
    PreviewDisclosureUnavailableReason, PreviewEvaluation, PreviewFutureSeam,
    PreviewLateWorkRuleField, PreviewLimitField, PreviewPlaneResponse, PreviewPriorRunCount,
    PreviewResolvedPolicy, PreviewSelectedMoment, PreviewTimeField, StudentFeedbackReleaseView,
    StudentViewScenario, StudentViewScenarioKind, StudentViewScenarioRequest,
    SyntheticPreviewModifiers,
};
pub use crate::public_route::{
    AccountReference, AssignmentAttemptReference, AssignmentReference, AuthoringWorkspaceReference,
    BlueprintCourseReference, CourseInstanceReference, CourseInvitationReference,
    CourseMembershipReference, InstructorGradingOperationReference, MAX_PUBLIC_ROUTE_NUMBER,
    NavigationResolution, QuestionFolderReference, RESERVED_REFERENCE_PREFIXES,
    SavedQuestionSearchReference,
};
pub use crate::question_library::{
    CourseQuestionUse, MAX_QUESTION_ID_COUNT, MAX_QUESTION_SEARCH_BACKEND_FACETS,
    MAX_QUESTION_SEARCH_BYLINE_FACETS, MAX_QUESTION_SEARCH_BYLINE_FILTERS,
    MAX_QUESTION_SEARCH_CLASSIFICATION_FACETS, MAX_QUESTION_SEARCH_OWN_COURSE_USAGES,
    MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS, MAX_QUESTION_SEARCH_QUESTION_TYPE_FILTERS,
    MAX_QUESTION_SEARCH_TAG_FACETS, MAX_QUESTION_SEARCH_TAG_FILTERS, QUESTION_ID_ALPHABET,
    QUESTION_ID_COMPACT_LENGTH, QUESTION_ID_IDENTIFIER_LENGTH, QuestionBackend, QuestionDetails,
    QuestionId, QuestionPromptProjection, QuestionRevisionAvailability, QuestionRevisionReference,
    QuestionSearchAuthorship, QuestionSearchBackendFacet, QuestionSearchBylineFacet,
    QuestionSearchCapabilityFacet, QuestionSearchClassificationFacet,
    QuestionSearchClassificationFilter, QuestionSearchCourseUse, QuestionSearchCourseUseFacet,
    QuestionSearchFacets, QuestionSearchFilter, QuestionSearchLicense, QuestionSearchLicenseFacet,
    QuestionSearchPage, QuestionSearchRequest, QuestionSearchRequestError, QuestionSearchResult,
    QuestionSearchTagFacet, QuestionStatistics, QuestionStatisticsAvailability,
    QuestionStatisticsAvailabilityFacet, QuestionSummary, QuestionTypeFacet, QuestionUseDetails,
    QuestionUseSummary,
};
pub use crate::response::{
    QuestionResponseControl, QuestionResponseFormat, QuestionType, StudentResponse,
};
pub use crate::statistics::{
    DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE, QuestionStatisticsDisclosure, QuestionStatisticsView,
    StatisticsDisclosurePolicy, StatisticsDisclosurePolicyError, StudentClassStatistics,
};
pub use crate::student_work::{
    AccommodationId, ActivityTimestamp, AssignmentAttempt, AssignmentAttemptCompletion,
    AssignmentAttemptId, AssignmentEntryId, AssignmentGrade, AssignmentId, AssignmentProgress,
    AssignmentProgressRecord, AssignmentProgressScoreState, CourseId, CourseMembershipId,
    GradingResult, IssuedAttemptCapabilityV1, IssuedQuestion, IssuedQuestionId, QuestionAttempt,
    QuestionAttemptId, QuestionAttemptReproductionDetails, QuestionAttemptState,
    QuestionAttemptTiming, QuestionBackendVersion, QuestionGraderVersion, QuestionPoolCandidateId,
    QuestionPoolSelectedCandidate, QuestionPoolSelection, QuestionPoolSelectionId,
    QuestionPoolSelectionReuseError, QuestionRendererVersion, QuestionSubmission,
    QuestionSubmissionId, SourceObjectReference, StudentQuestionAttemptView, StudentRecordId,
};
pub use crate::teaching_authority::{
    CourseInvitation, CourseInvitationEvent, CourseInvitationEventKind, CourseInvitationId,
    CourseInvitationState, InstructorApprovalEvent, InstructorApprovalEventKind,
};
pub use crate::teaching_operations::{
    AccommodationAdjustmentUpdateRequest, AccommodationAdjustmentView,
    AccommodationApplicationRuleView, AccountApprovalView, AssignmentPolicySource,
    CourseInvitationStateView, CourseInvitationTargetSearchPage, CourseInvitationTargetSearchQuery,
    CourseInvitationTargetSearchRequest, CourseInvitationTargetView,
    CourseInvitationTerminalAction, CourseInvitationTerminalActionRequest,
    CourseStudentMembershipsPage, InstructorApprovalStateView,
    InstructorCourseInvitationCreateRequest, InstructorCourseInvitationView,
    InstructorCourseInvitationsPage, InstructorMembershipRemovalRequest, InstructorMembershipView,
    InstructorMembershipsPage, MembershipPageRequest, PendingCourseInvitationView,
    PendingCourseInvitationsPage, RetentionActionOutcomeView, RetentionActionResponse,
    RetentionAdditionalDays, RetentionArchiveRequest, RetentionDispositionView,
    RetentionExtendRequest, RetentionNotificationIntentView, RetentionNotificationView,
    RetentionReadView, RetentionStateView, StudentMembershipView,
    SyntheticPreviewAccommodationAdjustmentRequest, SysadminInstructorApprovalStateView,
    SysadminInstructorApprovalView, SysadminInstructorCandidateSearchPage,
    SysadminInstructorCandidateSearchRequest, SysadminInstructorCandidateView, TeachingAccountView,
    TeachingAssignmentAttemptTimeLimitFieldPatch, TeachingAssignmentAttemptTimeLimitSeconds,
    TeachingAssignmentStartDecision, TeachingAttemptLimit, TeachingAttemptLimitFieldPatch,
    TeachingDisplayLabel, TeachingOperationRevision, TeachingOperationRevisionResponse,
    TeachingPageSize, TeachingPreviewAssignmentDeadlineRuleField, TeachingPreviewDenialReason,
    TeachingPreviewLateWorkRuleField, TeachingPreviewLimitField, TeachingPreviewTimeField,
    TeachingPreviewView, TeachingStudentLateWorkStatus, TeachingTimeFieldPatch,
    project_teaching_preview_time_field, resolve_teaching_local_time,
};
