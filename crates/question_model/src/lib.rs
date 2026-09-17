//! Browser-safe domain contracts for the Peptidyle Learning Engine.
//!
//! Question types, Student Work Records, backend capabilities, identity, and
//! Question metadata includes exact licensing and free-form tags.
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

/// Account-owned exact IANA time-zone preference.
pub mod account_time_zone;
pub mod answer;
/// Stable Assessment Entries, Question Pools, and exact point values.
pub mod assessment;
pub mod assessment_activity_rules;
pub mod assessment_student_time_accommodation;
/// Browser-safe, no-write Instructor Student View contracts.
pub mod assessment_student_view;
/// Instructor-owned reusable Assessment settings.
pub mod assessment_template;
/// Strict browser contracts and derived readiness for the Instructor assessment workspace.
pub mod assessment_workspace;
pub mod auth;
/// Generated closed registry for PLE-provided profile avatars.
pub mod avatar_catalog_generated;
/// Browser-safe reusable BlueprintCourse contracts.
pub mod blueprint_course;
/// Exact Blueprint operations, immutable evidence, and target-term schedule resolution.
pub mod blueprint_operations;
pub mod capability;
/// Course and assessment browser projections.
pub mod course;
/// Mandatory current classification for both Course forms.
pub mod course_classification;
pub use course_classification::{
    CourseClassification, CourseClassificationError, CourseMetadataEtag,
};
/// Closed, browser-safe course appearance and banner presentation contracts.
pub mod course_appearance;
/// Validated inclusive course-calendar bounds and authoritative IANA zone.
pub mod course_term;
/// Private Question Feedback and policy-redacted Student Feedback.
pub mod feedback;
pub mod generation;
/// Browser-safe automated-grading operation status and safe explanation contracts.
pub mod identity;
/// Browser-safe, no-store Instructor samples of saved Assessment Question Pools.
pub mod pool_preview;
/// Browser-safe, attempt-presentation-scoped question contracts.
pub mod presentation;
/// Strict non-mutating preview-plane contracts, separate from mutating Teaching Operations.
pub mod preview_plane;
/// Opaque role-neutral identity for a self-owned Account Profile image.
pub mod profile_image;
/// Human-facing route References that resolve to internal identities under authorization.
pub mod public_route;
/// Immutable browser-safe Question Authorship display records.
pub mod question_authorship;
mod question_backend_fields;
/// Optional browser-safe Question Citation records.
pub mod question_citation;
/// Question Content, Draft Question Content, and immutable Question Revisions.
pub mod question_content;
/// Shared Question Library metadata, visibility, lineage, and browse projections.
pub mod question_library;
pub mod question_license;
/// Browser-safe reusable published Question Pool library read models.
pub mod question_pool_library;
/// Immutable Question Revision acceptance facts.
pub mod question_revision;
mod question_search;
/// Browser-safe activity vocabulary for Published Question stewardship.
pub mod question_stewardship;
pub mod question_tag;
mod question_variation;
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
pub use crate::account_time_zone::{AccountTimeZone, AccountTimeZoneError};
pub use crate::assessment::{
    AssessmentAuthoredContent, AssessmentAuthoredContentFailureCode,
    AssessmentAuthoredContentFailureReason, AssessmentAuthoredContentField,
    AssessmentAuthoredContentLocalError, AssessmentAuthoredContentValidationFailure,
    AssessmentEditNumber, AssessmentEditNumberError, AssessmentEntry, AssessmentEntryAvailability,
    AssessmentEntryScoringRule, AssessmentInstructions, AssessmentInstructionsError,
    AssessmentOrigin, AssessmentPointValue, AssessmentScoringState, AssessmentStatus,
    AssessmentTitle, AssessmentTitleError, AssessmentType, BaseAssessmentPolicy,
    FixedQuestionAssessmentEntry, InstructorAssessmentAuthoredContentLocal,
    InstructorAssessmentAvailabilityView, LateWorkRule, LocalDateAndTime, LocalDateAndTimeError,
    MAX_ASSESSMENT_ATTEMPT_LIMIT, MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS,
    MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS, MAX_ASSESSMENT_ORDERED_ENTRIES,
    MAX_ASSESSMENT_QUESTION_POOL_ITEMS, MAX_ASSESSMENT_TITLE_UNICODE_SCALARS,
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY, PoolRevisionMemberReference,
    QuestionPoolAssessmentEntry, QuestionPoolRevisionNumber, QuestionPoolRevisionReference,
    QuestionPoolSelectedQuestionOrder, QuestionPoolSelectionRule, ScoringGeneration,
    derive_instructor_assessment_availability,
};
pub use crate::assessment_activity_rules::{
    AssessmentActivityRules, AssessmentQuestionOrderRule, AssessmentQuestionVariationRule,
    QuestionAttemptLimit, QuestionAttemptTimeLimit, StudentFeedbackReleaseRule,
    StudentFeedbackReleaseTiming,
};
pub use crate::assessment_student_time_accommodation::{
    AssessmentStudentTimeAccommodation, SaveAssessmentStudentTimeAccommodationInput,
};
pub use crate::assessment_student_view::{
    InstructorStudentView, InstructorStudentViewDelivery, InstructorStudentViewEntry,
    InstructorStudentViewNotShownReason, InstructorStudentViewQuestionReference,
};
pub use crate::assessment_template::{
    AssessmentTemplate, AssessmentTemplateEditNumber, AssessmentTemplateEditNumberError,
    AssessmentTemplateId, AssessmentTemplateName, AssessmentTemplateNameError,
    AssessmentTemplateSettings, AssessmentTemplateSettingsError,
    MAX_ASSESSMENT_TEMPLATE_NAME_UNICODE_SCALARS,
};
pub use crate::assessment_workspace::{
    AssessmentEntryRequest, AssessmentPoliciesValidationFailure,
    AssessmentPoliciesValidationFailureCode, AssessmentPoliciesValidationIssue,
    AssessmentReleaseIssue, AssessmentReleaseValidation, CreateAssessmentRequest,
    ReplaceAssessmentContentRequest, ReplaceAssessmentPoliciesRequest,
};
pub use crate::auth::{AccountId, ProductRole};
pub use crate::blueprint_course::canonical_exchange::{
    CanonicalBlueprintAssessment, CanonicalBlueprintAssessmentEntry, CanonicalBlueprintCourse,
    CanonicalBlueprintMetadata, CanonicalBlueprintModule,
};
pub use crate::blueprint_course::{
    BlueprintAssessmentContentInput, BlueprintAssessmentContentView, BlueprintAssessmentDefaults,
    BlueprintAssessmentEditChoice, BlueprintAssessmentEntryInput, BlueprintAssessmentEntryView,
    BlueprintAssessmentReference, BlueprintAssessmentReplacementInput, BlueprintChildIdError,
    BlueprintCourseAssessmentContentView, BlueprintCourseReadAccess, BlueprintCourseSummaryView,
    BlueprintCourseTitleError, BlueprintCourseValidationError, BlueprintCourseView,
    BlueprintModuleEditChoice, BlueprintModuleReference, BlueprintModuleReplacementInput,
    BlueprintModuleView, BlueprintPoolInputChoice, BlueprintRevision, CreateBlueprintCourseInput,
    CreateBlueprintModuleInput, MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS,
    ReplaceBlueprintCourseContentInput, ReusableFixedQuestionInput, ReusablePoolInput,
    ReusablePoolView, ReusableQuestionView, ReusableSelectionAvailability,
    validate_blueprint_course_title,
};
pub use crate::blueprint_operations::*;
pub use crate::capability::{
    BackendOwnedLifecycleState, BackendOwnedLifecycleStateError, Capability,
    QuestionBackendCapabilities,
};
pub use crate::course::{
    AssessmentEntrySummary, AssessmentOverview, AssessmentSummary, CourseInstanceRouteSummary,
    CourseMembershipRole, CourseSummary, FixedQuestionAssessmentEntrySummary, GradebookSummaryRow,
    QuestionPoolAssessmentEntrySummary, StudentAssessmentDelivery, StudentAssessmentDetail,
    StudentAssessmentLandingSummary, StudentLateWorkStatus,
};
pub use crate::course_appearance::{
    CourseAppearanceView, CourseBanner, CourseBannerAlternativeText, CourseBannerInformativeText,
    CourseBannerReference, CourseBannerRendition, CourseBannerUpdate, CourseBannerUploadReceipt,
    CourseBannerUploadReference, CourseTheme, CourseThemeUpdate,
};
pub use crate::course_term::{
    CourseDate, CourseDateError, CourseTerm, CourseTermError, CourseTermFailureCode,
    CourseTermFailureReason, CourseTermField, CourseTermValidationFailure,
};
pub use crate::feedback::{
    QuestionAnswer, QuestionAnswerExplanation, QuestionFeedback, QuestionHint, StudentFeedback,
    StudentResponseInspectionFeedback,
};
pub use crate::generation::{QuestionReproduction, QuestionSeed, QuestionSourceSelection};
pub use crate::identity::{
    ObjectId, QuestionAssetId, QuestionRevisionNumber, WorkspaceId, WorkspaceImportId,
};
pub use crate::pool_preview::{
    QuestionPoolPreview, QuestionPoolPreviewItem, QuestionPoolPreviewRequest,
};
pub use crate::presentation::{
    PresentationResponseItemReference, PresentedHotspotRegion, PresentedHotspotSurface,
    PresentedMatchingChoice, PresentedMatchingPrompt, PresentedOrderingItem,
    PresentedQuestionChoice, PresentedResponseItemContent, PresentedTextEntrySlot,
    QuestionAssetRendition, QuestionPresentation, QuestionPresentationBinding,
    QuestionPresentationChecksum, QuestionPresentationNonce, QuestionPresentationResponseFormat,
    QuestionPresentationToken, StudentAssessmentAttemptScreen,
    StudentAssessmentAttemptScreenAttempt, StudentAssessmentAttemptScreenScope,
    StudentAttemptDescriptor,
};
pub use crate::preview_plane::{
    ActiveStudentCourseMembershipDenialReason, ActiveStudentCourseMembershipGrantReason,
    ActiveStudentCourseMembershipOutcome, AssessmentPolicySourceKind,
    EffectiveAssessmentPolicyView, HypotheticalStudentViewScenarioModifiers,
    HypotheticalStudentViewScenarioRequest, InstructorPreviewSchedulePage,
    InstructorPreviewScheduleRow, PreviewAccommodationComparison, PreviewDeferredCapability,
    PreviewDenialReason, PreviewDisclosureFlags, PreviewDisclosureMoment,
    PreviewDisclosureUnavailableReason, PreviewEvaluation, PreviewFutureSeam,
    PreviewLateWorkRuleField, PreviewLimitField, PreviewPlaneResponse,
    PreviewPriorAssessmentAttemptCount, PreviewResolvedPolicy, PreviewSelectedMoment,
    PreviewTimeField, SelectedStudentViewScenarioRequest, StudentFeedbackReleaseView,
    StudentViewScenario, StudentViewScenarioAdmission, StudentViewScenarioOrigin,
};
pub use crate::profile_image::ProfileImageReference;
pub use crate::public_route::{
    AccountReference, AssessmentAttemptReference, AssessmentReference, AuthoringWorkspaceReference,
    BlueprintCourseReference, CourseInstanceReference, CourseInvitationReference,
    CourseMembershipReference, DraftQuestionReference, MAX_PUBLIC_ROUTE_NUMBER,
    NavigationResolution, RESERVED_REFERENCE_PREFIXES,
};
pub use crate::question_authorship::{
    QuestionAuthor, QuestionAuthorDisplayName, QuestionAuthorship, QuestionAuthorshipError,
};
pub use crate::question_backend_fields::{
    DraftImathasQuestionBackendBinding, ImathasDeploymentReference, ImathasItemReference,
    ImathasProfile, ImathasQuestionBackendBinding, ImathasQuestionBackendBindingError,
    MAX_IMATHAS_IDENTIFIER_BYTES, QuestionBackendFieldsError,
};
pub use crate::question_citation::{QuestionCitation, QuestionCitationError};
pub use crate::question_content::{
    DraftQuestionSummary, MAX_QUESTION_DESCRIPTION_UNICODE_SCALARS,
    MAX_QUESTION_TITLE_UNICODE_SCALARS, QuestionAssetReference, QuestionContentBlock,
    QuestionDescriptionError, QuestionFormat, QuestionMetadata, QuestionTitleError,
    validate_question_description, validate_question_title,
};
pub use crate::question_library::{
    CourseQuestionUse, MAX_BULK_QUESTION_METADATA_ITEMS, MAX_QUESTION_ID_COUNT,
    MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS, MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS,
    MAX_QUESTION_SEARCH_BACKEND_FACETS, MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES,
    MAX_QUESTION_SEARCH_OWN_COURSE_USAGES, MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS,
    MAX_QUESTION_SEARCH_QUESTION_TYPE_FILTERS, MAX_QUESTION_SEARCH_TAG_FACETS,
    MAX_QUESTION_SEARCH_TAG_FILTERS, PublishedQuestionSharedMetadata, QUESTION_ID_ALPHABET,
    QUESTION_ID_COMPACT_LENGTH, QUESTION_ID_IDENTIFIER_LENGTH, QuestionAvailability,
    QuestionAvailabilityEditNumber, QuestionAvailabilityEditNumberError, QuestionAvailabilityEvent,
    QuestionBackend, QuestionDetails, QuestionDetailsPromptView, QuestionId, QuestionLineageView,
    QuestionRevisionReference, QuestionSearchAuthorFacet, QuestionSearchAuthorship,
    QuestionSearchBackendFacet, QuestionSearchCapabilityFacet, QuestionSearchCourseUse,
    QuestionSearchCourseUseFacet, QuestionSearchFacets, QuestionSearchFilter, QuestionSearchPage,
    QuestionSearchQuestionLicenseFacet, QuestionSearchRequest, QuestionSearchRequestError,
    QuestionSearchResult, QuestionSearchSort, QuestionSearchSubjectFacet, QuestionSearchTagFacet,
    QuestionSearchTopicFacet, QuestionStatistics, QuestionSummary, QuestionTypeFacet,
    QuestionUseDetails, QuestionUseSummary, normalized_question_search_group_value,
};
pub use crate::question_license::QuestionLicense;
pub use crate::question_pool_library::{
    AssessmentQuestionPoolForkView, AssessmentQuestionPoolSelectionCountReceipt,
    QuestionPoolLibrarySummary, QuestionPoolMetadata, QuestionPoolRevisionMemberView,
    QuestionPoolRevisionView,
};
pub use crate::question_revision::{
    MAX_QUESTION_REVISION_REASON_UNICODE_SCALARS, QuestionRevisionReason,
};
pub use crate::question_stewardship::QuestionStewardshipEvent;
pub use crate::question_tag::Tag;
pub use crate::question_variation::{
    AuthorContentLibraryId, AuthorContentPresentation, NativeChoiceOrder, QuestionVariation,
    QuestionVariationPresentation,
};
pub use crate::response::{
    MAX_BACKEND_OWNED_PAYLOAD_BYTES, QuestionResponseControl, QuestionResponseFormat, QuestionType,
    StudentResponse,
};
pub use crate::statistics::{ClassStatistics, DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE};
pub use crate::student_work::{
    AccommodationId, AssessmentAttempt, AssessmentAttemptCompletion, AssessmentAttemptEvidence,
    AssessmentAttemptId, AssessmentAttemptPolicySource, AssessmentAttemptPolicySources,
    AssessmentEntryId, AssessmentGrade, AssessmentGradeScoreState, AssessmentId,
    AssessmentProgress, AssessmentProgressRecord, CourseId, CourseMembershipId, GradingResult,
    IssuedAttemptCapability, IssuedQuestion, IssuedQuestionId, QuestionAttempt, QuestionAttemptId,
    QuestionAttemptReproductionDetails, QuestionAttemptState, QuestionAttemptTiming,
    QuestionBackendVersion, QuestionEvaluation, QuestionEvaluationError, QuestionGraderVersion,
    QuestionPoolSelectedItem, QuestionPoolSelection, QuestionPoolSelectionId,
    QuestionRendererVersion, QuestionResponse, QuestionResponseId, RecordedCredit,
    SourceObjectChecksum, SourceObjectChecksumError, SourceObjectReference,
    StudentAssessmentAttemptPosition, StudentAssessmentAttemptProgress,
    StudentAssessmentAttemptResponseState, StudentAssessmentGrade, StudentAssessmentProgress,
    StudentQuestionAttemptView, StudentRecordId, Timestamp,
};
pub use crate::teaching_authority::{
    CourseInvitation, CourseInvitationEvent, CourseInvitationEventKind, CourseInvitationId,
    CourseInvitationState,
};
pub use crate::teaching_operations::{
    AccommodationAdjustmentView, AccommodationApplicationRuleView,
    CourseInvitationStatePrecondition, CourseInvitationStateView, CourseInvitationTerminalAction,
    CourseInvitationTerminalActionRequest, CourseRosterChangeNumber, PendingCourseInvitationView,
    PendingCourseInvitationsPage, TeachingAssessmentAttemptTimeLimitFieldPatch,
    TeachingAssessmentAttemptTimeLimitSeconds, TeachingAttemptLimit,
    TeachingAttemptLimitFieldPatch, TeachingDisplayLabel, TeachingTimeFieldPatch,
};
