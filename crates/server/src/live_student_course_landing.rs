//! Student Course and released Assignment landing routes.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    body::to_bytes,
    extract::Request,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use browser_api_contract::student_assignment_decision::StudentAssignmentDecisionSummary;
use learning_data_access::{
    AccountTimeZoneStore, LiveStudentCourseLandingStore, SessionTokenHash, StoreError,
    postgres::{
        PostgresAccountTimeZoneStore, PostgresLiveStudentCourseLandingStore, PostgresSessionStore,
    },
};
use question_model::{
    AccountTimeZone, AssignmentAttemptCompletion, AssignmentReference, CourseInstanceReference,
    ProductRole,
};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    landing: PostgresLiveStudentCourseLandingStore,
    time_zones: PostgresAccountTimeZoneStore,
}

const MAX_STUDENT_TIME_ZONE_UPDATE_BYTES: usize = 256;

/// Registers Student-only Course and released Assignment landing routes.
pub fn live_student_course_landing_router(
    sessions: Arc<PostgresSessionStore>,
    landing: PostgresLiveStudentCourseLandingStore,
    time_zones: PostgresAccountTimeZoneStore,
) -> Router {
    Router::new()
        .route("/api/student/course-instances", get(list_courses))
        .route(
            "/api/student/course-invitations",
            get(list_pending_invitations),
        )
        .route(
            "/api/student/profile/time-zone",
            get(read_student_time_zone).put(update_student_time_zone),
        )
        .route(
            "/api/course-instances/{course}/assignment-landing",
            get(list_assignments),
        )
        .with_state(RouteState {
            sessions,
            landing,
            time_zones,
        })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudentTimeZoneProfile {
    time_zone: AccountTimeZone,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateStudentTimeZoneInput {
    time_zone: AccountTimeZone,
}

#[derive(Serialize)]
struct CourseListResponse {
    courses: Vec<CourseSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CourseSummary {
    reference: CourseInstanceReference,
    short_name: String,
    long_name: String,
}

#[derive(Serialize)]
struct CourseInvitationListResponse {
    invitations: Vec<CourseInvitationSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CourseInvitationSummary {
    reference: CourseInstanceReference,
    short_name: String,
    long_name: String,
}

#[derive(Serialize)]
struct AssignmentListResponse {
    assignments: Vec<AssignmentSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentSummary {
    reference: AssignmentReference,
    title: String,
    decision: StudentAssignmentDecisionSummary,
    assignment_attempt_number: Option<u32>,
    assignment_attempt_completion: Option<AssignmentAttemptCompletion>,
    graded_question_count: u32,
    question_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<learning_data_access::LiveAssignmentAttemptScore>,
}

async fn read_student_time_zone(State(state): State<RouteState>, headers: HeaderMap) -> Response {
    let session_hash = match student_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .time_zones
        .authenticated_student_time_zone(session_hash)
        .await
    {
        Ok(time_zone) => {
            crate::auth::no_store(Json(StudentTimeZoneProfile { time_zone }).into_response())
        }
        Err(error) => student_time_zone_error_response(error),
    }
}

async fn update_student_time_zone(State(state): State<RouteState>, request: Request) -> Response {
    let session_hash = match student_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 4.1.1 and 5.1.1: authorize before reading one bounded, closed JSON body.
    if !has_content_type(request.headers(), "application/json") {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Student time zone is invalid",
        );
    }
    let input = match to_bytes(request.into_body(), MAX_STUDENT_TIME_ZONE_UPDATE_BYTES).await {
        Ok(bytes) => match serde_json::from_slice::<UpdateStudentTimeZoneInput>(&bytes) {
            Ok(input) => input,
            Err(_) => {
                return route_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Student time zone is invalid",
                );
            }
        },
        Err(_) => {
            return route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Student time zone is too large",
            );
        }
    };
    match state
        .time_zones
        .update_authenticated_student_time_zone(session_hash, input.time_zone)
        .await
    {
        Ok(time_zone) => {
            crate::auth::no_store(Json(StudentTimeZoneProfile { time_zone }).into_response())
        }
        Err(error) => student_time_zone_error_response(error),
    }
}

async fn list_courses(State(state): State<RouteState>, headers: HeaderMap) -> Response {
    let session_hash = match student_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.landing.list_live_student_courses(session_hash).await {
        Ok(courses) => crate::auth::no_store(
            Json(CourseListResponse {
                courses: courses
                    .into_iter()
                    .map(|course| CourseSummary {
                        reference: course.course,
                        short_name: course.short_name,
                        long_name: course.long_name,
                    })
                    .collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn list_pending_invitations(State(state): State<RouteState>, headers: HeaderMap) -> Response {
    let session_hash = match student_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 2.2.2: the trusted persistence boundary derives the current Student
    // and returns only the public Course reference and learner-facing title.
    match state
        .landing
        .list_pending_live_student_course_invitations(session_hash)
        .await
    {
        // ASVS 4.1.1: Json supplies the matching application/json Content-Type.
        Ok(invitations) => crate::auth::no_store(
            Json(CourseInvitationListResponse {
                invitations: invitations
                    .into_iter()
                    .map(|invitation| CourseInvitationSummary {
                        reference: invitation.course,
                        short_name: invitation.short_name,
                        long_name: invitation.long_name,
                    })
                    .collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn list_assignments(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(course): Path<String>,
) -> Response {
    let course = match CourseInstanceReference::from_str(&course) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match student_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 2.2.1/2.3.1 and 14.2.6: the Store procedure repeats active Student
    // and exact Course Membership authorization, then returns only effective
    // Student-visible values without Accommodation identity.
    match state
        .landing
        .list_released_live_student_assignments(session_hash, course)
        .await
    {
        Ok(assignments) => crate::auth::no_store(
            Json(AssignmentListResponse {
                assignments: assignments
                    .into_iter()
                    .map(|assignment| AssignmentSummary {
                        reference: assignment.assignment,
                        title: assignment.title,
                        decision: assignment.decision,
                        assignment_attempt_number: assignment.assignment_attempt_number,
                        assignment_attempt_completion: assignment.assignment_attempt_completion,
                        graded_question_count: assignment.graded_question_count,
                        question_count: assignment.question_count,
                        score: assignment.score,
                    })
                    .collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn student_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if student_profile_role_is_allowed(session.record.product_role) => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student Course landing authentication unavailable",
        ))),
    }
}

fn student_profile_role_is_allowed(role: ProductRole) -> bool {
    role == ProductRole::Student
}

fn has_content_type(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(expected))
        })
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Student Course landing changed",
        ),
        StoreError::LifecycleConflict => route_error(
            StatusCode::CONFLICT,
            "Student Course landing lifecycle conflict",
        ),
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Student Course landing is invalid",
        ),
        StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Student Course landing conflict")
        }
        StoreError::AssignmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student Course landing unavailable",
        ),
    }
}

fn student_time_zone_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Student time zone is invalid",
        ),
        StoreError::Conflict | StoreError::RetryableTransaction | StoreError::LifecycleConflict => {
            route_error(StatusCode::CONFLICT, "Student time zone changed")
        }
        StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Student time zone conflict")
        }
        StoreError::AssignmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student time zone unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Student Course landing not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

#[cfg(test)]
mod tests {
    use question_model::ProductRole;

    use super::student_profile_role_is_allowed;

    #[test]
    fn student_time_zone_route_admits_only_the_student_product_role() {
        assert!(student_profile_role_is_allowed(ProductRole::Student));
        assert!(!student_profile_role_is_allowed(ProductRole::Instructor));
        assert!(!student_profile_role_is_allowed(ProductRole::Sysadmin));
    }
}
