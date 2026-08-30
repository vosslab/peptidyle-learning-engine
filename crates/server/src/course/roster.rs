//! Instructor-owned roster HTTP boundary with narrow audited Sysadmin support.

use std::collections::BTreeSet;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use learning_data_access::{
    AllowedEmailDomain, AuthenticationEmail, CourseEnrollmentPolicy, CourseInvitation,
    CourseInvitationDeliveryState, CourseInvitationDeliveryStore, CourseInvitationLifetime,
    CourseInvitationStatus, CourseMemberId, CourseMemberStatus, CourseRecordsAccessStore,
    CourseRosterEntry, CourseRosterId, CourseRosterStore, CourseSignupPosture,
    CreateCourseInvitation, Cursor, PageRequest, PageSize, ReplaceCourseEnrollmentPolicy,
    RevokeCourseInvitation, RevokeCourseMember, RosterIdempotencyKey, RosterRevision, SessionStore,
    Store,
};
use question_model::{ActivityTimestamp, CourseId, UserRole};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthenticatedSession, auth_error_response, no_store, resolve_request_session};
use crate::http_refusal::HttpResult;

use super::invitation_capability::CourseInvitationIssuer;
use super::policy::{course_records_are_visible, require_course_access};
use super::projection::{error_response, store_error_response};

#[path = "roster/import.rs"]
mod import;

const DEFAULT_INVITATION_LIFETIME_SECONDS: u32 = 7 * 24 * 60 * 60;
const DEFAULT_ROSTER_PAGE_SIZE: u16 = 50;
const IDEMPOTENCY_HEADER: &str = "idempotency-key";

struct CourseRosterRouteState<S> {
    store: Arc<S>,
    issuer: CourseInvitationIssuer,
}

impl<S> Clone for CourseRosterRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            issuer: self.issuer.clone(),
        }
    }
}

pub(super) fn roster_router<S>(store: Arc<S>, issuer: CourseInvitationIssuer) -> Router
where
    S: Store
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + SessionStore
        + 'static,
{
    // ASVS 3.5.3, 4.1.4, 8.2.1-8.2.2, 8.3.1: expose only explicit methods whose
    // handlers authenticate and authorize the course-scoped operation server-side.
    Router::new()
        .route("/api/courses/{course}/roster", get(list_roster::<S>))
        .route(
            "/api/courses/{course}/members/{member}",
            delete(revoke_member::<S>),
        )
        .route(
            "/api/courses/{course}/invitations",
            post(create_invitation::<S>),
        )
        .route(
            "/api/courses/{course}/invitations/{invitation}",
            delete(revoke_invitation::<S>),
        )
        .route(
            "/api/courses/{course}/enrollment-policy",
            put(replace_policy::<S>),
        )
        .route(
            "/api/courses/{course}/roster-imports/preview",
            post(import::preview::<S>),
        )
        .route(
            "/api/courses/{course}/roster-imports/{import}/commit",
            post(import::commit::<S>),
        )
        .with_state(CourseRosterRouteState { store, issuer })
}

pub(super) async fn require_roster_support_access<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    course: CourseId,
) -> HttpResult<()>
where
    S: Store + CourseRecordsAccessStore,
{
    if authenticated.record.subject.role() == UserRole::Sysadmin {
        return match course_records_are_visible(store, authenticated, course).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(error_response(StatusCode::NOT_FOUND, "course not found").into()),
            Err(response) => Err(response),
        };
    }
    require_course_access(store, authenticated, course, true).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RosterQuery {
    cursor: Option<String>,
    page_size: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateInvitationRequest {
    email: String,
    roster_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReplacePolicyRequest {
    allowed_email_domains: Vec<AllowedEmailDomainRequest>,
    signup_posture: SignupPostureRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AllowedEmailDomainRequest {
    domain: String,
    include_subdomains: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum SignupPostureRequest {
    InvitationOnly,
    PermittedDomains,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmailEnrollmentRosterResponse {
    roster_mode: &'static str,
    members: Vec<RosterMemberResponse>,
    pending_invitations: Vec<InvitationResponse>,
    allowed_email_domains: Vec<AllowedEmailDomainResponse>,
    signup_posture: &'static str,
    next_cursor: Option<String>,
    roster_revision: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RosterMemberResponse {
    member_id: String,
    display_name: String,
    roster_email: Option<String>,
    roster_id: Option<String>,
    role: &'static str,
    status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InvitationResponse {
    invitation_id: String,
    email: String,
    roster_id: String,
    status: &'static str,
    expires_at: ActivityTimestamp,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AllowedEmailDomainResponse {
    domain: String,
    include_subdomains: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InvitationAcceptedResponse {
    invitation: InvitationResponse,
    redemption_path: String,
    email_delivery: &'static str,
}

async fn list_roster<S>(
    State(state): State<CourseRosterRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Query(query): Query<RosterQuery>,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + SessionStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_roster_support_access(state.store.as_ref(), &authenticated, course).await
    {
        return response.into_response();
    }
    let page = match roster_page_request(query) {
        Ok(page) => page,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    match state
        .store
        .list_course_roster(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
            page,
        )
        .await
    {
        Ok(roster) => roster_response(StatusCode::OK, roster),
        Err(error) => store_error_response(error),
    }
}

async fn create_invitation<S>(
    State(state): State<CourseRosterRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Json(request): Json<CreateInvitationRequest>,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + SessionStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_roster_support_access(state.store.as_ref(), &authenticated, course).await
    {
        return response.into_response();
    }
    let email = match AuthenticationEmail::parse(&request.email) {
        Ok(email) => email,
        Err(_) => return invalid_invitation(),
    };
    let roster_id = match CourseRosterId::parse(&request.roster_id) {
        Ok(roster_id) => roster_id,
        Err(_) => return invalid_invitation(),
    };
    let idempotency_key = match required_idempotency_key(&headers) {
        Ok(key) => key,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    let secret = match state.issuer.issue(
        authenticated.tenant_context.tenant_id(),
        course,
        &email,
        &roster_id,
        &idempotency_key,
    ) {
        Ok(secret) => secret,
        Err(_) => return enrollment_unavailable(),
    };
    let lifetime = CourseInvitationLifetime::from_seconds(DEFAULT_INVITATION_LIFETIME_SECONDS)
        .expect("seven days is inside the course-invitation bound");
    let invitation = match state
        .store
        .create_course_invitation(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            CreateCourseInvitation {
                course,
                email: email.clone(),
                roster_id,
                token_hash: secret.hash(),
                idempotency_key,
                lifetime,
            },
        )
        .await
    {
        Ok(invitation) => invitation,
        Err(error) => return store_error_response(error),
    };
    // The request commits only durable invitation intent. External delivery
    // belongs to the leased server worker: returning here never means an SMTP
    // submission was attempted or accepted.
    let email_delivery = match state
        .store
        .course_invitation_delivery_state(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            course,
            invitation.id,
        )
        .await
    {
        Ok(Some(delivery)) => coarse_delivery_outcome(delivery),
        Ok(None) => return enrollment_unavailable(),
        Err(error) => return store_error_response(error),
    };
    no_store(
        (
            StatusCode::ACCEPTED,
            Json(InvitationAcceptedResponse {
                invitation: invitation_projection(invitation),
                redemption_path: secret.redemption_path(),
                email_delivery,
            }),
        )
            .into_response(),
    )
}

pub(super) fn coarse_delivery_outcome(state: CourseInvitationDeliveryState) -> &'static str {
    match state {
        CourseInvitationDeliveryState::Pending | CourseInvitationDeliveryState::RetryableFailed => {
            "queued"
        }
        CourseInvitationDeliveryState::AcceptedByProvider => "sentToProvider",
        CourseInvitationDeliveryState::Ambiguous
        | CourseInvitationDeliveryState::PermanentFailed => "needsAttention",
        CourseInvitationDeliveryState::Cancelled => "cancelled",
    }
}

async fn replace_policy<S>(
    State(state): State<CourseRosterRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Json(request): Json<ReplacePolicyRequest>,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + SessionStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_roster_support_access(state.store.as_ref(), &authenticated, course).await
    {
        return response.into_response();
    }
    let expected_revision = match required_roster_revision(&headers) {
        Ok(revision) => revision,
        Err(RevisionHeaderError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match roster revision is required",
            );
        }
        Err(RevisionHeaderError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match roster revision is invalid",
            );
        }
    };
    let allowed_domains = match request
        .allowed_email_domains
        .into_iter()
        .map(|rule| {
            Ok(AllowedEmailDomain {
                domain: learning_data_access::EmailDomain::parse(&rule.domain).map_err(|_| ())?,
                include_subdomains: rule.include_subdomains,
            })
        })
        .collect::<Result<BTreeSet<_>, ()>>()
    {
        Ok(domains) => domains,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "course enrollment policy is invalid",
            );
        }
    };
    let signup_posture = match request.signup_posture {
        SignupPostureRequest::InvitationOnly => CourseSignupPosture::InvitationOnly,
        SignupPostureRequest::PermittedDomains => CourseSignupPosture::PermittedDomains,
    };
    match state
        .store
        .replace_course_enrollment_policy(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            ReplaceCourseEnrollmentPolicy {
                course,
                expected_revision,
                allowed_domains,
                signup_posture,
            },
        )
        .await
    {
        Ok(policy) => policy_response(StatusCode::OK, policy),
        Err(error) => store_error_response(error),
    }
}

async fn revoke_member<S>(
    State(state): State<CourseRosterRouteState<S>>,
    headers: HeaderMap,
    Path((course, member)): Path<(CourseId, uuid::Uuid)>,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + SessionStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_roster_support_access(state.store.as_ref(), &authenticated, course).await
    {
        return response.into_response();
    }
    let expected_revision = match required_roster_revision(&headers) {
        Ok(revision) => revision,
        Err(RevisionHeaderError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match roster revision is required",
            );
        }
        Err(RevisionHeaderError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match roster revision is invalid",
            );
        }
    };
    match state
        .store
        .revoke_course_member(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            RevokeCourseMember {
                course,
                member: CourseMemberId::from_uuid(member),
                expected_revision,
            },
        )
        .await
    {
        Ok(revision) => empty_revision_response(revision),
        Err(error) => store_error_response(error),
    }
}

async fn revoke_invitation<S>(
    State(state): State<CourseRosterRouteState<S>>,
    headers: HeaderMap,
    Path((course, invitation)): Path<(CourseId, uuid::Uuid)>,
) -> Response
where
    S: Store
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + SessionStore
        + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_roster_support_access(state.store.as_ref(), &authenticated, course).await
    {
        return response.into_response();
    }
    let expected_revision = match required_roster_revision(&headers) {
        Ok(revision) => revision,
        Err(RevisionHeaderError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match roster revision is required",
            );
        }
        Err(RevisionHeaderError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match roster revision is invalid",
            );
        }
    };
    match state
        .store
        .revoke_course_invitation(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            RevokeCourseInvitation {
                course,
                invitation: learning_data_access::CourseInvitationId::from_uuid(invitation),
                expected_revision,
            },
        )
        .await
    {
        Ok(revision) => empty_revision_response(revision),
        Err(error) => store_error_response(error),
    }
}

fn roster_response(status: StatusCode, roster: learning_data_access::CourseRosterPage) -> Response {
    let revision = roster.policy.revision;
    let mut members = Vec::new();
    let mut pending_invitations = Vec::new();
    for entry in roster.entries.items {
        match entry {
            CourseRosterEntry::Member(member) => members.push(RosterMemberResponse {
                member_id: member.id.as_uuid().to_string(),
                display_name: member.display_name,
                roster_email: member
                    .roster_email
                    .map(|email| email.delivery().to_string()),
                roster_id: member.roster_id.map(|value| value.as_str().to_string()),
                role: "student",
                status: member_status(member.status),
            }),
            CourseRosterEntry::Invitation(invitation) => {
                pending_invitations.push(invitation_projection(invitation));
            }
        }
    }
    let next_cursor = roster
        .entries
        .next_cursor
        .map(|cursor| cursor.as_str().to_string());
    response_with_revision(
        status,
        EmailEnrollmentRosterResponse {
            roster_mode: "emailEnrollment",
            members,
            pending_invitations,
            allowed_email_domains: roster
                .policy
                .allowed_domains
                .into_iter()
                .map(|rule| AllowedEmailDomainResponse {
                    domain: rule.domain.as_str().to_string(),
                    include_subdomains: rule.include_subdomains,
                })
                .collect(),
            signup_posture: posture_name(roster.policy.signup_posture),
            next_cursor,
            roster_revision: revision.value(),
        },
        revision,
    )
}

fn invitation_projection(invitation: CourseInvitation) -> InvitationResponse {
    InvitationResponse {
        invitation_id: invitation.id.as_uuid().to_string(),
        email: invitation.email.delivery().to_string(),
        roster_id: invitation.roster_id.as_str().to_string(),
        status: invitation_status(invitation.status),
        expires_at: invitation.expires_at,
    }
}

fn policy_response(status: StatusCode, policy: CourseEnrollmentPolicy) -> Response {
    let revision = policy.revision;
    let body = serde_json::json!({
        "allowedEmailDomains": policy.allowed_domains.into_iter().map(|rule| {
            serde_json::json!({
                "domain": rule.domain.as_str(),
                "includeSubdomains": rule.include_subdomains,
            })
        }).collect::<Vec<_>>(),
        "signupPosture": posture_name(policy.signup_posture),
        "rosterRevision": revision.value(),
    });
    response_with_revision(status, body, revision)
}

fn empty_revision_response(revision: RosterRevision) -> Response {
    response_with_revision(
        StatusCode::OK,
        serde_json::json!({ "rosterRevision": revision.value() }),
        revision,
    )
}

fn response_with_revision<T: Serialize>(
    status: StatusCode,
    body: T,
    revision: RosterRevision,
) -> Response {
    let mut response = (status, Json(body)).into_response();
    let etag = HeaderValue::from_str(&format!("\"{}\"", revision.value()))
        .expect("positive roster revision forms a valid ETag");
    response.headers_mut().insert(ETAG, etag);
    no_store(response)
}

fn roster_page_request(query: RosterQuery) -> Result<PageRequest, &'static str> {
    let size = PageSize::new(query.page_size.unwrap_or(DEFAULT_ROSTER_PAGE_SIZE))
        .map_err(|_| "roster page size is invalid")?;
    match query.cursor {
        Some(cursor) => Ok(PageRequest::after(
            Cursor::parse(cursor).map_err(|_| "roster cursor is invalid")?,
            size,
        )),
        None => Ok(PageRequest::first(size)),
    }
}

fn required_idempotency_key(headers: &HeaderMap) -> Result<RosterIdempotencyKey, &'static str> {
    let mut values = headers.get_all(IDEMPOTENCY_HEADER).iter();
    let value = values
        .next()
        .ok_or("idempotency-key is required")?
        .to_str()
        .map_err(|_| "idempotency-key is invalid")?;
    if values.next().is_some() {
        return Err("idempotency-key is invalid");
    }
    RosterIdempotencyKey::parse(value).map_err(|_| "idempotency-key is invalid")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RevisionHeaderError {
    Missing,
    Malformed,
}

fn required_roster_revision(headers: &HeaderMap) -> Result<RosterRevision, RevisionHeaderError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let value = values.next().ok_or(RevisionHeaderError::Missing)?;
    if values.next().is_some() {
        return Err(RevisionHeaderError::Malformed);
    }
    let value = value
        .to_str()
        .map_err(|_| RevisionHeaderError::Malformed)?
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or(RevisionHeaderError::Malformed)?;
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RevisionHeaderError::Malformed);
    }
    let value = value
        .parse::<i64>()
        .map_err(|_| RevisionHeaderError::Malformed)?;
    RosterRevision::from_stored(value).map_err(|_| RevisionHeaderError::Malformed)
}

fn member_status(status: CourseMemberStatus) -> &'static str {
    match status {
        CourseMemberStatus::Active => "active",
        CourseMemberStatus::Revoked => "revoked",
    }
}

fn invitation_status(status: CourseInvitationStatus) -> &'static str {
    match status {
        CourseInvitationStatus::Pending => "pending",
        CourseInvitationStatus::Claimed => "claimed",
        CourseInvitationStatus::Revoked => "revoked",
        CourseInvitationStatus::Expired => "expired",
    }
}

fn posture_name(posture: CourseSignupPosture) -> &'static str {
    match posture {
        CourseSignupPosture::InvitationOnly => "invitationOnly",
        CourseSignupPosture::PermittedDomains => "permittedDomains",
    }
}

fn invalid_invitation() -> Response {
    error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        "course invitation is invalid",
    )
}

fn enrollment_unavailable() -> Response {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "course invitation delivery is unavailable",
    )
}
