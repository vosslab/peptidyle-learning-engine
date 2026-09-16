//! Focused persistence foundations for the clean single-installation baseline.
//!
//! Product adapters are added only with an exact account, course-membership,
//! Student-ownership, workspace, observer-grant, or worker-lease contract.

use domain::assessment_activity::AssessmentActivityError;

mod account_avatar;
mod account_time_zone;
mod assessment_attempt;
mod assessment_delivery;
mod assessment_pool_fork;
mod assessment_pool_selection_count;
mod assessment_release;
mod assessment_student_view;
mod assessment_template;
mod attempt_expiry;
mod authentication_ceremony;
mod authentication_email;
mod authoring;
mod blueprint_course;
mod blueprint_history;
mod blueprint_lineage;
mod blueprint_stewardship;
mod course_banner;
mod course_instance;
mod course_roster;
mod course_theme;
mod imathas_question_backend_session;
mod instructor_account;
mod invitation_export;
mod live_gradebook;
mod live_student_course_landing;
mod object_record;
mod pagination;
pub mod postgres;
mod public_asset_publication;
mod question_asset_delivery;
mod question_bulk_metadata;
mod question_fork;
mod question_library;
mod question_pool_creation;
mod question_pool_library;
mod question_source;
mod question_star;
mod question_watch;
mod question_watch_notification;
mod random_uuid;
mod retention;
mod retention_notification;
pub mod session;
#[path = "contracts/store_error.rs"]
mod store_error;
mod support_capability;

pub use account_avatar::{
    AccountAvatar, AccountAvatarStore, AccountProfileImageDeleteWork, FinalizedAccountProfileImage,
    PreparedAccountProfileImage, ProfileImageReference, ProvidedAvatarId,
    SelectableProvidedAvatarId,
};
pub use account_time_zone::AccountTimeZoneStore;
pub use assessment_attempt::{
    AssessmentAttemptStart, AssessmentAttemptStartResult, AssessmentAttemptStore,
    PreparedIssuedQuestion, PreparedQuestionPoolSelection,
};
pub use assessment_delivery::{
    IssuedQuestionPresentation, LiveAssessmentAccess, LiveAssessmentAttempt,
    LiveAssessmentAttemptScore, LiveAssessmentDeliveryStore, LiveAssessmentPreviousAttempt,
    LiveAssessmentPreviousAttemptState, NativeAssessmentIssuanceBatch, NativePleIssuanceSource,
    NativePresentationInput, NativeWebworkIssuanceSource, QuestionIssuanceReproductionInput,
    ReadyQuestionAssetRendition, StudentAssessmentAttemptBackendDocument,
    StudentAssessmentAttemptBackendDocumentResume, StudentAssessmentAttemptContext,
    StudentAssessmentAttemptFinalization, StudentAssessmentAttemptFinalizationBackend,
    StudentAssessmentAttemptFinalizationEvaluation, StudentAssessmentAttemptFinalizationKind,
    StudentAssessmentAttemptFinalizationPreparation,
    StudentAssessmentAttemptFinalizationPreparationOutcome,
    StudentAssessmentAttemptFinalizationSource, StudentAssessmentAttemptHistory,
    StudentAssessmentAttemptHistoryAssessment, StudentAssessmentAttemptHistoryCourse,
    StudentAssessmentAttemptHistoryEvidence, StudentAssessmentAttemptHistoryQuestion,
    StudentAssessmentAttemptHistoryResponseSource, StudentAssessmentAttemptPresentationEvidence,
    StudentAssessmentAttemptPresentationSource, StudentAssessmentAttemptSavedResponse,
};
pub use assessment_pool_fork::{
    AppendAssessmentPoolForkRevisionInput, AppendedAssessmentPoolForkRevision,
    AssessmentPoolForkStore, ImportAssessmentPoolForkInput, ImportedAssessmentPoolFork,
};
pub use assessment_pool_selection_count::{
    AssessmentPoolSelectionCountInput, AssessmentPoolSelectionCountStore,
};
pub use assessment_release::{
    ApplyAssessmentBlueprintUpdateInput, AssessmentBlueprintUpdateCannotApplyReason,
    AssessmentBlueprintUpdateContent, AssessmentBlueprintUpdateEntry,
    AssessmentBlueprintUpdateReview, AssessmentQuestionPickerEntry, AssessmentReleaseIssue,
    AssessmentReleaseValidation, AssessmentUnreleaseImpact, AuthoredAssessmentQuestion,
    CourseAssessmentBlueprintUpdateSummary, CourseAssessmentSummary, CourseBlueprintUpdateReview,
    CreateLiveAssessmentInput, DueSoonAssessmentSummary, DueSoonAssessments, LiveAssessmentStore,
    LiveAssessmentWorkspace, SaveBaseAssessmentPolicyInput, SaveLiveAssessmentInlineInput,
    SaveLiveAssessmentInput, UnreleasedLiveAssessment,
};
pub use assessment_student_view::{
    InstructorStudentViewSnapshot, InstructorStudentViewSnapshotEntry, InstructorStudentViewSource,
    InstructorStudentViewStore,
};
pub use assessment_template::{
    AssessmentTemplateStore, CreateAssessmentFromTemplateInput, SaveAssessmentTemplateInput,
};
pub use attempt_expiry::{
    AssessmentAttemptExpirySweepStore, ExpiredAssessmentAttemptFinalizationPreparation,
};
pub use authentication_ceremony::{
    AuthenticatedAccount, AuthenticationCeremonyLifetime, AuthenticationCeremonyStore,
    AuthenticationSecretHash, EmailAuthenticationChallenge, EmailAuthenticationChallengeId,
    EmailAuthenticationPurpose, MAX_AUTHENTICATION_CEREMONY_SECONDS, Passkey, PasskeyCeremonyId,
    PasskeyId, PendingSysadminTotpAttestation, SysadminTotpAttestationId, SysadminTotpCounter,
    SysadminTotpSeed, SysadminTotpStore, SysadminTotpVerificationReservation,
};
pub use authentication_email::{
    AuthenticationEmail, AuthenticationEmailError, EmailDomain, MAX_AUTHENTICATION_EMAIL_BYTES,
};
pub use authoring::{
    AuthoringDraft, AuthoringDraftStore, AuthoringDraftSummary, CreateAuthoringDraftInput,
    DeleteAuthoringDraftInput, SaveAuthoringDraftGeneralFeedbackInput, SaveAuthoringDraftInput,
};
pub use blueprint_course::{
    ApplyBlueprintForkInput, ApplyBlueprintForkResult, BlueprintCourseStore,
    StoredBlueprintAssessment, StoredBlueprintAssessmentContent, StoredBlueprintAssessmentEntry,
    StoredBlueprintCourse, StoredBlueprintCourseContent, StoredBlueprintCourseSummary,
    StoredBlueprintModule, StoredBlueprintPoolMembers, StoredBlueprintRevision,
};
pub use blueprint_history::{BlueprintHistoryKind, BlueprintHistoryStore};
pub use blueprint_lineage::{
    BlueprintComparisonSources, BlueprintForkSource, BlueprintLineageStore,
    ForkBlueprintCourseReceipt, StoredKnownBlueprintFork,
};
pub use blueprint_stewardship::{
    BlueprintCourseStarProjection, BlueprintCourseStarredInstructor, BlueprintCourseWatchEvent,
    BlueprintCourseWatchEventKind, BlueprintCourseWatchProjection, BlueprintStewardshipState,
    BlueprintStewardshipStore,
};
pub use browser_api_contract::student_assessment_decision::{
    AssessmentStartDecision, StudentAssessmentDecisionSummary,
};
pub use course_banner::{
    ClaimedCourseBannerUpload, CourseBannerDeleteWork, CourseBannerObjectMetadata,
    CourseBannerStorageCheckResult, CourseBannerStore, FinalizedCourseBannerPromotion,
    PrepareCourseBannerPromotion, PreparedCourseBannerPromotion, PreparedCourseBannerRemoval,
    StageCourseBannerUpload, StagedCourseBannerUpload,
};
pub use course_instance::{
    CourseCreationInstructor, CourseInstanceCreationSource, CourseInstancePoolIdIssuer,
    CourseInstanceStore, CourseInstanceSummary, CourseInstanceView, CreateCourseInstanceInput,
    CreatedCourseInstance,
};
pub use course_roster::{
    ClaimedCourseInvitation, CourseRosterEntry, CourseRosterEntryState, CourseRosterImportEntry,
    CourseRosterImportInput, CourseRosterStore,
};
pub use course_theme::CourseThemeStore;
pub use imathas_question_backend_session::{
    ImathasGradingContext, ImathasLaunchBindingChecksum, ImathasNormalizedScore,
    ImathasQuestionBackendLaunchPreparationValidation, ImathasQuestionBackendSession,
    ImathasQuestionBackendSessionAuthentication, ImathasQuestionBackendSessionChallenge,
    ImathasQuestionBackendSessionCreate, ImathasQuestionBackendSessionPreparationContext,
    ImathasQuestionBackendSessionReference, ImathasQuestionBackendSessionRestoreExpectation,
    ImathasQuestionBackendSessionStore, ImathasQuestionBackendSessionValidation,
    ImathasQuestionBackendStateCipher, ImathasQuestionBackendStateKeyId,
    ImathasQuestionBackendStateKeyRing, ImathasQuestionBackendStatePlaintext,
    ImathasResponseChecksum, ImathasResult, ImathasResultToken, ImathasResultTokenChecksum,
    LoadedImathasQuestionBackendSession, MAX_IMATHAS_QUESTION_BACKEND_STATE_CIPHERTEXT_BYTES,
    MAX_IMATHAS_QUESTION_BACKEND_STATE_PLAINTEXT_BYTES, MemoryImathasQuestionBackendSessionStore,
    derive_imathas_question_backend_evaluation,
};
#[allow(unused_imports)] // Crate-private PostgreSQL Store row-binding surface.
pub(crate) use imathas_question_backend_session::{
    ImathasQuestionBackendSessionCreateParts, ImathasQuestionBackendSessionRestoreParts,
    ImathasQuestionBackendSessionStorageParts, ImathasQuestionBackendStateCipherStorageParts,
};
pub use instructor_account::{
    CompleteInstructorIdentityVettingInput, CreateInstructorAccountInput,
    DeactivateInstructorAccountInput, InstructorAccountList, InstructorAccountState,
    InstructorAccountStore, InstructorAccountSummary, InstructorIdentityVettingDecisionReference,
};
pub use invitation_export::{
    InvitationExportStore, InvitationMailerExport, InvitationMailerRecipient,
    PendingInvitationExport, PendingInvitationRecipient,
};
pub use live_gradebook::{CourseGradebook, CourseGradebookStore, CourseGradebookStudentWork};
pub use live_student_course_landing::{
    LiveAssessmentGradeContribution, LiveStudentAssessmentLandingSummary,
    LiveStudentCourseInvitationSummary, LiveStudentCourseLandingStore,
    LiveStudentCourseLandingSummary,
};
pub use object_record::{
    WorkspaceQuestionSourceObjectRecordStore, validate_workspace_question_source_object_record,
};
pub use pagination::{Cursor, Page, PageRequest, PageSize, PaginationError};
pub use public_asset_publication::{ClaimedQuestionAssetPublication, PublicAssetPublicationStore};
pub use question_asset_delivery::{QuestionAssetDeliveryStore, ReadyQuestionAssetDelivery};
pub use question_bulk_metadata::{
    BulkPublishedQuestionMetadataInput, BulkPublishedQuestionMetadataPatch,
    BulkPublishedQuestionMetadataResult, BulkPublishedQuestionMetadataSelection,
    BulkPublishedQuestionMetadataStore,
};
pub use question_fork::{
    ForkPublishedQuestionError, ForkPublishedQuestionInput, ForkedPublishedQuestionDraft,
    QuestionForkStore,
};
pub use question_library::{
    PublishedQuestionAvailability, PublishedQuestionLibraryEntry, QuestionLibraryStore,
};
pub use question_pool_creation::{
    CreateQuestionPoolError, CreateQuestionPoolInput, CreatedQuestionPool,
    QuestionPoolCreationStore,
};
pub use question_pool_library::{
    AssessmentQuestionPoolForkRecord, PublishedQuestionPoolRevision, QuestionPoolLibraryStore,
};
pub use question_source::{
    DraftQuestionEditNumber, DraftQuestionPublicationSourceStore, DraftQuestionSourceBindingInput,
    DraftQuestionSourceBindingStore, DraftQuestionUuid, ExistingQuestionRevisionPublicationError,
    ExistingQuestionRevisionPublicationInput, ExistingQuestionRevisionPublicationStore,
    NewQuestionLineagePublicationError, NewQuestionLineagePublicationInput,
    NewQuestionLineagePublicationStore,
};
pub use question_star::{QuestionStarProjection, QuestionStarStore, QuestionStarredInstructor};
pub use question_watch::{QuestionWatchProjection, QuestionWatchStore};
pub use question_watch_notification::QuestionWatchNotificationStore;
pub use retention::{CourseRetentionDueAction, CourseRetentionDueActionKind, CourseRetentionStore};
pub use retention_notification::{
    ClaimedCourseRetentionNotification, CourseRetentionNotificationFailure,
    CourseRetentionNotificationStore, VerifiedCourseRetentionNotificationDestination,
};
pub use session::{
    SessionId, SessionLifetime, SessionRecord, SessionStore, SessionTokenHash,
    SessionTokenHashParseError,
};
pub use store_error::StoreError;
pub use support_capability::{
    IssueSupportRepairCapabilityInput, SupportRepairCapabilityReceipt,
    SupportRepairCapabilityStore, SupportRepairCapabilityUseReceipt, SupportRepairResourceClass,
};
