//! Explicit Instructor recovery action, not ordinary browsing or restoration.

use crate::auth::{AuthError, resolve_session};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::post,
};
use learning_data_access::{
    ArchivedStudentWorkRecoveryStore, NumberedPageRequest, PageNumber, PageSize, RecoveredAttempt,
    RecoveryAssessmentChoice, RecoveryBrowse, RecoverySummary, StoreError,
    postgres::{PostgresArchivedStudentWorkRecoveryStore, PostgresSessionStore},
};
use question_model::{AssessmentAttemptId, AssessmentId, CourseInstanceId, UserRole};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    recovery: PostgresArchivedStudentWorkRecoveryStore,
}

pub fn archived_student_work_recovery_router(
    sessions: Arc<PostgresSessionStore>,
    recovery: PostgresArchivedStudentWorkRecoveryStore,
) -> Router {
    Router::new()
        .route(
            "/api/course-instances/{course_instance_id}/student-work/recovery",
            post(recover),
        )
        .with_state(RouteState { sessions, recovery })
        .layer(axum::middleware::map_response(|response: Response| async {
            crate::auth::no_store(response)
        }))
}

// ASVS 1.5.2/2.2.1: closed explicit action; no caller identity/time/authority flags.
#[derive(Deserialize)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Input {
    Select {
        page_number: Option<u32>,
        search: String,
        assessment_ids: Vec<AssessmentId>,
        sort: String,
        page_size: Option<u16>,
    },
    Recover {
        #[serde(rename = "assessmentAttemptId")]
        assessment_attempt_id: AssessmentAttemptId,
    },
}

#[derive(Serialize)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum Output {
    Select {
        course_instance_id: CourseInstanceId,
        attempts: Vec<RecoverySummary>,
        assessment_choices: Vec<RecoveryAssessmentChoice>,
        page_number: PageNumber,
        page_size: PageSize,
        total_count: u64,
    },
    Recover {
        attempt: Box<RecoveredAttempt>,
    },
}

async fn recover(
    State(state): State<RouteState>,
    Path(course): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Input>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let course = match CourseInstanceId::from_str(&course) {
        Ok(c) => c,
        Err(_) => return concealed(),
    };
    let cookie = headers
        .get_all(COOKIE)
        .iter()
        .map(|h| h.to_str().ok())
        .collect::<Option<Vec<_>>>()
        .filter(|v| !v.is_empty())
        .map(|v| v.join("; "));
    let session = match resolve_session(state.sessions.as_ref(), cookie.as_deref()).await {
        Ok(s) if s.record.user_role == UserRole::Instructor => s,
        Ok(_) | Err(AuthError::Unauthenticated) => return concealed(),
        Err(_) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Recovery authentication unavailable",
            );
        }
    };
    let input = match input {
        Ok(Json(i)) => i,
        Err(_) => return error(StatusCode::BAD_REQUEST, "Recovery request invalid"),
    };
    let result = match input {
        Input::Select {
            page_number,
            search,
            mut assessment_ids,
            sort,
            page_size,
        } => {
            let search = search.trim().to_owned();
            assessment_ids.sort_by_key(|id| id.as_string());
            assessment_ids.dedup();
            if search.chars().count() > 200
                || search.chars().any(char::is_control)
                || assessment_ids.len() > 100
                || !matches!(sort.as_str(), "attemptAscending" | "attemptDescending")
            {
                return error(StatusCode::BAD_REQUEST, "Recovery selection query invalid");
            }
            let page_number = match PageNumber::new(page_number.unwrap_or(1)) {
                Ok(value) => value,
                Err(_) => return error(StatusCode::BAD_REQUEST, "Recovery page invalid"),
            };
            let page_size = match PageSize::new(page_size.unwrap_or(50)) {
                Ok(value) => value,
                Err(_) => return error(StatusCode::BAD_REQUEST, "Recovery page invalid"),
            };
            state
                .recovery
                .select_retained_work(
                    session.session_hash,
                    course.clone(),
                    RecoveryBrowse {
                        search,
                        assessment_ids,
                        sort,
                        page: NumberedPageRequest {
                            page_number,
                            page_size,
                        },
                    },
                )
                .await
                .map(|page| {
                    let numbered = page.attempts;
                    Output::Select {
                        course_instance_id: course,
                        attempts: numbered.items,
                        assessment_choices: page.assessment_choices,
                        page_number: numbered.page_number,
                        page_size: numbered.page_size,
                        total_count: numbered.total_count,
                    }
                })
        }
        Input::Recover {
            assessment_attempt_id,
        } => state
            .recovery
            .recover_retained_work(session.session_hash, course, assessment_attempt_id)
            .await
            .map(|attempt| Output::Recover {
                attempt: Box::new(attempt),
            }),
    };
    // ASVS 14.2.2/16.5.1: no-store on every outcome and no SQL error reflection.
    match result {
        Ok(value) => crate::auth::no_store(Json(value).into_response()),
        Err(StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch) => {
            concealed()
        }
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "Recovery unavailable"),
    }
}

fn concealed() -> Response {
    error(StatusCode::NOT_FOUND, "Retained Work not found")
}
fn error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
