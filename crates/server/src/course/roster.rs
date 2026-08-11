//! Manager-only course roster HTTP boundary and invitation delivery seam.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{Path, Query, State};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use learning_data_access::{
    AllowedEmailDomain, AuthenticationEmail, CourseEnrollmentPolicy, CourseInvitation,
    CourseInvitationLifetime, CourseInvitationSecretHash, CourseInvitationStatus, CourseMemberId,
    CourseMemberStatus, CourseRecordsAccessStore, CourseRosterEntry, CourseRosterId,
    CourseRosterStore, CourseSignupPosture, CreateCourseInvitation, Cursor, PageRequest, PageSize,
    ReplaceCourseEnrollmentPolicy, RevokeCourseInvitation, RevokeCourseMember,
    RosterIdempotencyKey, RosterRevision, SessionStore, Store,
};
use question_model::{ActivityTimestamp, CourseId, TenantId, UserId, UserRole};
use serde::{Deserialize, Serialize};

use crate::auth::{auth_error_response, no_store, resolve_request_session};

use super::policy::require_course_access;
use super::projection::{error_response, store_error_response};

#[path = "roster/export.rs"]
mod export;
#[path = "roster/import.rs"]
mod import;

const INVITATION_TOKEN_BYTES: usize = 32;
const DEFAULT_INVITATION_LIFETIME_SECONDS: u32 = 7 * 24 * 60 * 60;
const DEFAULT_ROSTER_PAGE_SIZE: u16 = 50;
const IDEMPOTENCY_HEADER: &str = "idempotency-key";

/// Redacted raw invitation capability used only between issuer and mailer.
pub struct CourseInvitationSecret([u8; INVITATION_TOKEN_BYTES]);

impl CourseInvitationSecret {
    /// Canonical URL-safe value consumed by an invitation delivery or the
    /// manager-only one-time response. It must never be logged or persisted.
    pub fn encoded(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }

    fn redemption_path(&self) -> String {
        format!("/course-invitations/redeem#token={}", self.encoded())
    }

    fn hash(&self) -> CourseInvitationSecretHash {
        CourseInvitationSecretHash::compute(&self.0)
    }
}

impl std::fmt::Debug for CourseInvitationSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CourseInvitationSecret([redacted])")
    }
}

/// Server-held keyed issuer. Replaying the same idempotent request reproduces
/// the same secret without storing it or returning it to the browser.
#[derive(Clone)]
pub struct CourseInvitationIssuer(Option<[u8; 32]>);

impl CourseInvitationIssuer {
    /// Creates a configured issuer from a dedicated 256-bit server secret.
    pub fn from_server_secret(secret: [u8; 32]) -> Self {
        Self(Some(secret))
    }

    /// Fail-closed issuer for deployments without invitation configuration.
    pub fn unavailable() -> Self {
        Self(None)
    }

    fn issue(
        &self,
        tenant: question_model::TenantId,
        course: CourseId,
        email: &AuthenticationEmail,
        roster_id: &CourseRosterId,
        idempotency_key: &RosterIdempotencyKey,
    ) -> Result<CourseInvitationSecret, CourseInvitationDeliveryError> {
        let secret = self.0.ok_or(CourseInvitationDeliveryError::Unavailable)?;
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(&secret)
            .map_err(|_| CourseInvitationDeliveryError::Unavailable)?;
        mac.update(b"ple-course-invitation-v1\0");
        update_mac_part(&mut mac, tenant.as_uuid().as_bytes());
        update_mac_part(&mut mac, course.as_uuid().as_bytes());
        update_mac_part(&mut mac, email.normalized().as_bytes());
        update_mac_part(&mut mac, roster_id.as_str().as_bytes());
        update_mac_part(&mut mac, idempotency_key.as_str().as_bytes());
        Ok(CourseInvitationSecret(mac.finalize().into_bytes().into()))
    }

    fn issue_import(
        &self,
        tenant: question_model::TenantId,
        course: CourseId,
        import: learning_data_access::CourseRosterImportId,
        row_number: u16,
        idempotency_key: &RosterIdempotencyKey,
    ) -> Result<(CourseInvitationSecret, RosterIdempotencyKey), CourseInvitationDeliveryError> {
        let secret = self.0.ok_or(CourseInvitationDeliveryError::Unavailable)?;
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(&secret)
            .map_err(|_| CourseInvitationDeliveryError::Unavailable)?;
        mac.update(b"ple-course-roster-import-v1\0");
        update_mac_part(&mut mac, tenant.as_uuid().as_bytes());
        update_mac_part(&mut mac, course.as_uuid().as_bytes());
        update_mac_part(&mut mac, import.as_uuid().as_bytes());
        update_mac_part(&mut mac, &row_number.to_be_bytes());
        update_mac_part(&mut mac, idempotency_key.as_str().as_bytes());
        let invitation_secret = CourseInvitationSecret(mac.finalize().into_bytes().into());
        let row_key = format!(
            "bulk-{}",
            URL_SAFE_NO_PAD.encode(invitation_secret.hash().as_bytes())
        );
        let row_key = RosterIdempotencyKey::parse(&row_key)
            .map_err(|_| CourseInvitationDeliveryError::Unavailable)?;
        Ok((invitation_secret, row_key))
    }
}

impl std::fmt::Debug for CourseInvitationIssuer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CourseInvitationIssuer")
            .field("configured", &self.0.is_some())
            .finish()
    }
}

fn update_mac_part(mac: &mut Hmac<sha2::Sha256>, value: &[u8]) {
    mac.update(&(value.len() as u64).to_be_bytes());
    mac.update(value);
}

/// Mail-delivery failure with no recipient, token, or provider diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseInvitationDeliveryError {
    /// No mail service is configured or the service cannot accept the message.
    Unavailable,
}

/// Server-only invitation delivery. Implementations must never log the URL.
#[async_trait]
pub trait CourseInvitationDelivery: Send + Sync {
    /// Returns false before any roster mutation when delivery is unavailable.
    fn is_configured(&self) -> bool;

    /// Sends one invitation without exposing the account-existence outcome.
    async fn send_course_invitation(
        &self,
        email: &AuthenticationEmail,
        invitation_secret: &CourseInvitationSecret,
    ) -> Result<(), CourseInvitationDeliveryError>;
}

/// Fail-closed delivery used when production mail settings are absent.
#[derive(Debug, Clone, Copy)]
pub struct UnavailableCourseInvitationDelivery;

#[async_trait]
impl CourseInvitationDelivery for UnavailableCourseInvitationDelivery {
    fn is_configured(&self) -> bool {
        false
    }

    async fn send_course_invitation(
        &self,
        _email: &AuthenticationEmail,
        _invitation_secret: &CourseInvitationSecret,
    ) -> Result<(), CourseInvitationDeliveryError> {
        Err(CourseInvitationDeliveryError::Unavailable)
    }
}

struct CourseRosterRouteState<S> {
    store: Arc<S>,
    issuer: CourseInvitationIssuer,
    delivery: Arc<dyn CourseInvitationDelivery>,
    local_development_roster: Option<Arc<LocalDevelopmentRosterDirectory>>,
}

impl<S> Clone for CourseRosterRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            issuer: self.issuer.clone(),
            delivery: Arc::clone(&self.delivery),
            local_development_roster: self.local_development_roster.clone(),
        }
    }
}

/// Server-owned configured learner aliases for the local development router.
#[derive(Clone)]
pub(crate) struct LocalDevelopmentRosterDirectory {
    identities: BTreeMap<String, LocalDevelopmentRosterIdentity>,
}

#[derive(Clone)]
pub(crate) struct LocalDevelopmentRosterIdentity {
    pub(crate) tenant: TenantId,
    pub(crate) user: UserId,
    pub(crate) display_name: String,
    pub(crate) roles: Vec<UserRole>,
}

impl LocalDevelopmentRosterDirectory {
    pub(crate) fn new(
        identities: impl IntoIterator<Item = (String, LocalDevelopmentRosterIdentity)>,
    ) -> Option<Self> {
        let mut resolved = BTreeMap::new();
        for (alias, identity) in identities {
            if resolved.insert(alias, identity).is_some() {
                return None;
            }
        }
        (!resolved.is_empty()).then_some(Self {
            identities: resolved,
        })
    }

    fn learner(&self, alias: &str, tenant: TenantId) -> Option<&LocalDevelopmentRosterIdentity> {
        self.identities.get(alias).filter(|identity| {
            identity.tenant == tenant && identity.roles.as_slice() == [UserRole::Student]
        })
    }
}

pub(super) fn roster_router<S>(
    store: Arc<S>,
    issuer: CourseInvitationIssuer,
    delivery: Arc<dyn CourseInvitationDelivery>,
    local_development_roster: Option<Arc<LocalDevelopmentRosterDirectory>>,
) -> Router
where
    S: Store
        + CourseRecordsAccessStore
        + CourseRosterStore
        + learning_data_access::ManualGradeExportStore
        + SessionStore
        + 'static,
{
    let router = Router::new()
        .route("/api/courses/{course}/roster", get(list_roster::<S>))
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
            "/api/courses/{course}/members/{member}",
            delete(revoke_member::<S>),
        )
        .route(
            "/api/courses/{course}/roster-imports/preview",
            post(import::preview::<S>),
        )
        .route(
            "/api/courses/{course}/roster-imports/{import}/commit",
            post(import::commit::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/grade-export.csv",
            post(export::create::<S>),
        );
    let router = if local_development_roster.is_some() {
        router.route(
            "/api/courses/{course}/local-development-members",
            post(activate_local_development_member::<S>),
        )
    } else {
        router
    };
    router.with_state(CourseRosterRouteState {
        store,
        issuer,
        delivery,
        local_development_roster,
    })
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
struct RosterResponse {
    members: Vec<RosterMemberResponse>,
    pending_invitations: Vec<InvitationResponse>,
    allowed_email_domains: Vec<AllowedEmailDomainResponse>,
    signup_posture: &'static str,
    local_development_roster: bool,
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
    source: &'static str,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActivateLocalDevelopmentMemberRequest {
    learner_alias: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalDevelopmentMemberAcceptedResponse {
    member: RosterMemberResponse,
    roster_revision: u64,
}

async fn list_roster<S>(
    State(state): State<CourseRosterRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Query(query): Query<RosterQuery>,
) -> Response
where
    S: Store + CourseRecordsAccessStore + CourseRosterStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
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
        Ok(roster) => roster_response(
            StatusCode::OK,
            roster,
            state.local_development_roster.is_some(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn activate_local_development_member<S>(
    State(state): State<CourseRosterRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Json(request): Json<ActivateLocalDevelopmentMemberRequest>,
) -> Response
where
    S: Store + CourseRecordsAccessStore + CourseRosterStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
    }
    let Some(directory) = &state.local_development_roster else {
        return error_response(
            StatusCode::NOT_FOUND,
            "local development roster is unavailable",
        );
    };
    let Some(learner) = directory.learner(
        &request.learner_alias,
        authenticated.tenant_context.tenant_id(),
    ) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "configured local learner was not found",
        );
    };
    match state
        .store
        .activate_local_development_course_member(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            learning_data_access::ActivateLocalDevelopmentCourseMember {
                course,
                learner_user: learner.user,
                learner_display_name: learner.display_name.clone(),
            },
        )
        .await
    {
        Ok(accepted) => local_development_member_response(accepted),
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
    S: Store + CourseRecordsAccessStore + CourseRosterStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
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
    let email_delivery = if state.delivery.is_configured()
        && state
            .delivery
            .send_course_invitation(&email, &secret)
            .await
            .is_ok()
    {
        "sent"
    } else {
        "notSent"
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

async fn replace_policy<S>(
    State(state): State<CourseRosterRouteState<S>>,
    headers: HeaderMap,
    Path(course): Path<CourseId>,
    Json(request): Json<ReplacePolicyRequest>,
) -> Response
where
    S: Store + CourseRecordsAccessStore + CourseRosterStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
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
    S: Store + CourseRecordsAccessStore + CourseRosterStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
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
    S: Store + CourseRecordsAccessStore + CourseRosterStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_course_access(state.store.as_ref(), &authenticated, course, true).await
    {
        return response;
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

fn roster_response(
    status: StatusCode,
    roster: learning_data_access::CourseRosterPage,
    local_development_roster: bool,
) -> Response {
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
                source: member_source(member.source),
                role: "student",
                status: member_status(member.status),
            }),
            CourseRosterEntry::Invitation(invitation) => {
                pending_invitations.push(invitation_projection(invitation));
            }
        }
    }
    let response = RosterResponse {
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
        local_development_roster,
        next_cursor: roster
            .entries
            .next_cursor
            .map(|cursor| cursor.as_str().to_string()),
        roster_revision: revision.value(),
    };
    response_with_revision(status, response, revision)
}

fn local_development_member_response(
    accepted: learning_data_access::ClaimedCourseMembership,
) -> Response {
    let revision = accepted.roster_revision;
    let member = accepted.member;
    response_with_revision(
        StatusCode::OK,
        LocalDevelopmentMemberAcceptedResponse {
            member: RosterMemberResponse {
                member_id: member.id.as_uuid().to_string(),
                display_name: member.display_name,
                roster_email: None,
                roster_id: None,
                source: "localDevelopment",
                role: "student",
                status: member_status(member.status),
            },
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

fn member_source(source: learning_data_access::CourseRosterMemberSource) -> &'static str {
    match source {
        learning_data_access::CourseRosterMemberSource::Invitation => "invitation",
        learning_data_access::CourseRosterMemberSource::LocalDevelopment => "localDevelopment",
        learning_data_access::CourseRosterMemberSource::Legacy => "legacy",
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
