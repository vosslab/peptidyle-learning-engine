//! Private owner-only Assessment Template HTTP routes.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::COOKIE, header::ETAG, header::IF_MATCH},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use learning_data_access::{
    AssessmentTemplateStore, CreateAssessmentFromTemplateInput, LiveAssessmentWorkspace,
    SaveAssessmentTemplateInput, SessionTokenHash, StoreError,
    postgres::{PostgresAssessmentTemplateStore, PostgresSessionStore},
};
use question_model::{
    AssessmentTemplate, AssessmentTemplateEditNumber, AssessmentTemplateId, AssessmentTemplateName,
    AssessmentTemplateSettings, AssessmentTitle, AssessmentType, CourseInstanceId, ProductRole,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AuthError, resolve_session};

const MAX_ASSESSMENT_TEMPLATE_REQUEST_BYTES: usize = 128 * 1024;

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    templates: Arc<dyn AssessmentTemplateStore>,
}

/// Registers the private owner-only Assessment Template CRUD subset.
pub fn assessment_template_router(
    sessions: Arc<PostgresSessionStore>,
    templates: PostgresAssessmentTemplateStore,
) -> Router {
    Router::new()
        .route(
            "/api/assessment-templates",
            get(list_templates).post(create_template),
        )
        .route(
            "/api/assessment-templates/{assessment_template_id}",
            get(read_template).put(save_template),
        )
        .route(
            "/api/course-instances/{course_instance_id}/assessments/from-template",
            post(create_assessment_from_template),
        )
        .with_state(RouteState {
            sessions,
            templates: Arc::new(templates),
        })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssessmentTemplateListResponse {
    items: Vec<AssessmentTemplate>,
}

/// The server supplies identity, defaults, ownership, and initial Edit Number.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateAssessmentTemplateRequest {
    name: AssessmentTemplateName,
    assessment_type: AssessmentType,
}

/// Full current-value replacement; the strong `If-Match` header supplies CAS.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SaveAssessmentTemplateRequest {
    name: AssessmentTemplateName,
    assessment_type: AssessmentType,
    settings: AssessmentTemplateSettings,
}

/// One owned Template source and one title for a fresh Course Assessment.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateAssessmentFromTemplateRequest {
    template_id: AssessmentTemplateId,
    title: AssessmentTitle,
}

async fn list_templates(State(state): State<RouteState>, headers: HeaderMap) -> Response {
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.templates.list_assessment_templates(session).await {
        Ok(items) => {
            crate::auth::no_store(Json(AssessmentTemplateListResponse { items }).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

async fn read_template(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(value): Path<String>,
) -> Response {
    let id = match assessment_template_id(&value) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.templates.read_assessment_template(session, id).await {
        Ok(template) => template_response(StatusCode::OK, template),
        Err(error) => store_error_response(error),
    }
}

async fn create_template(State(state): State<RouteState>, request: Request) -> Response {
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let request = match json_request::<CreateAssessmentTemplateRequest>(request).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let template = AssessmentTemplate::new(
        AssessmentTemplateId::generate(),
        request.name,
        request.assessment_type,
    );
    match state
        .templates
        .create_assessment_template(session, template)
        .await
    {
        Ok(template) => template_response(StatusCode::CREATED, template),
        Err(error) => store_error_response(error),
    }
}

async fn save_template(
    State(state): State<RouteState>,
    Path(value): Path<String>,
    request: Request,
) -> Response {
    let id = match assessment_template_id(&value) {
        Some(value) => value,
        None => return concealed(),
    };
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(request.headers()) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let request = match json_request::<SaveAssessmentTemplateRequest>(request).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 2.2.3: Assessment Type and settings are independently explicit.
    // A Type change never causes the server to overwrite supplied settings.
    let input = SaveAssessmentTemplateInput {
        id,
        expected_edit_number,
        name: request.name,
        assessment_type: request.assessment_type,
        settings: request.settings,
    };
    match state
        .templates
        .save_assessment_template(session, input)
        .await
    {
        Ok(template) => template_response(StatusCode::OK, template),
        Err(error) => store_error_response(error),
    }
}

async fn create_assessment_from_template(
    State(state): State<RouteState>,
    Path(value): Path<String>,
    request: Request,
) -> Response {
    let course = match CourseInstanceId::from_str(&value) {
        Ok(value) => value,
        Err(_) => return concealed_assessment(),
    };
    let session = match instructor_session_hash(&state, request.headers()).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let request = match json_request::<CreateAssessmentFromTemplateRequest>(request).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let input = CreateAssessmentFromTemplateInput {
        template_id: request.template_id,
        title: request.title,
    };
    match state
        .templates
        .create_assessment_from_template(session, course, input)
        .await
    {
        Ok(assessment) => assessment_response(StatusCode::CREATED, &assessment),
        Err(error) => copy_store_error_response(error),
    }
}

async fn json_request<T: for<'de> Deserialize<'de>>(request: Request) -> Result<T, Box<Response>> {
    if !has_json_content_type(request.headers()) {
        return Err(Box::new(route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Assessment Template requires JSON",
        )));
    }
    let bytes = to_bytes(request.into_body(), MAX_ASSESSMENT_TEMPLATE_REQUEST_BYTES)
        .await
        .map_err(|_| {
            Box::new(route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Assessment Template is too large",
            ))
        })?;
    // ASVS 1.5.2 and 2.2.1-2.2.2: deserialize only closed model-backed
    // request shapes, after authentication, and reject every unknown field.
    serde_json::from_slice(&bytes).map_err(|_| {
        Box::new(route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Assessment Template is invalid",
        ))
    })
}

fn assessment_template_id(value: &str) -> Option<AssessmentTemplateId> {
    Uuid::parse_str(value)
        .ok()
        .map(AssessmentTemplateId::from_uuid)
}

fn expected_edit_number(
    headers: &HeaderMap,
) -> Result<AssessmentTemplateEditNumber, Box<Response>> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next().and_then(|value| value.to_str().ok()) else {
        return Err(Box::new(route_error(
            StatusCode::PRECONDITION_REQUIRED,
            "Assessment Template Edit Number is required",
        )));
    };
    if values.next().is_some() {
        return Err(Box::new(route_error(
            StatusCode::BAD_REQUEST,
            "Assessment Template Edit Number is invalid",
        )));
    }
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|candidate| candidate.strip_suffix('"'))
    else {
        return Err(Box::new(route_error(
            StatusCode::BAD_REQUEST,
            "Assessment Template Edit Number is invalid",
        )));
    };
    AssessmentTemplateEditNumber::from_str(value).map_err(|_| {
        Box::new(route_error(
            StatusCode::BAD_REQUEST,
            "Assessment Template Edit Number is invalid",
        ))
    })
}

fn template_response(status: StatusCode, template: AssessmentTemplate) -> Response {
    let edit_number = template.edit_number;
    let mut response = crate::auth::no_store((status, Json(template)).into_response());
    match HeaderValue::from_str(&format!("\"{edit_number}\"")) {
        Ok(value) => {
            response.headers_mut().insert(ETAG, value);
            response
        }
        Err(_) => unavailable(),
    }
}

fn assessment_response(status: StatusCode, assessment: &LiveAssessmentWorkspace) -> Response {
    let mut response = crate::auth::no_store((status, Json(assessment)).into_response());
    match HeaderValue::from_str(&format!("\"{}\"", assessment.edit_number.value())) {
        Ok(value) => {
            response.headers_mut().insert(ETAG, value);
            response
        }
        Err(_) => route_error(StatusCode::SERVICE_UNAVAILABLE, "Assessment is unavailable"),
    }
}

async fn instructor_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        // ASVS 8.2.1 and 8.3.1: Product Role comes only from the server-side
        // session. Each Store call repeats active-Instructor owner authority.
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Assessment Template authentication is unavailable",
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

fn has_json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.split(';').next().is_some_and(|media_type| {
                media_type.trim().eq_ignore_ascii_case("application/json")
            })
        })
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => route_error(
            StatusCode::CONFLICT,
            "Assessment Template changed before this save",
        ),
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Assessment Template is invalid",
        ),
        StoreError::AlreadyExists | StoreError::LifecycleConflict => {
            route_error(StatusCode::CONFLICT, "Assessment Template conflicts")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => unavailable(),
    }
}

fn copy_store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => {
            concealed_assessment()
        }
        StoreError::Conflict
        | StoreError::RetryableTransaction
        | StoreError::AlreadyExists
        | StoreError::LifecycleConflict => {
            route_error(StatusCode::CONFLICT, "Assessment conflicts")
        }
        StoreError::InvalidRecord(_) => {
            route_error(StatusCode::UNPROCESSABLE_ENTITY, "Assessment is invalid")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => {
            route_error(StatusCode::SERVICE_UNAVAILABLE, "Assessment is unavailable")
        }
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Assessment Template not found")
}

fn concealed_assessment() -> Response {
    route_error(StatusCode::NOT_FOUND, "Assessment not found")
}

fn unavailable() -> Response {
    route_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Assessment Template is unavailable",
    )
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_number_precondition_accepts_one_strong_canonical_value() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_MATCH, HeaderValue::from_static("\"12\""));

        assert_eq!(
            expected_edit_number(&headers).expect("valid strong ETag"),
            AssessmentTemplateEditNumber::new(12).expect("positive Edit Number")
        );
    }

    #[test]
    fn edit_number_precondition_distinguishes_missing_and_invalid_values() {
        let missing = expected_edit_number(&HeaderMap::new()).expect_err("missing precondition");
        assert_eq!(missing.status(), StatusCode::PRECONDITION_REQUIRED);

        for value in ["W/\"1\"", "\"01\"", "\"0\"", "1"] {
            let mut headers = HeaderMap::new();
            headers.insert(
                IF_MATCH,
                HeaderValue::from_str(value).expect("header value"),
            );
            let invalid = expected_edit_number(&headers).expect_err("invalid precondition");
            assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        }
    }

    #[test]
    fn create_payload_is_closed_and_uses_canonical_assessment_type_values() {
        assert!(
            serde_json::from_str::<CreateAssessmentTemplateRequest>(
                r#"{"name":"Weekly practice","assessmentType":"practice_question_assignment"}"#,
            )
            .is_ok()
        );
        assert!(
            serde_json::from_str::<CreateAssessmentTemplateRequest>(
                r#"{"name":"Weekly practice","assessmentType":"practice_question_assignment","owner":"someone"}"#,
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<CreateAssessmentTemplateRequest>(
                r#"{"name":"Weekly practice","assessmentType":"assignment"}"#,
            )
            .is_err()
        );
    }

    #[test]
    fn assessment_copy_payload_is_closed_and_validated() {
        assert!(
            serde_json::from_str::<CreateAssessmentFromTemplateRequest>(
                r#"{"templateId":"00000000-0000-0000-0000-000000000001","title":"Weekly practice"}"#,
            )
            .is_ok()
        );
        assert!(
            serde_json::from_str::<CreateAssessmentFromTemplateRequest>(
                r#"{"templateId":"00000000-0000-0000-0000-000000000001","title":"Weekly practice","course":"C1"}"#,
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<CreateAssessmentFromTemplateRequest>(
                r#"{"templateId":"not-a-uuid","title":"Weekly practice"}"#,
            )
            .is_err()
        );
    }
}
