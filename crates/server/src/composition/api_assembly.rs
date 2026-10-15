//! API store, authentication, route, and HTTP-boundary assembly.

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{Router, routing::get};
use learning_data_access::postgres::{
    PostgresAccountAppearanceStore, PostgresAccountAvatarGallery, PostgresAccountTimeZoneStore,
    PostgresArchivedStudentWorkRecoveryStore, PostgresAssessmentTemplateStore,
    PostgresAuthenticationCeremonyStore, PostgresAuthoringDraftStore, PostgresBlueprintCourseStore,
    PostgresBlueprintLineageStore, PostgresBlueprintStewardshipStore,
    PostgresBulkPublishedQuestionMetadataStore, PostgresBulkQuestionPoolSearchMetadataStore,
    PostgresContentClassificationStore, PostgresCourseBannerStore,
    PostgresCourseBlueprintPublicationStore, PostgresCourseGradebookStore,
    PostgresCourseInstanceStore, PostgresCourseRosterStore, PostgresCourseThemeStore,
    PostgresDraftQuestionImageStore, PostgresDraftQuestionSourceBindingStore,
    PostgresInstructorAccountStore, PostgresInstructorStudentViewStore,
    PostgresInvitationExportStore, PostgresLibraryWatchNotificationStore,
    PostgresLiveAssessmentDeliveryStore, PostgresLiveAssessmentStore,
    PostgresLiveStudentCourseLandingStore, PostgresQuestionForkStore,
    PostgresQuestionImageDeliveryStore, PostgresQuestionLibraryStore,
    PostgresQuestionMetadataStore, PostgresQuestionPoolCreationStore,
    PostgresQuestionPoolForkStore, PostgresQuestionPoolLibraryStore,
    PostgresQuestionPoolStewardshipStore, PostgresQuestionPoolSupportStore,
    PostgresQuestionStarStore, PostgresQuestionWatchStore, PostgresSessionStore,
    PostgresSysadminStudentDataStore, ProductionLoginProfile, local_development_pool,
    production_pool,
};

use crate::auth::{
    AuthenticationStores, email_code_router, live_demo_mfa_router, live_demo_router,
    session_router, sysadmin_totp_router,
};

use super::{composition_live_demo, environment};

/// Builds the current route surface from explicit environment configuration.
pub async fn production_router_from_env() -> Result<Router> {
    let database_url = environment::required_env("DATABASE_URL")?;
    let pool = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local")
    {
        local_development_pool(&database_url, ProductionLoginProfile::Api)
    } else {
        production_pool(&database_url, ProductionLoginProfile::Api)
    }
    .context("could not construct the attested API database pool")?;
    pool.acquire()
        .await
        .context("the attested API database pool could not connect")?;

    let readiness = crate::health::ReadinessState::from_environment(pool.clone())
        .map_err(anyhow::Error::msg)
        .context("could not configure API readiness checks")?;
    let sessions = Arc::new(PostgresSessionStore::new(pool.clone()));
    let email_ceremonies = Arc::new(PostgresAuthenticationCeremonyStore::new(pool.clone()));
    let instructor_setup_email = Arc::new(environment::instructor_setup_email_delivery_from_env()?);
    let email_code_delivery = instructor_setup_email.is_configured().then(|| {
        Arc::clone(&instructor_setup_email)
            as Arc<dyn crate::instructor_setup_email_delivery::InstructorSetupEmailDelivery>
    });
    let sysadmin_totp = composition_live_demo::local_sysadmin_totp_store_from_env(pool.clone())?;
    let question_library_store = PostgresQuestionLibraryStore::new(pool.clone());
    let question_bulk_metadata = PostgresBulkPublishedQuestionMetadataStore::new(pool.clone());
    let question_pool_bulk_metadata =
        PostgresBulkQuestionPoolSearchMetadataStore::new(pool.clone());
    let question_metadata_store = PostgresQuestionMetadataStore::new(pool.clone());
    let question_pool_support = PostgresQuestionPoolSupportStore::new(pool.clone());
    let content_classification = PostgresContentClassificationStore::new(pool.clone());
    let question_pool_creation = PostgresQuestionPoolCreationStore::new(pool.clone());
    let question_pool_fork = PostgresQuestionPoolForkStore::new(pool.clone());
    let question_pool_library = PostgresQuestionPoolLibraryStore::new(pool.clone());
    let question_pool_stewardship = PostgresQuestionPoolStewardshipStore::new(pool.clone());
    let question_forks = PostgresQuestionForkStore::new(pool.clone());
    let question_stars = PostgresQuestionStarStore::new(pool.clone());
    let question_watches = PostgresQuestionWatchStore::new(pool.clone());
    let library_watch_notifications = PostgresLibraryWatchNotificationStore::new(pool.clone());
    let blueprint_stewardship = PostgresBlueprintStewardshipStore::new(pool.clone());
    let course_themes = PostgresCourseThemeStore::new(pool.clone());
    let course_banners = PostgresCourseBannerStore::new(pool.clone());
    let course_roster = PostgresCourseRosterStore::new(pool.clone());
    let instructor_accounts = PostgresInstructorAccountStore::new(pool.clone());
    let account_avatar_gallery = PostgresAccountAvatarGallery::new(pool.clone());
    let sysadmin_student_data = PostgresSysadminStudentDataStore::new(pool.clone());
    let invitation_exports = PostgresInvitationExportStore::new(pool.clone());
    let gradebook = PostgresCourseGradebookStore::new(pool.clone());
    let student_course_landing = PostgresLiveStudentCourseLandingStore::new(pool.clone());
    let archived_student_work_recovery =
        PostgresArchivedStudentWorkRecoveryStore::new(pool.clone());
    let profile_time_zones = PostgresAccountTimeZoneStore::new(pool.clone());
    let profile_appearance = PostgresAccountAppearanceStore::new(pool.clone());
    let assessment_student_time_accommodations =
        learning_data_access::postgres::PostgresAssessmentStudentTimeAccommodationStore::new(
            pool.clone(),
        );
    let assessment_templates = PostgresAssessmentTemplateStore::new(pool.clone());
    let assessment_delivery = PostgresLiveAssessmentDeliveryStore::new(pool.clone());
    let assessment_student_view = PostgresInstructorStudentViewStore::new(pool.clone());
    let question_image_delivery = PostgresQuestionImageDeliveryStore::new(pool.clone());
    let authoring_drafts = PostgresAuthoringDraftStore::new(pool.clone());
    let draft_question_images = PostgresDraftQuestionImageStore::new(pool.clone());
    let authoring_publication = PostgresDraftQuestionSourceBindingStore::new(pool.clone());
    let question_library_objects = environment::question_library_object_store_from_env().await?;
    let webwork_adapter = environment::webwork_adapter_from_env()?;
    let webwork_asset_proxy = environment::webwork_asset_proxy_from_env()?;
    let question_id_issuer = environment::question_id_issuer();
    let blueprint_lineage = PostgresBlueprintLineageStore::new(pool.clone());
    let assessments = PostgresLiveAssessmentStore::new(pool.clone());
    let blueprint_courses = PostgresBlueprintCourseStore::new(pool.clone());
    let course_blueprint_publication = PostgresCourseBlueprintPublicationStore::new(pool.clone());
    let course_instances = PostgresCourseInstanceStore::new(pool.clone());
    let browser_boundary = environment::production_browser_boundary_from_env()?;
    let session_config = environment::production_session_config();
    let readiness_router = Router::new()
        .route("/health", get(crate::health::readiness_handler))
        .with_state(readiness);
    let authentication_router = match sysadmin_totp {
        Some(sysadmin_totp) => {
            let authentication_stores = Arc::new(AuthenticationStores::new(
                Arc::clone(&sessions),
                sysadmin_totp,
            ));
            let router = session_router(Arc::clone(&sessions), session_config);
            let router = if let Some(delivery) = email_code_delivery {
                router.merge(email_code_router(
                    Arc::clone(&email_ceremonies),
                    Arc::clone(&sessions),
                    delivery,
                    session_config,
                ))
            } else {
                router
            };
            router
                .merge(live_demo_mfa_router(
                    Arc::clone(&authentication_stores),
                    composition_live_demo::live_demo_config_from_env()?,
                    session_config,
                ))
                .merge(sysadmin_totp_router(authentication_stores, session_config))
        }
        None => {
            let router = session_router(Arc::clone(&sessions), session_config);
            let router = if let Some(delivery) = email_code_delivery {
                router.merge(email_code_router(
                    email_ceremonies,
                    Arc::clone(&sessions),
                    delivery,
                    session_config,
                ))
            } else {
                router
            };
            router.merge(live_demo_router(
                Arc::clone(&sessions),
                composition_live_demo::live_demo_config_from_env()?,
                session_config,
            ))
        }
    };
    let router = Router::new()
        .merge(readiness_router)
        .merge(authentication_router)
        .merge(crate::content_classification::content_classification_router(
            Arc::clone(&sessions),
            content_classification.clone(),
        ))
        .merge(crate::author_content_dependency_assets::author_content_dependency_asset_router())
        .merge(crate::question_library::question_library_router(
            Arc::clone(&sessions),
            question_library_store.clone(),
            content_classification.clone(),
            question_library_objects.clone(),
            Arc::clone(&webwork_adapter),
        ))
        .merge(crate::question_metadata::question_metadata_router(
            Arc::clone(&sessions) as Arc<dyn learning_data_access::SessionStore>,
            Arc::new(question_metadata_store),
        ))
        .merge(crate::question_bulk_metadata::question_bulk_metadata_router(
            Arc::clone(&sessions) as Arc<dyn learning_data_access::SessionStore>,
            Arc::new(question_bulk_metadata),
        ))
        .merge(
            crate::question_pool_bulk_metadata::question_pool_search_metadata_router(
                Arc::clone(&sessions) as Arc<dyn learning_data_access::SessionStore>,
                Arc::new(question_pool_bulk_metadata),
            ),
        )
        .merge(crate::question_pool_library::question_pool_library_router(
            Arc::clone(&sessions),
            question_pool_library,
            question_library_store.clone(),
            question_library_objects.clone(),
        ))
        .merge(
            crate::question_pool_stewardship::question_pool_stewardship_router(
                Arc::clone(&sessions),
                question_pool_stewardship,
            ),
        )
        .merge(crate::question_pool_support::question_pool_support_router(
            Arc::clone(&sessions) as Arc<dyn learning_data_access::SessionStore>,
            Arc::new(question_pool_support),
        ))
        .merge(
            crate::question_pool_creation::question_pool_creation_router(
                Arc::clone(&sessions),
                question_pool_creation,
                question_id_issuer,
            ),
        )
        .merge(crate::question_pool_creation::question_pool_fork_router(
            Arc::clone(&sessions),
            question_pool_fork,
            question_id_issuer,
        ))
        .merge(crate::question_fork::question_fork_router(
            Arc::clone(&sessions),
            question_forks.clone(),
            question_library_store.clone(),
            authoring_drafts.clone(),
            question_library_objects.clone(),
        ))
        .merge(crate::question_correction_draft::question_correction_draft_router(
            Arc::clone(&sessions),
            question_library_store.clone(),
            question_forks,
            authoring_drafts.clone(),
            draft_question_images.clone(),
            question_library_objects.clone(),
        ))
        .merge(crate::question_stewardship::question_stewardship_router(
            Arc::clone(&sessions),
            question_stars,
        ))
        .merge(crate::question_watch::question_watch_router(
            Arc::clone(&sessions),
            question_watches,
        ))
        .merge(
            crate::library_watch_notification::library_watch_notification_router(
                Arc::clone(&sessions),
                library_watch_notifications,
            ),
        )
        .merge(crate::draft_preview::draft_preview_router(
            Arc::clone(&sessions),
            authoring_drafts.clone(),
            draft_question_images.clone(),
            question_library_objects.clone(),
            Arc::clone(&webwork_adapter),
        ))
        .merge(crate::authoring::authoring_router(
            Arc::clone(&sessions),
            authoring_drafts,
            draft_question_images,
            authoring_publication,
            question_library_objects.clone(),
            Arc::clone(&webwork_adapter),
            question_id_issuer,
        ))
        .merge(crate::blueprint_course::blueprint_course_router(
            Arc::clone(&sessions),
            blueprint_courses,
            course_blueprint_publication,
            blueprint_lineage,
            question_library_store,
            question_library_objects.clone(),
        ))
        .merge(crate::blueprint_stewardship::blueprint_stewardship_router(
            Arc::clone(&sessions),
            blueprint_stewardship,
        ))
        .merge(crate::navigation::navigation_router(
            Arc::clone(&sessions),
            course_instances.clone(),
        ))
        .merge(crate::course_instance::course_instance_router(
            Arc::clone(&sessions),
            course_instances.clone(),
        ))
        .merge(crate::course_appearance::course_appearance_router(
            Arc::clone(&sessions),
            course_instances,
            course_themes,
            course_banners,
            question_library_objects.clone(),
        ))
        .merge(crate::course_roster::course_roster_router(
            Arc::clone(&sessions),
            course_roster,
            environment::invitation_signup_url_from_env()?,
        ))
        .merge(crate::instructor_account::instructor_account_router(
            Arc::clone(&sessions),
            instructor_accounts,
            instructor_setup_email,
        ))
        .merge(crate::profile_avatar::profile_avatar_router(
            Arc::clone(&sessions),
            account_avatar_gallery,
            question_library_objects.clone(),
        ))
        .merge(crate::profile_settings::profile_settings_router(
            Arc::clone(&sessions),
            profile_time_zones,
            profile_appearance,
        ))
        .merge(crate::support_capability::support_capability_router(
            Arc::clone(&sessions),
            sysadmin_student_data,
        ))
        .merge(crate::invitation_export::invitation_export_router(
            Arc::clone(&sessions),
            invitation_exports,
            environment::invitation_signup_url_from_env()?,
        ))
        .merge(crate::live_gradebook::live_gradebook_router(
            Arc::clone(&sessions),
            gradebook,
        ))
        .merge(
            crate::archived_student_work_recovery::archived_student_work_recovery_router(
                Arc::clone(&sessions),
                archived_student_work_recovery,
            ),
        )
        .merge(
            crate::live_student_course_landing::live_student_course_landing_router(
                Arc::clone(&sessions),
                student_course_landing,
            ),
        )
        .merge(crate::assessment_release::assessment_release_router(
            Arc::clone(&sessions),
            assessments,
        ))
        .merge(crate::assessment_template::assessment_template_router(
            Arc::clone(&sessions),
            assessment_templates,
        ))
        .merge(crate::assessment_student_time_accommodation::assessment_student_time_accommodation_router(
            Arc::clone(&sessions), assessment_student_time_accommodations,
        ))
        .merge(crate::assessment_delivery::assessment_delivery_router(
            Arc::clone(&sessions),
            assessment_delivery,
            question_library_objects.clone(),
            Arc::clone(&webwork_adapter),
            Arc::clone(&browser_boundary.origin),
        ))
        .merge(
            crate::assessment_student_view::assessment_student_view_router(
                Arc::clone(&sessions),
                assessment_student_view,
                question_library_objects.clone(),
                Arc::clone(&webwork_adapter),
                Arc::clone(&browser_boundary.origin),
            ),
        )
        .merge(
            crate::webwork_asset_proxy::webwork_asset_proxy_router(webwork_asset_proxy)
                .map_err(anyhow::Error::msg)?,
        )
        .merge(
            crate::question_image_delivery::question_image_delivery_router(
                Arc::clone(&sessions),
                question_image_delivery,
                crate::question_image_delivery::public_asset_base_url(&environment::required_env(
                    "PLE_PUBLIC_ASSET_BASE_URL",
                )?)
                .map_err(anyhow::Error::msg)?,
            ),
        );
    Ok(crate::http_security::apply_api_security_headers(
        router.layer(axum::middleware::from_fn_with_state(
            browser_boundary,
            crate::auth::production_cookie_boundary,
        )),
    ))
}
