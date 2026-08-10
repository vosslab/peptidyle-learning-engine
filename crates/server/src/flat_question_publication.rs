//! Author-only staging and publication of PLE flat-question source documents.
//!
//! The browser submits answer-bearing authoring JSON only to this narrow route.
//! It persists canonical private source bytes, exposes only the compiled public
//! draft, and stages the private grading half with every successful save. The
//! atomic catalog promotion later copies only that current stored value.

use std::sync::Arc;

#[cfg(test)]
use adapter_native::flat_question::FLAT_SINGLE_CHOICE_FAMILY;
use adapter_native::flat_question::{
    FLAT_QUESTION_MEDIA_TYPE, FlatQuestionDocument, is_flat_question_family,
};
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::header::{CONTENT_TYPE, ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use learning_data_access::{
    AuthoritativeTimeStore, CatalogStore, DraftRecord, FlatImportProvenanceStore,
    FlatImportPublicationPromotion, FlatQuestionGradingPayload, FlatQuestionPublicationPromotion,
    FlatQuestionStore, PublishDraftCommand, PublishedSourceArtifact,
    QTI_PROFILE_ARCHIVE_MEDIA_TYPE, SessionStore, Store, StoreError, TenantContext,
    UpsertFlatQuestionCommand, WorkspaceDraft, WorkspaceDraftRevision, WorkspaceFlatImportOrigin,
};
use objects::{
    Bucket, ObjectCategory, ObjectKey, ObjectRecord, ObjectStore, ObjectStoreError, PutObject,
    Sha256Digest, StoredObject, published_import_archive_object_id,
};
use question_model::{
    DraftQuestionSource, ObjectId, PublicationScope, QuestionBackend, QuestionSource, UserId,
    UserRole, WorkspaceId,
};
use serde::Deserialize;

use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::catalog::{
    BackendRegistry, BackendRegistryError, PublicReviewGate, error_response, may_publish,
    mint_publication_reference,
};

const MAX_FLAT_QUESTION_BODY_BYTES: usize = 256 * 1024;

/// Builds isolated flat-question authoring and publication endpoints.
pub fn router<S, O, B, R>(
    store: Arc<S>,
    objects: Arc<O>,
    backends: Arc<B>,
    review_gate: Arc<R>,
) -> Router
where
    S: Store
        + CatalogStore
        + FlatQuestionStore
        + FlatImportProvenanceStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
    O: ObjectStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    Router::new()
        .route(
            "/api/workspaces/{workspace}/flat-question",
            get(read_flat_question_source::<S, O, B, R>).put(save_flat_question::<S, O, B, R>),
        )
        .route(
            "/api/problems/{workspace}/flat-question-publish",
            post(publish_flat_question::<S, O, B, R>),
        )
        .layer(DefaultBodyLimit::max(MAX_FLAT_QUESTION_BODY_BYTES))
        .layer(middleware::map_response(no_store_response))
        .with_state(FlatQuestionRouteState {
            store,
            objects,
            backends,
            review_gate,
        })
}

/// Returns the exact private authoring bytes only after rebuilding and checking
/// the complete staged source-to-public-draft binding.  This is intentionally
/// separate from generic workspace reads, whose DTOs must remain answer-free.
async fn read_flat_question_source<S, O, B, R>(
    State(state): State<FlatQuestionRouteState<S, O, B, R>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
) -> Response
where
    S: Store
        + CatalogStore
        + FlatQuestionStore
        + FlatImportProvenanceStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
    O: ObjectStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    // Do not let a valid but non-author session use this endpoint to discover
    // workspace IDs that happen to hold answer-bearing source material.
    if !may_author(authenticated.record.subject.roles()) {
        return error_response(StatusCode::NOT_FOUND, "workspace not found");
    }
    let actor = authenticated.record.subject.user();
    let draft = match load_draft(
        state.store.as_ref(),
        authenticated.tenant_context,
        actor,
        workspace,
    )
    .await
    {
        Ok(draft) => draft,
        Err(response) => return response,
    };
    // A normal workspace draft that is not a flat source is indistinguishable
    // from a workspace the caller may not read through this narrow endpoint.
    if !matches!(&draft.record.question.source, DraftQuestionSource::Native { family } if is_flat_question_family(family))
    {
        return error_response(StatusCode::NOT_FOUND, "workspace not found");
    }
    let staged = match state
        .store
        .flat_question_source(authenticated.tenant_context, actor, workspace)
        .await
    {
        Ok(Some(source)) => source,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "workspace not found"),
        Err(error) => return private_store_error(error),
    };
    if !is_flat_question_family(&staged.source_family) {
        return error_response(StatusCode::NOT_FOUND, "workspace not found");
    }
    if staged.workspace_revision != draft.revision
        || staged.tenant != authenticated.tenant_context.tenant_id()
        || staged.workspace != workspace
    {
        return flat_source_changed_response();
    }
    let stored = match state.objects.get(&staged.source_record.key).await {
        Ok(object) => object,
        Err(error) => return object_error_response(error),
    };
    if stored.record != staged.source_record
        || Sha256Digest::compute(&stored.bytes) != staged.source_record.sha256
        || staged.canonical_source_sha256 != staged.source_record.sha256.to_string()
    {
        return flat_source_changed_response();
    }
    let document = match FlatQuestionDocument::parse(&stored.bytes) {
        Ok(document) => document,
        Err(_) => return flat_source_changed_response(),
    };
    let canonical_source = match document.canonical_bytes() {
        Ok(value) if value == stored.bytes => value,
        _ => return flat_source_changed_response(),
    };
    let compiled = match document.compile(workspace) {
        Ok(value) => value,
        Err(_) => return flat_source_changed_response(),
    };
    let (compiled_draft, private) = compiled.into_parts();
    if compiled_draft != draft.record.question
        || private.public_binding_sha256() != staged.public_binding_sha256
    {
        return flat_source_changed_response();
    }
    canonical_source_response(draft.revision, canonical_source)
}

struct FlatQuestionRouteState<S, O, B, R> {
    store: Arc<S>,
    objects: Arc<O>,
    backends: Arc<B>,
    review_gate: Arc<R>,
}

impl<S, O, B, R> Clone for FlatQuestionRouteState<S, O, B, R> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            objects: Arc::clone(&self.objects),
            backends: Arc::clone(&self.backends),
            review_gate: Arc::clone(&self.review_gate),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlatQuestionPublishRequest {
    scope: PublicationScope,
}

async fn save_flat_question<S, O, B, R>(
    State(state): State<FlatQuestionRouteState<S, O, B, R>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
    body: Bytes,
) -> Response
where
    S: Store
        + CatalogStore
        + FlatQuestionStore
        + FlatImportProvenanceStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
    O: ObjectStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    let expected_revision = match optional_revision(&headers) {
        Ok(value) => value,
        Err(()) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "If-Match must contain one strong workspace revision",
            );
        }
    };
    let document = match FlatQuestionDocument::parse(&body) {
        Ok(document) => document,
        Err(_) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "flat-question source is invalid",
            );
        }
    };
    let canonical_source = match document.canonical_bytes() {
        Ok(bytes) => bytes,
        Err(_) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "flat-question source cannot be canonicalized",
            );
        }
    };
    let compiled = match document.compile(workspace) {
        Ok(compiled) => compiled,
        Err(_) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "flat-question source is invalid",
            );
        }
    };
    let (question, private) = compiled.into_parts();
    let public_binding_sha256 = private.public_binding_sha256().to_string();
    let grading = match FlatQuestionGradingPayload::from_private(&private) {
        Ok(value) => value,
        Err(_) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "flat-question source is invalid",
            );
        }
    };
    let existing = match state
        .store
        .get_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
        )
        .await
    {
        Ok(existing) => existing,
        Err(error) => return private_store_error(error),
    };
    let draft = DraftRecord {
        tenant: authenticated.tenant_context.tenant_id(),
        question,
        revises: existing.as_ref().and_then(|draft| draft.record.revises),
        derived_from: existing.and_then(|draft| draft.record.derived_from),
    };
    let created_at = match state
        .store
        .authoritative_time(authenticated.tenant_context)
        .await
    {
        Ok(value) => value,
        Err(error) => return private_store_error(error),
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
            bytes: canonical_source.clone(),
            media_type: FLAT_QUESTION_MEDIA_TYPE.to_string(),
            license: publication_license(&draft),
            provenance: "PLE flat-question authoring source".to_string(),
            created_at,
        })
        .await
    {
        Ok(record) => record,
        Err(error) => return object_error_response(error),
    };
    match state
        .store
        .upsert_flat_question(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            UpsertFlatQuestionCommand {
                expected_revision,
                draft: draft.clone(),
                source,
                canonical_source_sha256: Sha256Digest::compute(&canonical_source).to_string(),
                public_binding_sha256,
                grading,
            },
        )
        .await
    {
        Ok(saved) => revisioned_response(saved.workspace_revision, draft.question),
        Err(error) => private_store_error(error),
    }
}

async fn publish_flat_question<S, O, B, R>(
    State(state): State<FlatQuestionRouteState<S, O, B, R>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
    Json(request): Json<FlatQuestionPublishRequest>,
) -> Response
where
    S: Store
        + CatalogStore
        + FlatQuestionStore
        + FlatImportProvenanceStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
    O: ObjectStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_publish(authenticated.record.subject.roles(), request.scope) {
        return error_response(StatusCode::FORBIDDEN, "publication is not authorized");
    }
    let expected_revision = match required_revision(&headers) {
        Ok(revision) => revision,
        Err(RevisionError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match is required to publish a workspace",
            );
        }
        Err(RevisionError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match must contain one strong workspace revision",
            );
        }
    };
    let publisher = authenticated.record.subject.user();
    let first = match load_draft(
        state.store.as_ref(),
        authenticated.tenant_context,
        publisher,
        workspace,
    )
    .await
    {
        Ok(draft) => draft,
        Err(response) => return response,
    };
    if first.revision != expected_revision {
        return error_response(StatusCode::CONFLICT, "draft changed; reload it");
    }
    let _capabilities = match validate_flat_draft(state.backends.as_ref(), &first.record) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if request.scope == PublicationScope::Public {
        match state
            .review_gate
            .allows_publication(authenticated.tenant_context, publisher, &first.record)
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                return error_response(
                    StatusCode::FORBIDDEN,
                    "public publication requires institutional review",
                );
            }
            Err(_) => {
                return error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "publication review unavailable",
                );
            }
        }
    }
    let current = match load_draft(
        state.store.as_ref(),
        authenticated.tenant_context,
        publisher,
        workspace,
    )
    .await
    {
        Ok(draft) => draft,
        Err(response) => return response,
    };
    if current.revision != expected_revision {
        return error_response(StatusCode::CONFLICT, "draft changed; reload it");
    }
    let capabilities = match validate_flat_draft(state.backends.as_ref(), &current.record) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let staged = match state
        .store
        .flat_question_source(authenticated.tenant_context, publisher, workspace)
        .await
    {
        Ok(Some(source)) => source,
        Ok(None) => {
            return error_response(StatusCode::CONFLICT, "flat-question source is not staged");
        }
        Err(error) => return private_store_error(error),
    };
    if staged.workspace_revision != expected_revision {
        return error_response(StatusCode::CONFLICT, "draft changed; reload it");
    }
    let stored = match state.objects.get(&staged.source_record.key).await {
        Ok(object) => object,
        Err(error) => return object_error_response(error),
    };
    if stored.record != staged.source_record
        || Sha256Digest::compute(&stored.bytes) != staged.source_record.sha256
    {
        return error_response(
            StatusCode::CONFLICT,
            "flat-question source changed; reload it",
        );
    }
    let document = match FlatQuestionDocument::parse(&stored.bytes) {
        Ok(document) => document,
        Err(_) => {
            return error_response(
                StatusCode::CONFLICT,
                "flat-question source changed; reload it",
            );
        }
    };
    let canonical_source = match document.canonical_bytes() {
        Ok(value) if value == stored.bytes => value,
        _ => {
            return error_response(
                StatusCode::CONFLICT,
                "flat-question source changed; reload it",
            );
        }
    };
    let compiled = match document.compile(workspace) {
        Ok(value) => value,
        Err(_) => {
            return error_response(
                StatusCode::CONFLICT,
                "flat-question source changed; reload it",
            );
        }
    };
    let (compiled_draft, private) = compiled.into_parts();
    if compiled_draft != current.record.question
        || private.public_binding_sha256() != staged.public_binding_sha256
    {
        return error_response(
            StatusCode::CONFLICT,
            "flat-question source changed; reload it",
        );
    }
    let publication = mint_publication_reference(current.record.revises);
    let import_promotion = match state
        .store
        .workspace_flat_import_origin(authenticated.tenant_context, publisher, workspace)
        .await
    {
        Ok(Some(origin)) => {
            match prepare_flat_import_promotion(state.objects.as_ref(), &origin, publication).await
            {
                Ok(promotion) => Some(promotion),
                Err(response) => return response,
            }
        }
        Ok(None) => None,
        Err(error) => return private_store_error(error),
    };
    let source_object = ObjectId::generate();
    let published_object = match state
        .objects
        .put(PutObject {
            key: ObjectKey::ProblemSource {
                problem: publication.problem,
                version: publication.version,
                object: source_object,
            },
            bytes: canonical_source,
            media_type: FLAT_QUESTION_MEDIA_TYPE.to_string(),
            license: publication_license(&current.record),
            provenance: "PLE flat-question published source".to_string(),
            created_at: staged.source_record.created_at,
        })
        .await
    {
        Ok(record) => record,
        Err(error) => return object_error_response(error),
    };
    let published_family = match &current.record.question.source {
        DraftQuestionSource::Native { family } if is_flat_question_family(family) => family.clone(),
        _ => return flat_source_changed_response(),
    };
    let command = PublishDraftCommand {
        expected_draft: current.record,
        expected_revision,
        publication,
        published_source: QuestionSource::Native {
            family: published_family,
        },
        source_artifact: Some(PublishedSourceArtifact {
            reference: publication,
            backend: QuestionBackend::Native,
            object: published_object,
        }),
        qti_promotion: None,
        flat_question_promotion: Some(FlatQuestionPublicationPromotion {
            source: staged,
            import_origin: import_promotion,
        }),
        publisher,
        scope: request.scope,
        capabilities,
    };
    match state
        .store
        .publish_draft(authenticated.tenant_context, publisher, command)
        .await
    {
        Ok(record) => no_store((StatusCode::CREATED, Json(record.question)).into_response()),
        Err(error) => private_store_error(error),
    }
}

/// Copies one exact, verified workspace import archive to its deterministic,
/// non-signable publication key. Candidate bytes precede the Store promotion;
/// a later Store refusal therefore leaves only a reconcilable typed orphan.
async fn prepare_flat_import_promotion<O>(
    objects: &O,
    origin: &WorkspaceFlatImportOrigin,
    publication: question_model::ProblemVersionRef,
) -> Result<FlatImportPublicationPromotion, Response>
where
    O: ObjectStore,
{
    let stored = objects
        .get(&origin.source_archive().key)
        .await
        .map_err(flat_import_archive_object_error)?;
    if !is_exact_workspace_import_archive(origin, &stored) {
        return Err(flat_source_changed_response());
    }

    let import = origin.import();
    let object = published_import_archive_object_id(
        import.tenant,
        publication.problem,
        publication.version,
        import.import,
        stored.record.sha256,
    );
    let candidate = PutObject {
        key: ObjectKey::PublishedImportArchive {
            tenant: import.tenant,
            problem: publication.problem,
            version: publication.version,
            import: import.import,
            object,
        },
        bytes: stored.bytes,
        media_type: QTI_PROFILE_ARCHIVE_MEDIA_TYPE.to_string(),
        license: stored.record.license,
        provenance: "published from verified QTI workspace import archive".to_string(),
        created_at: stored.record.created_at,
    };
    let published_archive = match objects.put(candidate.clone()).await {
        Ok(record) => {
            if !is_exact_published_archive_replay(&record, &candidate) {
                return Err(flat_source_changed_response());
            }
            record
        }
        Err(ObjectStoreError::AlreadyExists) => {
            let replay = objects
                .get(&candidate.key)
                .await
                .map_err(flat_import_archive_object_error)?;
            if !is_exact_published_archive_replay(&replay.record, &candidate)
                || replay.bytes != candidate.bytes
            {
                return Err(flat_source_changed_response());
            }
            replay.record
        }
        Err(error) => return Err(flat_import_archive_object_error(error)),
    };
    FlatImportPublicationPromotion::new(origin, publication, published_archive)
        .map_err(private_store_error)
}

fn is_exact_workspace_import_archive(
    origin: &WorkspaceFlatImportOrigin,
    stored: &StoredObject,
) -> bool {
    let expected = origin.source_archive();
    let import = origin.import();
    let ObjectKey::WorkspaceSource {
        tenant,
        workspace,
        import: archive_import,
        object,
    } = &expected.key
    else {
        return false;
    };
    let Ok(size_bytes) = u64::try_from(stored.bytes.len()) else {
        return false;
    };
    stored.record == *expected
        && *tenant == import.tenant
        && *workspace == import.workspace
        && *archive_import == import.import
        && *object == expected.id
        && expected.bucket == Bucket::Content
        && expected.key.bucket() == Bucket::Content
        && expected.category == ObjectCategory::Source
        && expected.key.category() == ObjectCategory::Source
        && expected.version.is_none()
        && expected.media_type == QTI_PROFILE_ARCHIVE_MEDIA_TYPE
        && size_bytes > 0
        && expected.size_bytes == size_bytes
        && expected.sha256 == Sha256Digest::compute(&stored.bytes)
}

fn is_exact_published_archive_replay(record: &ObjectRecord, candidate: &PutObject) -> bool {
    let Ok(size_bytes) = u64::try_from(candidate.bytes.len()) else {
        return false;
    };
    // The object contract's minimum immutable comparison covers key,
    // classification, media type, size, and digest. This server-owned copy is
    // stricter: license, provenance, and creation time are fixed inputs too,
    // so an earlier writer cannot silently substitute publication metadata.
    record.id == candidate.key.object_id()
        && record.key == candidate.key
        && record.bucket == candidate.key.bucket()
        && record.category == candidate.key.category()
        && record.version == candidate.key.version_id()
        && record.media_type == candidate.media_type
        && record.size_bytes == size_bytes
        && record.sha256 == Sha256Digest::compute(&candidate.bytes)
        && record.license == candidate.license
        && record.provenance == candidate.provenance
        && record.created_at == candidate.created_at
}

fn flat_import_archive_object_error(error: ObjectStoreError) -> Response {
    match error {
        ObjectStoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "source storage unavailable",
        ),
        ObjectStoreError::NotFound
        | ObjectStoreError::ChecksumMismatch
        | ObjectStoreError::AlreadyExists
        | ObjectStoreError::NotSignable
        | ObjectStoreError::NumericOverflow => flat_source_changed_response(),
    }
}

async fn load_draft<S>(
    store: &S,
    context: TenantContext,
    actor: UserId,
    workspace: WorkspaceId,
) -> Result<WorkspaceDraft, Response>
where
    S: Store,
{
    match store.get_draft(context, actor, workspace).await {
        Ok(Some(draft)) => Ok(draft),
        Ok(None) => Err(error_response(StatusCode::NOT_FOUND, "workspace not found")),
        Err(error) => Err(private_store_error(error)),
    }
}

#[allow(clippy::result_large_err)] // Route validation deliberately returns the exact HTTP refusal.
fn validate_flat_draft<B>(
    backends: &B,
    draft: &DraftRecord,
) -> Result<question_model::BackendCapabilities, Response>
where
    B: BackendRegistry,
{
    if draft.question.metadata.validate_title().is_err()
        || !matches!(&draft.question.source, DraftQuestionSource::Native { family } if is_flat_question_family(family))
    {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "draft is not a flat question",
        ));
    }
    let capabilities = match backends.capabilities(&draft.question.source) {
        Ok(value) => value,
        Err(BackendRegistryError::Unsupported) => {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "question backend is not registered",
            ));
        }
        Err(BackendRegistryError::Unavailable(_)) => {
            return Err(error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "backend registry unavailable",
            ));
        }
    };
    let violations = domain::policy::validate_draft_for_publication(&draft.question, &capabilities);
    if violations.is_empty() {
        Ok(capabilities)
    } else {
        Err(no_store((StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({"error":"publication validation failed","violations":violations}))).into_response()))
    }
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

fn optional_revision(headers: &HeaderMap) -> Result<Option<WorkspaceDraftRevision>, ()> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err(());
    }
    let value = value.to_str().map_err(|_| ())?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(());
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(());
    }
    let number = value.parse::<u64>().map_err(|_| ())?;
    if number == 0 || number > i64::MAX as u64 {
        return Err(());
    }
    serde_json::from_str(value).map(Some).map_err(|_| ())
}

fn required_revision(headers: &HeaderMap) -> Result<WorkspaceDraftRevision, RevisionError> {
    match optional_revision(headers) {
        Ok(Some(revision)) => Ok(revision),
        Ok(None) => Err(RevisionError::Missing),
        Err(()) => Err(RevisionError::Malformed),
    }
}

fn revisioned_response<T: serde::Serialize>(revision: WorkspaceDraftRevision, body: T) -> Response {
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{}\"", revision.value())).expect("decimal revision"),
    );
    no_store(response)
}

fn canonical_source_response(revision: WorkspaceDraftRevision, source: Vec<u8>) -> Response {
    let mut response = Response::new(source.into());
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static(FLAT_QUESTION_MEDIA_TYPE),
    );
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{}\"", revision.value())).expect("decimal revision"),
    );
    no_store(response)
}

fn flat_source_changed_response() -> Response {
    error_response(
        StatusCode::CONFLICT,
        "flat-question source changed; reload it",
    )
}

fn private_store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::TenantMismatch | StoreError::Forbidden => {
            error_response(StatusCode::NOT_FOUND, "workspace not found")
        }
        StoreError::AlreadyExists | StoreError::Conflict | StoreError::TimedOut => {
            error_response(StatusCode::CONFLICT, "workspace changed; reload it")
        }
        StoreError::InvalidRecord(message) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &message)
        }
        StoreError::RunModel(error) => {
            error_response(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string())
        }
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "workspace storage unavailable",
        ),
    }
}

fn object_error_response(error: ObjectStoreError) -> Response {
    match error {
        ObjectStoreError::Unavailable(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "source storage unavailable",
        ),
        ObjectStoreError::NotFound | ObjectStoreError::ChecksumMismatch => error_response(
            StatusCode::CONFLICT,
            "flat-question source changed; reload it",
        ),
        ObjectStoreError::AlreadyExists
        | ObjectStoreError::NotSignable
        | ObjectStoreError::NumericOverflow => {
            error_response(StatusCode::CONFLICT, "source storage conflict")
        }
    }
}

fn publication_license(draft: &DraftRecord) -> String {
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
#[path = "flat_question_publication/tests.rs"]
mod tests;
