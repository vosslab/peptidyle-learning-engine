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
/// Exact Blueprint operations, immutable evidence, and target-term schedule resolution.
pub mod blueprint_operations;
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
/// Answer-free Question Presentation contracts with server-held generator details.
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
/// Immutable browser-safe Question Authorship display records.
pub mod question_authorship;
/// Optional browser-safe Question Citation records.
pub mod question_citation;
/// Question Content, Draft Question Content, and immutable Question Revisions.
pub mod question_content;
/// Shared Question Library metadata, visibility, lineage, and browse projections.
pub mod question_library;
mod question_search;
pub mod response;
/// Browser-safe anonymous-statistics projections and disclosure policy.
pub mod statistics;
pub mod student_work;
/// Internal target-bound Course Invitation lifecycle facts.
pub mod teaching_authority;
/// Browser/server teaching-operations wire contracts.
pub mod teaching_operations;

// The crate's front door. These are the types a caller reaches for first, so
// they are re-exported to keep call sites short. Everything else stays
// available under its module.
pub use crate::assignment::{
    AssignmentAuthoredContent, AssignmentAuthoredContentFailureCode,
    AssignmentAuthoredContentFailureReason, AssignmentAuthoredContentField,
    AssignmentAuthoredContentLocalError, AssignmentAuthoredContentValidationFailure,
    AssignmentDeadlineRule, AssignmentEditNumber, AssignmentEntry, AssignmentEntryAvailability,
    AssignmentEntryScoringRule, AssignmentInstructions, AssignmentInstructionsError,
    AssignmentPointValue, AssignmentRevisionNumber, AssignmentRevisionNumberError,
    AssignmentScoringState, AssignmentStatus, AssignmentTitle, AssignmentTitleError,
    BaseAssignmentPolicy, CourseLocalDateAndTime, CourseLocalDateAndTimeError,
    FixedQuestionAssignmentEntry, InstructorAssignmentAuthoredContentLocal,
    InstructorAssignmentAvailabilityView, LateWorkRule, MAX_ASSIGNMENT_ATTEMPT_LIMIT,
    MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS, MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS,
    MAX_ASSIGNMENT_ORDERED_ENTRIES, MAX_ASSIGNMENT_QUESTION_POOL_ITEMS,
    MAX_ASSIGNMENT_TITLE_UNICODE_SCALARS, MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY,
    QuestionPoolAssignmentEntry, QuestionPoolItem, QuestionPoolItemAvailability,
    QuestionPoolSelectedQuestionOrder, QuestionPoolSelectionRule, ScoringGeneration,
    derive_instructor_assignment_availability,
};
pub use crate::assignment_activity_rules::{
    AssignmentActivityRules, AssignmentAttemptContinuationRule, AssignmentAttemptGradeRule,
    AssignmentAttemptResumeRule, AssignmentCompletionRule, AssignmentNavigationRule,
    AssignmentQuestionDisplayRule, AssignmentQuestionOrderRule, AssignmentQuestionVariationRule,
    QuestionAttemptLimit, QuestionPoolPreviewNonce, QuestionPoolReuseRule,
    QuestionPoolSelectionInputs, StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming,
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
    BlueprintAssignmentContentInput, BlueprintAssignmentContentView, BlueprintAssignmentDefaults,
    BlueprintAssignmentEditHandle, BlueprintAssignmentEntryInput, BlueprintAssignmentEntryView,
    BlueprintAssignmentId, BlueprintChildIdError, BlueprintCourseAccess,
    BlueprintCourseAssignmentContentView, BlueprintCourseAssignmentReplacementInput,
    BlueprintCourseModuleReplacementInput, BlueprintCourseModuleView, BlueprintCourseSummaryView,
    BlueprintCourseTitleError, BlueprintCourseValidationError, BlueprintCourseView,
    BlueprintModuleEditHandle, BlueprintModuleId, BlueprintRevision,
    CreateBlueprintCourseContentInput, CreateBlueprintCourseModuleInput, LocalTimeOfDay,
    LocalTimeOfDayError, MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS, RelativeAssignmentSchedule,
    RelativeAssignmentScheduleMoment, ReplaceBlueprintCourseContentInput,
    ReusableFixedQuestionInput, ReusablePoolEntryView, ReusablePoolInput, ReusablePoolView,
    ReusableQuestionView, ReusableSelectionAvailability, validate_blueprint_course_title,
};
pub use crate::blueprint_operations::*;
pub use crate::capability::{Capability, QuestionBackendCapabilities};
pub use crate::course::{
    AssignmentEntrySummary, AssignmentOverview, AssignmentSummary, CourseMembershipRole,
    CourseSummary, FixedQuestionAssignmentEntrySummary, GradebookSummaryRow,
    QuestionPoolAssignmentEntrySummary, QuestionPoolItemSummary, StudentAssignmentDelivery,
    StudentAssignmentDetail, StudentAssignmentLandingSummary, StudentLateWorkStatus,
};
pub use crate::course_appearance::{
    CourseAppearanceRevision, CourseAppearanceUpdate, CourseAppearanceView, CourseBanner,
    CourseBannerAlternativeText, CourseBannerInformativeText, CourseBannerMutation,
    CourseBannerReference, CourseBannerUploadReceipt, CourseBannerUploadReference, CourseThemeId,
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
pub use crate::envelope::{QuestionVariation, QuestionVariationPresentation};
pub use crate::feedback::{
    QuestionAnswer, QuestionAnswerExplanation, QuestionFeedback, QuestionHint, StudentFeedback,
    StudentResponseInspectionFeedback,
};
pub use crate::generation::QuestionGeneratorReference;
pub use crate::grading_operations::{
    GradingOperationAction, GradingOperationReason, GradingOperationVisibleState,
    InstructorGradingOperationActionRequest, InstructorGradingOperationReceipt,
    InstructorGradingOperationReplay, InstructorGradingOperationReplayError,
    InstructorGradingOperationReplayLedger, InstructorGradingOperationRequestChecksum,
    InstructorGradingOperationRetryToken, InstructorGradingOperationRetryTokenError,
    InstructorGradingOperationState, QuestionSubmissionGradingState,
    StudentQuestionSubmissionGradingState,
};
pub use crate::identity::{
    ObjectId, QuestionAssetId, QuestionRevisionNumber, WorkspaceId, WorkspaceImportId,
};
pub use crate::pool_preview::{
    QuestionPoolPreview, QuestionPoolPreviewQuestion, QuestionPoolPreviewRequest,
};
pub use crate::presentation::{
    PresentationResponseItemReference, PresentedHotspotRegion, PresentedHotspotSurface,
    PresentedMatchingChoice, PresentedMatchingPrompt, PresentedOrderingItem,
    PresentedQuestionChoice, PresentedResponseItemContent, PresentedTextEntrySlot,
    QuestionAssetRendition, QuestionPresentation, QuestionPresentationBinding,
    QuestionPresentationChecksum, QuestionPresentationNonce, QuestionPresentationResponseFormat,
    QuestionPresentationToken, StudentAssignmentAttemptScreen,
    StudentAssignmentAttemptScreenAttempt, StudentAssignmentAttemptScreenScope,
    StudentAttemptDescriptor,
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
    CourseMembershipReference, DraftQuestionReference, InstructorGradingOperationReference,
    MAX_PUBLIC_ROUTE_NUMBER, NavigationResolution, QuestionFolderReference,
    RESERVED_REFERENCE_PREFIXES, SavedQuestionSearchReference,
};
pub use crate::question_authorship::{
    QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship, QuestionAuthorshipError,
};
pub use crate::question_citation::{QuestionCitation, QuestionCitationError};
pub use crate::question_content::{
    DraftImathasQuestionBackendBinding, DraftQuestionBackendLocator, DraftQuestionContent,
    DraftQuestionSummary, ImathasDeploymentReference, ImathasItemReference, ImathasProfile,
    ImathasQuestionBackendBinding, ImathasQuestionBackendBindingError,
    MAX_IMATHAS_IDENTIFIER_BYTES, MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS,
    MAX_QUESTION_TITLE_UNICODE_SCALARS, QuestionBackendLocator,
    QuestionBackendLocatorPreparationError, QuestionDescriptionError, QuestionFormat,
    QuestionGradingRule, QuestionMetadata, QuestionRevision, QuestionTitleError,
    validate_question_description, validate_question_title,
};
pub use crate::question_library::{
    CourseQuestionUse, MAX_QUESTION_ID_COUNT, MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS,
    MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS, MAX_QUESTION_SEARCH_BACKEND_FACETS,
    MAX_QUESTION_SEARCH_CLASSIFICATION_FACETS, MAX_QUESTION_SEARCH_OWN_COURSE_USAGES,
    MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS, MAX_QUESTION_SEARCH_QUESTION_TYPE_FILTERS,
    MAX_QUESTION_SEARCH_TAG_FACETS, MAX_QUESTION_SEARCH_TAG_FILTERS, QUESTION_ID_ALPHABET,
    QUESTION_ID_COMPACT_LENGTH, QUESTION_ID_IDENTIFIER_LENGTH, QuestionBackend, QuestionDetails,
    QuestionId, QuestionPromptProjection, QuestionRevisionAvailability, QuestionRevisionReference,
    QuestionSearchAuthorFacet, QuestionSearchAuthorship, QuestionSearchBackendFacet,
    QuestionSearchCapabilityFacet, QuestionSearchClassificationFacet,
    QuestionSearchClassificationFilter, QuestionSearchCourseUse, QuestionSearchCourseUseFacet,
    QuestionSearchFacets, QuestionSearchFilter, QuestionSearchPage,
    QuestionSearchQuestionLicenseFacet, QuestionSearchRequest, QuestionSearchRequestError,
    QuestionSearchResult, QuestionSearchTagFacet, QuestionStatistics,
    QuestionStatisticsAvailability, QuestionStatisticsAvailabilityFacet, QuestionSummary,
    QuestionTypeFacet, QuestionUseDetails, QuestionUseSummary,
};
pub use crate::response::{
    QuestionResponseControl, QuestionResponseFormat, QuestionType, StudentResponse,
};
pub use crate::statistics::{
    DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE, QuestionStatisticsDisclosure, QuestionStatisticsView,
    StatisticsDisclosurePolicy, StatisticsDisclosurePolicyError, StudentClassStatistics,
};
pub use crate::student_work::{
    AccommodationId, AssignmentAttempt, AssignmentAttemptCompletion, AssignmentAttemptId,
    AssignmentEntryId, AssignmentGrade, AssignmentId, AssignmentProgress, AssignmentProgressRecord,
    AssignmentProgressScoreState, CourseId, CourseMembershipId, GradingResult,
    IssuedAttemptCapability, IssuedQuestion, IssuedQuestionId, QuestionAttempt, QuestionAttemptId,
    QuestionAttemptReproductionDetails, QuestionAttemptState, QuestionAttemptTiming,
    QuestionBackendVersion, QuestionGraderVersion, QuestionPoolItemId, QuestionPoolSelectedItem,
    QuestionPoolSelection, QuestionPoolSelectionId, QuestionPoolSelectionReuseError,
    QuestionRendererVersion, QuestionSubmission, QuestionSubmissionId, SourceObjectChecksum,
    SourceObjectChecksumError, SourceObjectReference, StudentQuestionAttemptView, StudentRecordId,
    Timestamp,
};
pub use crate::teaching_authority::{
    CourseInvitation, CourseInvitationEvent, CourseInvitationEventKind, CourseInvitationId,
    CourseInvitationState,
};
pub use crate::teaching_operations::{
    AccommodationAdjustmentUpdateRequest, AccommodationAdjustmentView,
    AccommodationApplicationRuleView, AssignmentPolicySource, CourseInvitationStateView,
    CourseInvitationTargetSearchPage, CourseInvitationTargetSearchRequest,
    CourseInvitationTargetView, CourseInvitationTerminalAction,
    CourseInvitationTerminalActionRequest, CourseStudentMembershipsPage,
    InstructorCourseInvitationCreateRequest, InstructorCourseInvitationView,
    InstructorCourseInvitationsPage, InstructorMembershipRemovalRequest, InstructorMembershipView,
    InstructorMembershipsPage, MembershipPageRequest, PendingCourseInvitationView,
    PendingCourseInvitationsPage, RetentionActionOutcomeView, RetentionActionResponse,
    RetentionAdditionalDays, RetentionArchiveRequest, RetentionDispositionView,
    RetentionExtendRequest, RetentionNotificationIntentView, RetentionNotificationView,
    RetentionReadView, RetentionStateView, StudentMembershipView,
    SyntheticPreviewAccommodationAdjustmentRequest, TeachingAccountSearchQuery,
    TeachingAccountView, TeachingAssignmentAttemptTimeLimitFieldPatch,
    TeachingAssignmentAttemptTimeLimitSeconds, TeachingAssignmentStartDecision,
    TeachingAttemptLimit, TeachingAttemptLimitFieldPatch, TeachingDisplayLabel,
    TeachingOperationRevision, TeachingOperationRevisionResponse, TeachingPageSize,
    TeachingPreviewAssignmentDeadlineRuleField, TeachingPreviewDenialReason,
    TeachingPreviewLateWorkRuleField, TeachingPreviewLimitField, TeachingPreviewTimeField,
    TeachingPreviewView, TeachingStudentLateWorkStatus, TeachingTimeFieldPatch,
    project_teaching_preview_time_field, resolve_teaching_local_time,
};
