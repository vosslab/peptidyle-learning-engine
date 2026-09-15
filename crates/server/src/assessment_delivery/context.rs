//! Student Assessment Attempt context route and browser-safe projection.

use std::str::FromStr;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use learning_data_access::LiveAssessmentDeliveryStore;
use question_model::{
    AccountTimeZone, AssessmentAttemptReference, AssessmentReference, CourseInstanceReference,
    Timestamp,
};
use serde::Serialize;

use super::{StateData, concealed, store_error, student};

pub(super) async fn student_context(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path(assessment_attempt): Path<String>,
) -> Response {
    let assessment_attempt = match AssessmentAttemptReference::from_str(&assessment_attempt) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match student(&state, &headers).await {
        Ok(value) => value,
        Err(value) => return *value,
    };
    match state
        .delivery
        .student_assessment_attempt_context(token, assessment_attempt)
        .await
    {
        // ASVS 2.2.1/2.2.2 and 4.1.1: the typed route reference is validated
        // before the trusted store boundary; only its minimal projection is JSON.
        Ok(value) => crate::auth::no_store(
            Json(StudentAssessmentAttemptContextResponse::from(value)).into_response(),
        ),
        Err(value) => {
            tracing::error!(
                assessment_attempt = %assessment_attempt,
                error = %value,
                "Student Assessment Attempt context store read failed"
            );
            store_error(value)
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudentAssessmentAttemptContextResponse {
    assessment_attempt: AssessmentAttemptReference,
    attempt_number: u32,
    display_time_zone: AccountTimeZone,
    expires_at: Option<Timestamp>,
    timer_remaining_milliseconds: Option<u64>,
    course: StudentAssessmentAttemptCourseContext,
    assessment: StudentAssessmentAttemptAssessmentContext,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudentAssessmentAttemptCourseContext {
    reference: CourseInstanceReference,
    short_name: String,
    long_name: String,
    theme: question_model::CourseTheme,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudentAssessmentAttemptAssessmentContext {
    reference: AssessmentReference,
    title: String,
}

impl From<learning_data_access::StudentAssessmentAttemptContext>
    for StudentAssessmentAttemptContextResponse
{
    fn from(value: learning_data_access::StudentAssessmentAttemptContext) -> Self {
        Self {
            assessment_attempt: value.assessment_attempt,
            attempt_number: value.attempt_number,
            display_time_zone: value.display_time_zone,
            expires_at: value.expires_at,
            timer_remaining_milliseconds: value.timer_remaining_milliseconds,
            course: StudentAssessmentAttemptCourseContext {
                reference: value.course,
                short_name: value.course_short_name,
                long_name: value.course_long_name,
                theme: value.course_theme,
            },
            assessment: StudentAssessmentAttemptAssessmentContext {
                reference: value.assessment,
                title: value.assessment_title,
            },
        }
    }
}
