//! Focused persistence foundations for the clean single-installation baseline.
//!
//! Product adapters are added only with an exact account, course-membership,
//! Student-ownership, workspace, observer-grant, or worker-lease contract.

use domain::assignment_activity::AssignmentActivityError;

mod account_time_zone;
mod assignment_attempt;
mod assignment_delivery;
mod assignment_release;
mod attempt_expiry;
mod authentication_ceremony;
mod authentication_email;
mod authoring;
mod blueprint_course;
mod course_banner;
mod course_instance;
mod course_roster;
mod course_theme;
mod imathas_question_backend_session;
mod instructor_account;
mod instructor_profile;
mod invitation_export;
mod live_gradebook;
mod live_student_course_landing;
mod object_record;
mod pagination;
pub mod postgres;
mod profile_thumbnail;
mod public_asset_publication;
mod question_asset_delivery;
mod question_library;
mod question_source;
mod random_uuid;
pub mod session;
#[path = "contracts/store_error.rs"]
mod store_error;
mod support_capability;

pub use account_time_zone::AccountTimeZoneStore;
pub use assignment_attempt::{
    AssignmentAttemptStart, AssignmentAttemptStartResult, AssignmentAttemptStore,
    PreparedIssuedQuestion, PreparedQuestionPoolSelection,
};
pub use assignment_delivery::{
    IssuedQuestionPresentation, LiveAssignmentAccess, LiveAssignmentAttempt,
    LiveAssignmentAttemptScore, LiveAssignmentDeliveryStore, LiveAssignmentPreviousAttempt,
    LiveAssignmentPreviousAttemptState, NativeAssignmentIssuanceBatch, NativePleIssuanceSource,
    NativePresentationInput, NativeWebworkIssuanceSource, ReadyQuestionAssetRendition,
    StudentAssignmentAttemptBackendDocument, StudentAssignmentAttemptBackendDocumentResume,
    StudentAssignmentAttemptContext, StudentAssignmentAttemptFinalization,
    StudentAssignmentAttemptFinalizationBackend, StudentAssignmentAttemptFinalizationEvaluation,
    StudentAssignmentAttemptFinalizationKind, StudentAssignmentAttemptFinalizationPreparation,
    StudentAssignmentAttemptFinalizationPreparationOutcome,
    StudentAssignmentAttemptFinalizationSource, StudentAssignmentAttemptHistory,
    StudentAssignmentAttemptHistoryAssignment, StudentAssignmentAttemptHistoryCourse,
    StudentAssignmentAttemptHistoryEvidence, StudentAssignmentAttemptHistoryQuestion,
    StudentAssignmentAttemptHistoryResponseSource, StudentAssignmentAttemptPresentationEvidence,
    StudentAssignmentAttemptPresentationSource, StudentAssignmentAttemptSavedResponse,
};
pub use assignment_release::{
    AssignmentPreview, AssignmentQuestionPickerEntry, AssignmentReleaseIssue,
    AssignmentReleaseValidation, AssignmentUnreleaseImpact, AuthoredAssignmentQuestion,
    CourseAssignmentSourceChoice, CourseAssignmentSummary, CreateLiveAssignmentInput,
    DueSoonAssignmentSummary, DueSoonAssignments, LiveAssignmentStore, LiveAssignmentWorkspace,
    SaveBaseAssignmentPolicyInput, SaveLiveAssignmentInlineInput, SaveLiveAssignmentInput,
    UnreleasedLiveAssignment,
};
pub use attempt_expiry::{
    AssignmentAttemptExpirySweepStore, ExpiredAssignmentAttemptFinalizationPreparation,
};
pub use authentication_ceremony::{
    AuthenticatedAccount, AuthenticationCeremonyLifetime, AuthenticationCeremonyStore,
    AuthenticationSecretHash, EmailAuthenticationChallenge, EmailAuthenticationChallengeId,
    EmailAuthenticationPurpose, MAX_AUTHENTICATION_CEREMONY_SECONDS, Passkey, PasskeyCeremonyId,
    PasskeyId,
};
pub use authentication_email::{
    AuthenticationEmail, AuthenticationEmailError, EmailDomain, MAX_AUTHENTICATION_EMAIL_BYTES,
};
pub use authoring::{
    AuthoringDraft, AuthoringDraftStore, AuthoringDraftSummary, CreateAuthoringDraftInput,
    SaveAuthoringDraftInput,
};
pub use blueprint_course::{
    BlueprintCourseStore, StoredBlueprintAssignment, StoredBlueprintAssignmentContent,
    StoredBlueprintAssignmentEntry, StoredBlueprintCourse, StoredBlueprintCourseContent,
    StoredBlueprintCourseSummary, StoredBlueprintModule, StoredBlueprintRevision,
};
pub use browser_api_contract::student_assignment_decision::{
    AssignmentStartDecision, StudentAssignmentDecisionSummary,
};
pub use course_banner::{
    ClaimedCourseBannerUpload, CourseBannerDeleteWork, CourseBannerObjectMetadata,
    CourseBannerStorageCheckResult, CourseBannerStore, FinalizedCourseBannerPromotion,
    PrepareCourseBannerPromotion, PreparedCourseBannerPromotion, PreparedCourseBannerRemoval,
    StageCourseBannerUpload, StagedCourseBannerUpload,
};
pub use course_instance::{
    CourseCreationInstructor, CourseInstanceStore, CourseInstanceSummary, CourseInstanceView,
    CreateCourseInstanceInput, CreatedCourseInstance,
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
    CreateInstructorAccountInput, DeactivateInstructorAccountInput, InstructorAccountList,
    InstructorAccountState, InstructorAccountStore, InstructorAccountSummary,
};
pub use instructor_profile::{
    InstructorProfile, InstructorProfileStore, UpdateInstructorProfileInput,
};
pub use invitation_export::{
    InvitationExportStore, InvitationMailerExport, InvitationMailerRecipient,
    PendingInvitationExport, PendingInvitationRecipient,
};
pub use live_gradebook::{CourseGradebook, CourseGradebookStore, CourseGradebookStudentWork};
pub use live_student_course_landing::{
    LiveStudentAssignmentLandingSummary, LiveStudentCourseInvitationSummary,
    LiveStudentCourseLandingStore, LiveStudentCourseLandingSummary,
};
pub use object_record::{
    WorkspaceQuestionSourceObjectRecordStore, validate_workspace_question_source_object_record,
};
pub use pagination::{Cursor, Page, PageRequest, PageSize, PaginationError};
pub use profile_thumbnail::{
    FinalizedProfileThumbnail, PreparedProfileThumbnail, ProfileThumbnailDeleteWork,
    ProfileThumbnailStore,
};
pub use public_asset_publication::{ClaimedQuestionAssetPublication, PublicAssetPublicationStore};
pub use question_asset_delivery::{QuestionAssetDeliveryStore, ReadyQuestionAssetDelivery};
pub use question_library::{
    PublishedQuestionAvailability, PublishedQuestionLibraryEntry, QuestionLibraryStore,
};
pub use question_source::{
    DraftQuestionEditNumber, DraftQuestionPublicationSourceStore, DraftQuestionSourceBindingInput,
    DraftQuestionSourceBindingStore, DraftQuestionUuid, ExistingQuestionRevisionPublicationError,
    ExistingQuestionRevisionPublicationInput, ExistingQuestionRevisionPublicationStore,
    NewQuestionLineagePublicationInput, NewQuestionLineagePublicationStore,
};
pub use session::{
    SessionId, SessionLifetime, SessionRecord, SessionStore, SessionTokenHash,
    SessionTokenHashParseError,
};
pub use store_error::StoreError;
pub use support_capability::{
    IssueSupportCapabilityInput, SupportCapabilityReceipt, SupportCapabilityStore,
    SupportMinimumProjection, SupportOperationKind,
};
