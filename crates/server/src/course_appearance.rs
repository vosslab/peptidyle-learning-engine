//! Current Course Appearance reader for every active Course Member.
//!
//! Theme storage is available before banner promotion.  The response remains
//! the complete aggregate contract, with an absent banner until the banner
//! store supplies a current banner projection.

use std::{
    str::FromStr,
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
    CourseBannerStore, CourseInstanceStore, CourseThemeStore, SessionTokenHash, StoreError,
    postgres::{
        PostgresCourseBannerStore, PostgresCourseInstanceStore, PostgresCourseThemeStore,
        PostgresSessionStore,
    },
};
use objects::s3::S3ObjectStore;
use question_model::{
    CourseAppearanceView, CourseInstanceId, CourseThemeUpdate, ProductRole, Timestamp,
};

use crate::auth::{AuthError, resolve_session};

mod banner;
use banner::{deliver_banner, promote_banner, remove_banner, stage_banner_upload};

const MAX_THEME_UPDATE_BYTES: usize = 1_024;
pub(super) const MAX_BANNER_UPDATE_BYTES: usize = 1_024;

#[derive(Clone)]
pub(super) struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    courses: PostgresCourseInstanceStore,
    themes: PostgresCourseThemeStore,
    banners: PostgresCourseBannerStore,
    objects: S3ObjectStore,
}

/// Registers the browser-safe current Course Appearance reader.
///
/// Each route resolves its canonical Course Instance course_instance_id through the
/// installed session before the Appearance Stores repeat exact Course
/// Membership authorization for the requested operation.
pub fn course_appearance_router(
    sessions: Arc<PostgresSessionStore>,
    courses: PostgresCourseInstanceStore,
    themes: PostgresCourseThemeStore,
    banners: PostgresCourseBannerStore,
    objects: S3ObjectStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course_instance_id}/appearance",
            get(read_appearance).put(update_theme),
        )
        .route(
            "/api/course-instances/{course_instance_id}/appearance/banner-uploads",
            post(stage_banner_upload),
        )
        .route(
            "/api/course-instances/{course_instance_id}/appearance/banner",
            axum::routing::put(promote_banner).delete(remove_banner),
        )
        .route(
            "/api/course-banners/{course_banner_id}/delivery",
            post(deliver_banner),
        )
        .with_state(RouteState {
            sessions,
            courses,
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
    let course_instance_id = match CourseInstanceId::from_str(&course) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let course = match resolve_course(&state, session_hash, course_instance_id).await {
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
        course.clone(),
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
    course_instance_id: CourseInstanceId,
    theme: question_model::CourseTheme,
) -> Result<CourseAppearanceView, StoreError> {
    let theme = themes
        .update_course_theme(session_hash, course_instance_id.clone().clone(), theme)
        .await?;
    let banner = banners
        .read_current_course_banner(session_hash, course_instance_id.clone())
        .await?;
    Ok(CourseAppearanceView { theme, banner })
}

async fn read_appearance(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(course): Path<String>,
) -> Response {
    let course_instance_id = match CourseInstanceId::from_str(&course) {
        Ok(value) => value,
        // ASVS 1.2.3 and 8.2.2: malformed identities receive the same
        // no-store concealment response as an authenticated nonmember.
        Err(_) => return concealed(),
    };
    let session_hash = match authenticated_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let course = match resolve_course(&state, session_hash, course_instance_id).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 1.2.3 and 8.2.2: this session-bound Store operation invokes
    // current_session_account_is_course_member; role alone never authorizes
    // a Course Appearance read.
    match state
        .themes
        .read_course_theme(session_hash, course.clone().clone())
        .await
    {
        Ok(theme) => match state
            .banners
            .read_current_course_banner(session_hash, course.clone())
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

pub(super) fn now() -> Timestamp {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(milliseconds).unwrap_or(i64::MAX))
}

pub(super) async fn resolve_course(
    state: &RouteState,
    token: SessionTokenHash,
    course_instance_id: CourseInstanceId,
) -> Result<CourseInstanceId, Box<Response>> {
    // ASVS 2.2.1, 8.2.2, and 8.3.1: the route accepts only a canonical public
    // Course Instance course_instance_id, then resolves its private ID through the
    // current session's active Course Membership before any Appearance read or
    // mutation. Each Appearance Store repeats its operation-specific check.
    state
        .courses
        .resolve_course_navigation(token, course_instance_id)
        .await
        .map_err(|error| Box::new(store_error_response(error)))
}

pub(super) async fn authenticated_session_hash(
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

pub(super) async fn instructor_session_hash(
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

pub(super) fn has_content_type(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case(expected))
}

pub(super) fn store_error_response(error: StoreError) -> Response {
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

pub(super) fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Course Appearance not found")
}

pub(super) fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

#[cfg(test)]
mod tests;
