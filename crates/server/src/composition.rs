//! Public startup and publication entry points over focused composition owners.

mod api_assembly;
#[path = "composition_live_demo.rs"]
mod composition_live_demo;
mod environment;
mod object_storage;
mod question_thumbnail_backfill;
pub use question_thumbnail_backfill::backfill_question_thumbnails_from_env;
mod worker_runners;

pub use api_assembly::production_router_from_env;
pub use composition_live_demo::provision_local_sysadmin_totp_from_env;
pub use environment::{
    bind_address_from_env, browser_authority_from_env, question_id_issuer,
    question_library_object_store_from_env,
};
pub use worker_runners::{
    run_attempt_expiry_worker_from_env, run_course_retention_process_from_env,
    run_public_asset_publisher_from_env, verify_public_asset_publisher_database_login_from_env,
    verify_worker_database_login_from_env,
};
