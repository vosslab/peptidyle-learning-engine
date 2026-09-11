//! Student Assignment Attempt context route and browser-safe projection.

use std::str::FromStr;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use learning_data_access::LiveAssignmentDeliveryStore;
use question_model::{AssignmentAttemptReference, AssignmentReference, CourseInstanceReference};
use serde::Serialize;

use super::{StateData, concealed, store_error, student};

pub(super) async fn student_context(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(assignment_attempt): Path<String>,
) -> Response {
    let assignment_attempt = match AssignmentAttemptReference::from_str(&assignment_attempt) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    match state
        .delivery
        .student_assignment_attempt_context(token, assignment_attempt)
        .await
    {
        // ASVS 2.2.1/2.2.2 and 4.1.1: the typed route reference is validated
        // before the trusted store boundary; only its minimal projection is JSON.
        Ok(value) => crate::auth::no_store(
            Json(StudentAssignmentAttemptContextResponse::from(value)).into_response(),
        ),
        Err(value) => {
            tracing::error!(
                assignment_attempt = %assignment_attempt,
                error = %value,
                "Student Assignment Attempt context store read failed"
            );
            store_error(value)
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudentAssignmentAttemptContextResponse {
    assignment_attempt: AssignmentAttemptReference,
    attempt_number: u32,
    timer_remaining_milliseconds: Option<u64>,
    course: StudentAssignmentAttemptCourseContext,
    assignment: StudentAssignmentAttemptAssignmentContext,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudentAssignmentAttemptCourseContext {
    reference: CourseInstanceReference,
    short_name: String,
    long_name: String,
    theme: question_model::CourseTheme,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudentAssignmentAttemptAssignmentContext {
    reference: AssignmentReference,
    title: String,
}

impl From<learning_data_access::StudentAssignmentAttemptContext>
    for StudentAssignmentAttemptContextResponse
{
    fn from(value: learning_data_access::StudentAssignmentAttemptContext) -> Self {
        Self {
            assignment_attempt: value.assignment_attempt,
            attempt_number: value.attempt_number,
            timer_remaining_milliseconds: value.timer_remaining_milliseconds,
            course: StudentAssignmentAttemptCourseContext {
                reference: value.course,
                short_name: value.course_short_name,
                long_name: value.course_long_name,
                theme: value.course_theme,
            },
            assignment: StudentAssignmentAttemptAssignmentContext {
                reference: value.assignment,
                title: value.assignment_title,
            },
        }
    }
}
