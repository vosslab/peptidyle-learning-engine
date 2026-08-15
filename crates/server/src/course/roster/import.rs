//! Bounded CSV roster preview and explicit atomic commit boundary.

use std::collections::BTreeSet;

use axum::Json;
use axum::body::to_bytes;
use axum::extract::{Path, Request, State};
use axum::http::header::{CONTENT_TYPE, ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    AuthenticationEmail, CommitCourseRosterImport, CourseInvitationDeliveryStore,
    CourseInvitationLifetime, CourseRosterId, CourseRosterImportId, CourseRosterImportLifetime,
    CourseRosterImportPreview, CourseRosterImportRowInput, CourseRosterImportState,
    RosterImportInvitation, RosterImportRevision, RosterImportRowStatus, StageCourseRosterImport,
};
use objects::Sha256Digest;
use question_model::CourseId;
use serde::{Deserialize, Serialize};

use super::{
    CourseRosterRouteState, DEFAULT_INVITATION_LIFETIME_SECONDS, RevisionHeaderError,
    coarse_delivery_outcome, enrollment_unavailable, require_roster_support_access,
    required_idempotency_key, required_roster_revision,
};
use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::course::projection::{error_response, store_error_response};

const MAX_ROSTER_CSV_BYTES: usize = 1_048_576;
const MAX_COMMIT_BODY_BYTES: usize = 64 * 1_024;
const DEFAULT_PREVIEW_LIFETIME_SECONDS: u32 = 60 * 60;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewResponse {
    import_id: String,
    state: &'static str,
    expires_at: question_model::ActivityTimestamp,
    roster_revision: u64,
    import_revision: u64,
    rows: Vec<PreviewRowResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewRowResponse {
    row_number: u16,
    email: Option<String>,
    roster_id: Option<String>,
    status: &'static str,
    reason: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CommitRequest {
    row_numbers: Vec<u16>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommitResponse {
    import_id: String,
    import_revision: u64,
    roster_revision: u64,
    invitations_created: usize,
    delivery: Vec<CommitDeliveryResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommitDeliveryResponse {
    row_number: u16,
    outcome: &'static str,
}

pub(super) async fn preview<S>(
    State(state): State<CourseRosterRouteState<S>>,
    Path(course): Path<CourseId>,
    request: Request,
) -> Response
where
    S: learning_data_access::Store
        + learning_data_access::CourseRecordsAccessStore
        + learning_data_access::CourseRosterStore
        + CourseInvitationDeliveryStore
        + learning_data_access::SessionStore
        + 'static,
{
    let headers = request.headers().clone();
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_roster_support_access(state.store.as_ref(), &authenticated, course).await
    {
        return response;
    }
    if !is_csv(&headers) {
        return error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "roster import must be CSV",
        );
    }
    let expected_roster_revision = match required_roster_revision(&headers) {
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
    let idempotency_key = match required_idempotency_key(&headers) {
        Ok(key) => key,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    let body = match to_bytes(request.into_body(), MAX_ROSTER_CSV_BYTES).await {
        Ok(body) => body,
        Err(_) => return error_response(StatusCode::PAYLOAD_TOO_LARGE, "roster CSV is too large"),
    };
    let rows = match parse_csv(&body) {
        Ok(rows) => rows,
        Err(message) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    let normalized_digest = normalized_digest(&rows);
    let lifetime = CourseRosterImportLifetime::from_seconds(DEFAULT_PREVIEW_LIFETIME_SECONDS)
        .expect("one hour is inside the roster-preview lifetime bound");
    match state
        .store
        .stage_course_roster_import(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            StageCourseRosterImport {
                course,
                expected_roster_revision,
                normalized_digest,
                idempotency_key,
                rows,
                lifetime,
            },
        )
        .await
    {
        Ok(preview) => preview_response(preview),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn commit<S>(
    State(state): State<CourseRosterRouteState<S>>,
    Path((course, import)): Path<(CourseId, uuid::Uuid)>,
    request: Request,
) -> Response
where
    S: learning_data_access::Store
        + learning_data_access::CourseRecordsAccessStore
        + learning_data_access::CourseRosterStore
        + CourseInvitationDeliveryStore
        + learning_data_access::SessionStore
        + 'static,
{
    let headers = request.headers().clone();
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if let Err(response) =
        require_roster_support_access(state.store.as_ref(), &authenticated, course).await
    {
        return response;
    }
    let expected_import_revision = match required_import_revision(&headers) {
        Ok(revision) => revision,
        Err(RevisionHeaderError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match roster import revision is required",
            );
        }
        Err(RevisionHeaderError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match roster import revision is invalid",
            );
        }
    };
    let idempotency_key = match required_idempotency_key(&headers) {
        Ok(key) => key,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    let body = match to_bytes(request.into_body(), MAX_COMMIT_BODY_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "roster import commit is too large",
            );
        }
    };
    let request = match serde_json::from_slice::<CommitRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "roster import commit is invalid",
            );
        }
    };
    let supplied_row_count = request.row_numbers.len();
    let row_numbers = request.row_numbers.into_iter().collect::<BTreeSet<_>>();
    if row_numbers.is_empty()
        || row_numbers.len() != supplied_row_count
        || row_numbers.len() > learning_data_access::MAX_ROSTER_IMPORT_ROWS
        || row_numbers.iter().any(|row_number| *row_number < 2)
    {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "roster import commit is invalid",
        );
    }
    let import = CourseRosterImportId::from_uuid(import);
    let mut secrets = Vec::with_capacity(row_numbers.len());
    let invitation_lifetime =
        CourseInvitationLifetime::from_seconds(DEFAULT_INVITATION_LIFETIME_SECONDS)
            .expect("seven days is inside the course-invitation bound");
    for row_number in row_numbers {
        let (secret, row_key) = match state.issuer.issue_import(
            authenticated.tenant_context.tenant_id(),
            course,
            import,
            row_number,
            &idempotency_key,
        ) {
            Ok(value) => value,
            Err(_) => return enrollment_unavailable(),
        };
        secrets.push((row_number, secret, row_key));
    }
    let committed = match state
        .store
        .commit_course_roster_import(
            authenticated.tenant_context,
            authenticated.record.token_hash,
            CommitCourseRosterImport {
                course,
                import,
                expected_import_revision,
                idempotency_key,
                invitations: secrets
                    .iter()
                    .map(|(row_number, secret, row_key)| RosterImportInvitation {
                        row_number: *row_number,
                        token_hash: secret.hash(),
                        idempotency_key: row_key.clone(),
                        lifetime: invitation_lifetime,
                    })
                    .collect(),
            },
        )
        .await
    {
        Ok(committed) => committed,
        Err(error) => return store_error_response(error),
    };
    // Committing the import creates the durable delivery rows in the same
    // Store transaction.  Never send here: a request retry must replay the
    // stored commit rather than re-submit every message.
    let mut delivery = Vec::with_capacity(committed.invitations.len());
    for (row_number, invitation) in &committed.invitations {
        let state = match state
            .store
            .course_invitation_delivery_state(
                authenticated.tenant_context,
                authenticated.record.token_hash,
                course,
                invitation.id,
            )
            .await
        {
            Ok(Some(value)) => value,
            Ok(None) => return enrollment_unavailable(),
            Err(error) => return store_error_response(error),
        };
        delivery.push(CommitDeliveryResponse {
            row_number: *row_number,
            outcome: coarse_delivery_outcome(state),
        });
    }
    commit_response(committed, delivery)
}

fn parse_csv(body: &[u8]) -> Result<Vec<CourseRosterImportRowInput>, &'static str> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .trim(csv::Trim::None)
        .from_reader(body);
    let headers = reader
        .headers()
        .map_err(|_| "roster CSV header is invalid")?;
    if headers.len() != 2 || headers.get(0) != Some("email") || headers.get(1) != Some("roster_id")
    {
        return Err("roster CSV header must be email,roster_id");
    }
    let mut rows = Vec::new();
    for (index, record) in reader.records().enumerate() {
        if rows.len() == learning_data_access::MAX_ROSTER_IMPORT_ROWS {
            return Err("roster CSV has too many rows");
        }
        let record = record.map_err(|_| "roster CSV row is malformed")?;
        let row_number = u16::try_from(index + 2).map_err(|_| "roster CSV has too many rows")?;
        let normalized = match (record.get(0), record.get(1)) {
            (Some(email), Some(roster_id)) => {
                match (
                    AuthenticationEmail::parse(email),
                    CourseRosterId::parse(roster_id),
                ) {
                    (Ok(email), Ok(roster_id)) => (Some(email), Some(roster_id)),
                    _ => (None, None),
                }
            }
            _ => (None, None),
        };
        rows.push(CourseRosterImportRowInput {
            row_number,
            email: normalized.0,
            roster_id: normalized.1,
        });
    }
    if rows.is_empty() {
        return Err("roster CSV has no data rows");
    }
    Ok(rows)
}

fn normalized_digest(rows: &[CourseRosterImportRowInput]) -> Sha256Digest {
    let mut canonical = Vec::new();
    canonical.extend_from_slice(b"ple-normalized-roster-import-v1\0");
    canonical.extend_from_slice(&(rows.len() as u64).to_be_bytes());
    for row in rows {
        canonical.extend_from_slice(&row.row_number.to_be_bytes());
        match (&row.email, &row.roster_id) {
            (Some(email), Some(roster_id)) => {
                append_part(&mut canonical, email.normalized().as_bytes());
                append_part(&mut canonical, roster_id.as_str().as_bytes());
            }
            _ => canonical.push(0xff),
        }
    }
    Sha256Digest::compute(&canonical)
}

fn append_part(target: &mut Vec<u8>, value: &[u8]) {
    target.extend_from_slice(&(value.len() as u64).to_be_bytes());
    target.extend_from_slice(value);
}

fn preview_response(preview: CourseRosterImportPreview) -> Response {
    let import_revision = preview.revision;
    let response = PreviewResponse {
        import_id: preview.id.as_uuid().to_string(),
        state: match preview.state {
            CourseRosterImportState::Preview => "preview",
            CourseRosterImportState::Committed => "committed",
        },
        expires_at: preview.expires_at,
        roster_revision: preview.roster_revision.value(),
        import_revision: preview.revision.value(),
        rows: preview
            .rows
            .into_iter()
            .map(|row| {
                let safe_to_echo = row.status != RosterImportRowStatus::Invalid;
                PreviewRowResponse {
                    row_number: row.row_number,
                    email: safe_to_echo
                        .then(|| row.email.map(|email| email.delivery().to_string()))
                        .flatten(),
                    roster_id: safe_to_echo
                        .then(|| row.roster_id.map(|value| value.as_str().to_string()))
                        .flatten(),
                    status: status_name(row.status),
                    reason: reason_name(row.status),
                }
            })
            .collect(),
    };
    response_with_import_revision(StatusCode::OK, response, import_revision)
}

fn commit_response(
    committed: learning_data_access::CommittedCourseRosterImport,
    delivery: Vec<CommitDeliveryResponse>,
) -> Response {
    let revision = committed.import_revision;
    response_with_import_revision(
        StatusCode::OK,
        CommitResponse {
            import_id: committed.import.as_uuid().to_string(),
            import_revision: committed.import_revision.value(),
            roster_revision: committed.roster_revision.value(),
            invitations_created: committed.invitations.len(),
            delivery,
        },
        revision,
    )
}

fn response_with_import_revision<T: Serialize>(
    status: StatusCode,
    body: T,
    revision: RosterImportRevision,
) -> Response {
    let mut response = (status, Json(body)).into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{}\"", revision.value()))
            .expect("positive import revision forms a valid ETag"),
    );
    no_store(response)
}

fn required_import_revision(
    headers: &HeaderMap,
) -> Result<RosterImportRevision, RevisionHeaderError> {
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
    RosterImportRevision::from_stored(value).map_err(|_| RevisionHeaderError::Malformed)
}

fn is_csv(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/csv"))
}

fn status_name(status: RosterImportRowStatus) -> &'static str {
    match status {
        RosterImportRowStatus::ReadyToInvite => "readyToInvite",
        RosterImportRowStatus::AlreadyMember => "alreadyMember",
        RosterImportRowStatus::AlreadyPending => "alreadyPending",
        RosterImportRowStatus::Duplicate => "duplicate",
        RosterImportRowStatus::Invalid => "invalid",
    }
}

/// Safe instructional category: never source CSV text or account identity.
fn reason_name(status: RosterImportRowStatus) -> &'static str {
    match status {
        RosterImportRowStatus::ReadyToInvite => "ready",
        RosterImportRowStatus::AlreadyMember => "alreadyOnRoster",
        RosterImportRowStatus::AlreadyPending => "invitationPending",
        RosterImportRowStatus::Duplicate => "duplicateInFile",
        RosterImportRowStatus::Invalid => "correctEmailOrRosterId",
    }
}
