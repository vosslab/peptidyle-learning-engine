//! Student Course and released Assessment landing routes.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use browser_api_contract::{
    student_assessment_decision::StudentAssessmentDecisionSummary,
    student_course_active_attempt::StudentCourseActiveAttempt,
    student_course_attempt_history::{
        StudentCourseAttemptHistoryEntry, StudentCourseAttemptHistoryPage,
    },
    student_course_progress::{AssessmentPointScore, StudentCourseProgressAssessment},
    student_course_response_stats::{
        StudentCourseResponseQuestionStats, StudentCourseResponseStats,
    },
    student_latest_feedback::StudentLatestFeedback,
};
use learning_data_access::{
    Cursor, LiveStudentCourseAttemptHistoryEntry as StoreAttemptHistoryEntry,
    LiveStudentCourseInvitationSummary, LiveStudentCourseLandingStore,
    LiveStudentCourseProgressAssessment,
    LiveStudentCourseResponseQuestionStats as StoreResponseQuestionStats, PageRequest, PageSize,
    SessionTokenHash, StoreError,
    postgres::{PostgresLiveStudentCourseLandingStore, PostgresSessionStore},
};
use question_model::{
    AssessmentAttemptCompletion, AssessmentId, AssessmentType, CourseInstanceId, CourseTerm,
    ProductRole, Timestamp,
};
use serde::{Deserialize, Serialize};

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
            "/api/course-instances/{course_instance_id}/assessment-landing",
            get(list_assessments),
        )
        .route(
            "/api/student/course-instances/{course_instance_id}/progress",
            get(list_course_progress),
        )
        .route(
            "/api/student/course-instances/{course_instance_id}/active-attempt",
            get(read_course_active_attempt),
        )
        .route("/api/student/latest-feedback", get(read_latest_feedback))
        .route(
            "/api/student/course-instances/{course_instance_id}/assessment-attempts",
            get(list_course_attempt_history),
        )
        .route(
            "/api/student/course-instances/{course_instance_id}/response-stats",
            get(list_course_response_stats),
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
    id: CourseInstanceId,
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
    id: CourseInstanceId,
    short_name: String,
    long_name: String,
    instructor_display_name: String,
    term: CourseTerm,
}

impl From<LiveStudentCourseInvitationSummary> for CourseInvitationSummary {
    fn from(invitation: LiveStudentCourseInvitationSummary) -> Self {
        Self {
            id: invitation.course_instance_id,
            short_name: invitation.short_name,
            long_name: invitation.long_name,
            instructor_display_name: invitation.instructor_display_name,
            term: invitation.term,
        }
    }
}

#[derive(Serialize)]
struct AssessmentListResponse {
    assessments: Vec<AssessmentSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AssessmentSummary {
    id: AssessmentId,
    title: String,
    assessment_type: AssessmentType,
    decision: StudentAssessmentDecisionSummary,
    assessment_attempt_number: Option<u32>,
    assessment_attempt_completion: Option<AssessmentAttemptCompletion>,
    can_resume_assessment_attempt: bool,
    graded_question_count: u32,
    saved_question_count: u32,
    question_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    assessment_score: Option<learning_data_access::LiveAssessmentGradeContribution>,
}

#[derive(Serialize)]
struct CourseProgressResponse {
    assessments: Vec<StudentCourseProgressAssessment>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AttemptHistoryQuery {
    page_size: Option<u16>,
    cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AttemptHistoryCursor {
    version: u8,
    course_instance_id: CourseInstanceId,
    page_size: u16,
    after: String,
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
                        id: course.course_instance_id,
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
    // and returns only the public Course Instance ID, names, verified Instructor
    // display name, and Course term.
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
                    .map(CourseInvitationSummary::from)
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
    let course = match CourseInstanceId::from_str(&course) {
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
                        id: assessment.assessment_id,
                        title: assessment.title,
                        assessment_type: assessment.assessment_type,
                        decision: assessment.decision,
                        assessment_attempt_number: assessment.assessment_attempt_number,
                        assessment_attempt_completion: assessment.assessment_attempt_completion,
                        can_resume_assessment_attempt: assessment.can_resume_assessment_attempt,
                        graded_question_count: assessment.graded_question_count,
                        saved_question_count: assessment.saved_question_count,
                        question_count: assessment.question_count,
                        assessment_score: assessment.assessment_score,
                    })
                    .collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn list_course_progress(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(course): Path<String>,
) -> Response {
    let course = match CourseInstanceId::from_str(&course) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match student_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 2.2.2/2.3.1 and 14.2.6: the Store re-derives the signed-in Student,
    // checks exact active Course Membership, and returns only released scores.
    match state
        .landing
        .list_live_student_course_progress(session_hash, course)
        .await
    {
        Ok(assessments) => crate::auth::no_store(
            Json(CourseProgressResponse {
                assessments: assessments
                    .into_iter()
                    .map(|assessment: LiveStudentCourseProgressAssessment| {
                        let assessment_score_is_latest_attempt = assessment
                            .assessment_score
                            .as_ref()
                            .map(|_| assessment.assessment_score_is_latest_attempt);
                        StudentCourseProgressAssessment {
                            id: assessment.assessment_id,
                            title: assessment.title,
                            assessment_type: assessment.assessment_type,
                            assessment_attempt_count: assessment.assessment_attempt_count,
                            submitted_assessment_attempt_count: assessment
                                .submitted_assessment_attempt_count,
                            latest_assessment_attempt_number: assessment
                                .latest_assessment_attempt_number,
                            latest_assessment_attempt_completion: assessment
                                .latest_assessment_attempt_completion,
                            latest_activity_at: assessment
                                .latest_activity_at_millis
                                .map(Timestamp::from_unix_millis),
                            assessment_score: assessment.assessment_score.map(|score| {
                                AssessmentPointScore {
                                    points_earned: score.points_earned,
                                    points_possible: score.points_possible,
                                }
                            }),
                            assessment_score_is_latest_attempt,
                        }
                    })
                    .collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn read_course_active_attempt(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(course): Path<String>,
) -> Response {
    let course = match CourseInstanceId::from_str(&course) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match student_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .landing
        .get_live_student_course_active_attempt(session_hash, course)
        .await
    {
        Ok(active_attempt) => crate::auth::no_store(
            Json(StudentCourseActiveAttempt {
                assessment_attempt_id: active_attempt.map(|attempt| attempt.assessment_attempt_id),
                started_at: active_attempt.map(|attempt| attempt.started_at),
                latest_activity_at: active_attempt.map(|attempt| attempt.latest_activity_at),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn read_latest_feedback(State(state): State<RouteState>, headers: HeaderMap) -> Response {
    let session_hash = match student_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .landing
        .get_live_student_latest_feedback_attempt(session_hash)
        .await
    {
        Ok(assessment_attempt_id) => crate::auth::no_store(
            Json(StudentLatestFeedback {
                assessment_attempt_id,
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn list_course_attempt_history(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(course): Path<String>,
    query: Result<Query<AttemptHistoryQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(value) => value,
        Err(_) => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Student Attempt History page is invalid",
            );
        }
    };
    let course = match CourseInstanceId::from_str(&course) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match student_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let page = match attempt_history_page_request(&course, &query) {
        Some(value) => value,
        None => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Student Attempt History page is invalid",
            );
        }
    };
    let page_size = page.size.get();
    match state
        .landing
        .list_live_student_course_attempt_history(session_hash, course.clone(), page)
        .await
    {
        Ok(page) => {
            let next_cursor = match page.next_cursor {
                Some(after) => match encode_attempt_history_cursor(course, page_size, after) {
                    Some(value) => Some(value),
                    None => {
                        return route_error(
                            StatusCode::SERVICE_UNAVAILABLE,
                            "Student Attempt History unavailable",
                        );
                    }
                },
                None => None,
            };
            crate::auth::no_store(
                Json(StudentCourseAttemptHistoryPage {
                    items: page.items.into_iter().map(attempt_history_entry).collect(),
                    next_cursor,
                })
                .into_response(),
            )
        }
        Err(error) => store_error_response(error),
    }
}

async fn list_course_response_stats(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(course): Path<String>,
) -> Response {
    let course = match CourseInstanceId::from_str(&course) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let session_hash = match student_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .landing
        .list_live_student_course_response_stats(session_hash, course)
        .await
    {
        Ok(questions) => crate::auth::no_store(
            Json(StudentCourseResponseStats {
                questions: questions.into_iter().map(response_question_stats).collect(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

fn response_question_stats(
    stats: StoreResponseQuestionStats,
) -> StudentCourseResponseQuestionStats {
    StudentCourseResponseQuestionStats {
        published_question_revision_tuple: stats.published_question_revision_tuple,
        full_credit_attempt_count: stats.full_credit_attempt_count,
        partial_credit_attempt_count: stats.partial_credit_attempt_count,
        incorrect_attempt_count: stats.incorrect_attempt_count,
        unanswered_attempt_count: stats.unanswered_attempt_count,
        disclosed_attempt_count: stats.disclosed_attempt_count,
        not_full_credit_count: stats.not_full_credit_count,
        average_display_duration_ms: stats.average_display_duration_ms,
        display_duration_sample_count: stats.display_duration_sample_count,
        relevant_assessment_attempt_id: stats.relevant_assessment_attempt_id,
    }
}

fn attempt_history_page_request(
    course: &CourseInstanceId,
    query: &AttemptHistoryQuery,
) -> Option<PageRequest> {
    let size = PageSize::new(query.page_size.unwrap_or(50)).ok()?;
    let after = match query.cursor.as_ref() {
        Some(token) => {
            if token.len() > 512 {
                return None;
            }
            let bytes = URL_SAFE_NO_PAD.decode(token).ok()?;
            let cursor: AttemptHistoryCursor = serde_json::from_slice(&bytes).ok()?;
            if cursor.version != 1
                || cursor.course_instance_id != *course
                || cursor.page_size != size.get()
                || !valid_attempt_history_key(&cursor.after)
            {
                return None;
            }
            Some(Cursor::parse(cursor.after).ok()?)
        }
        None => None,
    };
    Some(PageRequest { after, size })
}

fn valid_attempt_history_key(key: &str) -> bool {
    let Some((timestamp, attempt_id)) = key.split_once('|') else {
        return false;
    };
    let bytes = timestamp.as_bytes();
    if bytes.len() != 27
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'.'
        || bytes[26] != b'Z'
    {
        return false;
    }
    if bytes.iter().enumerate().any(|(index, byte)| {
        !matches!(index, 4 | 7 | 10 | 13 | 16 | 19 | 26) && !byte.is_ascii_digit()
    }) {
        return false;
    }
    uuid::Uuid::parse_str(attempt_id).is_ok_and(|value| value.to_string() == attempt_id)
}

fn encode_attempt_history_cursor(
    course_instance_id: CourseInstanceId,
    page_size: u16,
    after: Cursor,
) -> Option<String> {
    serde_json::to_vec(&AttemptHistoryCursor {
        version: 1,
        course_instance_id,
        page_size,
        after: after.as_str().to_owned(),
    })
    .ok()
    .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
}

fn attempt_history_entry(entry: StoreAttemptHistoryEntry) -> StudentCourseAttemptHistoryEntry {
    StudentCourseAttemptHistoryEntry {
        assessment_attempt_id: entry.assessment_attempt_id,
        assessment_id: entry.assessment_id,
        assessment_title: entry.assessment_title,
        assessment_attempt_number: entry.assessment_attempt_number,
        started_at: entry.started_at,
        submitted_at: entry.submitted_at,
        assessment_score: entry.assessment_score.map(|score| AssessmentPointScore {
            points_earned: score.points_earned,
            points_possible: score.points_possible,
        }),
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
    use learning_data_access::{Cursor, PageRequest};
    use learning_data_access::{LiveStudentCourseInvitationSummary, StoreError};
    use question_model::{CourseInstanceId, CourseTerm, ProductRole};
    use serde_json::json;

    use super::{
        AttemptHistoryQuery, CourseInvitationSummary, attempt_history_page_request,
        encode_attempt_history_cursor, store_error_response, student_profile_role_is_allowed,
        valid_attempt_history_key,
    };

    #[test]
    fn pending_invitation_projects_only_pre_acceptance_course_context() {
        let invitation = LiveStudentCourseInvitationSummary {
            course_instance_id: CourseInstanceId::new("CI6F2R8TA0")
                .expect("canonical Course Instance ID"),
            short_name: "Mol Bio".into(),
            long_name: "Molecular Biology".into(),
            instructor_display_name: "Elena Voss".into(),
            term: CourseTerm::from_parts("2026-08-24", "2026-12-12").expect("Course term"),
        };
        let projection = serde_json::to_value(CourseInvitationSummary::from(invitation))
            .expect("invitation projection serializes");
        assert_eq!(
            projection,
            json!({
                "id": "CI6F2R8TA0",
                "shortName": "Mol Bio",
                "longName": "Molecular Biology",
                "instructorDisplayName": "Elena Voss",
                "term": { "startDate": "2026-08-24", "endDate": "2026-12-12" }
            })
        );
    }

    #[tokio::test]
    async fn unavailable_student_context_conceals_identity_and_disables_caching() {
        for error in [
            StoreError::NotFound,
            StoreError::Forbidden,
            StoreError::OwnershipMismatch,
        ] {
            let response = store_error_response(error);
            assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
            assert_eq!(
                response.headers()[axum::http::header::CACHE_CONTROL],
                "no-store"
            );
            let body = axum::body::to_bytes(response.into_body(), 1024)
                .await
                .expect("bounded error body");
            assert_eq!(body.as_ref(), b"Student Course landing not found");
        }
    }

    #[test]
    fn student_course_landing_admits_only_the_student_product_role() {
        assert!(student_profile_role_is_allowed(ProductRole::Student));
        assert!(!student_profile_role_is_allowed(ProductRole::Instructor));
        assert!(!student_profile_role_is_allowed(ProductRole::Sysadmin));
    }

    #[test]
    fn attempt_history_cursor_is_bound_to_course_and_page_size() {
        let course = CourseInstanceId::new("CI6F2R8TA0").expect("Course Instance ID");
        let after = Cursor::parse(
            "2026-09-23T13:14:15.000001Z|00000000-0000-0000-0000-000000000001".into(),
        )
        .expect("stable continuation key");
        assert!(valid_attempt_history_key(after.as_str()));
        assert!(!valid_attempt_history_key("2026-09-23T13:14:15Z|bad"));
        let token = encode_attempt_history_cursor(course.clone(), 25, after.clone())
            .expect("opaque cursor encodes");
        let query = AttemptHistoryQuery {
            page_size: Some(25),
            cursor: Some(token),
        };
        let page = attempt_history_page_request(&course, &query).expect("bound page parses");
        assert_eq!(
            page,
            PageRequest::after(after, learning_data_access::PageSize::new(25).unwrap())
        );
        let other_course = CourseInstanceId::new("CI7K3M2QAZ").expect("other Course ID");
        assert!(attempt_history_page_request(&other_course, &query).is_none());
        assert!(
            attempt_history_page_request(
                &course,
                &AttemptHistoryQuery {
                    page_size: Some(20),
                    cursor: query.cursor.clone(),
                }
            )
            .is_none()
        );
    }
}
