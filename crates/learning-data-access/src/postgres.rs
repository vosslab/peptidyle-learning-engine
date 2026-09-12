//! PostgreSQL connection and migration administration for the clean baseline.
//!
//! Feature adapters return only after their exact clean-schema contracts exist.

#[cfg(feature = "postgres")]
mod account_time_zone;
#[cfg(feature = "postgres")]
mod assignment_attempt;
#[cfg(feature = "postgres")]
mod assignment_attempt_context;
#[cfg(feature = "postgres")]
mod assignment_delivery;
#[cfg(feature = "postgres")]
mod assignment_delivery_access;
#[cfg(feature = "postgres")]
mod assignment_delivery_history;
#[cfg(feature = "postgres")]
mod assignment_delivery_history_response;
#[cfg(feature = "postgres")]
mod assignment_delivery_source;
#[cfg(feature = "postgres")]
mod assignment_delivery_start;
#[cfg(feature = "postgres")]
mod assignment_release;
#[cfg(feature = "postgres")]
mod assignment_workspace_policy;
#[cfg(feature = "postgres")]
mod assignment_workspace_save;
#[cfg(feature = "postgres")]
mod authoring;
#[cfg(feature = "postgres")]
mod blueprint_course;
#[cfg(feature = "postgres")]
mod connection;
#[cfg(feature = "postgres")]
mod course_banner;
#[cfg(feature = "postgres")]
mod course_instance;
#[cfg(feature = "postgres")]
mod course_roster;
#[cfg(feature = "postgres")]
mod course_theme;
#[cfg(feature = "postgres")]
mod imathas_question_backend_session;
#[cfg(feature = "postgres")]
mod instructor_account;
#[cfg(feature = "postgres")]
mod instructor_profile;
#[cfg(feature = "postgres")]
mod invitation_export;
#[cfg(feature = "postgres")]
mod live_gradebook;
#[cfg(feature = "postgres")]
mod live_student_course_landing;
#[cfg(feature = "postgres")]
mod migrations;
#[cfg(feature = "postgres")]
mod native_ple_grading;
#[cfg(feature = "postgres")]
mod native_ple_submission;
#[cfg(feature = "postgres")]
mod object_record;
#[cfg(feature = "postgres")]
mod profile_thumbnail;
#[cfg(feature = "postgres")]
mod public_asset_publication;
#[cfg(feature = "postgres")]
mod question_asset_delivery;
#[cfg(feature = "postgres")]
mod question_library;
#[cfg(feature = "postgres")]
mod question_source;
#[cfg(feature = "postgres")]
mod sessions;
#[cfg(feature = "postgres")]
mod support_capability;
#[cfg(feature = "postgres")]
mod webwork_grading;
#[cfg(feature = "postgres")]
mod webwork_submission;

#[cfg(feature = "postgres")]
pub use account_time_zone::PostgresAccountTimeZoneStore;
#[cfg(feature = "postgres")]
pub use assignment_attempt::PostgresAssignmentAttemptStore;
#[cfg(feature = "postgres")]
pub use assignment_delivery::PostgresLiveAssignmentDeliveryStore;
#[cfg(feature = "postgres")]
pub use assignment_release::PostgresLiveAssignmentStore;
#[cfg(feature = "postgres")]
pub use authoring::PostgresAuthoringDraftStore;
#[cfg(feature = "postgres")]
pub use blueprint_course::PostgresBlueprintCourseStore;
#[cfg(feature = "postgres")]
pub use connection::{ProductionLoginProfile, lazy_pool, local_development_pool, production_pool};
#[cfg(feature = "postgres")]
pub use course_banner::PostgresCourseBannerStore;
#[cfg(feature = "postgres")]
pub use course_instance::PostgresCourseInstanceStore;
#[cfg(feature = "postgres")]
pub use course_roster::PostgresCourseRosterStore;
#[cfg(feature = "postgres")]
pub use course_theme::PostgresCourseThemeStore;
#[cfg(feature = "postgres")]
pub use imathas_question_backend_session::PostgresImathasQuestionBackendSessionStore;
#[cfg(feature = "postgres")]
pub use instructor_account::PostgresInstructorAccountStore;
#[cfg(feature = "postgres")]
pub use instructor_profile::PostgresInstructorProfileStore;
#[cfg(feature = "postgres")]
pub use invitation_export::PostgresInvitationExportStore;
#[cfg(feature = "postgres")]
pub use live_gradebook::PostgresCourseGradebookStore;
#[cfg(feature = "postgres")]
pub use live_student_course_landing::PostgresLiveStudentCourseLandingStore;
#[cfg(feature = "postgres")]
pub use migrations::{
    BASE_RELEASE_IDENTITY, SchemaCompatibilityError, SchemaLifecycleGuard,
    acquire_schema_lifecycle, verify_application_schema,
};
#[cfg(feature = "postgres")]
pub use native_ple_grading::PostgresNativePleGradingStore;
#[cfg(feature = "postgres")]
pub use native_ple_submission::PostgresNativePleSubmissionStore;
#[cfg(feature = "postgres")]
pub use object_record::PostgresWorkspaceQuestionSourceObjectRecordStore;
#[cfg(feature = "postgres")]
pub use profile_thumbnail::PostgresProfileThumbnailStore;
#[cfg(feature = "postgres")]
pub use public_asset_publication::PostgresPublicAssetPublicationStore;
#[cfg(feature = "postgres")]
pub use question_asset_delivery::PostgresQuestionAssetDeliveryStore;
#[cfg(feature = "postgres")]
pub use question_library::PostgresQuestionLibraryStore;
#[cfg(feature = "postgres")]
pub use question_source::PostgresDraftQuestionSourceBindingStore;
#[cfg(feature = "postgres")]
pub use sessions::PostgresSessionStore;
#[cfg(feature = "postgres")]
pub use support_capability::PostgresSupportCapabilityStore;
#[cfg(feature = "postgres")]
pub use webwork_grading::PostgresWebworkGradingStore;
#[cfg(feature = "postgres")]
pub use webwork_submission::PostgresWebworkSubmissionStore;

#[cfg(feature = "postgres")]
pub type Pool = sqlx::postgres::PgPool;
