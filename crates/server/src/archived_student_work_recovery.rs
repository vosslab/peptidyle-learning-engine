//! Explicit Instructor recovery action, not ordinary browsing or restoration.

use crate::auth::{AuthError, resolve_session};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::post,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::{
    ArchivedStudentWorkRecoveryStore, RecoveredAttempt, RecoverySummary, StoreError,
    postgres::{PostgresArchivedStudentWorkRecoveryStore, PostgresSessionStore},
};
use question_model::{AssessmentAttemptId, CourseInstanceId, ProductRole};
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
            "/api/course-instances/{course}/student-work/recovery",
            post(recover),
        )
        .with_state(RouteState { sessions, recovery })
        .layer(axum::middleware::map_response(|response: Response| async {
            crate::auth::no_store(response)
        }))
}

// ASVS 1.5.2/2.2.1: closed explicit action; no caller identity/time/authority flags.
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
enum Input {
    Select {
        #[serde(deserialize_with = "required_nullable_cursor")]
        cursor: Option<String>,
    },
    Recover {
        #[serde(rename = "assessmentAttempt")]
        assessment_attempt: AssessmentAttemptId,
    },
}

fn required_nullable_cursor<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}
#[derive(Serialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum Output {
    Select {
        course: CourseInstanceId,
        attempts: Vec<RecoverySummary>,
        #[serde(rename = "nextCursor")]
        next_cursor: Option<String>,
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
        Ok(s) if s.record.product_role == ProductRole::Instructor => s,
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
        Input::Select { cursor } => {
            let after = match cursor
                .as_deref()
                .map(|c| decode_cursor(c, &course))
                .transpose()
            {
                Ok(a) => a,
                Err(_) => return error(StatusCode::BAD_REQUEST, "Recovery cursor invalid"),
            };
            state
                .recovery
                .select_retained_work(session.session_hash, course.clone(), after)
                .await
                .map(|mut attempts| {
                    let next_cursor = if attempts.len() > 100 {
                        attempts.truncate(100);
                        attempts.last().map(|last| {
                            URL_SAFE_NO_PAD
                                .encode(format!("{}:{}", course, last.assessment_attempt))
                        })
                    } else {
                        None
                    };
                    Output::Select {
                        course,
                        attempts,
                        next_cursor,
                    }
                })
        }
        Input::Recover { assessment_attempt } => state
            .recovery
            .recover_retained_work(session.session_hash, course, assessment_attempt)
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

fn decode_cursor(cursor: &str, course: &CourseInstanceId) -> Result<AssessmentAttemptId, ()> {
    if cursor.len() > 80 {
        return Err(());
    }
    let bytes = URL_SAFE_NO_PAD.decode(cursor).map_err(|_| ())?;
    if URL_SAFE_NO_PAD.encode(&bytes) != cursor {
        return Err(());
    }
    let value = std::str::from_utf8(&bytes).map_err(|_| ())?;
    let (bound, attempt) = value.split_once(':').ok_or(())?;
    if bound != course.as_string() {
        return Err(());
    }
    attempt.parse().map_err(|_| ())
}
fn concealed() -> Response {
    error(StatusCode::NOT_FOUND, "Retained Work not found")
}
fn error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
