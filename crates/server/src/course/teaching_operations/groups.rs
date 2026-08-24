//! Exact-course Instructor HTTP boundary for course group management.

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::body::to_bytes;
use axum::extract::{Path, Query, Request, State};
use axum::http::header::{CONTENT_TYPE, ETAG, IF_MATCH, LOCATION};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use learning_data_access::{
    CourseGroupManagementStore, CourseGroupRecord, CourseRecordsAccessStore, Cursor, PageRequest,
    PageSize, PutCourseGroupCommand, SessionStore, Store, StoreError,
    TeachingAuthorityReferenceStore, UpdateCourseGroupPurposePolicyCommand,
};
use question_model::teaching_operations::{
    TeachingDisplayLabel, TeachingMembershipRole, TeachingMembershipStatus,
};
use question_model::{
    CourseGroupCreateRequest, CourseGroupDetailView, CourseGroupId, CourseGroupListPage,
    CourseGroupMemberView, CourseGroupMembershipWarningView, CourseGroupPurpose,
    CourseGroupPurposePolicy, CourseGroupPurposePolicyUpdateRequest, CourseGroupPurposePolicyView,
    CourseGroupReference, CourseGroupSummaryView, CourseGroupUpdateRequest, CourseId,
    CourseMembershipRole, MultipleMembershipDisposition, TeachingOperationRevision,
    TeachingPageSize,
};

use super::super::policy::require_course_access;
use super::super::projection::{error_response, store_error_response};
use super::super::routing::CourseRouteState;
use crate::auth::{auth_error_response, no_store, resolve_request_session};

const MAX_GROUP_JSON_BYTES: usize = 64 * 1024;

/// Builds the exact-course Instructor group-management route group.
pub(super) fn router<S>(store: Arc<S>) -> Router
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + CourseGroupManagementStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    Router::new()
        .route(
            "/api/courses/{course}/groups",
            get(list_groups::<S>).post(create_group::<S>),
        )
        .route(
            "/api/courses/{course}/groups/{group}",
            get(get_group::<S>)
                .put(update_group::<S>)
                .delete(delete_group::<S>),
        )
        .route(
            "/api/courses/{course}/group-purpose-policies/{purpose}",
            get(get_purpose_policy::<S>).put(update_purpose_policy::<S>),
        )
        .route(
            "/api/courses/{course}/group-membership-warnings",
            get(get_membership_warnings::<S>),
        )
        .with_state(CourseRouteState { store })
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PageQuery {
    after: Option<String>,
    size: Option<TeachingPageSize>,
}

async fn list_groups<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + CourseGroupManagementStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(v) => v,
        Err(e) => return auth_error_response(e),
    };
    if let Err(r) = require_course_access(state.store.as_ref(), &auth, course, true).await {
        return r;
    }
    let page = match page_request(query) {
        Ok(v) => v,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e),
    };
    match state
        .store
        .list_course_groups(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            page,
        )
        .await
    {
        Ok(page) => no_store(
            Json(CourseGroupListPage {
                groups: page.items.into_iter().map(summary).collect(),
                next_cursor: page.next_cursor.map(|v| v.as_str().to_owned()),
            })
            .into_response(),
        ),
        Err(e) => group_error(e),
    }
}

async fn get_group<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, group)): Path<(CourseId, CourseGroupReference)>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + CourseGroupManagementStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(v) => v,
        Err(e) => return auth_error_response(e),
    };
    if let Err(r) = require_course_access(state.store.as_ref(), &auth, course, true).await {
        return r;
    }
    let page = match page_request(query) {
        Ok(v) => v,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e),
    };
    let record = match state
        .store
        .get_course_group_by_reference(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            group,
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "group not found"),
        Err(e) => return group_error(e),
    };
    let members = match state
        .store
        .list_course_group_membership_reference_views(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            group,
            page,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return group_error(e),
    };
    let revision = record.group.revision.value();
    let body = CourseGroupDetailView {
        group: summary(record),
        members: members.items.into_iter().map(member).collect(),
        next_cursor: members.next_cursor.map(|v| v.as_str().to_owned()),
    };
    response_with_revision(StatusCode::OK, body, revision)
}

async fn create_group<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + CourseGroupManagementStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let (auth, body) = match authorize_body(state.store.as_ref(), course, request).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let input: CourseGroupCreateRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => return invalid(),
    };
    let members = match members(state.store.as_ref(), &auth, course, input.members.into()).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let record = CourseGroupRecord {
        id: CourseGroupId::from_uuid(uuid::Uuid::now_v7()),
        tenant: auth.tenant_context.tenant_id(),
        course,
        purpose: input.purpose,
        title: input.title.into(),
        members,
    };
    match state
        .store
        .put_course_group(
            auth.tenant_context,
            PutCourseGroupCommand {
                actor: auth.record.subject.user(),
                expected_revision: None,
                record,
            },
        )
        .await
    {
        Ok(v) => {
            group_mutation_response(
                state.store.as_ref(),
                &auth,
                course,
                v.record.id,
                StatusCode::CREATED,
            )
            .await
        }
        Err(e) => group_error(e),
    }
}

async fn update_group<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, group)): Path<(CourseId, CourseGroupReference)>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + CourseGroupManagementStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let headers = request.headers().clone();
    let auth = match authorize(state.store.as_ref(), course, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let expected = match revision(&headers) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let current = match state
        .store
        .get_course_group_by_reference(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            group,
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "group not found"),
        Err(e) => return group_error(e),
    };
    if expected.value() != current.group.revision.value() {
        return error_response(StatusCode::PRECONDITION_FAILED, "group changed; reload it");
    }
    let input: CourseGroupUpdateRequest = match read_json(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let values = match members(state.store.as_ref(), &auth, course, input.members.into()).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let record = CourseGroupRecord {
        id: current.group.record.id,
        tenant: auth.tenant_context.tenant_id(),
        course,
        purpose: input.purpose,
        title: input.title.into(),
        members: values,
    };
    match state
        .store
        .put_course_group(
            auth.tenant_context,
            PutCourseGroupCommand {
                actor: auth.record.subject.user(),
                expected_revision: Some(current.group.revision),
                record,
            },
        )
        .await
    {
        Ok(v) => {
            group_mutation_response(
                state.store.as_ref(),
                &auth,
                course,
                v.record.id,
                StatusCode::OK,
            )
            .await
        }
        Err(e) => group_error(e),
    }
}

async fn get_purpose_policy<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, purpose)): Path<(CourseId, CourseGroupPurpose)>,
    headers: HeaderMap,
) -> Response
where
    S: Store + CourseRecordsAccessStore + SessionStore + CourseGroupManagementStore + 'static,
{
    let auth = match authorize(state.store.as_ref(), course, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .get_course_group_purpose_policy(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            purpose,
        )
        .await
    {
        Ok(Some(value)) => purpose_policy_response(StatusCode::OK, value),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "group purpose policy not found"),
        Err(error) => group_error(error),
    }
}

async fn update_purpose_policy<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, purpose)): Path<(CourseId, CourseGroupPurpose)>,
    request: Request,
) -> Response
where
    S: Store + CourseRecordsAccessStore + SessionStore + CourseGroupManagementStore + 'static,
{
    let headers = request.headers().clone();
    let auth = match authorize(state.store.as_ref(), course, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let expected = match revision(&headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let current = match state
        .store
        .get_course_group_purpose_policy(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            purpose,
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "group purpose policy not found"),
        Err(error) => return group_error(error),
    };
    if expected.value() != current.revision.value() {
        return error_response(
            StatusCode::PRECONDITION_FAILED,
            "group purpose policy changed; reload it",
        );
    }
    let input: CourseGroupPurposePolicyUpdateRequest = match read_json(request).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .update_course_group_purpose_policy(
            auth.tenant_context,
            UpdateCourseGroupPurposePolicyCommand {
                session: auth.record.token_hash,
                course,
                expected_revision: current.revision,
                policy: CourseGroupPurposePolicy {
                    purpose,
                    multiple_membership: input.multiple_membership,
                },
            },
        )
        .await
    {
        Ok(value) => purpose_policy_response(StatusCode::OK, value),
        Err(error) => group_error(error),
    }
}

async fn get_membership_warnings<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    headers: HeaderMap,
) -> Response
where
    S: Store + CourseRecordsAccessStore + SessionStore + CourseGroupManagementStore + 'static,
{
    let auth = match authorize(state.store.as_ref(), course, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .course_group_membership_warnings(auth.tenant_context, auth.record.subject.user(), course)
        .await
    {
        Ok(warnings) => no_store(
            Json(CourseGroupMembershipWarningView {
                disposition: if warnings.is_empty() {
                    MultipleMembershipDisposition::Allowed
                } else {
                    MultipleMembershipDisposition::AllowedWithWarning
                },
                warning_count: u32::try_from(warnings.len()).expect("bounded warning count"),
            })
            .into_response(),
        ),
        Err(error) => group_error(error),
    }
}

async fn delete_group<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, group)): Path<(CourseId, CourseGroupReference)>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + CourseGroupManagementStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match resolve_request_session(state.store.as_ref(), request.headers()).await {
        Ok(v) => v,
        Err(e) => return auth_error_response(e),
    };
    if let Err(r) = require_course_access(state.store.as_ref(), &auth, course, true).await {
        return r;
    }
    let expected = match revision(request.headers()) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let current = match state
        .store
        .get_course_group_by_reference(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            group,
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "group not found"),
        Err(e) => return group_error(e),
    };
    if expected.value() != current.group.revision.value() {
        return error_response(StatusCode::PRECONDITION_FAILED, "group changed; reload it");
    }
    match state
        .store
        .delete_course_group(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            current.group.record.id,
            current.group.revision,
        )
        .await
    {
        Ok(true) => no_store(StatusCode::NO_CONTENT.into_response()),
        Ok(false) => error_response(StatusCode::NOT_FOUND, "group not found"),
        Err(e) => group_error(e),
    }
}

struct Authorized {
    tenant_context: learning_data_access::TenantContext,
    record: learning_data_access::SessionRecord,
}
async fn authorize_body<S>(
    store: &S,
    course: CourseId,
    request: Request,
) -> Result<(Authorized, axum::body::Bytes), Response>
where
    S: Store + CourseRecordsAccessStore + SessionStore + 'static,
{
    let auth = authorize(store, course, request.headers()).await?;
    Ok((auth, read_body(request).await?))
}

async fn authorize<S>(
    store: &S,
    course: CourseId,
    headers: &HeaderMap,
) -> Result<Authorized, Response>
where
    S: Store + CourseRecordsAccessStore + SessionStore + 'static,
{
    let auth = resolve_request_session(store, headers)
        .await
        .map_err(auth_error_response)?;
    require_course_access(store, &auth, course, true).await?;
    Ok(Authorized {
        tenant_context: auth.tenant_context,
        record: auth.record,
    })
}

async fn read_json<T: serde::de::DeserializeOwned>(request: Request) -> Result<T, Response> {
    if !is_json(request.headers()) {
        return Err(error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "group request must be JSON",
        ));
    }
    let body = read_body(request).await?;
    serde_json::from_slice(&body).map_err(|_| invalid())
}

async fn read_body(request: Request) -> Result<axum::body::Bytes, Response> {
    if !is_json(request.headers()) {
        return Err(error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "group request must be JSON",
        ));
    }
    let body = to_bytes(request.into_body(), MAX_GROUP_JSON_BYTES + 1)
        .await
        .map_err(|_| error_response(StatusCode::PAYLOAD_TOO_LARGE, "group request is too large"))?;
    if body.len() > MAX_GROUP_JSON_BYTES {
        return Err(error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "group request is too large",
        ));
    }
    Ok(body)
}

async fn group_mutation_response<S>(
    store: &S,
    auth: &Authorized,
    course: CourseId,
    group: CourseGroupId,
    status: StatusCode,
) -> Response
where
    S: CourseGroupManagementStore,
{
    match store
        .get_course_group_by_id_for_instructor(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            group,
        )
        .await
    {
        Ok(Some(value)) => {
            let view = summary(value.clone());
            let mut response =
                response_with_revision(status, view.clone(), value.group.revision.value());
            if status == StatusCode::CREATED {
                response.headers_mut().insert(
                    LOCATION,
                    HeaderValue::from_str(&format!(
                        "/api/courses/{course}/groups/{}",
                        view.reference
                    ))
                    .expect("safe location"),
                );
            }
            response
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "group not found"),
        Err(error) => group_error(error),
    }
}

fn purpose_policy_response(
    status: StatusCode,
    value: learning_data_access::StoredCourseGroupPurposePolicy,
) -> Response {
    response_with_revision(
        status,
        CourseGroupPurposePolicyView {
            purpose: value.policy.purpose,
            multiple_membership: value.policy.multiple_membership,
            revision: TeachingOperationRevision::new(value.revision.value())
                .expect("stored policy revision is valid"),
        },
        value.revision.value(),
    )
}
async fn members<S>(
    store: &S,
    auth: &Authorized,
    course: CourseId,
    references: Vec<question_model::CourseMembershipReference>,
) -> Result<Vec<question_model::CourseMembershipId>, Response>
where
    S: TeachingAuthorityReferenceStore,
{
    let mut result = Vec::with_capacity(references.len());
    for reference in references {
        result.push(
            store
                .resolve_course_membership_reference(
                    auth.tenant_context,
                    auth.record.subject.user(),
                    course,
                    reference,
                )
                .await
                .map_err(group_error)?
                .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "membership not found"))?,
        );
    }
    Ok(result)
}
fn page_request(query: PageQuery) -> Result<PageRequest, &'static str> {
    let size = query.size.map(TeachingPageSize::get).unwrap_or(50);
    let size = PageSize::new(size as u16).map_err(|_| "group page size is invalid")?;
    match query.after {
        Some(v) => Ok(PageRequest::after(
            Cursor::parse(v).map_err(|_| "group cursor is invalid")?,
            size,
        )),
        None => Ok(PageRequest::first(size)),
    }
}
fn summary(value: learning_data_access::CourseGroupView) -> CourseGroupSummaryView {
    CourseGroupSummaryView {
        reference: value.reference,
        title: value
            .group
            .record
            .title
            .try_into()
            .expect("stored group title is valid"),
        purpose: value.group.record.purpose,
        revision: TeachingOperationRevision::new(value.group.revision.value())
            .expect("stored revision is valid"),
        member_count: value.group.record.members.len() as u32,
    }
}
fn member(value: learning_data_access::CourseMembershipReferenceView) -> CourseGroupMemberView {
    CourseGroupMemberView {
        reference: value.reference,
        display: TeachingDisplayLabel::try_from(value.display_name)
            .expect("stored display is valid"),
        role: match value.role {
            CourseMembershipRole::Instructor => TeachingMembershipRole::Instructor,
            CourseMembershipRole::Student => TeachingMembershipRole::Student,
        },
        status: match value.status {
            learning_data_access::CourseMemberStatus::Active => TeachingMembershipStatus::Active,
            learning_data_access::CourseMemberStatus::Revoked => TeachingMembershipStatus::Revoked,
        },
    }
}
fn response_with_revision<T: serde::Serialize>(
    status: StatusCode,
    body: T,
    value: u64,
) -> Response {
    let mut response = (status, Json(body)).into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{value}\"")).expect("valid etag"),
    );
    no_store(response)
}
fn is_json(headers: &HeaderMap) -> bool {
    let mut all = headers.get_all(CONTENT_TYPE).iter();
    let Some(value) = all.next().and_then(|v| v.to_str().ok()) else {
        return false;
    };
    all.next().is_none()
        && value
            .split(';')
            .next()
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"))
}
#[allow(clippy::result_large_err)] // HTTP validation returns its exact refusal.
fn revision(headers: &HeaderMap) -> Result<TeachingOperationRevision, Response> {
    let mut all = headers.get_all(IF_MATCH).iter();
    let Some(value) = all.next() else {
        return Err(error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "If-Match is required",
        ));
    };
    if all.next().is_some() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "If-Match is malformed",
        ));
    };
    let raw = value
        .to_str()
        .ok()
        .and_then(|v| v.strip_prefix('"').and_then(|v| v.strip_suffix('"')))
        .ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "If-Match is malformed"))?;
    let value = raw
        .parse()
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, "If-Match is malformed"))?;
    Ok(value)
}
fn invalid() -> Response {
    error_response(StatusCode::UNPROCESSABLE_ENTITY, "group request is invalid")
}
fn group_error(error: StoreError) -> Response {
    match error {
        StoreError::Conflict => error_response(
            StatusCode::PRECONDITION_FAILED,
            "group changed or is referenced",
        ),
        StoreError::Forbidden | StoreError::TenantMismatch | StoreError::NotFound => {
            error_response(StatusCode::NOT_FOUND, "group not found")
        }
        other => store_error_response(other),
    }
}

#[cfg(test)]
#[path = "groups_tests.rs"]
mod tests;
