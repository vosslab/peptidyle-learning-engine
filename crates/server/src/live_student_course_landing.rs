//! Student Course and released Assessment landing routes.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use browser_api_contract::student_assessment_decision::StudentAssessmentDecisionSummary;
use learning_data_access::{
    LiveStudentCourseLandingStore, SessionTokenHash, StoreError,
    postgres::{PostgresLiveStudentCourseLandingStore, PostgresSessionStore},
};
use question_model::{
    AssessmentAttemptCompletion, AssessmentReference, CourseInstanceReference, ProductRole,
};
use serde::Serialize;

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    landing: PostgresLiveStudentCourseLandingStore,
}

/// Registers Student-only Course and released Assessment landing routes.
pub fn live_student_course_landing_router(
    sessions: Arc<PostgresSessionStore>,
    landing: PostgresLiveStudentCourseLandingStore,
) -> Router {
    Router::new()
        .route("/api/student/course-instances", get(list_courses))
        .route(
            "/api/student/course-invitations",
            get(list_pending_invitations),
        )
        .route(
            "/api/course-instances/{course}/assessment-landing",
            get(list_assessments),
        )
        .with_state(RouteState { sessions, landing })
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
struct AssessmentListResponse {
    assessments: Vec<AssessmentSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AssessmentSummary {
    reference: AssessmentReference,
    title: String,
    decision: StudentAssessmentDecisionSummary,
    assessment_attempt_number: Option<u32>,
    assessment_attempt_completion: Option<AssessmentAttemptCompletion>,
    graded_question_count: u32,
    question_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<learning_data_access::LiveAssessmentAttemptScore>,
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

async fn list_assessments(
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
        .list_released_live_student_assessments(session_hash, course)
        .await
    {
        Ok(assessments) => crate::auth::no_store(
            Json(AssessmentListResponse {
                assessments: assessments
                    .into_iter()
                    .map(|assessment| AssessmentSummary {
                        reference: assessment.assessment,
                        title: assessment.title,
                        decision: assessment.decision,
                        assessment_attempt_number: assessment.assessment_attempt_number,
                        assessment_attempt_completion: assessment.assessment_attempt_completion,
                        graded_question_count: assessment.graded_question_count,
                        question_count: assessment.question_count,
                        score: assessment.score,
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
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student Course landing unavailable",
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
    fn student_course_landing_admits_only_the_student_product_role() {
        assert!(student_profile_role_is_allowed(ProductRole::Student));
        assert!(!student_profile_role_is_allowed(ProductRole::Instructor));
        assert!(!student_profile_role_is_allowed(ProductRole::Sysadmin));
    }
}
