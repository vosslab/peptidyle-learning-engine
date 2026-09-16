//! PostgreSQL connection and migration administration for the clean baseline.
//!
//! Feature adapters return only after their exact clean-schema contracts exist.

#[cfg(feature = "postgres")]
mod account_avatar;
#[cfg(feature = "postgres")]
mod account_time_zone;
#[cfg(feature = "postgres")]
mod assessment_attempt;
#[cfg(feature = "postgres")]
mod assessment_attempt_context;
#[cfg(feature = "postgres")]
mod assessment_blueprint_update;
#[cfg(feature = "postgres")]
mod assessment_delivery;
#[cfg(feature = "postgres")]
mod assessment_delivery_access;
#[cfg(feature = "postgres")]
mod assessment_delivery_finalization;
#[cfg(feature = "postgres")]
mod assessment_delivery_history;
#[cfg(feature = "postgres")]
mod assessment_delivery_history_response;
#[cfg(feature = "postgres")]
mod assessment_delivery_source;
#[cfg(feature = "postgres")]
mod assessment_delivery_start;
#[cfg(feature = "postgres")]
mod assessment_pool_fork;
#[cfg(feature = "postgres")]
mod assessment_pool_selection_count;
#[cfg(feature = "postgres")]
mod assessment_release;
mod assessment_student_time_accommodation;
#[cfg(feature = "postgres")]
mod assessment_student_view;
#[cfg(feature = "postgres")]
mod assessment_template;
#[cfg(feature = "postgres")]
mod assessment_workspace_connection;
#[cfg(feature = "postgres")]
mod assessment_workspace_policy;
#[cfg(feature = "postgres")]
mod assessment_workspace_save;
#[cfg(feature = "postgres")]
mod attempt_expiry;
#[cfg(feature = "postgres")]
mod authoring;
#[cfg(feature = "postgres")]
mod authoring_assets;
#[cfg(feature = "postgres")]
mod blueprint_course;
#[cfg(feature = "postgres")]
mod blueprint_fork_apply;
#[cfg(feature = "postgres")]
mod blueprint_history;
#[cfg(feature = "postgres")]
mod blueprint_lineage;
#[cfg(feature = "postgres")]
mod blueprint_pools;
#[cfg(feature = "postgres")]
mod blueprint_stewardship;
#[cfg(feature = "postgres")]
mod connection;
#[cfg(feature = "postgres")]
mod content_classification;
#[cfg(feature = "postgres")]
pub use content_classification::PostgresContentClassificationStore;
#[cfg(feature = "postgres")]
mod course_banner;
#[cfg(feature = "postgres")]
mod course_blueprint_adoption;
#[cfg(feature = "postgres")]
mod course_instance;
#[cfg(feature = "postgres")]
mod course_roster;
#[cfg(feature = "postgres")]
mod course_theme;
#[cfg(feature = "postgres")]
#[cfg(feature = "postgres")]
mod imathas_question_backend_session;
#[cfg(feature = "postgres")]
mod instructor_account;
#[cfg(feature = "postgres")]
mod invitation_export;
#[cfg(feature = "postgres")]
mod live_gradebook;
#[cfg(feature = "postgres")]
mod live_student_course_landing;
#[cfg(feature = "postgres")]
mod migrations;
#[cfg(feature = "postgres")]
#[cfg(feature = "postgres")]
#[cfg(feature = "postgres")]
mod object_record;
#[cfg(feature = "postgres")]
mod public_asset_publication;
#[cfg(feature = "postgres")]
mod question_asset_delivery;
#[cfg(feature = "postgres")]
mod question_bulk_metadata;
#[cfg(feature = "postgres")]
mod question_fork;
#[cfg(feature = "postgres")]
mod question_library;
#[cfg(feature = "postgres")]
mod question_pool_creation;
#[cfg(feature = "postgres")]
mod question_pool_library;
#[cfg(feature = "postgres")]
mod question_source;
#[cfg(feature = "postgres")]
mod question_star;
#[cfg(feature = "postgres")]
mod question_watch;
#[cfg(feature = "postgres")]
mod question_watch_notification;
#[cfg(feature = "postgres")]
mod retention;
#[cfg(feature = "postgres")]
mod retention_notification;
#[cfg(feature = "postgres")]
mod sessions;
#[cfg(feature = "postgres")]
mod student_assessment_decision;
#[cfg(feature = "postgres")]
#[cfg(feature = "postgres")]
mod support_capability;
#[cfg(feature = "postgres")]
mod sysadmin_totp;

#[cfg(feature = "postgres")]
pub use account_avatar::PostgresAccountAvatarGallery;
#[cfg(feature = "postgres")]
pub use account_time_zone::PostgresAccountTimeZoneStore;
#[cfg(feature = "postgres")]
pub use assessment_attempt::PostgresAssessmentAttemptStore;
#[cfg(feature = "postgres")]
pub use assessment_delivery::PostgresLiveAssessmentDeliveryStore;
#[cfg(feature = "postgres")]
pub use assessment_pool_fork::PostgresAssessmentPoolForkStore;
#[cfg(feature = "postgres")]
pub use assessment_pool_selection_count::PostgresAssessmentPoolSelectionCountStore;
#[cfg(feature = "postgres")]
pub use assessment_release::PostgresLiveAssessmentStore;
pub use assessment_student_time_accommodation::PostgresAssessmentStudentTimeAccommodationStore;
#[cfg(feature = "postgres")]
pub use assessment_student_view::PostgresInstructorStudentViewStore;
#[cfg(feature = "postgres")]
pub use assessment_template::PostgresAssessmentTemplateStore;
#[cfg(feature = "postgres")]
pub use attempt_expiry::PostgresAssessmentAttemptExpirySweepStore;
#[cfg(feature = "postgres")]
pub use authoring::PostgresAuthoringDraftStore;
#[cfg(feature = "postgres")]
pub use authoring_assets::PostgresAuthoringAssetsStore;
#[cfg(feature = "postgres")]
pub use blueprint_course::PostgresBlueprintCourseStore;
#[cfg(feature = "postgres")]
pub use blueprint_lineage::PostgresBlueprintLineageStore;
#[cfg(feature = "postgres")]
pub use blueprint_stewardship::PostgresBlueprintStewardshipStore;
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
#[cfg(feature = "postgres")]
pub use imathas_question_backend_session::PostgresImathasQuestionBackendSessionStore;
#[cfg(feature = "postgres")]
pub use instructor_account::PostgresInstructorAccountStore;
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
#[cfg(feature = "postgres")]
#[cfg(feature = "postgres")]
pub use object_record::PostgresWorkspaceQuestionSourceObjectRecordStore;
#[cfg(feature = "postgres")]
pub use public_asset_publication::PostgresPublicAssetPublicationStore;
#[cfg(feature = "postgres")]
pub use question_asset_delivery::PostgresQuestionAssetDeliveryStore;
#[cfg(feature = "postgres")]
pub use question_bulk_metadata::PostgresBulkPublishedQuestionMetadataStore;
#[cfg(feature = "postgres")]
pub use question_fork::PostgresQuestionForkStore;
#[cfg(feature = "postgres")]
pub use question_library::PostgresQuestionLibraryStore;
#[cfg(feature = "postgres")]
pub use question_pool_creation::PostgresQuestionPoolCreationStore;
#[cfg(feature = "postgres")]
pub use question_pool_library::PostgresQuestionPoolLibraryStore;
#[cfg(feature = "postgres")]
pub use question_source::PostgresDraftQuestionSourceBindingStore;
#[cfg(feature = "postgres")]
pub use question_star::PostgresQuestionStarStore;
#[cfg(feature = "postgres")]
pub use question_watch::PostgresQuestionWatchStore;
#[cfg(feature = "postgres")]
pub use question_watch_notification::PostgresQuestionWatchNotificationStore;
#[cfg(feature = "postgres")]
pub use retention::PostgresCourseRetentionStore;
#[cfg(feature = "postgres")]
pub use retention_notification::PostgresCourseRetentionNotificationStore;
#[cfg(feature = "postgres")]
pub use sessions::PostgresSessionStore;
#[cfg(feature = "postgres")]
#[cfg(feature = "postgres")]
pub use support_capability::PostgresSupportCapabilityStore;
#[cfg(feature = "postgres")]
pub use sysadmin_totp::{
    PostgresSysadminTotpStore, SysadminTotpSeedKeyId, SysadminTotpSeedKeyRing,
};

#[cfg(feature = "postgres")]
pub type Pool = sqlx::postgres::PgPool;
