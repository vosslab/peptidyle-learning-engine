//! Author-only QTI archive upload and answer-free profile reports.
//!
//! The upload path authorizes an existing actor-visible draft before reading
//! archive bytes. The worker owns ZIP/XML parsing; GET projects only the
//! validated persisted registry produced by that worker.

use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{DefaultBodyLimit, Path, Request, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use learning_data_access::{
    AuthoritativeTimeStore, QTI_PROFILE_ARCHIVE_MEDIA_TYPE, QtiImportApiState, QtiImportApiStore,
    QtiImportApiView, QtiImportItemStatus, QtiImportRef, QtiImportRegistry, QtiUnsupportedFeature,
    QueueQtiImportCommand, SessionStore, Store, StoreError,
};
use objects::{
    ObjectCategory, ObjectKey, ObjectRecord, ObjectStore, ObjectStoreError, PutObject,
    Sha256Digest, StoredObject, workspace_qti_archive_object_id,
};
use question_model::{UserRole, WorkspaceId, WorkspaceImportId};
use serde::Serialize;

use crate::auth::{auth_error_response, no_store, resolve_request_session};

const MAX_QTI_ARCHIVE_BODY_BYTES: usize = 32 * 1024 * 1024;
const QTI_IMPORT_MAX_ATTEMPTS: u16 = 3;
const WORKSPACE_ARCHIVE_LICENSE: &str = "allRightsReserved";
const WORKSPACE_ARCHIVE_PROVENANCE: &str = "author-uploaded QTI workspace import archive";
const REPORT_REVISION_DOMAIN: &[u8] = b"ple:qti-profile-visible-report:v1\0";
const REVIEW_TOKEN_DOMAIN: &[u8] = b"ple:qti-profile-visible-review:v1\0";

/// Builds the author-only archive upload and status/report route group.
pub fn router<S, O>(store: Arc<S>, objects: Arc<O>) -> Router
where
    S: Store + QtiImportApiStore + SessionStore + AuthoritativeTimeStore + 'static,
    O: ObjectStore + 'static,
{
    Router::new()
        .route(
            "/api/workspaces/{workspace}/qti-imports/{import}",
            get(get_qti_import::<S, O>).put(put_qti_import::<S, O>),
        )
        .layer(DefaultBodyLimit::max(MAX_QTI_ARCHIVE_BODY_BYTES))
        // Path, header, and body extractor refusals must carry the same
        // private-response cache policy as handler-produced responses.
        .layer(middleware::map_response(no_store_response))
        .with_state(QtiProfileImportRouteState { store, objects })
}

struct QtiProfileImportRouteState<S, O> {
    store: Arc<S>,
    objects: Arc<O>,
}

impl<S, O> Clone for QtiProfileImportRouteState<S, O> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            objects: Arc::clone(&self.objects),
        }
    }
}

async fn put_qti_import<S, O>(
    State(state): State<QtiProfileImportRouteState<S, O>>,
    Path((workspace, import)): Path<(WorkspaceId, WorkspaceImportId)>,
    request: Request,
) -> Response
where
    S: Store + QtiImportApiStore + SessionStore + AuthoritativeTimeStore + 'static,
    O: ObjectStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), request.headers()).await
    {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author(authenticated.record.subject.roles()) {
        return import_not_found();
    }
    let actor = authenticated.record.subject.user();
    match state
        .store
        .get_draft(authenticated.tenant_context, actor, workspace)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => return import_not_found(),
        Err(error) => return store_error_response(error),
    }
    if !has_exact_zip_content_type(request.headers()) {
        return error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "QTI import body must use application/zip",
        );
    }
    let created_at = match state
        .store
        .authoritative_time(authenticated.tenant_context)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    // Authorization and workspace ownership are deliberately resolved before
    // the request body is consumed.
    let bytes = match to_bytes(request.into_body(), MAX_QTI_ARCHIVE_BODY_BYTES).await {
        Ok(bytes) if !bytes.is_empty() => bytes.to_vec(),
        Ok(_) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "QTI import archive is empty",
            );
        }
        Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "QTI import archive exceeds the upload limit",
            );
        }
    };

    let tenant = authenticated.tenant_context.tenant_id();
    let object = workspace_qti_archive_object_id(tenant, workspace, import);
    let candidate = PutObject {
        key: ObjectKey::WorkspaceSource {
            tenant,
            workspace,
            import,
            object,
        },
        bytes,
        media_type: QTI_PROFILE_ARCHIVE_MEDIA_TYPE.to_string(),
        license: WORKSPACE_ARCHIVE_LICENSE.to_string(),
        provenance: WORKSPACE_ARCHIVE_PROVENANCE.to_string(),
        created_at,
    };
    let source = match state.objects.put(candidate.clone()).await {
        Ok(record) if fresh_object_record_matches_candidate(&record, &candidate) => record,
        Ok(_) => return archive_conflict(),
        Err(ObjectStoreError::AlreadyExists) => {
            let existing = match state.objects.get(&candidate.key).await {
                Ok(existing) => existing,
                Err(error) => return object_error_response(error),
            };
            if !stored_object_matches_candidate(&existing, &candidate) {
                return archive_conflict();
            }
            existing.record
        }
        Err(error) => return object_error_response(error),
    };
    let view = match state
        .store
        .queue_qti_import(
            authenticated.tenant_context,
            actor,
            QueueQtiImportCommand {
                reference: QtiImportRef {
                    tenant,
                    workspace,
                    import,
                },
                source,
                max_attempts: QTI_IMPORT_MAX_ATTEMPTS,
            },
        )
        .await
    {
        Ok(view) => view,
        Err(error) => return store_error_response(error),
    };
    import_view_response(view)
}

async fn get_qti_import<S, O>(
    State(state): State<QtiProfileImportRouteState<S, O>>,
    headers: HeaderMap,
    Path((workspace, import)): Path<(WorkspaceId, WorkspaceImportId)>,
) -> Response
where
    S: Store + QtiImportApiStore + SessionStore + AuthoritativeTimeStore + 'static,
    O: ObjectStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author(authenticated.record.subject.roles()) {
        return import_not_found();
    }
    let view = match state
        .store
        .qti_import_view(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
            import,
        )
        .await
    {
        Ok(Some(view)) => view,
        Ok(None) => return import_not_found(),
        Err(error) => return store_error_response(error),
    };
    import_view_response(view)
}

fn import_view_response(view: QtiImportApiView) -> Response {
    match view.state {
        QtiImportApiState::Queued => {
            processing_response(view.reference.import, VisibleState::Queued)
        }
        QtiImportApiState::Processing => {
            processing_response(view.reference.import, VisibleState::Processing)
        }
        QtiImportApiState::Failed => failure_response(
            view.reference.import,
            VisibleState::Failed,
            "QTI import could not be processed",
        ),
        QtiImportApiState::Ready => {
            let Some(registry) = view.registry.as_ref() else {
                return error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "QTI import report is unavailable",
                );
            };
            match project_qti_profile_report(view.reference.import, registry) {
                Ok(report) => no_store((StatusCode::OK, Json(report)).into_response()),
                Err(QtiProfileReportProjectionError::UnsupportedProfile) => failure_response(
                    view.reference.import,
                    VisibleState::UnsupportedProfile,
                    "QTI package does not match a supported conversion profile",
                ),
                Err(QtiProfileReportProjectionError::InvalidRegistry) => error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "QTI import report is unavailable",
                ),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum VisibleState {
    Queued,
    Processing,
    Ready,
    Failed,
    UnsupportedProfile,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessingReceipt {
    import_id: WorkspaceImportId,
    state: VisibleState,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FailureReceipt {
    import_id: WorkspaceImportId,
    state: VisibleState,
    error: &'static str,
}

fn processing_response(import: WorkspaceImportId, state: VisibleState) -> Response {
    no_store(
        (
            StatusCode::ACCEPTED,
            Json(ProcessingReceipt {
                import_id: import,
                state,
            }),
        )
            .into_response(),
    )
}

fn failure_response(
    import: WorkspaceImportId,
    state: VisibleState,
    error: &'static str,
) -> Response {
    no_store(
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(FailureReceipt {
                import_id: import,
                state,
                error,
            }),
        )
            .into_response(),
    )
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct VisibleItemReport {
    source_identifier: String,
    title: Option<String>,
    status: QtiImportItemStatus,
    diagnostics: Vec<QtiUnsupportedFeature>,
    defaults: Vec<QtiUnsupportedFeature>,
    warnings: Vec<QtiUnsupportedFeature>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VisibleProfileReport {
    import_id: WorkspaceImportId,
    state: VisibleState,
    profile_id: &'static str,
    profile_label: &'static str,
    profile_version: &'static str,
    report_revision: String,
    items: Vec<VisibleItemReport>,
    ple_defaults: Vec<QtiUnsupportedFeature>,
    review_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportRevisionInput<'a> {
    import_id: WorkspaceImportId,
    state: VisibleState,
    profile_id: &'static str,
    profile_label: &'static str,
    profile_version: &'static str,
    items: &'a [VisibleItemReport],
    ple_defaults: &'a [QtiUnsupportedFeature],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewTokenInput<'a> {
    import_id: WorkspaceImportId,
    report_revision: &'a str,
    ple_defaults: &'a [QtiUnsupportedFeature],
    item_acknowledgements: Vec<VisibleItemAcknowledgement<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VisibleItemAcknowledgement<'a> {
    source_identifier: &'a str,
    defaults: &'a [QtiUnsupportedFeature],
    warnings: &'a [QtiUnsupportedFeature],
}

fn project_qti_profile_report(
    import: WorkspaceImportId,
    registry: &QtiImportRegistry,
) -> Result<VisibleProfileReport, QtiProfileReportProjectionError> {
    if registry.reference.import != import {
        return Err(QtiProfileReportProjectionError::InvalidRegistry);
    }
    let summary = registry
        .profile_summary
        .as_ref()
        .ok_or(QtiProfileReportProjectionError::UnsupportedProfile)?;
    let (profile_id, profile_label, profile_version) = match summary.profile() {
        learning_data_access::PersistedFlatImportProfile::CanvasQti12V1 => (
            summary.profile_id(),
            "Canvas QTI 1.2 static single choice",
            summary.profile_version(),
        ),
        learning_data_access::PersistedFlatImportProfile::BlackboardQti21V1 => (
            summary.profile_id(),
            "Blackboard Original QTI 2.1 static single-choice pool",
            summary.profile_version(),
        ),
    };
    let items = registry
        .item_results
        .iter()
        .map(|item| VisibleItemReport {
            source_identifier: item.source_identifier.clone(),
            title: item.title.clone(),
            status: item.status,
            diagnostics: item.diagnostics.clone(),
            defaults: item.defaults.clone(),
            warnings: item.warnings.clone(),
        })
        .collect::<Vec<_>>();
    let ple_defaults = summary.defaults().to_vec();
    let revision_input = ReportRevisionInput {
        import_id: import,
        state: VisibleState::Ready,
        profile_id,
        profile_label,
        profile_version,
        items: &items,
        ple_defaults: &ple_defaults,
    };
    let report_revision = domain_separated_digest(REPORT_REVISION_DOMAIN, &revision_input)?;
    let item_acknowledgements = items
        .iter()
        .map(|item| VisibleItemAcknowledgement {
            source_identifier: &item.source_identifier,
            defaults: &item.defaults,
            warnings: &item.warnings,
        })
        .collect();
    let review_input = ReviewTokenInput {
        import_id: import,
        report_revision: &report_revision,
        ple_defaults: &ple_defaults,
        item_acknowledgements,
    };
    let review_token = domain_separated_digest(REVIEW_TOKEN_DOMAIN, &review_input)?;
    Ok(VisibleProfileReport {
        import_id: import,
        state: VisibleState::Ready,
        profile_id,
        profile_label,
        profile_version,
        report_revision,
        items,
        ple_defaults,
        review_token,
    })
}

/// Opaque acknowledgement derived from the current safe visible report.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct QtiProfileReportAcknowledgement {
    report_revision: String,
    review_token: String,
}

impl QtiProfileReportAcknowledgement {
    pub(crate) fn report_revision(&self) -> &str {
        &self.report_revision
    }

    pub(crate) fn review_token(&self) -> &str {
        &self.review_token
    }
}

/// Projection failures distinguish honest generic imports from corrupted
/// recognized-profile registry evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QtiProfileReportProjectionError {
    UnsupportedProfile,
    InvalidRegistry,
}

/// Reuses the exact GET projection for conversion acknowledgement checks.
#[allow(dead_code)] // The sibling conversion route is the first consumer.
pub(crate) fn qti_profile_report_acknowledgement(
    import: WorkspaceImportId,
    registry: &QtiImportRegistry,
) -> Result<QtiProfileReportAcknowledgement, QtiProfileReportProjectionError> {
    let report = project_qti_profile_report(import, registry)?;
    Ok(QtiProfileReportAcknowledgement {
        report_revision: report.report_revision,
        review_token: report.review_token,
    })
}

fn domain_separated_digest<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<String, QtiProfileReportProjectionError> {
    let canonical =
        serde_json::to_vec(value).map_err(|_| QtiProfileReportProjectionError::InvalidRegistry)?;
    let mut input = Vec::with_capacity(domain.len() + canonical.len());
    input.extend_from_slice(domain);
    input.extend_from_slice(&canonical);
    Ok(Sha256Digest::compute(&input).to_string())
}

fn has_exact_zip_content_type(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(CONTENT_TYPE).iter();
    matches!(values.next(), Some(value) if value.as_bytes() == QTI_PROFILE_ARCHIVE_MEDIA_TYPE.as_bytes())
        && values.next().is_none()
}

fn replay_object_record_matches_candidate(record: &ObjectRecord, candidate: &PutObject) -> bool {
    let Ok(size_bytes) = u64::try_from(candidate.bytes.len()) else {
        return false;
    };
    record.id == candidate.key.object_id()
        && record.key == candidate.key
        && record.bucket == candidate.key.bucket()
        && record.category == ObjectCategory::Source
        && record.category == candidate.key.category()
        && record.version == candidate.key.version_id()
        && record.media_type == candidate.media_type
        && record.license == candidate.license
        && record.provenance == candidate.provenance
        && record.size_bytes == size_bytes
        && record.sha256 == Sha256Digest::compute(&candidate.bytes)
}

fn fresh_object_record_matches_candidate(record: &ObjectRecord, candidate: &PutObject) -> bool {
    replay_object_record_matches_candidate(record, candidate)
        && record.created_at == candidate.created_at
}

fn stored_object_matches_candidate(stored: &StoredObject, candidate: &PutObject) -> bool {
    replay_object_record_matches_candidate(&stored.record, candidate)
        && stored.bytes == candidate.bytes
}

fn may_author(roles: &[UserRole]) -> bool {
    roles.iter().any(|role| {
        matches!(
            role,
            UserRole::Instructor | UserRole::Publisher | UserRole::Administrator
        )
    })
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden => {
            import_not_found()
        }
        StoreError::AlreadyExists | StoreError::Conflict => archive_conflict(),
        StoreError::InvalidRecord(_) => error_response(
            StatusCode::CONFLICT,
            "QTI import state conflicts with this upload",
        ),
        StoreError::RunModel(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "QTI import service is unavailable",
        ),
        StoreError::TimedOut | StoreError::RetryableTransaction | StoreError::Unavailable(_) => {
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "QTI import service is unavailable",
            )
        }
    }
}

fn object_error_response(error: ObjectStoreError) -> Response {
    match error {
        ObjectStoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "QTI archive storage is unavailable",
        ),
        ObjectStoreError::AlreadyExists
        | ObjectStoreError::NotFound
        | ObjectStoreError::ChecksumMismatch
        | ObjectStoreError::NotSignable
        | ObjectStoreError::NumericOverflow => archive_conflict(),
    }
}

fn archive_conflict() -> Response {
    error_response(
        StatusCode::CONFLICT,
        "QTI import identity is already bound to different archive bytes",
    )
}

fn import_not_found() -> Response {
    error_response(StatusCode::NOT_FOUND, "QTI import not found")
}

fn error_response(status: StatusCode, message: &str) -> Response {
    no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

async fn no_store_response(response: Response) -> Response {
    no_store(response)
}

#[cfg(test)]
#[path = "qti_profile_import/tests.rs"]
mod tests;
