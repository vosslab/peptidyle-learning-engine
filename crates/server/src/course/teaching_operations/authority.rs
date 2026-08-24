//! Operator approvals and direct-Instructor co-instructor authority routes.

use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, Query, Request, State};
use axum::http::header::{CONTENT_TYPE, ETAG, IF_MATCH, LOCATION};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use learning_data_access::{
    ApproveInstructorAccount, CoInstructorInvitationRevision, CourseRecordsAccessStore, Cursor,
    InstructorApprovalRevision, PageRequest, PageSize, RemoveDirectInstructorMembership,
    RespondToCoInstructorInvitation, RevokeCoInstructorInvitation, RevokeInstructorApproval,
    RosterRevision, SessionStore, Store, StoreError, TeachingAuthorityReferenceStore,
    TeachingAuthorityStore,
};
use question_model::teaching_operations::{
    AccountApprovalView, CoInstructorInvitationStateView, CoInstructorInvitationTerminalAction,
    CoInstructorInvitationTerminalActionRequest, CoInstructorTargetView,
    CourseCoInstructorInvitationView, CourseCoInstructorInvitationsPage, InstructorMembershipView,
    InstructorMembershipsPage, PendingCoInstructorInvitationView,
    PendingCoInstructorInvitationsPage, SysadminInstructorCandidateSearchRequest,
    TeachingAccountView, TeachingDisplayLabel, TeachingMembershipStatus, TeachingOperationRevision,
};
use question_model::{
    AccountReference, CoInstructorInvitationReference, CoInstructorInvitationState,
    CoInstructorTargetSearchQuery, CoInstructorTargetSearchRequest, CourseId,
    CourseMembershipReference, TeachingPageSize, UserRole,
};
use serde::Deserialize;

use super::super::policy::require_course_access;
use super::super::projection::{error_response, store_error_response};
use super::super::routing::CourseRouteState;
use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};

const MAX_AUTHORITY_JSON_BYTES: usize = 16 * 1024;

/// Builds operator eligibility and exact-course co-instructor routes.
pub(super) fn router<S>(store: Arc<S>) -> Router
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    Router::new()
        .route(
            "/api/teaching/instructor-approvals/{account}",
            put(put_approval::<S>).delete(delete_approval::<S>),
        )
        .route(
            "/api/teaching/instructor-approval-candidates",
            get(search_sysadmin_instructor_candidates::<S>),
        )
        .route(
            "/api/courses/{course}/co-instructor-invitations",
            get(list_course_invitations::<S>).post(create_invitation::<S>),
        )
        .route(
            "/api/courses/{course}/co-instructor-targets",
            get(search_co_instructor_targets::<S>),
        )
        .route(
            "/api/courses/{course}/co-instructor-invitations/{invitation}",
            delete(revoke_invitation::<S>),
        )
        .route(
            "/api/account/co-instructor-invitations",
            get(list_pending_invitations::<S>),
        )
        .route(
            "/api/account/co-instructor-invitations/{invitation}",
            post(respond_to_invitation::<S>),
        )
        .route(
            "/api/courses/{course}/instructors",
            get(list_instructors::<S>),
        )
        .route(
            "/api/courses/{course}/instructors/{membership}",
            delete(remove_instructor::<S>),
        )
        .with_state(CourseRouteState { store })
}

async fn search_sysadmin_instructor_candidates<S>(
    State(state): State<CourseRouteState<S>>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match require_operator(state.store.as_ref(), request.headers()).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let search = match sysadmin_instructor_candidate_search_request(request.uri().query()) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match state
        .store
        .search_sysadmin_instructor_candidates(auth.tenant_context, auth.record.token_hash, search)
        .await
    {
        Ok(page) => no_store(Json(page).into_response()),
        Err(e) => authority_error(e),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PageQuery {
    after: Option<String>,
    size: Option<TeachingPageSize>,
}

async fn search_co_instructor_targets<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match instructor(state.store.as_ref(), course, request.headers()).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let search = match co_instructor_target_search_request(request.uri().query()) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match state
        .store
        .search_course_co_instructor_targets(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            search,
        )
        .await
    {
        Ok(page) => no_store(Json(page).into_response()),
        Err(e) => authority_error(e),
    }
}

/// Parses the compact GET transport without echoing private search input in a
/// rejection. The qmodel request remains the one contract handed to the Store.
#[allow(clippy::result_large_err)] // HTTP validation returns its exact refusal.
fn co_instructor_target_search_request(
    raw_query: Option<&str>,
) -> Result<CoInstructorTargetSearchRequest, Response> {
    let mut query = None;
    let mut after = None;
    let mut size = None;
    for (key, value) in url::form_urlencoded::parse(raw_query.unwrap_or_default().as_bytes()) {
        let slot = match key.as_ref() {
            "query" => &mut query,
            "after" => &mut after,
            "size" => &mut size,
            _ => return Err(invalid_target_search_response()),
        };
        if slot.replace(value.into_owned()).is_some() {
            return Err(invalid_target_search_response());
        }
    }
    let query = query.ok_or_else(invalid_target_search_response)?;
    let query = CoInstructorTargetSearchQuery::try_from(query)
        .map_err(|_| invalid_target_search_response())?;
    if let Some(cursor) = after.as_deref() {
        Cursor::parse(cursor.to_owned()).map_err(|_| invalid_target_search_response())?;
    }
    let size = match size {
        Some(value) => value
            .parse::<u32>()
            .ok()
            .and_then(|value| TeachingPageSize::try_from(value).ok())
            .ok_or_else(invalid_target_search_response)?,
        None => TeachingPageSize::try_from(50).expect("default teaching page size is valid"),
    };
    Ok(CoInstructorTargetSearchRequest { query, after, size })
}

/// Parses candidate discovery only after the persisted Sysadmin session has
/// been authorized, keeping malformed input from reaching account discovery.
#[allow(clippy::result_large_err)]
fn sysadmin_instructor_candidate_search_request(
    raw_query: Option<&str>,
) -> Result<SysadminInstructorCandidateSearchRequest, Response> {
    let search = co_instructor_target_search_request(raw_query)
        .map_err(|_| invalid_sysadmin_candidate_search_response())?;
    Ok(SysadminInstructorCandidateSearchRequest {
        query: search.query,
        after: search.after,
        size: search.size,
    })
}

fn invalid_sysadmin_candidate_search_response() -> Response {
    no_store(error_response(
        StatusCode::BAD_REQUEST,
        "instructor candidate search is invalid",
    ))
}

fn invalid_target_search_response() -> Response {
    no_store(error_response(
        StatusCode::BAD_REQUEST,
        "co-instructor target search is invalid",
    ))
}

async fn put_approval<S>(
    State(state): State<CourseRouteState<S>>,
    Path(account): Path<AccountReference>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match require_operator(state.store.as_ref(), request.headers()).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let expected = match optional_approval_revision(request.headers()) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let target = match state
        .store
        .resolve_account_reference_for_operator(
            auth.tenant_context,
            auth.record.token_hash,
            account,
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "account not found"),
        Err(e) => return authority_error(e),
    };
    match state
        .store
        .approve_instructor_account(
            auth.tenant_context,
            ApproveInstructorAccount {
                session: auth.record.token_hash,
                target,
                expected_revision: expected,
            },
        )
        .await
    {
        Ok(v) => approval_response(v.approval.revoked_at.is_none(), v.revision),
        Err(e) => authority_error(e),
    }
}

async fn delete_approval<S>(
    State(state): State<CourseRouteState<S>>,
    Path(account): Path<AccountReference>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match require_operator(state.store.as_ref(), request.headers()).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let expected = match required_approval_revision(request.headers()) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let target = match state
        .store
        .resolve_account_reference_for_operator(
            auth.tenant_context,
            auth.record.token_hash,
            account,
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "account not found"),
        Err(e) => return authority_error(e),
    };
    match state
        .store
        .revoke_instructor_approval(
            auth.tenant_context,
            RevokeInstructorApproval {
                session: auth.record.token_hash,
                target,
                expected_revision: expected,
            },
        )
        .await
    {
        Ok(v) => approval_response(v.approval.revoked_at.is_none(), v.revision),
        Err(e) => authority_error(e),
    }
}

async fn list_course_invitations<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match instructor(state.store.as_ref(), course, &headers).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let page = match page_request(query) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match state
        .store
        .list_course_co_instructor_invitation_reference_views(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            page,
        )
        .await
    {
        Ok(page) => no_store(
            Json(CourseCoInstructorInvitationsPage {
                invitations: page.items.into_iter().map(course_invitation_view).collect(),
                next_cursor: page.next_cursor.map(|v| v.as_str().to_owned()),
            })
            .into_response(),
        ),
        Err(e) => authority_error(e),
    }
}

async fn create_invitation<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match instructor(state.store.as_ref(), course, request.headers()).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let input =
        match json_body::<question_model::CoInstructorInvitationCreateRequest>(request).await {
            Ok(v) => v,
            Err(r) => return r,
        };
    let target = match state
        .store
        .resolve_approved_account_reference_for_course(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            input.target,
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "approved account not found"),
        Err(e) => return authority_error(e),
    };
    match state
        .store
        .create_co_instructor_invitation(
            auth.tenant_context,
            learning_data_access::CreateCoInstructorInvitation {
                session: auth.record.token_hash,
                actor: auth.record.subject.user(),
                course,
                target,
            },
        )
        .await
    {
        Ok(value) => match state
            .store
            .co_instructor_invitation_reference(
                auth.tenant_context,
                auth.record.subject.user(),
                course,
                value.invitation.id,
            )
            .await
        {
            Ok(Some(reference)) => created_invitation_response(course, reference, value.revision),
            Ok(None) => error_response(StatusCode::NOT_FOUND, "invitation not found"),
            Err(e) => authority_error(e),
        },
        Err(e) => authority_error(e),
    }
}

async fn revoke_invitation<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, invitation)): Path<(CourseId, CoInstructorInvitationReference)>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match instructor(state.store.as_ref(), course, request.headers()).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let expected = match required_invitation_revision(request.headers()) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let invitation = match state
        .store
        .resolve_pending_course_co_instructor_invitation_reference(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            invitation,
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "invitation not found"),
        Err(e) => return authority_error(e),
    };
    match state
        .store
        .revoke_co_instructor_invitation(
            auth.tenant_context,
            RevokeCoInstructorInvitation {
                session: auth.record.token_hash,
                actor: auth.record.subject.user(),
                course,
                invitation,
                expected_revision: expected,
            },
        )
        .await
    {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(e) => authority_error(e),
    }
}

async fn list_pending_invitations<S>(
    State(state): State<CourseRouteState<S>>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match authenticated(state.store.as_ref(), &headers).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let page = match page_request(query) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match state
        .store
        .list_pending_co_instructor_invitation_reference_views(
            auth.tenant_context,
            auth.record.token_hash,
            page,
        )
        .await
    {
        Ok(page) => no_store(
            Json(PendingCoInstructorInvitationsPage {
                invitations: page
                    .items
                    .into_iter()
                    .map(pending_invitation_view)
                    .collect(),
                next_cursor: page.next_cursor.map(|v| v.as_str().to_owned()),
            })
            .into_response(),
        ),
        Err(e) => authority_error(e),
    }
}

async fn respond_to_invitation<S>(
    State(state): State<CourseRouteState<S>>,
    Path(invitation): Path<CoInstructorInvitationReference>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match authenticated(state.store.as_ref(), request.headers()).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let expected = match required_invitation_revision(request.headers()) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let invitation = match state
        .store
        .resolve_pending_co_instructor_invitation_reference(
            auth.tenant_context,
            auth.record.token_hash,
            invitation,
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "invitation not found"),
        Err(e) => return authority_error(e),
    };
    let input = match json_body::<CoInstructorInvitationTerminalActionRequest>(request).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let command = RespondToCoInstructorInvitation {
        session: auth.record.token_hash,
        actor: auth.record.subject.user(),
        invitation,
        expected_revision: expected,
    };
    match input.action {
        CoInstructorInvitationTerminalAction::Accept => match state
            .store
            .accept_co_instructor_invitation(auth.tenant_context, command)
            .await
        {
            Ok(v) => empty_roster_revision_response(v.roster_revision),
            Err(e) => authority_error(e),
        },
        CoInstructorInvitationTerminalAction::Decline => match state
            .store
            .decline_co_instructor_invitation(auth.tenant_context, command)
            .await
        {
            Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
            Err(e) => authority_error(e),
        },
    }
}

async fn list_instructors<S>(
    State(state): State<CourseRouteState<S>>,
    Path(course): Path<CourseId>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match instructor(state.store.as_ref(), course, &headers).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let page = match page_request(query) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match state
        .store
        .list_course_instructor_membership_reference_views(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            page,
        )
        .await
    {
        Ok(value) => response_with_etag(
            StatusCode::OK,
            InstructorMembershipsPage {
                instructors: value.page.items.into_iter().map(instructor_view).collect(),
                next_cursor: value.page.next_cursor.map(|v| v.as_str().to_owned()),
                roster_revision: revision(value.roster_revision.value()),
            },
            value.roster_revision.value(),
        ),
        Err(e) => authority_error(e),
    }
}

async fn remove_instructor<S>(
    State(state): State<CourseRouteState<S>>,
    Path((course, membership)): Path<(CourseId, CourseMembershipReference)>,
    request: Request,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + 'static,
{
    let auth = match instructor(state.store.as_ref(), course, request.headers()).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let expected = match required_roster_revision(request.headers()) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let membership = match state
        .store
        .resolve_course_membership_reference(
            auth.tenant_context,
            auth.record.subject.user(),
            course,
            membership,
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "instructor not found"),
        Err(e) => return authority_error(e),
    };
    if let Err(r) = json_body::<question_model::InstructorMembershipRemovalRequest>(request).await {
        return r;
    }
    match state
        .store
        .remove_direct_instructor_membership(
            auth.tenant_context,
            RemoveDirectInstructorMembership {
                actor: auth.record.subject.user(),
                course,
                membership,
                expected_roster_revision: expected,
            },
        )
        .await
    {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(e) => authority_error(e),
    }
}

async fn authenticated<S>(store: &S, headers: &HeaderMap) -> Result<AuthenticatedSession, Response>
where
    S: SessionStore + 'static,
{
    resolve_request_session(store, headers)
        .await
        .map_err(auth_error_response)
}
async fn instructor<S>(
    store: &S,
    course: CourseId,
    headers: &HeaderMap,
) -> Result<AuthenticatedSession, Response>
where
    S: Store + CourseRecordsAccessStore + SessionStore + 'static,
{
    let auth = authenticated(store, headers).await?;
    require_course_access(store, &auth, course, true).await?;
    Ok(auth)
}
async fn require_operator<S>(
    store: &S,
    headers: &HeaderMap,
) -> Result<AuthenticatedSession, Response>
where
    S: SessionStore + 'static,
{
    let auth = authenticated(store, headers).await?;
    if auth.record.subject.roles().contains(&UserRole::Sysadmin) {
        Ok(auth)
    } else {
        Err(error_response(
            StatusCode::FORBIDDEN,
            "operator approval is not authorized",
        ))
    }
}

async fn json_body<T: serde::de::DeserializeOwned>(request: Request) -> Result<T, Response> {
    if !is_json(request.headers()) {
        return Err(error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "request must be JSON",
        ));
    }
    let body = to_bytes(request.into_body(), MAX_AUTHORITY_JSON_BYTES + 1)
        .await
        .map_err(|_| error_response(StatusCode::PAYLOAD_TOO_LARGE, "request is too large"))?;
    if body.len() > MAX_AUTHORITY_JSON_BYTES {
        return Err(error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request is too large",
        ));
    }
    serde_json::from_slice(&body)
        .map_err(|_| error_response(StatusCode::UNPROCESSABLE_ENTITY, "request is invalid"))
}
#[allow(clippy::result_large_err)] // HTTP validation returns its exact refusal.
fn page_request(query: PageQuery) -> Result<PageRequest, Response> {
    let size = PageSize::new(query.size.map(TeachingPageSize::get).unwrap_or(50) as u16)
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, "page size is invalid"))?;
    match query.after {
        Some(value) => Cursor::parse(value)
            .map(|cursor| PageRequest::after(cursor, size))
            .map_err(|_| error_response(StatusCode::BAD_REQUEST, "cursor is invalid")),
        None => Ok(PageRequest::first(size)),
    }
}
#[allow(clippy::result_large_err)] // HTTP validation returns its exact refusal.
fn optional_approval_revision(
    headers: &HeaderMap,
) -> Result<Option<InstructorApprovalRevision>, Response> {
    let Some(value) = one_if_match(headers)? else {
        return Ok(None);
    };
    InstructorApprovalRevision::try_from_i64(value)
        .map(Some)
        .map_err(|_| error_response(StatusCode::UNPROCESSABLE_ENTITY, "If-Match is invalid"))
}
#[allow(clippy::result_large_err)] // HTTP validation returns its exact refusal.
fn required_approval_revision(headers: &HeaderMap) -> Result<InstructorApprovalRevision, Response> {
    optional_approval_revision(headers)?
        .ok_or_else(|| error_response(StatusCode::PRECONDITION_REQUIRED, "If-Match is required"))
}
#[allow(clippy::result_large_err)] // HTTP validation returns its exact refusal.
fn required_invitation_revision(
    headers: &HeaderMap,
) -> Result<CoInstructorInvitationRevision, Response> {
    let value = one_if_match(headers)?
        .ok_or_else(|| error_response(StatusCode::PRECONDITION_REQUIRED, "If-Match is required"))?;
    CoInstructorInvitationRevision::try_from_i64(value)
        .map_err(|_| error_response(StatusCode::UNPROCESSABLE_ENTITY, "If-Match is invalid"))
}
#[allow(clippy::result_large_err)] // HTTP validation returns its exact refusal.
fn required_roster_revision(headers: &HeaderMap) -> Result<RosterRevision, Response> {
    let value = one_if_match(headers)?
        .ok_or_else(|| error_response(StatusCode::PRECONDITION_REQUIRED, "If-Match is required"))?;
    RosterRevision::from_stored(value)
        .map_err(|_| error_response(StatusCode::UNPROCESSABLE_ENTITY, "If-Match is invalid"))
}
#[allow(clippy::result_large_err)] // HTTP validation returns its exact refusal.
fn one_if_match(headers: &HeaderMap) -> Result<Option<i64>, Response> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "If-Match is malformed",
        ));
    }
    let raw = value
        .to_str()
        .ok()
        .and_then(|v| v.strip_prefix('"').and_then(|v| v.strip_suffix('"')))
        .ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "If-Match is malformed"))?;
    if raw.is_empty() || !raw.bytes().all(|v| v.is_ascii_digit()) {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "If-Match is malformed",
        ));
    }
    raw.parse()
        .map(Some)
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, "If-Match is malformed"))
}
fn is_json(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(CONTENT_TYPE).iter();
    values
        .next()
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"))
        })
        && values.next().is_none()
}
fn revision(value: u64) -> TeachingOperationRevision {
    TeachingOperationRevision::new(value).expect("stored revision is valid")
}
fn approval_response(active: bool, value: InstructorApprovalRevision) -> Response {
    response_with_etag(
        StatusCode::OK,
        AccountApprovalView {
            state: if active {
                question_model::teaching_operations::InstructorApprovalStateView::Approved
            } else {
                question_model::teaching_operations::InstructorApprovalStateView::Revoked
            },
            revision: revision(value.as_i64() as u64),
        },
        value.as_i64() as u64,
    )
}
fn created_invitation_response(
    course: CourseId,
    reference: CoInstructorInvitationReference,
    value: CoInstructorInvitationRevision,
) -> Response {
    let mut response = StatusCode::CREATED.into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{}\"", value.as_i64())).expect("etag"),
    );
    response.headers_mut().insert(
        LOCATION,
        HeaderValue::from_str(&format!(
            "/api/courses/{course}/co-instructor-invitations/{reference}"
        ))
        .expect("safe location"),
    );
    no_store(response)
}
fn empty_roster_revision_response(value: RosterRevision) -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{}\"", value.value())).expect("etag"),
    );
    no_store(response)
}
fn response_with_etag<T: serde::Serialize>(status: StatusCode, body: T, value: u64) -> Response {
    let mut response = (status, Json(body)).into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{value}\"")).expect("etag"),
    );
    no_store(response)
}
fn authority_error(error: StoreError) -> Response {
    match error {
        StoreError::Conflict => error_response(
            StatusCode::PRECONDITION_FAILED,
            "authority changed; reload it",
        ),
        StoreError::Forbidden | StoreError::TenantMismatch | StoreError::NotFound => {
            error_response(StatusCode::NOT_FOUND, "authority record not found")
        }
        other => store_error_response(other),
    }
}

#[cfg(test)]
#[path = "authority/tests.rs"]
mod tests;
fn invitation_state(value: CoInstructorInvitationState) -> CoInstructorInvitationStateView {
    match value {
        CoInstructorInvitationState::Pending => CoInstructorInvitationStateView::Pending,
        CoInstructorInvitationState::Expired => CoInstructorInvitationStateView::Expired,
        CoInstructorInvitationState::Accepted => CoInstructorInvitationStateView::Accepted,
        CoInstructorInvitationState::Declined => CoInstructorInvitationStateView::Declined,
        CoInstructorInvitationState::Revoked => CoInstructorInvitationStateView::Revoked,
    }
}
fn course_invitation_view(
    value: learning_data_access::CourseCoInstructorInvitationReferenceView,
) -> CourseCoInstructorInvitationView {
    CourseCoInstructorInvitationView {
        reference: value.reference,
        target: CoInstructorTargetView {
            account: TeachingAccountView {
                reference: value.target,
                display: TeachingDisplayLabel::try_from(value.target_display_name)
                    .expect("stored display is valid"),
            },
            approval: AccountApprovalView {
                state: value.target_approval_state,
                revision: revision(value.target_approval_revision.as_i64() as u64),
            },
        },
        state: invitation_state(value.state),
        created_at: value.created_at,
        expires_at: value.expires_at,
        revision: revision(value.revision.as_i64() as u64),
    }
}
fn pending_invitation_view(
    value: learning_data_access::PendingCoInstructorInvitationReferenceView,
) -> PendingCoInstructorInvitationView {
    PendingCoInstructorInvitationView {
        reference: value.reference,
        course_label: TeachingDisplayLabel::try_from(value.course_title)
            .expect("stored course title is valid"),
        state: CoInstructorInvitationStateView::Pending,
        expires_at: value.expires_at,
        revision: revision(value.revision.as_i64() as u64),
    }
}
fn instructor_view(
    value: learning_data_access::CourseInstructorMembershipReferenceView,
) -> InstructorMembershipView {
    InstructorMembershipView {
        membership: value.membership,
        account: TeachingAccountView {
            reference: value.account,
            display: TeachingDisplayLabel::try_from(value.account_display_name)
                .expect("stored display is valid"),
        },
        status: TeachingMembershipStatus::Active,
    }
}
