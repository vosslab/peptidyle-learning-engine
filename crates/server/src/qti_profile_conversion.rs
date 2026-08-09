//! Author-only conversion of recognized QTI profile items into native flat drafts.
//!
//! The route repeats the exact profile parse against the committed immutable
//! archive and compares the complete safe registry before creating an object.
//! Persistence remains the final compare-and-swap authority and commits the
//! draft, source, grading material, and current origin atomically.

use std::sync::Arc;

use adapter_native::flat_question::FLAT_QUESTION_MEDIA_TYPE;
use adapter_qti::profiles::QtiMappedItem;
use adapter_qti::{QtiImportIntegrityDigests, QtiImportLimits};
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use learning_data_access::{
    AuthoritativeTimeStore, DraftRecord, FlatImportIntegrityDigests, FlatImportProvenanceStore,
    FlatQuestionGradingPayload, QTI_PROFILE_ARCHIVE_MEDIA_TYPE, QtiImportApiState,
    QtiImportApiStore, QtiImportItem, QtiImportItemStatus, QtiImportProfileSummary, QtiImportRef,
    QtiImportRegistry, QtiProfileFlatConversionCommand, SessionStore, Store, StoreError,
    WorkspaceDraftRevision, WorkspaceFlatImportOrigin,
};
use objects::{
    Bucket, ObjectCategory, ObjectKey, ObjectStore, ObjectStoreError, PutObject, Sha256Digest,
    StoredObject, workspace_qti_archive_object_id,
};
use question_model::{DraftQuestionDefinition, ObjectId, UserRole, WorkspaceId, WorkspaceImportId};
use serde::Deserialize;

use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::catalog::error_response;
use crate::qti_import::profile::prepare_qti_profile_package;
use crate::qti_profile_flat_bridge::bridge_qti_mapped_item;
use crate::qti_profile_import::qti_profile_report_acknowledgement;

const MAX_CONVERSION_REQUEST_BYTES: usize = 4 * 1024;

/// Builds the isolated recognized-item conversion endpoint.
pub fn router<S, O>(store: Arc<S>, objects: Arc<O>) -> Router
where
    S: Store
        + QtiImportApiStore
        + FlatImportProvenanceStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
    O: ObjectStore + 'static,
{
    Router::new()
        .route(
            "/api/workspaces/{workspace}/qti-imports/{import}/items/{item}/convert-flat",
            post(convert_qti_profile_item::<S, O>),
        )
        .layer(DefaultBodyLimit::max(MAX_CONVERSION_REQUEST_BYTES))
        .layer(middleware::map_response(no_store_response))
        .with_state(QtiProfileConversionState { store, objects })
}

struct QtiProfileConversionState<S, O> {
    store: Arc<S>,
    objects: Arc<O>,
}

impl<S, O> Clone for QtiProfileConversionState<S, O> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            objects: Arc::clone(&self.objects),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QtiProfileConversionRequest {
    report_revision: String,
    review_token: String,
}

async fn convert_qti_profile_item<S, O>(
    State(state): State<QtiProfileConversionState<S, O>>,
    headers: HeaderMap,
    Path((workspace, import, item)): Path<(WorkspaceId, WorkspaceImportId, String)>,
    Json(request): Json<QtiProfileConversionRequest>,
) -> Response
where
    S: Store
        + QtiImportApiStore
        + FlatImportProvenanceStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
    O: ObjectStore + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author(authenticated.record.subject.roles()) {
        return not_found_response();
    }
    let expected_revision = match required_revision(&headers) {
        Ok(revision) => revision,
        Err(RevisionError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match is required to convert this draft",
            );
        }
        Err(RevisionError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match must contain one strong workspace revision",
            );
        }
    };
    let actor = authenticated.record.subject.user();
    let view = match state
        .store
        .qti_import_view(authenticated.tenant_context, actor, workspace, import)
        .await
    {
        Ok(Some(view)) => view,
        Ok(None) => return not_found_response(),
        Err(error) => return store_error_response(error),
    };
    if view.reference.workspace != workspace || view.reference.import != import {
        return not_found_response();
    }
    if view.state != QtiImportApiState::Ready {
        return error_response(StatusCode::CONFLICT, "QTI import is not ready");
    }
    let Some(registry) = view.registry else {
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "QTI import unavailable");
    };
    let reference = QtiImportRef {
        tenant: authenticated.tenant_context.tenant_id(),
        workspace,
        import,
    };
    if registry.reference != reference || registry.profile_summary.is_none() {
        return not_found_response();
    }
    let Some(committed_result) = registry.item_results.iter().find(|result| {
        result.source_identifier == item
            && result.item_id.as_deref() == Some(item.as_str())
            && result.status == QtiImportItemStatus::Accepted
    }) else {
        return not_found_response();
    };
    let acknowledgement = match qti_profile_report_acknowledgement(import, &registry) {
        Ok(value) => value,
        Err(_) => return import_changed_response(),
    };
    if request.report_revision != acknowledgement.report_revision()
        || request.review_token != acknowledgement.review_token()
    {
        return error_response(StatusCode::CONFLICT, "QTI report changed; review it again");
    }
    let existing = match state
        .store
        .get_draft(authenticated.tenant_context, actor, workspace)
        .await
    {
        Ok(Some(draft)) => draft,
        Ok(None) => return not_found_response(),
        Err(error) => return store_error_response(error),
    };
    if existing.revision != expected_revision {
        return error_response(StatusCode::CONFLICT, "draft changed; reload it");
    }
    let archive = match state.objects.get(&registry.source.key).await {
        Ok(archive) => archive,
        Err(error) => return archive_object_error_response(error),
    };
    if !archive_matches_registry(&archive, &registry, reference) {
        return import_changed_response();
    }

    let prepared = match prepare_qti_profile_package(&archive.bytes, QtiImportLimits::default()) {
        Ok(Some(prepared)) => prepared,
        Ok(None) | Err(_) => return import_changed_response(),
    };
    let prepared_profile = prepared.profile();
    let expected_summary = match QtiImportProfileSummary::new(
        prepared_profile,
        prepared.profile_report_sha256(),
        prepared.package_defaults().to_vec(),
    ) {
        Ok(summary) => summary,
        Err(_) => return import_changed_response(),
    };
    let expected_results = prepared.item_results().to_vec();
    let mut expected_items = Vec::new();
    let mut selected: Option<(QtiMappedItem, QtiImportIntegrityDigests, Sha256Digest)> = None;
    for prepared_item in prepared.into_items() {
        let source_identifier = prepared_item.source_identifier().to_string();
        let integrity = prepared_item.integrity();
        let mapped = prepared_item.into_mapped_item();
        let normalized = mapped.normalized_profile_item_sha256();
        expected_items.push(QtiImportItem {
            item_id: source_identifier.clone(),
            model_sha256: integrity.public_mapping_sha256,
            assets: Vec::new(),
        });
        if source_identifier == item {
            selected = Some((mapped, integrity, normalized));
        }
    }
    if registry.profile_summary.as_ref() != Some(&expected_summary)
        || registry.item_results != expected_results
        || registry.items != expected_items
        || !recognized_registry_metadata_matches(&registry, prepared_profile)
    {
        return import_changed_response();
    }
    let Some((mapped, integrity, normalized_item_sha256)) = selected else {
        return not_found_response();
    };
    if committed_result.normalized_sha256 != Some(normalized_item_sha256)
        || registry
            .items
            .iter()
            .find(|stored| stored.item_id == item)
            .is_none_or(|stored| stored.model_sha256 != integrity.public_mapping_sha256)
    {
        return import_changed_response();
    }

    let bridge = match bridge_qti_mapped_item(mapped, workspace) {
        Ok(bridge) => bridge,
        Err(_) => return conversion_refused_response(),
    };
    if bridge.persisted_profile() != prepared_profile {
        return import_changed_response();
    }
    let conversion_version = match bridge.persisted_conversion_version() {
        Ok(value) => value,
        Err(_) => return conversion_refused_response(),
    };
    let choice_map = match bridge.persisted_choice_map() {
        Ok(value) => value,
        Err(_) => return conversion_refused_response(),
    };
    if bridge.draft().workspace != workspace
        || bridge.mapping_parts().normalized_profile_item_sha256() != normalized_item_sha256
        || bridge.mapping_parts().public_mapping().source_identifier != item
    {
        return import_changed_response();
    }
    let choice_map_sha256 = choice_map.sha256();
    let mapped_canonical_source_sha256 = Sha256Digest::compute(bridge.canonical_source());
    let grading = match FlatQuestionGradingPayload::from_private(bridge.private()) {
        Ok(value) => value,
        Err(_) => return conversion_refused_response(),
    };
    let public_binding_sha256 = bridge.private().public_binding_sha256().to_string();
    let digests = FlatImportIntegrityDigests {
        normalized_item_sha256,
        profile_report_sha256: integrity.profile_report_sha256,
        public_mapping_sha256: integrity.public_mapping_sha256,
        private_mapping_sha256: integrity.private_mapping_sha256,
        mapping_sha256: integrity.mapping_sha256,
        warning_sha256: integrity.warning_sha256,
        choice_map_sha256,
    };
    let (canonical_source, question, _private, _mapping_parts) = bridge.into_parts();
    let draft = DraftRecord {
        tenant: authenticated.tenant_context.tenant_id(),
        question,
        revises: existing.record.revises,
        derived_from: existing.record.derived_from,
    };
    let acknowledged_at = match state
        .store
        .authoritative_time(authenticated.tenant_context)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let origin = match WorkspaceFlatImportOrigin::new(
        reference,
        item,
        prepared_profile,
        conversion_version,
        registry.source.clone(),
        digests,
        mapped_canonical_source_sha256,
        actor,
        acknowledged_at,
        choice_map,
    ) {
        Ok(value) => value,
        Err(_) => return conversion_refused_response(),
    };
    let object = ObjectId::generate();
    let source = match state
        .objects
        .put(PutObject {
            key: ObjectKey::WorkspaceQuestionSource {
                tenant: authenticated.tenant_context.tenant_id(),
                workspace,
                object,
            },
            bytes: canonical_source,
            media_type: FLAT_QUESTION_MEDIA_TYPE.to_string(),
            license: source_license(&draft),
            provenance: "PLE flat question converted from a recognized QTI profile".to_string(),
            created_at: acknowledged_at,
        })
        .await
    {
        Ok(record) => record,
        Err(error) => return source_object_error_response(error),
    };
    let command = match QtiProfileFlatConversionCommand::new(
        Some(expected_revision),
        draft.clone(),
        source,
        mapped_canonical_source_sha256.to_string(),
        public_binding_sha256,
        grading,
        origin,
    ) {
        Ok(value) => value,
        Err(_) => return conversion_refused_response(),
    };
    match state
        .store
        .convert_qti_profile_item_to_flat(authenticated.tenant_context, actor, command)
        .await
    {
        Ok(saved) => revisioned_response(saved.workspace_revision, draft.question),
        Err(error) => store_error_response(error),
    }
}

fn recognized_registry_metadata_matches(
    registry: &QtiImportRegistry,
    profile: learning_data_access::PersistedFlatImportProfile,
) -> bool {
    registry.source_format == "qti"
        && registry.source_identifier.is_none()
        && registry.importer == "adapter_qti"
        && registry.parse_schema == profile.profile_id()
        && registry.assets.is_empty()
        && registry.unsupported_features.is_empty()
}

fn archive_matches_registry(
    archive: &StoredObject,
    registry: &QtiImportRegistry,
    reference: QtiImportRef,
) -> bool {
    let expected_object =
        workspace_qti_archive_object_id(reference.tenant, reference.workspace, reference.import);
    matches!(
        &registry.source.key,
        ObjectKey::WorkspaceSource {
            tenant,
            workspace,
            import,
            object,
        } if *tenant == reference.tenant
            && *workspace == reference.workspace
            && *import == reference.import
            && *object == expected_object
    ) && registry.source.id == expected_object
        && registry.source.bucket == Bucket::Content
        && registry.source.key.bucket() == Bucket::Content
        && registry.source.category == ObjectCategory::Source
        && registry.source.key.category() == ObjectCategory::Source
        && registry.source.version.is_none()
        && registry.source.key.version_id().is_none()
        && registry.source.media_type == QTI_PROFILE_ARCHIVE_MEDIA_TYPE
        && registry.source.size_bytes > 0
        && registry.source.size_bytes
            <= u64::try_from(QtiImportLimits::default().max_archive_bytes)
                .expect("QTI archive limit fits in u64")
        && archive.record == registry.source
        && archive.bytes.len() as u64 == registry.source.size_bytes
        && Sha256Digest::compute(&archive.bytes) == registry.source.sha256
}

fn may_author(roles: &[UserRole]) -> bool {
    roles.iter().any(|role| {
        matches!(
            role,
            UserRole::Instructor | UserRole::Publisher | UserRole::Administrator
        )
    })
}

#[derive(Clone, Copy)]
enum RevisionError {
    Missing,
    Malformed,
}

fn required_revision(headers: &HeaderMap) -> Result<WorkspaceDraftRevision, RevisionError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(RevisionError::Missing);
    };
    if values.next().is_some() {
        return Err(RevisionError::Malformed);
    }
    let value = value.to_str().map_err(|_| RevisionError::Malformed)?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(RevisionError::Malformed);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RevisionError::Malformed);
    }
    let number = value.parse::<u64>().map_err(|_| RevisionError::Malformed)?;
    if number == 0 || number > i64::MAX as u64 {
        return Err(RevisionError::Malformed);
    }
    serde_json::from_str(value).map_err(|_| RevisionError::Malformed)
}

fn revisioned_response(
    revision: WorkspaceDraftRevision,
    body: DraftQuestionDefinition,
) -> Response {
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{}\"", revision.value()))
            .expect("decimal revision is always a valid strong ETag"),
    );
    no_store(response)
}

fn not_found_response() -> Response {
    error_response(StatusCode::NOT_FOUND, "QTI import item not found")
}

fn import_changed_response() -> Response {
    error_response(StatusCode::CONFLICT, "QTI import changed; import it again")
}

fn conversion_refused_response() -> Response {
    error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        "QTI item cannot be converted",
    )
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden => {
            not_found_response()
        }
        StoreError::AlreadyExists
        | StoreError::Conflict
        | StoreError::TimedOut
        | StoreError::InvalidRecord(_)
        | StoreError::RunModel(_) => {
            error_response(StatusCode::CONFLICT, "QTI conversion changed; reload it")
        }
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "QTI conversion unavailable",
        ),
    }
}

fn archive_object_error_response(error: ObjectStoreError) -> Response {
    match error {
        ObjectStoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "QTI archive storage unavailable",
        ),
        ObjectStoreError::NotFound
        | ObjectStoreError::ChecksumMismatch
        | ObjectStoreError::AlreadyExists
        | ObjectStoreError::NotSignable
        | ObjectStoreError::NumericOverflow => import_changed_response(),
    }
}

fn source_object_error_response(error: ObjectStoreError) -> Response {
    match error {
        ObjectStoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "flat-question source storage unavailable",
        ),
        ObjectStoreError::NotFound
        | ObjectStoreError::ChecksumMismatch
        | ObjectStoreError::AlreadyExists
        | ObjectStoreError::NotSignable
        | ObjectStoreError::NumericOverflow => error_response(
            StatusCode::CONFLICT,
            "flat-question source storage conflict",
        ),
    }
}

fn source_license(draft: &DraftRecord) -> String {
    match &draft.question.metadata.license {
        question_model::taxonomy::License::AllRightsReserved => "All rights reserved".to_string(),
        question_model::taxonomy::License::CcBy => "CC-BY-4.0".to_string(),
        question_model::taxonomy::License::CcBySa => "CC-BY-SA-4.0".to_string(),
        question_model::taxonomy::License::CcByNc => "CC-BY-NC-4.0".to_string(),
        question_model::taxonomy::License::Cc0 => "CC0-1.0".to_string(),
        question_model::taxonomy::License::Other { spdx } => spdx.clone(),
    }
}

async fn no_store_response(response: Response) -> Response {
    no_store(response)
}

#[cfg(test)]
#[path = "qti_profile_conversion/tests.rs"]
mod tests;
