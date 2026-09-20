//! Course Banner stage, promotion, removal, delivery, and object cleanup.

use std::str::FromStr;

use axum::{
    Json,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use learning_data_access::{
    CourseBannerObjectMetadata, CourseBannerStore, CourseThemeStore,
    FinalizedCourseBannerPromotion, PrepareCourseBannerPromotion, PreparedCourseBannerRemoval,
    SessionTokenHash, StageCourseBannerUpload, StoreError,
};
use objects::{
    ObjectAddress, ObjectRecord, ObjectStore, PutObject, Sha256Checksum,
    image_validation::{normalized_course_banner_webp, verify_course_banner_still_image},
};
use question_model::{
    CourseAppearanceView, CourseBannerId, CourseBannerRendition, CourseBannerUpdate,
    CourseBannerUploadId, CourseBannerUploadReceipt, CourseInstanceId,
};
use uuid::Uuid;

use super::{
    MAX_BANNER_UPDATE_BYTES, RouteState, authenticated_session_hash, concealed, has_content_type,
    instructor_session_hash, now, resolve_course, route_error, store_error_response,
};

pub(super) async fn stage_banner_upload(
    State(state): State<RouteState>,
    Path(course): Path<String>,
    request: Request,
) -> Response {
    let course_instance_id = match CourseInstanceId::from_str(&course) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let course = match resolve_course(&state, token, course_instance_id).await {
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
    let upload = CourseBannerUploadId::generate();
    let media_type = verified.media_type.canonical_media_type().to_string();
    let address = ObjectAddress::CourseBannerUpload {
        course_instance_id: course.clone(),
        course_banner_upload_id: upload,
    };
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
                course_instance_id: course.clone(),
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
        course.clone(),
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

pub(super) async fn promote_banner(
    State(state): State<RouteState>,
    Path(course): Path<String>,
    request: Request,
) -> Response {
    let course_instance_id = match CourseInstanceId::from_str(&course) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let course = match resolve_course(&state, token, course_instance_id).await {
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
        .read_staged_course_banner_upload(token, course.clone(), update.upload)
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
            mark_repair(&state, token, course.clone(), None, claimed.object_id).await;
            return route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Course Appearance unavailable",
            );
        }
    };
    let banner = CourseBannerId::generate();
    let source_address = ObjectAddress::CourseBannerSource {
        course_instance_id: course.clone(),
        course_banner_id: banner,
    };
    let rendition_address = ObjectAddress::CourseBannerRendition {
        course_instance_id: course.clone(),
        course_banner_id: banner,
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
        mark_repair(&state, token, course.clone(), None, claimed.object_id).await;
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
                course_instance_id: course.clone(),
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
        course.clone(),
        banner,
        &prepared_objects,
    )
    .await
    .is_err()
    {
        compensate_prepared(&state, token, course.clone(), banner, &prepared_objects).await;
        return route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Course Appearance unavailable",
        );
    }
    let finalized = match state
        .banners
        .finalize_course_banner_promotion(token, course.clone(), update.upload, banner)
        .await
    {
        Ok(value) => value,
        Err(error) => {
            compensate_prepared(&state, token, course.clone(), banner, &prepared_objects).await;
            return store_error_response(error);
        }
    };
    cleanup_finalized_promotion(&state, token, course.clone(), &finalized).await;
    let theme = match state.themes.read_course_theme(token, course.clone()).await {
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

pub(super) async fn remove_banner(
    State(state): State<RouteState>,
    Path(course): Path<String>,
    headers: HeaderMap,
) -> Response {
    let course_instance_id = match CourseInstanceId::from_str(&course) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let course = match resolve_course(&state, token, course_instance_id).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .banners
        .prepare_course_banner_removal(token, course.clone())
        .await
    {
        Ok(removal) => {
            cleanup_removal(&state, token, course.clone(), &removal).await;
            match state.themes.read_course_theme(token, course.clone()).await {
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

pub(super) async fn deliver_banner(
    State(state): State<RouteState>,
    Path(banner): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Ok(banner) = Uuid::parse_str(&banner) else {
        return concealed();
    };
    let banner = CourseBannerId::from_uuid(banner);
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
                course_instance_id: course.clone(),
                course_banner_id: banner,
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

pub(crate) fn banner_metadata(
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
    course_instance_id: CourseInstanceId,
    banner: Option<CourseBannerId>,
    object_id: question_model::ObjectId,
) {
    let _ = state
        .banners
        .require_course_banner_object_repair(token, course_instance_id.clone(), banner, object_id)
        .await;
}

async fn compensate_prepared(
    state: &RouteState,
    token: SessionTokenHash,
    course_instance_id: CourseInstanceId,
    banner: CourseBannerId,
    objects: &[(&ObjectAddress, Vec<u8>, CourseBannerObjectMetadata, Uuid)],
) {
    for (address, _, _, put_work_id) in objects {
        cleanup_address_with(
            &state.banners,
            &state.objects,
            token,
            course_instance_id.clone(),
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
pub(crate) async fn write_prepared_objects<S: CourseBannerStore, O: ObjectStore>(
    banners: &S,
    objects: &O,
    token: SessionTokenHash,
    course_instance_id: CourseInstanceId,
    banner: CourseBannerId,
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
            .complete_prepared_course_banner_object(
                token,
                course_instance_id.clone(),
                banner,
                metadata.object_id,
            )
            .await
            .map_err(|_| ())?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // exact staged object facts are independently authenticated
pub(crate) async fn finalize_staged_upload<S: CourseBannerStore, O: ObjectStore>(
    banners: &S,
    objects: &O,
    token: SessionTokenHash,
    course_instance_id: CourseInstanceId,
    upload: CourseBannerUploadId,
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
            .require_course_banner_object_repair(
                token,
                course_instance_id.clone(),
                None,
                metadata.object_id,
            )
            .await;
        return Err(StoreError::Unavailable(
            "Course Banner stage object put failed".to_string(),
        ));
    }
    match banners
        .finalize_course_banner_upload_stage(token, course_instance_id.clone(), upload)
        .await
    {
        Ok(()) => Ok(()),
        Err(error) => {
            cleanup_address_with(
                banners,
                objects,
                token,
                course_instance_id.clone(),
                None,
                address,
                put_work_id,
            )
            .await;
            Err(error)
        }
    }
}

async fn cleanup_finalized_promotion(
    state: &RouteState,
    token: SessionTokenHash,
    course_instance_id: CourseInstanceId,
    finalized: &FinalizedCourseBannerPromotion,
) {
    cleanup_address(
        state,
        token,
        course_instance_id.clone(),
        None,
        &finalized.upload,
        finalized.upload_put_work_id,
    )
    .await;
    if let Some(retired) = &finalized.retired {
        cleanup_removal(state, token, course_instance_id.clone(), retired).await;
    }
}

async fn cleanup_removal(
    state: &RouteState,
    token: SessionTokenHash,
    course_instance_id: CourseInstanceId,
    removal: &PreparedCourseBannerRemoval,
) {
    for (address, put_work_id) in [
        (&removal.source, removal.source_put_work_id),
        (&removal.rendition, removal.rendition_put_work_id),
    ] {
        cleanup_address(
            state,
            token,
            course_instance_id.clone(),
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
    course_instance_id: CourseInstanceId,
    banner: Option<CourseBannerId>,
    address: &ObjectAddress,
    put_work_id: Uuid,
) {
    cleanup_address_with(
        &state.banners,
        &state.objects,
        token,
        course_instance_id.clone(),
        banner,
        address,
        put_work_id,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn cleanup_address_with<S: CourseBannerStore, O: ObjectStore>(
    banners: &S,
    objects: &O,
    token: SessionTokenHash,
    _course_instance_id: CourseInstanceId,
    _banner: Option<CourseBannerId>,
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
