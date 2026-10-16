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
        StudentAttemptHistoryEntry, StudentAttemptHistoryFacetChoice,
        StudentAttemptHistoryFacetPage, StudentAttemptHistoryPage,
    },
    student_course_progress::{AssessmentPointScore, StudentCourseProgressAssessment},
    student_course_response_stats::{
        StudentCourseResponseQuestionStats, StudentCourseResponseStats,
    },
    student_latest_feedback::StudentLatestFeedback,
};
use learning_data_access::{
    Cursor, LiveStudentAttemptHistoryEntry as StoreAttemptHistoryEntry,
    LiveStudentCourseInvitationSummary, LiveStudentCourseLandingStore,
    LiveStudentCourseProgressAssessment,
    LiveStudentCourseResponseQuestionStats as StoreResponseQuestionStats, PageRequest, PageSize,
    SessionTokenHash, StoreError, StudentAttemptHistoryQuery, StudentAttemptHistorySort,
    StudentAttemptHistoryState,
    postgres::{PostgresLiveStudentCourseLandingStore, PostgresSessionStore},
};
use question_model::{
    AssessmentAttemptCompletion, AssessmentId, AssessmentType, CourseInstanceId, CourseTerm,
    Timestamp, UserRole,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
            "/api/student/assessment-attempts",
            get(list_student_attempt_history),
        )
        .route(
            "/api/student/assessment-attempts/facets",
            get(list_student_attempt_history_facets),
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
    search: Option<String>,
    state: Option<StudentAttemptHistoryState>,
    courses: Option<String>,
    sort: Option<StudentAttemptHistorySort>,
    field: Option<String>,
    choice_search: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AttemptHistoryCursor {
    version: u8,
    page_size: u16,
    query_fingerprint: String,
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
    // and returns only the public Course Instance ID, names, public Instructor
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
        Ok(target) => crate::auth::no_store(
            Json(target.map(|target| StudentLatestFeedback {
                assessment_attempt_id: target.assessment_attempt_id,
                course_instance_id: target.course_instance_id,
            }))
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn list_student_attempt_history(
    State(state): State<RouteState>,
    headers: HeaderMap,
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
    let session_hash = match student_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if query.field.is_some() || query.choice_search.is_some() {
        return route_error(
            StatusCode::BAD_REQUEST,
            "Student Attempt History page is invalid",
        );
    }
    let search = match student_attempt_history_query(&query) {
        Some(value) => value,
        None => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Student Attempt History query is invalid",
            );
        }
    };
    let fingerprint = attempt_history_fingerprint(&search, "page", session_hash);
    let page = match attempt_history_page_request(&query, &fingerprint, false) {
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
        .list_student_attempt_history(session_hash, search, page)
        .await
    {
        Ok(page) => {
            let next_cursor = match page.next_cursor {
                Some(after) => {
                    match encode_attempt_history_cursor(&fingerprint, page_size, after) {
                        Some(value) => Some(value),
                        None => {
                            return route_error(
                                StatusCode::SERVICE_UNAVAILABLE,
                                "Student Attempt History unavailable",
                            );
                        }
                    }
                }
                None => None,
            };
            crate::auth::no_store(
                Json(StudentAttemptHistoryPage {
                    items: page.items.into_iter().map(attempt_history_entry).collect(),
                    next_cursor,
                })
                .into_response(),
            )
        }
        Err(error) => store_error_response(error),
    }
}

async fn list_student_attempt_history_facets(
    State(state): State<RouteState>,
    headers: HeaderMap,
    query: Result<Query<AttemptHistoryQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(value) => value,
        Err(_) => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Attempt History facet query is invalid",
            );
        }
    };
    let session_hash = match student_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Some(field) = query
        .field
        .as_deref()
        .filter(|value| matches!(*value, "course" | "state"))
    else {
        return route_error(
            StatusCode::BAD_REQUEST,
            "Attempt History facet field is invalid",
        );
    };
    let choice_search = query.choice_search.as_deref().unwrap_or("").trim();
    if choice_search.chars().count() > 100 {
        return route_error(
            StatusCode::BAD_REQUEST,
            "Attempt History facet query is invalid",
        );
    }
    let search = match student_attempt_history_query(&query) {
        Some(value) => value,
        None => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Attempt History facet query is invalid",
            );
        }
    };
    let fingerprint = attempt_history_fingerprint(
        &search,
        &format!("facet:{field}:{choice_search}"),
        session_hash,
    );
    let page = match attempt_history_page_request(&query, &fingerprint, true) {
        Some(value) => value,
        None => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Attempt History facet page is invalid",
            );
        }
    };
    let page_size = page.size.get();
    match state
        .landing
        .list_student_attempt_history_facet_choices(
            session_hash,
            search,
            field,
            choice_search,
            page,
        )
        .await
    {
        Ok(page) => {
            let next_cursor = match page.next_cursor {
                Some(after) => {
                    match encode_attempt_history_cursor(&fingerprint, page_size, after) {
                        Some(value) => Some(value),
                        None => {
                            return route_error(
                                StatusCode::SERVICE_UNAVAILABLE,
                                "Student Attempt History unavailable",
                            );
                        }
                    }
                }
                None => None,
            };
            crate::auth::no_store(
                Json(StudentAttemptHistoryFacetPage {
                    choices: page
                        .items
                        .into_iter()
                        .map(|choice| StudentAttemptHistoryFacetChoice {
                            key: choice.key,
                            label: choice.label,
                            count: choice.count,
                        })
                        .collect(),
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
        question_title: stats.question_title,
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

fn student_attempt_history_query(
    query: &AttemptHistoryQuery,
) -> Option<StudentAttemptHistoryQuery> {
    let search = query.search.as_deref().unwrap_or("").trim();
    if search.chars().count() > 200 {
        return None;
    }
    let mut course_ids = match query.courses.as_deref() {
        Some(value) if !value.is_empty() => value
            .split(',')
            .map(CourseInstanceId::from_str)
            .collect::<Result<Vec<_>, _>>()
            .ok()?,
        Some(_) => return None,
        None => Vec::new(),
    };
    if course_ids.len() > 100 {
        return None;
    }
    course_ids.sort_by_key(CourseInstanceId::as_string);
    course_ids.dedup();
    Some(StudentAttemptHistoryQuery {
        search: search.to_owned(),
        state: query.state.unwrap_or(StudentAttemptHistoryState::All),
        course_ids,
        sort: query
            .sort
            .unwrap_or(StudentAttemptHistorySort::StartedNewest),
    })
}

fn attempt_history_fingerprint(
    query: &StudentAttemptHistoryQuery,
    scope: &str,
    session_hash: SessionTokenHash,
) -> String {
    let value = serde_json::to_vec(&(query, scope, session_hash.to_string()))
        .expect("Attempt History query serializes");
    URL_SAFE_NO_PAD.encode(Sha256::digest(value))
}

fn attempt_history_page_request(
    query: &AttemptHistoryQuery,
    fingerprint: &str,
    facet: bool,
) -> Option<PageRequest> {
    let size = PageSize::new(query.page_size.unwrap_or(50)).ok()?;
    if facet && size.get() > 50 {
        return None;
    }
    let after = match query.cursor.as_ref() {
        Some(token) => {
            if token.len() > 512 {
                return None;
            }
            let bytes = URL_SAFE_NO_PAD.decode(token).ok()?;
            let cursor: AttemptHistoryCursor = serde_json::from_slice(&bytes).ok()?;
            if cursor.version != 1
                || cursor.query_fingerprint != fingerprint
                || cursor.page_size != size.get()
                || (facet && cursor.after.is_empty())
                || (!facet && !valid_attempt_history_key(&cursor.after))
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
    uuid::Uuid::parse_str(key).is_ok_and(|value| value.to_string() == key)
}

fn encode_attempt_history_cursor(
    fingerprint: &str,
    page_size: u16,
    after: Cursor,
) -> Option<String> {
    serde_json::to_vec(&AttemptHistoryCursor {
        version: 1,
        page_size,
        query_fingerprint: fingerprint.to_owned(),
        after: after.as_str().to_owned(),
    })
    .ok()
    .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
}

fn attempt_history_entry(entry: StoreAttemptHistoryEntry) -> StudentAttemptHistoryEntry {
    StudentAttemptHistoryEntry {
        course_instance_id: entry.course_instance_id,
        course_short_name: entry.course_short_name,
        course_long_name: entry.course_long_name,
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
        Ok(session) if student_profile_role_is_allowed(session.record.user_role) => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Student Course landing authentication unavailable",
        ))),
    }
}

fn student_profile_role_is_allowed(role: UserRole) -> bool {
    role == UserRole::Student
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
