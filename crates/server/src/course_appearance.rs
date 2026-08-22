//! Authenticated course appearance and banner candidate routes (MOD-API-COURSE-APPEARANCE).

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::extract::{Path, Request, State};
use axum::http::header::{
    CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_MATCH, PRAGMA, REFERRER_POLICY,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use learning_data_access::{
    AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, AssetPublication,
    AuthoritativeTimeStore, CourseAppearanceStore, CourseBannerCleanupBatch,
    RegisterCourseBannerCandidate, SaveCourseAppearance, SessionStore, StoreError, TenantContext,
};
use objects::{
    ObjectCategory, ObjectKey, ObjectRecord, ObjectStore, ObjectStoreError, PutObject,
    Sha256Digest, StoredObject,
};
use question_model::{
    ActivityTimestamp, CourseAppearance, CourseAppearanceRevision, CourseAppearanceUpdate,
    CourseBannerCandidateId, CourseBannerCandidateReceipt, CourseBannerId, CourseBannerMutation,
    CourseId, UserRole,
};
use uuid::Uuid;

use crate::auth::{auth_error_response, no_store, resolve_request_session};

#[path = "course_appearance/image.rs"]
mod image;

use self::image::{BannerImageError, BannerImageMediaType, normalize_banner};

const MAX_BANNER_UPLOAD_BYTES: usize = 2 * 1_024 * 1_024;
const MAX_APPEARANCE_JSON_BYTES: usize = 64 * 1_024;
const CANDIDATE_LIFETIME_MILLIS: i64 = 60 * 60 * 1_000;
const CLEANUP_BATCH_SIZE: u16 = 10;
const BANNER_MEDIA_TYPE: &str = "image/webp";
const BANNER_LICENSE: &str = "tenant course branding";
const CANDIDATE_PROVENANCE: &str = "server-normalized course banner candidate";
const PROMOTED_PROVENANCE: &str = "promoted normalized course banner";

/// Builds the authenticated course-appearance route group.
pub fn router<S, O>(store: Arc<S>, objects: Arc<O>) -> Router
where
    S: AuthoritativeTimeStore + CourseAppearanceStore + SessionStore + 'static,
    O: ObjectStore + 'static,
{
    Router::new()
        .route(
            "/api/courses/{course}/appearance",
            get(get_course_appearance::<S, O>).put(put_course_appearance::<S, O>),
        )
        .route(
            "/api/courses/{course}/appearance/banner-candidates",
            post(upload_banner_candidate::<S, O>),
        )
        .route(
            "/api/course-banners/{banner}/delivery",
            post(deliver_course_banner::<S, O>),
        )
        .with_state(CourseAppearanceRouteState { store, objects })
}

struct CourseAppearanceRouteState<S, O> {
    store: Arc<S>,
    objects: Arc<O>,
}

impl<S, O> Clone for CourseAppearanceRouteState<S, O> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            objects: Arc::clone(&self.objects),
        }
    }
}

async fn get_course_appearance<S, O>(
    State(state): State<CourseAppearanceRouteState<S, O>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
) -> Response
where
    S: AuthoritativeTimeStore + CourseAppearanceStore + SessionStore + 'static,
    O: ObjectStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    match state
        .store
        .course_appearance(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
        )
        .await
    {
        Ok(Some(appearance)) => {
            cleanup_expired_course_banners(
                state.store.as_ref(),
                state.objects.as_ref(),
                authenticated.tenant_context,
            )
            .await;
            appearance_response(StatusCode::OK, appearance)
        }
        Ok(None)
        | Err(StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden) => {
            appearance_not_found()
        }
        Err(error) => appearance_store_error(error),
    }
}

async fn deliver_course_banner<S, O>(
    State(state): State<CourseAppearanceRouteState<S, O>>,
    headers: HeaderMap,
    Path(raw_banner): Path<String>,
) -> Response
where
    S: AuthoritativeTimeStore + CourseAppearanceStore + SessionStore + 'static,
    O: ObjectStore + 'static,
{
    let Ok(banner) = Uuid::parse_str(&raw_banner).map(CourseBannerId::from_uuid) else {
        return banner_not_found();
    };
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    // ASVS 3.5.3, 8.2.2, and 8.3.1: the POST is protected by the production
    // origin boundary and the trusted store rechecks the active session,
    // tenant, membership, retention state, and exact current banner pointer.
    let authorized = match state
        .store
        .authorize_course_banner_delivery(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            banner,
        )
        .await
    {
        Ok(authorized) => authorized,
        Err(StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden) => {
            return banner_not_found();
        }
        Err(error) => return appearance_store_error(error),
    };
    if !authorized_banner_record_matches(&authorized.record, authenticated.tenant_context, banner) {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "banner delivery is unavailable",
        );
    }
    let stored = match state.objects.get(&authorized.record.object.key).await {
        Ok(stored) => stored,
        Err(error) => return object_error_response(error),
    };
    if stored.record != authorized.record.object
        || stored.bytes.is_empty()
        || u64::try_from(stored.bytes.len()) != Ok(stored.record.size_bytes)
        || Sha256Digest::compute(&stored.bytes) != stored.record.sha256
    {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "banner delivery is unavailable",
        );
    }
    banner_delivery_response(stored)
}

fn authorized_banner_record_matches(
    record: &AssetDeliveryRecord,
    context: TenantContext,
    banner: CourseBannerId,
) -> bool {
    let AssetDeliveryScope::CourseBanner {
        tenant,
        course,
        banner: scoped_banner,
    } = record.scope
    else {
        return false;
    };
    matches!(
        record.object.key,
        ObjectKey::CourseBanner {
            tenant: key_tenant,
            course: key_course,
            banner: key_banner,
        } if key_tenant == tenant && key_course == course && key_banner == banner
    ) && tenant == context.tenant_id()
        && scoped_banner == banner
        && record.id == AssetDeliveryId::from_course_banner(banner)
        && record.object.bucket == objects::Bucket::PrivateContent
        && record.object.category == ObjectCategory::CourseContent
        && record.object.media_type == BANNER_MEDIA_TYPE
        && record.object.license == BANNER_LICENSE
        && record.object.provenance == PROMOTED_PROVENANCE
        && record.object.size_bytes > 0
        && record.object.size_bytes <= MAX_BANNER_UPLOAD_BYTES as u64
        && record.intrinsic_width == Some(learning_data_access::COURSE_BANNER_WIDTH)
        && record.intrinsic_height == Some(learning_data_access::COURSE_BANNER_HEIGHT)
        && record.publication == AssetPublication::Ready
        && record.pending_source.is_none()
}

fn banner_delivery_response(stored: StoredObject) -> Response {
    let content_length = HeaderValue::from_str(&stored.record.size_bytes.to_string())
        .expect("validated bounded banner size must form Content-Length");
    // ASVS 3.2.1, 3.4.4, 4.1.1, 14.2.3, and 14.3.2: the response uses one
    // closed server-owned media type, remains on the application origin, and
    // cannot be sniffed, embedded cross-origin, or retained in a cache.
    let mut response = no_store((StatusCode::OK, Body::from(stored.bytes)).into_response());
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(BANNER_MEDIA_TYPE));
    headers.insert(CONTENT_LENGTH, content_length);
    headers.insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"ple-course-banner.webp\""),
    );
    headers.insert(PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("same-origin"),
    );
    response
}

fn banner_not_found() -> Response {
    error_response(StatusCode::NOT_FOUND, "course banner not found")
}

async fn upload_banner_candidate<S, O>(
    State(state): State<CourseAppearanceRouteState<S, O>>,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: AuthoritativeTimeStore + CourseAppearanceStore + SessionStore + 'static,
    O: ObjectStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_manage_appearance(authenticated.record.subject.roles()) {
        return error_response(StatusCode::FORBIDDEN, "course appearance is read-only");
    }
    let media_type = match request_media_type(request.headers()) {
        Some(media_type) => media_type,
        None => {
            return error_response(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "banner must be JPEG, PNG, or WebP",
            );
        }
    };
    let body = match to_bytes(request.into_body(), MAX_BANNER_UPLOAD_BYTES + 1).await {
        Ok(body) if body.len() <= MAX_BANNER_UPLOAD_BYTES => body,
        Ok(_) | Err(_) => {
            return error_response(StatusCode::PAYLOAD_TOO_LARGE, "banner upload is too large");
        }
    };
    let normalized = match normalize_banner(media_type, &body) {
        Ok(normalized) => normalized,
        Err(error) => return image_error_response(error),
    };
    let now = match state
        .store
        .authoritative_time(authenticated.tenant_context)
        .await
    {
        Ok(now) => now,
        Err(error) => return appearance_store_error(error),
    };
    let Some(expires_at_millis) = now.as_unix_millis().checked_add(CANDIDATE_LIFETIME_MILLIS)
    else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "course appearance is unavailable",
        );
    };
    let candidate = CourseBannerCandidateId::generate();
    let banner = CourseBannerId::generate();
    let key = ObjectKey::CourseBannerCandidate {
        tenant: authenticated.tenant_context.tenant_id(),
        course,
        candidate,
    };
    let put = PutObject {
        key: key.clone(),
        bytes: normalized,
        media_type: BANNER_MEDIA_TYPE.to_string(),
        license: BANNER_LICENSE.to_string(),
        provenance: CANDIDATE_PROVENANCE.to_string(),
        created_at: now,
    };
    let object = match state.objects.put(put.clone()).await {
        Ok(record) if fresh_object_matches_put(&record, &put) => record,
        Ok(_) | Err(ObjectStoreError::AlreadyExists) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "banner storage returned conflicting metadata",
            );
        }
        Err(error) => return object_error_response(error),
    };
    let command = RegisterCourseBannerCandidate {
        candidate,
        object,
        banner,
        width: learning_data_access::COURSE_BANNER_WIDTH,
        height: learning_data_access::COURSE_BANNER_HEIGHT,
        expires_at: ActivityTimestamp::from_unix_millis(expires_at_millis),
    };
    if let Err(error) = state
        .store
        .register_course_banner_candidate(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
            command,
        )
        .await
    {
        if state.objects.delete(&key).await.is_err() {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "banner candidate cleanup failed",
            );
        }
        return mutation_store_error(error);
    }
    cleanup_expired_course_banners(
        state.store.as_ref(),
        state.objects.as_ref(),
        authenticated.tenant_context,
    )
    .await;
    no_store(
        (
            StatusCode::CREATED,
            Json(CourseBannerCandidateReceipt { candidate }),
        )
            .into_response(),
    )
}

async fn put_course_appearance<S, O>(
    State(state): State<CourseAppearanceRouteState<S, O>>,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: AuthoritativeTimeStore + CourseAppearanceStore + SessionStore + 'static,
    O: ObjectStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_manage_appearance(authenticated.record.subject.roles()) {
        return error_response(StatusCode::FORBIDDEN, "course appearance is read-only");
    }
    let expected_revision = match required_revision(request.headers()) {
        Ok(revision) => revision,
        Err(RevisionHeaderError::Missing) => {
            return error_response(StatusCode::PRECONDITION_REQUIRED, "If-Match is required");
        }
        Err(RevisionHeaderError::Malformed) => {
            return error_response(StatusCode::BAD_REQUEST, "If-Match is malformed");
        }
    };
    if !has_json_content_type(request.headers()) {
        return error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "appearance update must be JSON",
        );
    }
    let body = match to_bytes(request.into_body(), MAX_APPEARANCE_JSON_BYTES + 1).await {
        Ok(body) if body.len() <= MAX_APPEARANCE_JSON_BYTES => body,
        Ok(_) | Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "appearance update is too large",
            );
        }
    };
    let update: CourseAppearanceUpdate = match serde_json::from_slice(&body) {
        Ok(update) => update,
        Err(_) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "appearance update is invalid",
            );
        }
    };

    let promoted_object = match &update.banner {
        CourseBannerMutation::Replace { candidate, .. } => {
            match promote_candidate(
                &state,
                authenticated.tenant_context,
                authenticated.record.token_hash,
                course,
                *candidate,
            )
            .await
            {
                Ok(object) => Some(object),
                Err(response) => return response,
            }
        }
        CourseBannerMutation::Keep { .. } | CourseBannerMutation::Remove => None,
    };
    match state
        .store
        .save_course_appearance(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
            SaveCourseAppearance {
                expected_revision,
                update,
                promoted_object,
            },
        )
        .await
    {
        Ok(appearance) => {
            cleanup_expired_course_banners(
                state.store.as_ref(),
                state.objects.as_ref(),
                authenticated.tenant_context,
            )
            .await;
            appearance_response(StatusCode::OK, appearance)
        }
        Err(StoreError::Conflict) => error_response(
            StatusCode::PRECONDITION_FAILED,
            "course appearance changed; reload current settings",
        ),
        Err(error) => mutation_store_error(error),
    }
}

/// Runs one bounded, best-effort tenant cleanup after successful appearance
/// traffic. Expiry removes a candidate from authoring immediately; this sweep
/// reclaims its temporary bytes and any superseded immutable copy without
/// making object-store availability a prerequisite for reading a course.
async fn cleanup_expired_course_banners<S, O>(store: &S, objects: &O, context: TenantContext)
where
    S: CourseAppearanceStore,
    O: ObjectStore,
{
    let Some(batch) = CourseBannerCleanupBatch::new(CLEANUP_BATCH_SIZE) else {
        return;
    };
    let Ok(claims) = store.claim_course_banner_cleanup(context, batch).await else {
        return;
    };
    for claim in claims {
        if !cleanup_claim_is_tenant_owned(&claim, context) {
            continue;
        }
        for key in [&claim.candidate_object, &claim.promoted_object]
            .into_iter()
            .flatten()
        {
            match objects.delete(key).await {
                Ok(()) | Err(ObjectStoreError::NotFound) => {}
                Err(_) => return,
            }
        }
        let _ = store.complete_course_banner_cleanup(context, claim).await;
    }
}

fn cleanup_claim_is_tenant_owned(
    claim: &learning_data_access::CourseBannerCleanupClaim,
    context: TenantContext,
) -> bool {
    let candidate_is_valid = claim.candidate_object.as_ref().is_none_or(|key| {
        matches!(
            key,
            ObjectKey::CourseBannerCandidate {
                tenant,
                course,
                candidate,
            } if *tenant == context.tenant_id()
                && *course == claim.course
                && *candidate == claim.candidate
        )
    });
    let promoted_is_valid = claim.promoted_object.as_ref().is_none_or(|key| {
        matches!(
            key,
            ObjectKey::CourseBanner { tenant, course, .. }
                if *tenant == context.tenant_id() && *course == claim.course
        )
    });
    candidate_is_valid && promoted_is_valid
}

async fn promote_candidate<S, O>(
    state: &CourseAppearanceRouteState<S, O>,
    context: learning_data_access::TenantContext,
    session: learning_data_access::SessionTokenHash,
    course: CourseId,
    candidate: CourseBannerCandidateId,
) -> Result<ObjectRecord, Response>
where
    S: AuthoritativeTimeStore + CourseAppearanceStore + SessionStore + 'static,
    O: ObjectStore + 'static,
{
    let promotion = match state
        .store
        .course_banner_promotion(context, session, course, candidate)
        .await
    {
        Ok(promotion) => promotion,
        Err(StoreError::Conflict) => {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "banner candidate is no longer available",
            ));
        }
        Err(error) => return Err(mutation_store_error(error)),
    };
    let candidate_key = ObjectKey::CourseBannerCandidate {
        tenant: context.tenant_id(),
        course,
        candidate,
    };
    let stored = state
        .objects
        .get(&candidate_key)
        .await
        .map_err(object_error_response)?;
    if !candidate_matches_promotion(
        &stored,
        &candidate_key,
        promotion.sha256,
        promotion.size_bytes,
    ) {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "banner candidate verification failed",
        ));
    }
    let now = state
        .store
        .authoritative_time(context)
        .await
        .map_err(appearance_store_error)?;
    let put = PutObject {
        key: ObjectKey::CourseBanner {
            tenant: context.tenant_id(),
            course,
            banner: promotion.banner,
        },
        bytes: stored.bytes,
        media_type: BANNER_MEDIA_TYPE.to_string(),
        license: BANNER_LICENSE.to_string(),
        provenance: PROMOTED_PROVENANCE.to_string(),
        created_at: now,
    };
    match state.objects.put(put.clone()).await {
        Ok(record) if fresh_object_matches_put(&record, &put) => Ok(record),
        Ok(_) => Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "banner storage returned conflicting metadata",
        )),
        Err(ObjectStoreError::AlreadyExists) => {
            let existing = state
                .objects
                .get(&put.key)
                .await
                .map_err(object_error_response)?;
            if stored_object_matches_put(&existing, &put) {
                Ok(existing.record)
            } else {
                Err(error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "banner storage returned conflicting bytes",
                ))
            }
        }
        Err(error) => Err(object_error_response(error)),
    }
}

fn request_media_type(headers: &HeaderMap) -> Option<BannerImageMediaType> {
    let mut values = headers.get_all(CONTENT_TYPE).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    BannerImageMediaType::parse(value)
}

fn has_json_content_type(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(CONTENT_TYPE).iter();
    let Some(value) = values.next().and_then(|value| value.to_str().ok()) else {
        return false;
    };
    values.next().is_none()
        && value
            .split_once(';')
            .map_or(value, |(media_type, _)| media_type)
            .trim()
            .eq_ignore_ascii_case("application/json")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RevisionHeaderError {
    Missing,
    Malformed,
}

fn required_revision(headers: &HeaderMap) -> Result<CourseAppearanceRevision, RevisionHeaderError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(RevisionHeaderError::Missing);
    };
    if values.next().is_some() {
        return Err(RevisionHeaderError::Malformed);
    }
    let value = value.to_str().map_err(|_| RevisionHeaderError::Malformed)?;
    let value = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or(RevisionHeaderError::Malformed)?;
    value.parse().map_err(|_| RevisionHeaderError::Malformed)
}

fn appearance_response(status: StatusCode, appearance: CourseAppearance) -> Response {
    let revision = appearance.revision;
    let mut response = (status, Json(appearance)).into_response();
    let etag = HeaderValue::from_str(&format!("\"{revision}\""))
        .expect("validated appearance revision must form a strong ETag");
    response.headers_mut().insert(ETAG, etag);
    no_store(response)
}

fn fresh_object_matches_put(record: &ObjectRecord, put: &PutObject) -> bool {
    replay_object_matches_put(record, put) && record.created_at == put.created_at
}

fn stored_object_matches_put(stored: &StoredObject, put: &PutObject) -> bool {
    replay_object_matches_put(&stored.record, put) && stored.bytes == put.bytes
}

fn replay_object_matches_put(record: &ObjectRecord, put: &PutObject) -> bool {
    let Ok(size_bytes) = u64::try_from(put.bytes.len()) else {
        return false;
    };
    record.id == put.key.object_id()
        && record.key == put.key
        && record.bucket == put.key.bucket()
        && record.category == put.key.category()
        && record.version == put.key.version_id()
        && record.media_type == put.media_type
        && record.license == put.license
        && record.provenance == put.provenance
        && record.size_bytes == size_bytes
        && record.sha256 == Sha256Digest::compute(&put.bytes)
}

fn candidate_matches_promotion(
    stored: &StoredObject,
    key: &ObjectKey,
    sha256: Sha256Digest,
    size_bytes: u64,
) -> bool {
    stored.record.key == *key
        && stored.record.id == key.object_id()
        && stored.record.bucket == key.bucket()
        && stored.record.category == ObjectCategory::Temporary
        && stored.record.version.is_none()
        && stored.record.media_type == BANNER_MEDIA_TYPE
        && stored.record.sha256 == sha256
        && stored.record.size_bytes == size_bytes
        && u64::try_from(stored.bytes.len()) == Ok(size_bytes)
        && Sha256Digest::compute(&stored.bytes) == sha256
}

fn may_manage_appearance(roles: &[UserRole]) -> bool {
    roles
        .iter()
        .any(|role| matches!(role, UserRole::Instructor | UserRole::Sysadmin))
}

fn image_error_response(error: BannerImageError) -> Response {
    let message = match error {
        BannerImageError::Animated => "animated banners are not supported",
        BannerImageError::DecodedPixelLimit => "banner decoded dimensions are too large",
        BannerImageError::Malformed => "banner image is malformed",
        BannerImageError::TooSmall => "banner must supply a 1200 by 328 crop without upscaling",
    };
    error_response(StatusCode::UNPROCESSABLE_ENTITY, message)
}

fn mutation_store_error(error: StoreError) -> Response {
    match error {
        StoreError::Forbidden => {
            error_response(StatusCode::FORBIDDEN, "course appearance is read-only")
        }
        StoreError::NotFound | StoreError::TenantMismatch => appearance_not_found(),
        StoreError::InvalidRecord(_) => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "appearance update is invalid",
        ),
        StoreError::AlreadyExists | StoreError::Conflict => error_response(
            StatusCode::CONFLICT,
            "course appearance update conflicts with current state",
        ),
        StoreError::RunModel(_)
        | StoreError::TimedOut
        | StoreError::RetryableTransaction
        | StoreError::Unavailable(_) => appearance_store_error(error),
    }
}

fn appearance_store_error(_error: StoreError) -> Response {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "course appearance is unavailable",
    )
}

fn object_error_response(error: ObjectStoreError) -> Response {
    match error {
        ObjectStoreError::NotFound | ObjectStoreError::NotSignable => appearance_not_found(),
        ObjectStoreError::AlreadyExists
        | ObjectStoreError::ChecksumMismatch
        | ObjectStoreError::NumericOverflow
        | ObjectStoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "banner storage is unavailable",
        ),
    }
}

fn appearance_not_found() -> Response {
    error_response(StatusCode::NOT_FOUND, "course appearance not found")
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
#[path = "course_appearance/tests.rs"]
mod tests;
