//! Instructor-facing retention control and status routes (MOD-RETENTION).
//!
//! This module exposes only browser-safe retention state and immutable mutation
//! commands. Every request is tenant-authorized through a stored session and
//! course-role check.

use std::sync::Arc;

use axum::body::{Bytes, to_bytes};
use axum::extract::{DefaultBodyLimit, Path, Request, State};
use axum::http::header::{CONTENT_TYPE, ETAG, HeaderValue, IF_MATCH};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use learning_data_access::{
    AssignmentDefinitionDisposition, CourseRetentionView, RETENTION_ARCHIVE_NOTIFICATION_COPY,
    RetentionApiStore, RetentionDays, RetentionNotificationIntent, RetentionNotificationView,
    RetentionRequestOutcome, RetentionRequestResult, RetentionRevision, RetentionStore,
    SessionStore, Store, StoreError,
};
use question_model::{ActivityTimestamp, CourseId, CourseRole, UserRole};
use serde::de::DeserializeOwned;
use serde::de::MapAccess;
use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};

const JSON_MIME_TYPE: &str = "application/json";
const MAX_RETENTION_BODY_BYTES: usize = 64 * 1_024;

/// Builds the course-level retention route group.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: Store + SessionStore + RetentionStore + RetentionApiStore + 'static,
{
    Router::new()
        .route("/api/courses/{course}/retention", get(get_retention::<S>))
        .route(
            "/api/courses/{course}/retention/end",
            post(end_course_retention::<S>),
        )
        .route(
            "/api/courses/{course}/retention/archive",
            post(request_archive::<S>),
        )
        .route(
            "/api/courses/{course}/retention/delete",
            post(request_delete::<S>),
        )
        .route(
            "/api/courses/{course}/retention/extend",
            patch(request_extend::<S>),
        )
        .layer(DefaultBodyLimit::max(MAX_RETENTION_BODY_BYTES))
        .layer(middleware::map_response(no_store_response))
        .with_state(RetentionRouteState { store })
}

struct RetentionRouteState<S> {
    store: Arc<S>,
}

impl<S> Clone for RetentionRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CourseManagerAccess {
    is_admin: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RetentionNotificationProjection {
    intent: RetentionNotificationIntent,
    created_at: ActivityTimestamp,
    copy: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RetentionViewResponse {
    #[serde(flatten)]
    retention: CourseRetentionView,
    #[serde(skip_serializing_if = "Option::is_none")]
    notification: Option<RetentionNotificationProjection>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RetentionActionResponse {
    #[serde(flatten)]
    retention: CourseRetentionView,
    outcome: RetentionRequestOutcome,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArchiveRequest {
    assignment_definitions: AssignmentDefinitionDispositionRequest,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum AssignmentDefinitionDispositionRequest {
    Retain,
    Delete,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExtendRequest {
    additional_days: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IfMatchError {
    Missing,
    Malformed,
}

async fn get_retention<S>(
    State(state): State<RetentionRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
) -> Response
where
    S: Store + SessionStore + RetentionStore + RetentionApiStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_manager(state.store.as_ref(), &authenticated, course).await
    {
        return response;
    }

    let retention = match state
        .store
        .retention_view(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
        )
        .await
    {
        Ok(Some(retention)) => retention,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "course retention not found"),
        Err(error) => return route_store_error(error),
    };

    let notification = match state
        .store
        .retention_notification(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
        )
        .await
    {
        Ok(notification) => notification,
        Err(error) => return route_store_error(error),
    };

    retention_response(StatusCode::OK, retention, notification)
}

async fn end_course_retention<S>(
    State(state): State<RetentionRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store + SessionStore + RetentionStore + RetentionApiStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_manager(state.store.as_ref(), &authenticated, course).await
    {
        return response;
    }

    let body = match read_body(request).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    if !body.is_empty() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "end request body must be empty",
        );
    }

    let retention_view = match state
        .store
        .end_course_retention(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
        )
        .await
    {
        Ok(record) => match record.safe_view() {
            Ok(view) => view,
            Err(error) => {
                return error_response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    &format!("retention record is invalid: {error}"),
                );
            }
        },
        Err(error) => return route_store_error(error),
    };

    retention_response(StatusCode::OK, retention_view, None)
}

async fn request_archive<S>(
    State(state): State<RetentionRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store + SessionStore + RetentionStore + RetentionApiStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_manager(state.store.as_ref(), &authenticated, course).await
    {
        return response;
    }

    let expected_revision = match required_if_match_revision(&headers) {
        Ok(revision) => revision,
        Err(IfMatchError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match retention revision is required",
            );
        }
        Err(IfMatchError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match retention revision is invalid",
            );
        }
    };

    if !is_application_json_content_type(request.headers().get(CONTENT_TYPE)) {
        return error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "request content type must be application/json",
        );
    }

    let body = match read_body(request).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let request = match parse_strict_json::<ArchiveRequest>(body) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "retention archive request is invalid",
            );
        }
    };
    let disposition = match request.assignment_definitions {
        AssignmentDefinitionDispositionRequest::Retain => AssignmentDefinitionDisposition::Retain,
        AssignmentDefinitionDispositionRequest::Delete => AssignmentDefinitionDisposition::Delete,
    };

    match state
        .store
        .request_retention_archive_if_revision(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
            expected_revision,
            disposition,
        )
        .await
    {
        Ok(result) => retention_action_response(result),
        Err(error) => route_store_error(error),
    }
}

async fn request_delete<S>(
    State(state): State<RetentionRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store + SessionStore + RetentionStore + RetentionApiStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_manager(state.store.as_ref(), &authenticated, course).await
    {
        return response;
    }

    let expected_revision = match required_if_match_revision(&headers) {
        Ok(revision) => revision,
        Err(IfMatchError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match retention revision is required",
            );
        }
        Err(IfMatchError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match retention revision is invalid",
            );
        }
    };

    let body = match read_body(request).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    if !body.is_empty() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "delete request body must be empty",
        );
    }

    match state
        .store
        .request_retention_delete_if_revision(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
            expected_revision,
        )
        .await
    {
        Ok(result) => retention_action_response(result),
        Err(error) => route_store_error(error),
    }
}

async fn request_extend<S>(
    State(state): State<RetentionRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store + SessionStore + RetentionStore + RetentionApiStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let access = match require_course_manager(state.store.as_ref(), &authenticated, course).await {
        Ok(access) => access,
        Err(response) => return response,
    };
    if !access.is_admin {
        return error_response(
            StatusCode::FORBIDDEN,
            "retention extension is administrator-only",
        );
    }

    let expected_revision = match required_if_match_revision(&headers) {
        Ok(revision) => revision,
        Err(IfMatchError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match retention revision is required",
            );
        }
        Err(IfMatchError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match retention revision is invalid",
            );
        }
    };

    if !is_application_json_content_type(request.headers().get(CONTENT_TYPE)) {
        return error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "request content type must be application/json",
        );
    }

    let body = match read_body(request).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let request = match parse_strict_json::<ExtendRequest>(body) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "retention extension request is invalid",
            );
        }
    };

    let additional_days = match additional_days_from_request(request.additional_days) {
        Ok(additional_days) => additional_days,
        Err(message) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, message),
    };

    match state
        .store
        .extend_retention_if_revision(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
            expected_revision,
            additional_days,
        )
        .await
    {
        Ok(retention) => retention_response(StatusCode::OK, retention, None),
        Err(error) => route_store_error(error),
    }
}

fn additional_days_from_request(value: i64) -> Result<RetentionDays, &'static str> {
    if value <= 0 {
        return Err("additionalDays must be a positive integer");
    }

    let days = u16::try_from(value).map_err(|_| "additionalDays must be a positive integer")?;
    RetentionDays::new(days).map_err(|_| "additionalDays must be within retention bounds")
}

async fn require_course_manager<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    course: CourseId,
) -> Result<CourseManagerAccess, Response>
where
    S: Store,
{
    let user = authenticated.record.subject.user();
    let roles = authenticated.record.subject.roles();
    let is_platform_admin = roles.contains(&UserRole::Administrator);

    let course_record = match store.get_course(authenticated.tenant_context, course).await {
        Ok(None) => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "course does not authorize this action",
            ));
        }
        Ok(Some(record)) => record,
        Err(StoreError::Forbidden | StoreError::TenantMismatch | StoreError::NotFound) => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "course does not authorize this action",
            ));
        }
        Err(error) => return Err(route_store_error(error)),
    };

    let course_role = course_record.role_for(user);
    if !(is_platform_admin
        || matches!(
            course_role,
            Some(CourseRole::Instructor | CourseRole::Administrator)
        ))
    {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "course does not authorize this action",
        ));
    }

    let is_admin = is_platform_admin || matches!(course_role, Some(CourseRole::Administrator));

    Ok(CourseManagerAccess { is_admin })
}

fn retention_action_response(result: RetentionRequestResult) -> Response {
    let status = match result.outcome {
        RetentionRequestOutcome::Scheduled | RetentionRequestOutcome::InProgress => {
            StatusCode::ACCEPTED
        }
        RetentionRequestOutcome::Completed => StatusCode::OK,
    };
    let mut response = no_store(
        (
            status,
            Json(RetentionActionResponse {
                retention: result.retention,
                outcome: result.outcome,
            }),
        )
            .into_response(),
    );
    add_retention_etag(&mut response, result.retention.revision.value());
    response
}

fn retention_response(
    status: StatusCode,
    retention: CourseRetentionView,
    notification: Option<RetentionNotificationView>,
) -> Response {
    let notification = notification.map(|notification| RetentionNotificationProjection {
        intent: notification.intent,
        created_at: notification.created_at,
        copy: RETENTION_ARCHIVE_NOTIFICATION_COPY,
    });
    let mut response = no_store(
        (
            status,
            Json(RetentionViewResponse {
                retention,
                notification,
            }),
        )
            .into_response(),
    );
    add_retention_etag(&mut response, retention.revision.value());
    response
}

fn add_retention_etag(response: &mut Response, revision: u64) {
    let etag = format!("\"{}\"", revision)
        .parse()
        .expect("retention revision must be a valid ETag");
    response.headers_mut().insert(ETAG, etag);
}

fn is_application_json_content_type(content_type: Option<&HeaderValue>) -> bool {
    let Some(content_type) = content_type else {
        return false;
    };
    content_type
        .to_str()
        .map(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(JSON_MIME_TYPE))
        })
        .unwrap_or(false)
}

async fn no_store_response(response: Response) -> Response {
    no_store(response)
}

#[derive(Debug)]
enum DuplicateKeyJsonValue {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

impl<'de> de::Deserialize<'de> for DuplicateKeyJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct DuplicateKeyVisitor;

        impl<'de> Visitor<'de> for DuplicateKeyVisitor {
            type Value = DuplicateKeyJsonValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("valid JSON")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::Null)
            }

            fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::Bool)
            }

            fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::Number)
            }

            fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::Number)
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                serde_json::Number::from_f64(value)
                    .ok_or_else(|| de::Error::custom("invalid JSON number"))?;
                Ok(DuplicateKeyJsonValue::Number)
            }

            fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::String)
            }

            fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::String)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                de::Deserialize::deserialize(deserializer)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut items = Vec::new();
                while let Some(_value) = seq.next_element::<DuplicateKeyJsonValue>()? {
                    items.push(());
                }
                let _ = items;
                Ok(DuplicateKeyJsonValue::Array)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                use std::collections::HashMap;

                let mut fields: HashMap<String, ()> = HashMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    if fields.contains_key(&key) {
                        return Err(de::Error::custom(format!(
                            "JSON object has duplicate key: {key}"
                        )));
                    }
                    let _ = map.next_value::<DuplicateKeyJsonValue>()?;
                    let _ = fields.insert(key, ());
                }
                let _ = fields;
                Ok(DuplicateKeyJsonValue::Object)
            }
        }

        deserializer.deserialize_any(DuplicateKeyVisitor)
    }
}

fn required_if_match_revision(headers: &HeaderMap) -> Result<RetentionRevision, IfMatchError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(IfMatchError::Missing);
    };
    if values.next().is_some() {
        return Err(IfMatchError::Malformed);
    }

    let quoted = value.to_str().map_err(|_| IfMatchError::Malformed)?;
    let Some(value) = quoted
        .strip_prefix('\"')
        .and_then(|value| value.strip_suffix('\"'))
    else {
        return Err(IfMatchError::Malformed);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(IfMatchError::Malformed);
    }

    let revision = value.parse::<u64>().map_err(|_| IfMatchError::Malformed)?;
    RetentionRevision::new(revision).map_err(|_| IfMatchError::Malformed)
}

fn parse_strict_json<T>(body: Bytes) -> Result<T, ()>
where
    T: DeserializeOwned + Serialize,
{
    // Parse into the typed request first, then compare its canonical
    // JSON representation to the request body. This rejects unknown fields
    // and duplicate keys in request objects.
    let request: T = serde_json::from_slice(&body).map_err(|_| ())?;
    let _: DuplicateKeyJsonValue = serde_json::from_slice(&body).map_err(|_| ())?;
    let value: serde_json::Value = serde_json::from_slice(&body).map_err(|_| ())?;
    let canonical = serde_json::to_value(&request).map_err(|_| ())?;
    if canonical == value {
        Ok(request)
    } else {
        Err(())
    }
}

async fn read_body(request: Request) -> Result<Bytes, Response> {
    to_bytes(request.into_body(), MAX_RETENTION_BODY_BYTES)
        .await
        .map_err(|_| error_response(StatusCode::PAYLOAD_TOO_LARGE, "request body is too large"))
}

fn route_store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::TenantMismatch => {
            error_response(StatusCode::NOT_FOUND, "course retention not found")
        }
        StoreError::Conflict | StoreError::TimedOut | StoreError::AlreadyExists => {
            error_response(StatusCode::CONFLICT, "record changed; reload it")
        }
        StoreError::InvalidRecord(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        StoreError::RunModel(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message.to_string())
        }
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "retention service unavailable",
        ),
    }
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn parse_strict_json_only_accepts_strict_body() {
        let retain = serde_json::to_vec(&ArchiveRequest {
            assignment_definitions: AssignmentDefinitionDispositionRequest::Retain,
        })
        .expect("serialize archive fixture");
        assert!(parse_strict_json::<ArchiveRequest>(Bytes::from(retain)).is_ok());

        let expanded = serde_json::to_vec(&serde_json::json!({
            "assignmentDefinitions": "retain",
            "extra": "ignored",
        }))
        .expect("serialize archive fixture");
        assert!(parse_strict_json::<ArchiveRequest>(Bytes::from(expanded)).is_err());
    }

    #[test]
    fn parse_strict_json_rejects_wrong_types() {
        let missing = serde_json::to_vec(&serde_json::json!({})).expect("serialize empty body");
        assert!(parse_strict_json::<ExtendRequest>(Bytes::from(missing)).is_err());

        let wrong = serde_json::to_vec(&serde_json::json!({
            "additionalDays": 3.5,
        }))
        .expect("serialize wrong payload");
        assert!(parse_strict_json::<ExtendRequest>(Bytes::from(wrong)).is_err());
    }

    #[test]
    fn parse_strict_json_rejects_duplicate_members() {
        let duplicate = br#"{
            "assignmentDefinitions": "retain",
            "assignmentDefinitions": "delete"
        }"#;
        assert!(parse_strict_json::<ArchiveRequest>(Bytes::from(duplicate.to_vec())).is_err());
    }

    #[test]
    fn is_application_json_content_type_detects_accepted_headers() {
        let valid = HeaderValue::from_static("application/json; charset=utf-8");
        assert!(is_application_json_content_type(Some(&valid)));

        let invalid = HeaderValue::from_static("text/plain");
        assert!(!is_application_json_content_type(Some(&invalid)));

        assert!(!is_application_json_content_type(None));
    }

    #[test]
    fn required_if_match_rejects_missing_and_weak_values() {
        let missing = HeaderMap::new();
        assert_eq!(
            required_if_match_revision(&missing),
            Err(IfMatchError::Missing)
        );

        let mut weak = HeaderMap::new();
        weak.insert(IF_MATCH, HeaderValue::from_static("W/\"1\""));
        assert_eq!(
            required_if_match_revision(&weak),
            Err(IfMatchError::Malformed)
        );

        let mut malformed = HeaderMap::new();
        malformed.insert(IF_MATCH, HeaderValue::from_static("bad"));
        assert_eq!(
            required_if_match_revision(&malformed),
            Err(IfMatchError::Malformed)
        );
    }

    #[test]
    fn required_if_match_rejects_zero_or_out_of_range() {
        let mut zero = HeaderMap::new();
        zero.insert(IF_MATCH, HeaderValue::from_static("\"0\""));
        assert_eq!(
            required_if_match_revision(&zero),
            Err(IfMatchError::Malformed)
        );

        let mut huge = HeaderMap::new();
        huge.insert(
            IF_MATCH,
            HeaderValue::from_static("\"9223372036854775808\""),
        );
        assert_eq!(
            required_if_match_revision(&huge),
            Err(IfMatchError::Malformed)
        );
    }

    #[test]
    fn required_if_match_rejects_multiple() {
        let mut multiple = HeaderMap::new();
        multiple.insert(IF_MATCH, HeaderValue::from_static("\"1\""));
        multiple.append(IF_MATCH, HeaderValue::from_static("\"2\""));
        assert_eq!(
            required_if_match_revision(&multiple),
            Err(IfMatchError::Malformed)
        );
    }

    #[test]
    fn required_if_match_accepts_valid_value() {
        let mut valid = HeaderMap::new();
        valid.insert(IF_MATCH, HeaderValue::from_static("\"123\""));
        assert_eq!(
            required_if_match_revision(&valid)
                .expect("valid etag")
                .value(),
            123
        );
    }

    #[test]
    fn additional_days_from_request_rejects_invalid_values() {
        assert_eq!(
            additional_days_from_request(0),
            Err("additionalDays must be a positive integer")
        );
        assert_eq!(
            additional_days_from_request(-1),
            Err("additionalDays must be a positive integer")
        );
    }
}

#[cfg(test)]
mod route_tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::header::{CONTENT_TYPE, ETAG, IF_MATCH};
    use axum::http::{Method, Request, StatusCode};
    use learning_data_access::Store;
    use learning_data_access::in_memory::MemoryStore;
    use learning_data_access::{
        ClaimedJob, CourseRecord, JobLeaseDuration, JobPayload, JobStore, RetentionDispatchBatch,
        RetentionScheduleStore, RetentionWorkerCommand, RetentionWorkerStore, SessionLifetime,
        SessionSubject, TenantContext,
    };
    use question_model::{
        ActivityTimestamp, CourseId, CourseMembership, CourseMembershipRole, TenantId, UserId,
        UserRole,
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::auth::{CookieTransport, SessionConfig, issue_session};

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn course_route(course: CourseId, suffix: &str) -> String {
        format!("/api/courses/{course}/retention{suffix}")
    }

    fn assert_no_store(response: &Response) {
        assert_eq!(
            response
                .headers()
                .get("cache-control")
                .expect("cache-control header")
                .to_str()
                .expect("cache-control value"),
            "no-store"
        );
    }

    fn assert_private_projection_fields(value: &serde_json::Value) {
        let object = value.as_object().expect("object response");
        for key in [
            "policy",
            "deadline",
            "generation",
            "stage",
            "job",
            "lease",
            "recipient",
            "tenant",
            "user",
            "student",
            "object",
            "source",
            "provider",
            "answer",
            "key",
            "grading",
        ] {
            assert!(!object.contains_key(key), "field {key} must be excluded");
        }
        if let Some(notification) = object.get("notification") {
            let notification = notification
                .as_object()
                .expect("notification response should be an object");
            for key in [
                "policy",
                "deadline",
                "generation",
                "stage",
                "job",
                "lease",
                "recipient",
                "tenant",
                "user",
                "student",
                "object",
                "source",
                "provider",
                "answer",
                "key",
                "grading",
            ] {
                assert!(
                    !notification.contains_key(key),
                    "notification field {key} excluded"
                );
            }
        }
    }

    async fn response_json(response: Response) -> serde_json::Value {
        serde_json::from_slice(
            &to_bytes(response.into_body(), 128 * 1_024)
                .await
                .expect("response body"),
        )
        .expect("json response")
    }

    async fn issued_cookie(
        store: &MemoryStore,
        tenant: TenantId,
        roles: Vec<UserRole>,
        user: UserId,
    ) -> String {
        let issued = issue_session(
            store,
            SessionSubject::new(tenant, user, "Retention fixture", roles)
                .expect("fixture identity"),
            SessionConfig::new(
                SessionLifetime::from_seconds(3_600).expect("session lifetime"),
                CookieTransport::LocalHttp,
            ),
        )
        .await
        .expect("session issued");
        issued
            .set_cookie
            .split(';')
            .next()
            .expect("set-cookie")
            .to_string()
    }

    async fn create_course(
        store: &MemoryStore,
        tenant: TenantId,
        course: CourseId,
        members: Vec<(UserId, CourseMembershipRole)>,
    ) {
        store
            .upsert_course(
                TenantContext::from_authenticated_session(tenant),
                CourseRecord {
                    id: course,
                    tenant,
                    title: "BIOC 301".to_string(),
                    members: members
                        .into_iter()
                        .map(|(user, role)| CourseMembership { user, role })
                        .collect(),
                },
            )
            .await
            .expect("course persisted");
    }

    async fn make_request(
        app: &axum::Router,
        method: Method,
        uri: String,
        cookie: Option<&str>,
        if_match: &[&str],
        content_type: Option<&str>,
        body: &str,
    ) -> Response {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        for header in if_match {
            request = request.header(IF_MATCH, *header);
        }
        if let Some(content_type) = content_type {
            request = request.header(CONTENT_TYPE, content_type);
        }
        let response = app
            .clone()
            .oneshot(
                request
                    .body(Body::from(body.to_owned()))
                    .expect("request body"),
            )
            .await
            .expect("router response");
        assert_no_store(&response);
        response
    }

    async fn end_retention(
        app: &axum::Router,
        cookie: Option<&str>,
        course: CourseId,
        body: &str,
    ) -> Response {
        make_request(
            app,
            Method::POST,
            course_route(course, "/end"),
            cookie,
            &[],
            None,
            body,
        )
        .await
    }

    async fn get_retention(app: &axum::Router, cookie: Option<&str>, course: CourseId) -> Response {
        make_request(
            app,
            Method::GET,
            course_route(course, ""),
            cookie,
            &[],
            None,
            "",
        )
        .await
    }

    async fn archive_retention(
        app: &axum::Router,
        cookie: Option<&str>,
        course: CourseId,
        if_match: &[&str],
        content_type: Option<&str>,
        body: &str,
    ) -> Response {
        make_request(
            app,
            Method::POST,
            course_route(course, "/archive"),
            cookie,
            if_match,
            content_type,
            body,
        )
        .await
    }

    async fn delete_retention(
        app: &axum::Router,
        cookie: Option<&str>,
        course: CourseId,
        if_match: &[&str],
        body: &str,
    ) -> Response {
        make_request(
            app,
            Method::POST,
            course_route(course, "/delete"),
            cookie,
            if_match,
            Some("application/json"),
            body,
        )
        .await
    }

    async fn extend_retention(
        app: &axum::Router,
        cookie: Option<&str>,
        course: CourseId,
        if_match: &[&str],
        content_type: Option<&str>,
        body: &str,
    ) -> Response {
        make_request(
            app,
            Method::PATCH,
            course_route(course, "/extend"),
            cookie,
            if_match,
            content_type,
            body,
        )
        .await
    }

    fn worker_command_from_claim(claim: ClaimedJob) -> RetentionWorkerCommand {
        let (command_course, stage, generation) = match claim.payload {
            JobPayload::Retention {
                course,
                stage,
                generation,
            } => (course, stage, generation),
            _ => panic!("worker job is not retention payload"),
        };
        RetentionWorkerCommand {
            tenant: claim.tenant,
            course: command_course,
            stage,
            generation,
            job: claim.id,
            lease: claim.lease_token,
        }
    }

    #[tokio::test]
    async fn retention_routes_require_session_before_body_and_if_match() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(1));
        let course = CourseId::from_uuid(id(2));
        let instructor = UserId::from_uuid(id(3));
        create_course(
            &store,
            tenant,
            course,
            vec![(instructor, CourseMembershipRole::Instructor)],
        )
        .await;
        let app = router(Arc::clone(&store));

        let no_session_archive = archive_retention(
            &app,
            None,
            course,
            &["\"1\""],
            Some("application/json"),
            r#"{"assignmentDefinitions":"retain"}"#,
        )
        .await;
        assert_eq!(no_session_archive.status(), StatusCode::UNAUTHORIZED);
        assert_no_store(&no_session_archive);

        let no_session_delete = delete_retention(&app, None, course, &[], r#"{}"#).await;
        assert_eq!(no_session_delete.status(), StatusCode::UNAUTHORIZED);
        assert_no_store(&no_session_delete);

        let no_session_extend = extend_retention(
            &app,
            None,
            course,
            &[],
            Some("application/json"),
            r#"{"additionalDays":7}"#,
        )
        .await;
        assert_eq!(no_session_extend.status(), StatusCode::UNAUTHORIZED);
        assert_no_store(&no_session_extend);

        let no_session_end = end_retention(&app, None, course, "{}").await;
        assert_eq!(no_session_end.status(), StatusCode::UNAUTHORIZED);
        assert_no_store(&no_session_end);

        let no_session_get = get_retention(&app, None, course).await;
        assert_eq!(no_session_get.status(), StatusCode::UNAUTHORIZED);
        assert_no_store(&no_session_get);
    }

    #[tokio::test]
    async fn retention_route_authority_hides_courses_before_payload_inspection() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(10));
        let foreign_tenant = TenantId::from_uuid(id(20));
        let course = CourseId::from_uuid(id(11));
        let missing_course = CourseId::from_uuid(id(12));
        let instructor = UserId::from_uuid(id(13));
        let student = UserId::from_uuid(id(14));
        let outsider = UserId::from_uuid(id(15));
        let foreign_instructor = UserId::from_uuid(id(16));

        create_course(
            &store,
            tenant,
            course,
            vec![
                (instructor, CourseMembershipRole::Instructor),
                (student, CourseMembershipRole::Student),
            ],
        )
        .await;

        let instructor_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;
        let student_cookie = issued_cookie(&store, tenant, vec![UserRole::Student], student).await;
        let outsider_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], outsider).await;
        let foreign_cookie = issued_cookie(
            &store,
            foreign_tenant,
            vec![UserRole::Instructor],
            foreign_instructor,
        )
        .await;

        let app = router(Arc::clone(&store));
        let ended = end_retention(&app, Some(&instructor_cookie), course, "").await;
        assert_eq!(ended.status(), StatusCode::OK);

        let student_archive = archive_retention(
            &app,
            Some(&student_cookie),
            course,
            &[],
            Some("text/plain"),
            r#"{"assignmentDefinitions":"retain"}"#,
        )
        .await;
        assert_eq!(student_archive.status(), StatusCode::NOT_FOUND);
        assert_no_store(&student_archive);

        let outsider_delete = delete_retention(&app, Some(&outsider_cookie), course, &[], "").await;
        assert_eq!(outsider_delete.status(), StatusCode::NOT_FOUND);
        assert_no_store(&outsider_delete);

        let foreign_extend = extend_retention(
            &app,
            Some(&foreign_cookie),
            course,
            &[],
            None,
            r#"{"additionalDays":7}"#,
        )
        .await;
        assert_eq!(foreign_extend.status(), StatusCode::NOT_FOUND);
        assert_no_store(&foreign_extend);

        let missing_course_archive = archive_retention(
            &app,
            Some(&instructor_cookie),
            missing_course,
            &[],
            Some("text/plain"),
            r#"{"assignmentDefinitions":"retain"}"#,
        )
        .await;
        assert_eq!(missing_course_archive.status(), StatusCode::NOT_FOUND);
        assert_no_store(&missing_course_archive);
    }

    #[tokio::test]
    async fn retention_end_route_is_replayable_and_requires_exact_empty_body() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(21));
        let course = CourseId::from_uuid(id(22));
        let instructor = UserId::from_uuid(id(23));
        create_course(
            &store,
            tenant,
            course,
            vec![(instructor, CourseMembershipRole::Instructor)],
        )
        .await;
        let instructor_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;
        let app = router(Arc::clone(&store));

        let non_empty = end_retention(&app, Some(&instructor_cookie), course, "{}").await;
        assert_eq!(non_empty.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_no_store(&non_empty);

        let first = end_retention(&app, Some(&instructor_cookie), course, "").await;
        assert_eq!(first.status(), StatusCode::OK);
        let first_payload = response_json(first).await;
        let revision = first_payload["revision"].as_u64().expect("revision");

        let replay = end_retention(&app, Some(&instructor_cookie), course, "").await;
        assert_eq!(replay.status(), StatusCode::OK);
        let replay_payload = response_json(replay).await;
        assert_eq!(
            replay_payload["revision"].as_u64().expect("revision"),
            revision
        );
        assert_private_projection_fields(&replay_payload);
    }

    #[tokio::test]
    async fn retention_get_route_hides_private_fields_and_emits_etag_and_notification() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(31));
        let course = CourseId::from_uuid(id(32));
        let instructor = UserId::from_uuid(id(33));
        create_course(
            &store,
            tenant,
            course,
            vec![(instructor, CourseMembershipRole::Instructor)],
        )
        .await;
        let instructor_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;

        let app = router(Arc::clone(&store));
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
            .expect("clock set");
        let ended = end_retention(&app, Some(&instructor_cookie), course, "").await;
        assert_eq!(ended.status(), StatusCode::OK);

        let notification_due = ActivityTimestamp::from_unix_millis(30 * 86_400_000 + 2_000);
        store
            .set_authoritative_time(notification_due)
            .expect("clock set");
        let dispatched = store
            .dispatch_due_retention_stages(RetentionDispatchBatch::new(4).expect("dispatch batch"))
            .await
            .expect("due dispatch");
        assert_eq!(dispatched, 1);
        let claim = store
            .claim_next_job(
                &learning_data_access::JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(30).expect("lease"),
            )
            .await
            .expect("claimed job")
            .expect("job claim");
        let command = worker_command_from_claim(claim);
        store
            .prepare_retention_work(command)
            .await
            .expect("prepare notify job");
        store
            .commit_retention_work(command)
            .await
            .expect("commit notify job");

        let instructor_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;

        let viewed = get_retention(&app, Some(&instructor_cookie), course).await;
        assert_eq!(viewed.status(), StatusCode::OK);
        let etag = viewed
            .headers()
            .get(ETAG)
            .expect("etag")
            .to_str()
            .expect("etag")
            .to_string();
        let viewed = response_json(viewed).await;
        assert_eq!(viewed["state"], serde_json::json!("active"));
        assert_eq!(viewed["assignmentDefinitions"], serde_json::json!("retain"));
        if let Some(notification) = viewed.get("notification") {
            assert_eq!(
                notification["copy"],
                serde_json::json!(RETENTION_ARCHIVE_NOTIFICATION_COPY)
            );
            assert_eq!(notification["intent"], serde_json::json!("archive"));
            assert!(notification["createdAt"].is_number());
        }
        assert_private_projection_fields(&viewed);
        let revision = viewed["revision"].as_u64().expect("revision");
        assert_eq!(etag, format!("\"{}\"", revision));
    }

    #[tokio::test]
    async fn retention_archive_route_validates_if_match_and_body_grammar() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(41));
        let course = CourseId::from_uuid(id(42));
        let instructor = UserId::from_uuid(id(43));
        create_course(
            &store,
            tenant,
            course,
            vec![(instructor, CourseMembershipRole::Instructor)],
        )
        .await;
        let instructor_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;
        let app = router(Arc::clone(&store));
        let ended = end_retention(&app, Some(&instructor_cookie), course, "").await;
        assert_eq!(ended.status(), StatusCode::OK);

        let missing_if_match = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &[],
            Some("text/plain"),
            r#"{"assignmentDefinitions":"retain"}"#, // should still be 428
        )
        .await;
        assert_eq!(missing_if_match.status(), StatusCode::PRECONDITION_REQUIRED);

        for header in ["W/\"1\"", "0", "bad", "\"9223372036854775808\""] {
            let malformed = archive_retention(
                &app,
                Some(&instructor_cookie),
                course,
                &[header],
                Some("application/json"),
                r#"{"assignmentDefinitions":"retain"}"#, // malformed header only
            )
            .await;
            assert_eq!(
                malformed.status(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "{}",
                header
            );
            assert_no_store(&malformed);
        }

        let multiple = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &["\"1\"", "\"1\""],
            Some("application/json"),
            r#"{"assignmentDefinitions":"retain"}"#,
        )
        .await;
        assert_eq!(multiple.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let non_json = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &["\"1\""],
            Some("text/plain"),
            r#"{"assignmentDefinitions":"retain"}"#,
        )
        .await;
        assert_eq!(non_json.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

        let unknown = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &["\"1\""],
            Some("application/json"),
            r#"{"assignmentDefinitions":"retain","extra":"oops"}"#,
        )
        .await;
        assert_eq!(unknown.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let duplicate = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &["\"1\""],
            Some("application/json"),
            r#"{\"assignmentDefinitions\":\"retain\",\"assignmentDefinitions\":\"delete\"}"#,
        )
        .await;
        assert_eq!(duplicate.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let enum_value = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &["\"1\""],
            Some("application/json"),
            r#"{"assignmentDefinitions":"invalid"}"#,
        )
        .await;
        assert_eq!(enum_value.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let oversized = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &["\"1\""],
            Some("application/json"),
            &format!(
                "{{\"assignmentDefinitions\":\"retain\",\"padding\":\"{}\"}}",
                "a".repeat(MAX_RETENTION_BODY_BYTES + 10),
            ),
        )
        .await;
        assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);

        let valid = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &["\"1\""],
            Some("application/json"),
            r#"{"assignmentDefinitions":"delete"}"#,
        )
        .await;
        assert_eq!(valid.status(), StatusCode::ACCEPTED);
        let valid_json = response_json(valid).await;
        assert_eq!(valid_json["outcome"], serde_json::json!("scheduled"));
        assert_private_projection_fields(&valid_json);
    }

    #[tokio::test]
    async fn retention_archive_route_replays_scheduled_with_no_duplicate_jobs_and_complete_via_worker()
     {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(51));
        let course = CourseId::from_uuid(id(52));
        let instructor = UserId::from_uuid(id(53));
        let other_instructor = UserId::from_uuid(id(54));
        create_course(
            &store,
            tenant,
            course,
            vec![
                (instructor, CourseMembershipRole::Instructor),
                (other_instructor, CourseMembershipRole::Instructor),
            ],
        )
        .await;
        let instructor_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;
        let other_instructor_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], other_instructor).await;
        let app = router(Arc::clone(&store));
        let ended = end_retention(&app, Some(&instructor_cookie), course, "").await;
        assert_eq!(ended.status(), StatusCode::OK);

        let first = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &["\"1\""],
            Some("application/json"),
            r#"{"assignmentDefinitions":"delete"}"#,
        )
        .await;
        assert_eq!(first.status(), StatusCode::ACCEPTED);
        let first = response_json(first).await;
        assert_eq!(first["outcome"], serde_json::json!("scheduled"));
        let revision = first["revision"].as_u64().expect("revision");

        let stale_replay = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &["\"1\""],
            Some("application/json"),
            r#"{"assignmentDefinitions":"delete"}"#,
        )
        .await;
        assert_eq!(stale_replay.status(), StatusCode::ACCEPTED);
        assert_eq!(
            response_json(stale_replay).await["revision"],
            serde_json::json!(revision)
        );

        let first_job = store
            .claim_next_job(
                &learning_data_access::JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(30).unwrap(),
            )
            .await
            .expect("next job")
            .expect("archive job");
        assert!(
            store
                .claim_next_job(
                    &learning_data_access::JobClaimFilter::all(),
                    JobLeaseDuration::from_seconds(30).unwrap()
                )
                .await
                .expect("next job")
                .is_none(),
            "no duplicate job on replay"
        );

        let command = worker_command_from_claim(first_job);
        store
            .prepare_retention_work(command)
            .await
            .expect("prepare archive");
        let current_header = format!("\"{}\"", revision);
        let in_progress = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &[&current_header],
            Some("application/json"),
            r#"{"assignmentDefinitions":"delete"}"#,
        )
        .await;
        assert_eq!(in_progress.status(), StatusCode::ACCEPTED);
        assert_eq!(
            response_json(in_progress).await["outcome"],
            serde_json::json!("inProgress")
        );

        store
            .commit_retention_work(command)
            .await
            .expect("commit archive");
        let completed = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &[&current_header],
            Some("application/json"),
            r#"{"assignmentDefinitions":"delete"}"#,
        )
        .await;
        assert_eq!(completed.status(), StatusCode::OK);
        assert_eq!(
            response_json(completed).await["outcome"],
            serde_json::json!("completed")
        );

        let original_completed_replay = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &["\"1\""],
            Some("application/json"),
            r#"{"assignmentDefinitions":"delete"}"#,
        )
        .await;
        assert_eq!(original_completed_replay.status(), StatusCode::OK);
        assert_eq!(
            response_json(original_completed_replay).await["outcome"],
            serde_json::json!("completed")
        );

        let mismatched_actor = archive_retention(
            &app,
            Some(&other_instructor_cookie),
            course,
            &[&current_header],
            Some("application/json"),
            r#"{"assignmentDefinitions":"delete"}"#,
        )
        .await;
        assert_eq!(mismatched_actor.status(), StatusCode::CONFLICT);

        let mismatched_disposition = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &[&current_header],
            Some("application/json"),
            r#"{"assignmentDefinitions":"retain"}"#,
        )
        .await;
        assert_eq!(mismatched_disposition.status(), StatusCode::CONFLICT);

        let mismatched_action = delete_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &[&current_header],
            "",
        )
        .await;
        assert_eq!(mismatched_action.status(), StatusCode::CONFLICT);

        let stale = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &["\"999\""],
            Some("application/json"),
            r#"{"assignmentDefinitions":"delete"}"#,
        )
        .await;
        assert_eq!(stale.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn retention_delete_route_requires_exact_empty_body_and_stale_if_match() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(61));
        let course = CourseId::from_uuid(id(62));
        let instructor = UserId::from_uuid(id(63));
        create_course(
            &store,
            tenant,
            course,
            vec![(instructor, CourseMembershipRole::Instructor)],
        )
        .await;
        let instructor_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;
        let app = router(Arc::clone(&store));
        let ended = end_retention(&app, Some(&instructor_cookie), course, "").await;
        assert_eq!(ended.status(), StatusCode::OK);

        let missing_if_match =
            delete_retention(&app, Some(&instructor_cookie), course, &[], "").await;
        assert_eq!(missing_if_match.status(), StatusCode::PRECONDITION_REQUIRED);

        let non_empty = delete_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &["\"1\""],
            r#"{"junk":true}"#,
        )
        .await;
        assert_eq!(non_empty.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let first = delete_retention(&app, Some(&instructor_cookie), course, &["\"1\""], "").await;
        assert_eq!(first.status(), StatusCode::ACCEPTED);

        let stale =
            delete_retention(&app, Some(&instructor_cookie), course, &["\"999\""], "").await;
        assert_eq!(stale.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn retention_extend_route_is_admin_only_and_rejects_stale_requests() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(id(71));
        let course = CourseId::from_uuid(id(72));
        let instructor = UserId::from_uuid(id(73));
        let admin = UserId::from_uuid(id(74));
        create_course(
            &store,
            tenant,
            course,
            vec![
                (instructor, CourseMembershipRole::Instructor),
                (admin, CourseMembershipRole::Instructor),
            ],
        )
        .await;
        let instructor_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;
        let admin_cookie =
            issued_cookie(&store, tenant, vec![UserRole::Administrator], admin).await;
        let app = router(Arc::clone(&store));
        let ended = end_retention(&app, Some(&instructor_cookie), course, "").await;
        assert_eq!(ended.status(), StatusCode::OK);

        let instructor_forbidden = extend_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &[],
            Some("text/plain"),
            r#"{"additionalDays":3}"#,
        )
        .await;
        assert_eq!(instructor_forbidden.status(), StatusCode::FORBIDDEN);

        let admin_requires_if_match = extend_retention(
            &app,
            Some(&admin_cookie),
            course,
            &[],
            Some("text/plain"),
            r#"{"additionalDays":3}"#,
        )
        .await;
        assert_eq!(
            admin_requires_if_match.status(),
            StatusCode::PRECONDITION_REQUIRED
        );

        let admin_success = extend_retention(
            &app,
            Some(&admin_cookie),
            course,
            &["\"1\""],
            Some("application/json"),
            r#"{"additionalDays":3}"#,
        )
        .await;
        assert_eq!(admin_success.status(), StatusCode::OK);
        let admin_success = response_json(admin_success).await;
        assert_eq!(admin_success["state"], serde_json::json!("active"));
        assert_private_projection_fields(&admin_success);

        let stale = extend_retention(
            &app,
            Some(&admin_cookie),
            course,
            &["\"1\""],
            Some("application/json"),
            r#"{"additionalDays":3}"#,
        )
        .await;
        assert_eq!(stale.status(), StatusCode::CONFLICT);
    }
}
