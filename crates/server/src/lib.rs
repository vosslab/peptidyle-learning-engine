//! PLE server core for the single-installation application.
//!
//! The executable composes authentication, Live Demo, Question Library,
//! authoring, Blueprint Course, Course Instance, roster, Account management,
//! Gradebook, Assessment, asset-delivery, and support-capability routes over
//! their typed Store contracts and PostgreSQL capabilities.

/// Student-only Course and released Assessment landing routes.
pub(crate) mod archived_student_work_recovery;
/// Student Assessment Access and answer-free initial delivery routes.
pub(crate) mod assessment_delivery;
/// Closed Instructor commands for Assessment-owned immutable Question Pool forks.
mod assessment_pool_fork;
/// Count-only Instructor command for an Assessment-owned Question Pool entry.
mod assessment_pool_selection_count;
/// Direct-Instructor Assessment Workspace and immutable release routes.
pub(crate) mod assessment_release;
mod assessment_student_time_accommodation;
/// Answer-free, no-write Instructor Student View delivery routes.
pub(crate) mod assessment_student_view;
/// Private owner-only Assessment Template CRUD subset.
pub(crate) mod assessment_template;
/// Authentication, sessions, and the first-party browser boundary.
pub mod auth;
/// Fixed public routes for reviewed author-content runtime assets.
pub(crate) mod author_content_dependency_assets;
/// Generated closed registry for reviewed author-content runtimes.
mod author_content_dependency_registry_generated;
/// Student-authorized isolated author-content document route.
pub(crate) mod author_content_document_route;
/// Private Authoring Workspace and Draft Question routes.
pub mod authoring;
mod authoring_assets;
mod authoring_source;
/// Server-owned Bloom classifier and candidate-bound publication preparation.
pub mod bloom_classification;
/// Instructor-owned reusable Blueprint Course routes.
pub(crate) mod blueprint_course;
/// Vetted-Instructor Blueprint Course Star and private Watch routes.
mod blueprint_stewardship;
/// Production database/session composition.
pub mod composition;
mod content_classification;
/// Current Course Appearance reader for active Course Members.
pub(crate) mod course_appearance;
/// Course Instance creation and initial teaching-team routes.
pub(crate) mod course_instance;
/// Provider-neutral, fail-closed Course-retention notification delivery.
pub mod course_retention_notification_delivery;
/// Orchestrates the two isolated Course-retention database capabilities.
pub mod course_retention_worker;
/// Course Roster Import, invitation claim, and access-revocation routes.
pub(crate) mod course_roster;
/// Readiness probe support for the executable process.
pub mod health;
/// Uniform dynamic-response security headers.
pub(crate) mod http_security;
/// Sysadmin-only Instructor Account management routes.
pub(crate) mod instructor_account;
/// Protected direct-Instructor invitation-mailer export route.
pub(crate) mod invitation_export;
/// Retained Question and Pool improvement threads and impact notices.
mod library_discussion;
mod library_search_terms;
/// Private self-only Question and Pool Watch notification inbox.
mod library_watch_notification;
pub(crate) mod live_gradebook;
pub(crate) mod live_student_course_landing;
/// Authorized public Course-reference navigation route.
pub(crate) mod navigation;
/// Authenticated role-neutral self-only Account avatar routes.
pub(crate) mod profile_avatar;
/// Authenticated role-neutral self-only Profile Settings routes.
pub(crate) mod profile_settings;
/// One-shot immutable public Question Asset publisher with no HTTP surface.
pub mod public_asset_publisher;
/// Authorized immutable public Question Asset redirect route.
pub(crate) mod question_asset_delivery;
/// Active-vetted-Instructor atomic shared Published Question metadata edits.
mod question_bulk_metadata;
/// Active-Instructor Published Question to private Draft fork command.
mod question_fork;
/// Instructor Question Library browse and answer-free detail routes.
mod question_library;
/// Active-Instructor reusable Published Question Pool creation.
mod question_pool_creation;
/// Published Pool Library browse/current detail and owned fork detail routes.
mod question_pool_library;
/// Vetted-Instructor Pool Stars and actor-private Pool Watch state.
mod question_pool_stewardship;
/// Server-only verified Question Publication coordination.
pub mod question_publication;
mod question_publication_assets;
/// Vetted-Instructor Question Star state and aggregate endorsement routes.
mod question_stewardship;
/// Private self-only vetted-Instructor Question Watch routes.
mod question_watch;
/// Process-wide safe request lifecycle handling.
pub mod request_lifecycle;
/// Exact-course, direct-Instructor support-capability routes.
pub(crate) mod support_capability;
/// Public, bounded proxy for renderer-owned WeBWorK assets.
pub(crate) mod webwork_asset_proxy;
/// Student-authorized immutable WeBWorK document delivery route.
pub(crate) mod webwork_document_route;
/// Least-privilege worker process lifecycle with no browser or HTTP listener.
pub mod worker;
