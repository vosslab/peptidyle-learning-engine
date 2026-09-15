//! Current Course Appearance reader for every active Course Member.
//!
//! Theme storage is available before banner promotion.  The response remains
//! the complete aggregate contract, with an absent banner until the banner
//! store supplies a current banner projection.

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use learning_data_access::{
    CourseBannerObjectMetadata, CourseBannerStore, CourseThemeStore,
    FinalizedCourseBannerPromotion, PrepareCourseBannerPromotion, PreparedCourseBannerRemoval,
    SessionTokenHash, StageCourseBannerUpload, StoreError,
    postgres::{PostgresCourseBannerStore, PostgresCourseThemeStore, PostgresSessionStore},
};
use objects::s3::S3ObjectStore;
use objects::{
    ObjectAddress, ObjectRecord, ObjectStore, PutObject, Sha256Checksum,
    image_validation::{normalized_course_banner_webp, verify_course_banner_still_image},
};
use question_model::{
    CourseAppearanceView, CourseBannerReference, CourseBannerRendition, CourseBannerUpdate,
    CourseBannerUploadReceipt, CourseBannerUploadReference, CourseId, CourseThemeUpdate,
    ProductRole, Timestamp,
};
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

const MAX_THEME_UPDATE_BYTES: usize = 1_024;
const MAX_BANNER_UPDATE_BYTES: usize = 1_024;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    themes: PostgresCourseThemeStore,
    banners: PostgresCourseBannerStore,
    objects: S3ObjectStore,
}

/// Registers the browser-safe current Course Appearance reader.
///
/// The route uses the internal Course UUID after the browser has resolved its
/// visible Course reference.  The Store repeats exact Course Membership
/// authorization against the installed session before it projects the theme.
pub fn course_appearance_router(
    sessions: Arc<PostgresSessionStore>,
    themes: PostgresCourseThemeStore,
    banners: PostgresCourseBannerStore,
    objects: S3ObjectStore,
) -> Router {
    Router::new()
        .route(
            "/api/courses/{course}/appearance",
            get(read_appearance).put(update_theme),
        )
        .route(
            "/api/courses/{course}/appearance/banner-uploads",
            post(stage_banner_upload),
        )
        .route(
            "/api/courses/{course}/appearance/banner",
            axum::routing::put(promote_banner).delete(remove_banner),
        )
        .route(
            "/api/course-banners/{banner}/delivery",
            post(deliver_banner),
        )
        .with_state(RouteState {
            sessions,
            themes,
            banners,
            objects,
        })
}

/// Persists one independent Course Theme for the current Instructor.
///
/// The Product Role check rejects every non-Instructor before persistence; the
/// Store repeats exact active Instructor Course Membership authorization in
/// PostgreSQL.  Both checks are required because Product Role is not Course
/// Membership authority.
async fn update_theme(
    State(state): State<RouteState>,
    Path(course): Path<String>,
    request: Request,
) -> Response {
    let course = match Uuid::parse_str(&course) {
        Ok(value) => CourseId::from_uuid(value),
        Err(_) => return concealed(),
    };
    let session_hash = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 5.1.1: after concealment-safe authorization, accept only JSON for
    // the manually decoded body.  Axum's Json extractor is intentionally not
    // used here because authorization must precede any body read.
    if !has_content_type(request.headers(), "application/json") {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Course Theme is invalid",
        );
    }
    let update = match to_bytes(request.into_body(), MAX_THEME_UPDATE_BYTES).await {
        Ok(bytes) => match serde_json::from_slice::<CourseThemeUpdate>(&bytes) {
            Ok(value) => value,
            // ASVS 1.5.2 and 5.2.1: reject malformed and unknown request
            // fields without reaching persistence.  Parsing occurs only
            // after the complete Course authority boundary has succeeded.
            Err(_) => {
                return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Course Theme is invalid");
            }
        },
        // ASVS 1.5.2 and 5.2.1: reject malformed and unknown request fields
        // without reaching persistence.  The bounded read also refuses an
        // oversized body before any state-changing Store call.
        Err(_) => return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Course Theme is invalid"),
    };
    match update_course_appearance(
        &state.themes,
        &state.banners,
        session_hash,
        course,
        update.theme,
    )
    .await
    {
        Ok(appearance) => crate::auth::no_store(Json(appearance).into_response()),
        Err(error) => store_error_response(error),
    }
}

/// Updates the independent Course Theme, then returns the complete Course Appearance View.
///
/// After the theme is persisted, the existing Course Banner is read without changing it.
async fn update_course_appearance(
    themes: &(impl CourseThemeStore + ?Sized),
    banners: &(impl CourseBannerStore + ?Sized),
    session_hash: SessionTokenHash,
    course: CourseId,
    theme: question_model::CourseTheme,
) -> Result<CourseAppearanceView, StoreError> {
    let theme = themes
        .update_course_theme(session_hash, course, theme)
        .await?;
    let banner = banners
        .read_current_course_banner(session_hash, course)
        .await?;
    Ok(CourseAppearanceView { theme, banner })
}

async fn read_appearance(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(course): Path<String>,
) -> Response {
    let course = match Uuid::parse_str(&course) {
        Ok(value) => CourseId::from_uuid(value),
        // ASVS 1.2.3 and 8.2.2: malformed identities receive the same
        // no-store concealment response as an authenticated nonmember.
        Err(_) => return concealed(),
    };
    let session_hash = match authenticated_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 1.2.3 and 8.2.2: this session-bound Store operation invokes
    // current_session_account_is_course_member; role alone never authorizes
    // a Course Appearance read.
    match state.themes.read_course_theme(session_hash, course).await {
        Ok(theme) => match state
            .banners
            .read_current_course_banner(session_hash, course)
            .await
        {
            Ok(banner) => {
                crate::auth::no_store(Json(CourseAppearanceView { theme, banner }).into_response())
            }
            Err(error) => store_error_response(error),
        },
        Err(error) => store_error_response(error),
    }
}

async fn stage_banner_upload(
    State(state): State<RouteState>,
    Path(course): Path<String>,
    request: Request,
) -> Response {
    let course = match Uuid::parse_str(&course) {
        Ok(value) => CourseId::from_uuid(value),
        Err(_) => return concealed(),
    };
    let token = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if !has_content_type(request.headers(), "application/octet-stream") {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Course Banner is invalid",
        );
    }
    let bytes = match to_bytes(
        request.into_body(),
        objects::image_validation::MAX_STILL_IMAGE_BYTES,
    )
    .await
    {
        Ok(value) => value.to_vec(),
        Err(_) => return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Course Banner is invalid"),
    };
    // The exact visual 5:1 shape is a Course Banner contract, not a client
    // hint. Refuse it before the temporary object is staged so promotion can
    // never need to crop or pad user content.
    let verified = match verify_course_banner_still_image(&bytes) {
        Ok(value) => value,
        Err(_) => return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Course Banner is invalid"),
    };
    let upload = CourseBannerUploadReference::generate();
    let media_type = verified.media_type.canonical_media_type().to_string();
    let address = ObjectAddress::CourseBannerUpload { course, upload };
    let metadata = banner_metadata(
        &address,
        &bytes,
        media_type.clone(),
        verified.width,
        verified.height,
    );
    let expires = now().as_unix_millis().saturating_add(15 * 60 * 1_000);
    let staged_address = match state
        .banners
        .stage_course_banner_upload(
            token,
            StageCourseBannerUpload {
                course,
                upload,
                metadata: metadata.clone(),
                width: verified.width,
                height: verified.height,
                expires_at_unix_millis: expires,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    match finalize_staged_upload(
        &state.banners,
        &state.objects,
        token,
        course,
        upload,
        staged_address.put_work_id,
        &staged_address.address,
        bytes,
        media_type,
        &metadata,
    )
    .await
    {
        Ok(()) => crate::auth::no_store(Json(CourseBannerUploadReceipt { upload }).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn promote_banner(
    State(state): State<RouteState>,
    Path(course): Path<String>,
    request: Request,
) -> Response {
    let course = match Uuid::parse_str(&course) {
        Ok(value) => CourseId::from_uuid(value),
        Err(_) => return concealed(),
    };
    let token = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if !has_content_type(request.headers(), "application/json") {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Course Banner is invalid",
        );
    }
    let update = match to_bytes(request.into_body(), MAX_BANNER_UPDATE_BYTES)
        .await
        .ok()
        .and_then(|bytes| serde_json::from_slice::<CourseBannerUpdate>(&bytes).ok())
    {
        Some(value) => value,
        None => return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Course Banner is invalid"),
    };
    let claimed = match state
        .banners
        .read_staged_course_banner_upload(token, course, update.upload)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let staged = match state.objects.get(&claimed.address).await {
        Ok(value)
            if value.record.id == claimed.object_id
                && value.record.sha256 == claimed.sha256
                && value.record.size_bytes == claimed.byte_length
                && value.record.media_type == claimed.canonical_media_type
                && value.record.address == claimed.address =>
        {
            value
        }
        _ => {
            mark_repair(&state, token, course, None, claimed.object_id).await;
            return route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Course Appearance unavailable",
            );
        }
    };
    let banner = CourseBannerReference::generate();
    let source_address = ObjectAddress::CourseBannerSource { course, banner };
    let rendition_address = ObjectAddress::CourseBannerRendition {
        course,
        banner,
        rendition: CourseBannerRendition::Banner,
    };
    let source_bytes = staged.bytes;
    // Revalidate the staged bytes before promotion.  Its durable dimensions
    // are evidence only; the object store is re-read and decoded so a stale or
    // substituted temporary object cannot reach the immutable delivery path.
    let verified = match verify_course_banner_still_image(&source_bytes) {
        Ok(value) => value,
        Err(_) => return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Course Banner is invalid"),
    };
    if verified.width != claimed.width || verified.height != claimed.height {
        mark_repair(&state, token, course, None, claimed.object_id).await;
        return route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Course Appearance unavailable",
        );
    }
    let (rendition_width, rendition_height) = CourseBannerRendition::Banner.dimensions();
    let rendition_bytes =
        match normalized_course_banner_webp(&source_bytes, rendition_width, rendition_height) {
            Ok(value) => value,
            Err(_) => {
                return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Course Banner is invalid");
            }
        };
    let source_metadata = banner_metadata(
        &source_address,
        &source_bytes,
        claimed.canonical_media_type,
        verified.width,
        verified.height,
    );
    let rendition_metadata = banner_metadata(
        &rendition_address,
        &rendition_bytes,
        "image/webp".to_string(),
        rendition_width,
        rendition_height,
    );
    let prepared = match state
        .banners
        .prepare_course_banner_promotion(
            token,
            PrepareCourseBannerPromotion {
                course,
                upload: update.upload,
                banner,
                update: update.clone(),
                source: source_metadata.clone(),
                rendition: rendition_metadata.clone(),
            },
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let prepared_objects = [
        (
            &prepared.source,
            source_bytes,
            source_metadata,
            prepared.source_put_work_id,
        ),
        (
            &prepared.rendition,
            rendition_bytes,
            rendition_metadata,
            prepared.rendition_put_work_id,
        ),
    ];
    if write_prepared_objects(
        &state.banners,
        &state.objects,
        token,
        course,
        banner,
        &prepared_objects,
    )
    .await
    .is_err()
    {
        compensate_prepared(&state, token, course, banner, &prepared_objects).await;
        return route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Course Appearance unavailable",
        );
    }
    let finalized = match state
        .banners
        .finalize_course_banner_promotion(token, course, update.upload, banner)
        .await
    {
        Ok(value) => value,
        Err(error) => {
            compensate_prepared(&state, token, course, banner, &prepared_objects).await;
            return store_error_response(error);
        }
    };
    cleanup_finalized_promotion(&state, token, course, &finalized).await;
    let theme = match state.themes.read_course_theme(token, course).await {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    crate::auth::no_store(
        Json(CourseAppearanceView {
            theme,
            banner: Some(finalized.banner),
        })
        .into_response(),
    )
}

async fn remove_banner(
    State(state): State<RouteState>,
    Path(course): Path<String>,
    headers: HeaderMap,
) -> Response {
    let course = match Uuid::parse_str(&course) {
        Ok(value) => CourseId::from_uuid(value),
        Err(_) => return concealed(),
    };
    let token = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .banners
        .prepare_course_banner_removal(token, course)
        .await
    {
        Ok(removal) => {
            cleanup_removal(&state, token, course, &removal).await;
            match state.themes.read_course_theme(token, course).await {
                Ok(theme) => crate::auth::no_store(
                    Json(CourseAppearanceView {
                        theme,
                        banner: None,
                    })
                    .into_response(),
                ),
                Err(error) => store_error_response(error),
            }
        }
        Err(error) => store_error_response(error),
    }
}

async fn deliver_banner(
    State(state): State<RouteState>,
    Path(banner): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Ok(banner) = Uuid::parse_str(&banner) else {
        return concealed();
    };
    let banner = CourseBannerReference::from_uuid(banner);
    let token = match authenticated_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .banners
        .resolve_current_course_banner(token, banner)
        .await
    {
        Ok(course) => match state
            .objects
            .get(&ObjectAddress::CourseBannerRendition {
                course,
                banner,
                rendition: CourseBannerRendition::Banner,
            })
            .await
        {
            Ok(value) => {
                let size = value.bytes.len();
                if size == 0 || size > objects::image_validation::MAX_COURSE_BANNER_RENDITION_BYTES
                {
                    return route_error(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "Course Appearance unavailable",
                    );
                }
                let mut response = value.bytes.into_response();
                let headers = response.headers_mut();
                headers.insert(
                    axum::http::header::CONTENT_TYPE,
                    "image/webp".parse().unwrap(),
                );
                headers.insert(
                    axum::http::header::CONTENT_DISPOSITION,
                    "attachment; filename=\"ple-course-banner.webp\""
                        .parse()
                        .unwrap(),
                );
                headers.insert(
                    axum::http::header::CONTENT_LENGTH,
                    size.to_string().parse().unwrap(),
                );
                headers.insert(
                    axum::http::header::X_CONTENT_TYPE_OPTIONS,
                    "nosniff".parse().unwrap(),
                );
                headers.insert(
                    "cross-origin-resource-policy",
                    "same-origin".parse().unwrap(),
                );
                headers.insert(
                    axum::http::header::REFERRER_POLICY,
                    "no-referrer".parse().unwrap(),
                );
                crate::auth::no_store(response)
            }
            Err(_) => route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Course Appearance unavailable",
            ),
        },
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
            concealed()
        }
        Err(error) => store_error_response(error),
    }
}

fn now() -> Timestamp {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(milliseconds).unwrap_or(i64::MAX))
}

fn banner_metadata(
    address: &ObjectAddress,
    bytes: &[u8],
    media_type: String,
    width: u32,
    height: u32,
) -> CourseBannerObjectMetadata {
    CourseBannerObjectMetadata {
        object_id: address.object_id(),
        sha256: Sha256Checksum::compute(bytes),
        byte_length: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        media_type,
        width,
        height,
    }
}

fn record_matches_metadata(
    record: &ObjectRecord,
    address: &ObjectAddress,
    metadata: &CourseBannerObjectMetadata,
) -> bool {
    record.address == *address
        && record.id == metadata.object_id
        && record.sha256 == metadata.sha256
        && record.size_bytes == metadata.byte_length
        && record.media_type == metadata.media_type
}

async fn mark_repair(
    state: &RouteState,
    token: SessionTokenHash,
    course: CourseId,
    banner: Option<CourseBannerReference>,
    object_id: question_model::ObjectId,
) {
    let _ = state
        .banners
        .require_course_banner_object_repair(token, course, banner, object_id)
        .await;
}

async fn compensate_prepared(
    state: &RouteState,
    token: SessionTokenHash,
    course: CourseId,
    banner: CourseBannerReference,
    objects: &[(&ObjectAddress, Vec<u8>, CourseBannerObjectMetadata, Uuid)],
) {
    for (address, _, _, put_work_id) in objects {
        cleanup_address_with(
            &state.banners,
            &state.objects,
            token,
            course,
            Some(banner),
            address,
            *put_work_id,
        )
        .await;
    }
}

/// Generic, side-effect ordered promotion write seam.  HTTP owns authority;
/// this function is intentionally generic so a MemoryObjectStore and a
/// fault-injecting CourseBannerStore can exercise every external-write edge.
async fn write_prepared_objects<S: CourseBannerStore, O: ObjectStore>(
    banners: &S,
    objects: &O,
    token: SessionTokenHash,
    course: CourseId,
    banner: CourseBannerReference,
    prepared: &[(&ObjectAddress, Vec<u8>, CourseBannerObjectMetadata, Uuid)],
) -> Result<(), ()> {
    for (address, bytes, metadata, _) in prepared {
        let record = objects
            .put(PutObject {
                address: (*address).clone(),
                bytes: bytes.clone(),
                media_type: metadata.media_type.clone(),
                created_at: now(),
            })
            .await
            .map_err(|_| ())?;
        if !record_matches_metadata(&record, address, metadata) {
            return Err(());
        }
        banners
            .complete_prepared_course_banner_object(token, course, banner, metadata.object_id)
            .await
            .map_err(|_| ())?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // exact staged object facts are independently authenticated
async fn finalize_staged_upload<S: CourseBannerStore, O: ObjectStore>(
    banners: &S,
    objects: &O,
    token: SessionTokenHash,
    course: CourseId,
    upload: CourseBannerUploadReference,
    put_work_id: Uuid,
    address: &ObjectAddress,
    bytes: Vec<u8>,
    media_type: String,
    metadata: &CourseBannerObjectMetadata,
) -> Result<(), StoreError> {
    let put = objects
        .put(PutObject {
            address: address.clone(),
            bytes,
            media_type,
            created_at: now(),
        })
        .await;
    let valid = matches!(&put, Ok(record) if record_matches_metadata(record, address, metadata));
    if !valid {
        let _ = banners
            .require_course_banner_object_repair(token, course, None, metadata.object_id)
            .await;
        return Err(StoreError::Unavailable(
            "Course Banner stage object put failed".to_string(),
        ));
    }
    match banners
        .finalize_course_banner_upload_stage(token, course, upload)
        .await
    {
        Ok(()) => Ok(()),
        Err(error) => {
            cleanup_address_with(banners, objects, token, course, None, address, put_work_id).await;
            Err(error)
        }
    }
}

async fn cleanup_finalized_promotion(
    state: &RouteState,
    token: SessionTokenHash,
    course: CourseId,
    finalized: &FinalizedCourseBannerPromotion,
) {
    cleanup_address(
        state,
        token,
        course,
        None,
        &finalized.upload,
        finalized.upload_put_work_id,
    )
    .await;
    if let Some(retired) = &finalized.retired {
        cleanup_removal(state, token, course, retired).await;
    }
}

async fn cleanup_removal(
    state: &RouteState,
    token: SessionTokenHash,
    course: CourseId,
    removal: &PreparedCourseBannerRemoval,
) {
    for (address, put_work_id) in [
        (&removal.source, removal.source_put_work_id),
        (&removal.rendition, removal.rendition_put_work_id),
    ] {
        cleanup_address(
            state,
            token,
            course,
            Some(removal.banner),
            address,
            put_work_id,
        )
        .await;
    }
}

async fn cleanup_address(
    state: &RouteState,
    token: SessionTokenHash,
    course: CourseId,
    banner: Option<CourseBannerReference>,
    address: &ObjectAddress,
    put_work_id: Uuid,
) {
    cleanup_address_with(
        &state.banners,
        &state.objects,
        token,
        course,
        banner,
        address,
        put_work_id,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn cleanup_address_with<S: CourseBannerStore, O: ObjectStore>(
    banners: &S,
    objects: &O,
    token: SessionTokenHash,
    _course: CourseId,
    _banner: Option<CourseBannerReference>,
    address: &ObjectAddress,
    put_work_id: Uuid,
) {
    let Ok(delete_work) = banners
        .prepare_course_banner_object_deletion(token, put_work_id)
        .await
    else {
        return;
    };
    match objects.delete(address).await {
        Ok(()) => {
            if banners
                .complete_course_banner_object_deletion(token, delete_work)
                .await
                .is_err()
            {
                let _ = banners
                    .require_course_banner_deletion_repair(token, delete_work)
                    .await;
            }
        }
        Err(_) => {
            // A check is meaningful only after the exact delete work is
            // repair-required; the database then links its factual outcome
            // into the established cleanup-manifest lineage.
            if banners
                .require_course_banner_deletion_repair(token, delete_work)
                .await
                .is_err()
            {
                return;
            }
            match objects.get(address).await {
                // The only two check facts we can state truthfully after a failed
                // delete are absence, or a freshly verified stored checksum.
                Err(objects::ObjectStoreError::NotFound) => {
                    let _ = banners
                        .record_course_banner_cleanup_check(token, delete_work, false, None)
                        .await;
                }
                Ok(stored) => {
                    let _ = banners
                        .record_course_banner_cleanup_check(
                            token,
                            delete_work,
                            true,
                            Some(stored.record.sha256),
                        )
                        .await;
                }
                // The already-successful repair transition remains actionable
                // when a truthful probe is unavailable.
                Err(_) => {}
            }
        }
    }
}

async fn authenticated_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        // Deliberately accept every authenticated Product Role here.  The
        // Store's Course Membership predicate grants the actual read access.
        Ok(session) => Ok(session.session_hash),
        // ASVS 1.2.3 and 8.2.2: anonymous and nonmember callers are
        // indistinguishable at this route boundary.
        Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Course Appearance unavailable",
        ))),
    }
}

async fn instructor_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        // ASVS 4.1.1: Product Role is a fast route gate.  The Store still
        // invokes current_session_account_is_course_instructor for the exact
        // Course Membership authorization boundary.
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Course Appearance unavailable",
        ))),
    }
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

fn has_content_type(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case(expected))
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        // ASVS 1.2.3 and 8.2.2: neither an absent Course nor a foreign
        // membership is distinguishable through this Course-scoped reader.
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::LifecycleConflict => {
            route_error(StatusCode::CONFLICT, "Course Appearance lifecycle conflict")
        }
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Course Appearance changed")
        }
        StoreError::InvalidRecord(_)
        | StoreError::AlreadyExists
        | StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Course Appearance unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Course Appearance not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

#[cfg(test)]
#[cfg(test)]
#[path = "course_appearance/tests.rs"]
mod tests;
