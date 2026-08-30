//! Server-only validation and bytes-first preparation for QTI publication.

use adapter_qti::{QtiImporter, qti_question_asset_checksums};
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::header::IF_MATCH;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use std::{collections::BTreeMap, sync::Arc};

use learning_data_access::{
    AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, CatalogStore, DraftRecord,
    PublishDraftCommand, PublishedSourceArtifact, QtiImportItem, QtiImportRegistry, QtiImportStore,
    QtiPublicationPromotion, SessionStore, Store, StoreError, TenantContext,
    WorkspaceDraftRevision,
};
use objects::{ObjectCategory, ObjectKey, ObjectRecord, ObjectStore, PutObject, Sha256Digest};
use question_model::{
    DraftQuestionDefinition, ObjectId, ProblemVersionRef, PublicationScope, QuestionBackend,
    QuestionSource, UserId, WorkspaceId, WorkspaceImportId,
};
use serde::Deserialize;

use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::catalog::{
    BackendRegistry, BackendRegistryError, PublicReviewGate, dispatch_publication, error_response,
    may_publish, mint_publication_reference, store_error_response,
};
use crate::http_refusal::HttpResult;

const MAX_QTI_PUBLICATION_BODY_BYTES: usize = 4 * 1024;

/// A validated private QTI relationship that is safe to use when minting a
/// candidate published version. This type is intentionally crate-private:
/// browser input cannot manufacture a staging-to-publication bridge.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ValidatedQtiPublication {
    registry: QtiImportRegistry,
    item: QtiImportItem,
    draft: DraftQuestionDefinition,
}

/// Bytes-first candidate records for the later atomic QTI promotion
/// transaction. Object writes can safely precede that transaction: without a
/// catalog binding they are reconcilable candidates, never visible assets.
#[derive(Debug, Clone)]
pub(crate) struct PreparedQtiPublication {
    pub(crate) published_source: QuestionSource,
    pub(crate) source_artifact: PublishedSourceArtifact,
    pub(crate) promotion: QtiPublicationPromotion,
}

/// Builds the dedicated QTI publication endpoint. Generic catalog publication
/// stays QTI-closed: this route is the only HTTP path that can turn committed
/// private staging into candidate published objects and a promotion command.
pub fn router<S, O, B, R>(
    store: Arc<S>,
    objects: Arc<O>,
    backends: Arc<B>,
    review_gate: Arc<R>,
) -> Router
where
    S: Store + CatalogStore + QtiImportStore + SessionStore + 'static,
    O: ObjectStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    Router::new()
        .route(
            "/api/problems/{workspace}/qti-publish",
            post(publish_qti_problem::<S, O, B, R>),
        )
        .layer(DefaultBodyLimit::max(MAX_QTI_PUBLICATION_BODY_BYTES))
        .with_state(QtiPublicationRouteState {
            store,
            objects,
            backends,
            review_gate,
        })
}

struct QtiPublicationRouteState<S, O, B, R> {
    store: Arc<S>,
    objects: Arc<O>,
    backends: Arc<B>,
    review_gate: Arc<R>,
}

impl<S, O, B, R> Clone for QtiPublicationRouteState<S, O, B, R> {
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
struct QtiPublishRequest {
    scope: PublicationScope,
    byline: question_model::PublicByline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QtiPublishRevisionError {
    Missing,
    Malformed,
}

fn required_qti_publish_revision(
    headers: &HeaderMap,
) -> Result<WorkspaceDraftRevision, QtiPublishRevisionError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(QtiPublishRevisionError::Missing);
    };
    if values.next().is_some() {
        return Err(QtiPublishRevisionError::Malformed);
    }
    let value = value
        .to_str()
        .map_err(|_| QtiPublishRevisionError::Malformed)?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(QtiPublishRevisionError::Malformed);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(QtiPublishRevisionError::Malformed);
    }
    let numeric = value
        .parse::<u64>()
        .map_err(|_| QtiPublishRevisionError::Malformed)?;
    if numeric == 0 || numeric > i64::MAX as u64 {
        return Err(QtiPublishRevisionError::Malformed);
    }
    serde_json::from_str(value).map_err(|_| QtiPublishRevisionError::Malformed)
}

async fn publish_qti_problem<S, O, B, R>(
    State(state): State<QtiPublicationRouteState<S, O, B, R>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
    Json(request): Json<QtiPublishRequest>,
) -> Response
where
    S: Store + CatalogStore + QtiImportStore + SessionStore + 'static,
    O: ObjectStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_publish(authenticated.record.subject.role(), request.scope) {
        return error_response(StatusCode::FORBIDDEN, "publication is not authorized");
    }
    let expected_revision = match required_qti_publish_revision(&headers) {
        Ok(revision) => revision,
        Err(QtiPublishRevisionError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match is required to publish a workspace",
            );
        }
        Err(QtiPublishRevisionError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match must contain one strong workspace revision",
            );
        }
    };
    let publisher = authenticated.record.subject.user();
    let first = match load_qti_draft(
        state.store.as_ref(),
        authenticated.tenant_context,
        publisher,
        workspace,
    )
    .await
    {
        Ok(draft) => draft,
        Err(response) => return response.into_response(),
    };
    if first.revision != expected_revision {
        return error_response(StatusCode::CONFLICT, "draft changed; reload it");
    }
    let _capabilities = match validate_qti_draft(state.backends.as_ref(), &first.record) {
        Ok(capabilities) => capabilities,
        Err(response) => return response.into_response(),
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
    let current = match load_qti_draft(
        state.store.as_ref(),
        authenticated.tenant_context,
        publisher,
        workspace,
    )
    .await
    {
        Ok(draft) => draft,
        Err(response) => return response.into_response(),
    };
    if current.revision != expected_revision {
        return error_response(StatusCode::CONFLICT, "draft changed; reload it");
    }
    let capabilities = match validate_qti_draft(state.backends.as_ref(), &current.record) {
        Ok(capabilities) => capabilities,
        Err(response) => return response.into_response(),
    };
    let (item_id, import) = match &current.record.question.source {
        question_model::DraftQuestionSource::Qti { item_id, import_id } => {
            (item_id.clone(), *import_id)
        }
        _ => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "draft is not a QTI import",
            );
        }
    };
    let preparer =
        QtiPublicationPreparer::new(Arc::clone(&state.store), Arc::clone(&state.objects));
    let validated = match preparer
        .validate(
            authenticated.tenant_context,
            &current.record.question,
            import,
            &item_id,
        )
        .await
    {
        Ok(validated) => validated,
        Err(error) => return store_error_response(error),
    };
    // This is intentionally after all validation, including the committed
    // archive/model/draft agreement. Candidate-write failure leaves no
    // catalog identity or browser-visible binding.
    let publication = mint_publication_reference();
    let prepared = match preparer
        .copy_candidates(&current.record, publication, request.scope, validated)
        .await
    {
        Ok(prepared) => prepared,
        Err(error) => return store_error_response(error),
    };
    let command = PublishDraftCommand {
        expected_draft: current.record,
        // Preserve the author-reviewed strong ETag, rather than replacing it
        // with a later reread. Store compares this exact value under its
        // publication transaction lock.
        expected_revision,
        publication,
        published_source: prepared.published_source,
        source_artifact: Some(prepared.source_artifact),
        qti_promotion: Some(prepared.promotion),
        flat_question_promotion: None,
        publisher,
        scope: request.scope,
        byline: request.byline,
        capabilities,
    };
    match dispatch_publication(state.store.as_ref(), &authenticated, command).await {
        Ok(record) => no_store((StatusCode::CREATED, Json(record.summary())).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn load_qti_draft<S>(
    store: &S,
    context: TenantContext,
    actor: UserId,
    workspace: WorkspaceId,
) -> HttpResult<learning_data_access::WorkspaceDraft>
where
    S: Store,
{
    match store.get_draft(context, actor, workspace).await {
        Ok(Some(draft)) => Ok(draft),
        Ok(None) => Err(error_response(StatusCode::NOT_FOUND, "draft not found").into()),
        Err(error) => Err(store_error_response(error).into()),
    }
}

fn validate_qti_draft<B>(
    backends: &B,
    draft: &DraftRecord,
) -> HttpResult<question_model::BackendCapabilities>
where
    B: BackendRegistry,
{
    if draft.question.metadata.validate_title().is_err() {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "question title is invalid",
        )
        .into());
    }
    if !matches!(
        draft.question.source,
        question_model::DraftQuestionSource::Qti { .. }
    ) {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "draft is not a QTI import",
        )
        .into());
    }
    let capabilities = match backends.capabilities(&draft.question.source) {
        Ok(capabilities) => capabilities,
        Err(BackendRegistryError::Unsupported) => {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "question backend is not registered",
            )
            .into());
        }
        Err(BackendRegistryError::Unavailable(_)) => {
            return Err(error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "backend registry unavailable",
            )
            .into());
        }
    };
    let violations = domain::policy::validate_draft_for_publication(&draft.question, &capabilities);
    if violations.is_empty() {
        Ok(capabilities)
    } else {
        Err(no_store(
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": "publication validation failed",
                    "violations": violations,
                })),
            )
                .into_response(),
        )
        .into())
    }
}

/// Resolves QTI publication inputs only from committed private staging.
#[allow(dead_code)] // Route composition follows the atomic Store promotion.
pub(crate) struct QtiPublicationPreparer<S, O> {
    store: Arc<S>,
    objects: Arc<O>,
}

impl<S, O> QtiPublicationPreparer<S, O>
where
    S: QtiImportStore,
    O: ObjectStore,
{
    #[allow(dead_code)]
    pub(crate) fn new(store: Arc<S>, objects: Arc<O>) -> Self {
        Self { store, objects }
    }

    /// Refuses a browser-authored QTI relationship unless the committed ZIP,
    /// registry digest, selected item, and answer-free draft presentation all
    /// agree. This runs before a caller mints publication identities.
    pub(crate) async fn validate(
        &self,
        context: TenantContext,
        draft: &DraftQuestionDefinition,
        import: WorkspaceImportId,
        item_id: &str,
    ) -> Result<ValidatedQtiPublication, StoreError> {
        let registry = self
            .store
            .get_qti_import(context, draft.workspace, import)
            .await?
            .ok_or(StoreError::NotFound)?;
        if registry.reference.workspace != draft.workspace
            || registry.reference.tenant != context.tenant_id()
            || registry.reference.import != import
        {
            return Err(StoreError::NotFound);
        }
        let source = self
            .objects
            .get(&registry.source.key)
            .await
            .map_err(|_| StoreError::NotFound)?;
        if source.record != registry.source
            || Sha256Digest::compute(&source.bytes) != registry.source.sha256
        {
            return Err(StoreError::Conflict);
        }
        let package = QtiImporter::default()
            .import(&source.bytes)
            .map_err(|_| StoreError::Conflict)?;
        let parsed = package
            .questions
            .iter()
            .find(|question| question.item_id == item_id)
            .ok_or(StoreError::NotFound)?;
        let staged = registry
            .items
            .iter()
            .find(|item| item.item_id == item_id)
            .ok_or(StoreError::NotFound)?;
        let canonical = serde_json::to_vec(parsed)
            .map_err(|error| StoreError::Unavailable(error.to_string()))?;
        if staged.model_sha256 != Sha256Digest::compute(&canonical)
            || parsed.prompt != draft.prompt
            || parsed.response != draft.response
        {
            return Err(StoreError::Conflict);
        }
        let item = staged.clone();
        let parsed_assets: std::collections::BTreeSet<_> = qti_question_asset_checksums(parsed)
            .map_err(|_| StoreError::Conflict)?
            .into_keys()
            .collect();
        let staged_assets = item.assets.iter().copied().collect();
        if parsed_assets != staged_assets {
            return Err(StoreError::Conflict);
        }
        Ok(ValidatedQtiPublication {
            registry,
            item,
            draft: draft.clone(),
        })
    }

    /// Copies the exact validated source and prepares item assets for fresh
    /// published identities. Public item bytes are deferred to the durable
    /// post-commit publisher, while institutional item bytes remain private
    /// and can be copied before Store promotion.
    pub(crate) async fn copy_candidates(
        &self,
        draft: &DraftRecord,
        publication: ProblemVersionRef,
        scope: PublicationScope,
        validated: ValidatedQtiPublication,
    ) -> Result<PreparedQtiPublication, StoreError> {
        if draft.tenant != validated.registry.reference.tenant
            || draft.question.workspace != validated.registry.reference.workspace
            || draft.question != validated.draft
        {
            return Err(StoreError::NotFound);
        }
        let source = self
            .objects
            .get(&validated.registry.source.key)
            .await
            .map_err(object_error)?;
        if source.record != validated.registry.source
            || Sha256Digest::compute(&source.bytes) != validated.registry.source.sha256
        {
            return Err(StoreError::Conflict);
        }

        let source_object = ObjectId::generate();
        let source_record = self
            .objects
            .put(PutObject {
                key: ObjectKey::ProblemSource {
                    problem: publication.problem,
                    version: publication.version,
                    object: source_object,
                },
                bytes: source.bytes,
                media_type: validated.registry.source.media_type.clone(),
                license: publication_license(draft),
                provenance: "QTI imported package".to_string(),
                created_at: validated.registry.source.created_at,
            })
            .await
            .map_err(object_error)?;

        let staged_assets = validated
            .registry
            .assets
            .iter()
            .map(|record| match &record.key {
                ObjectKey::WorkspaceAsset { asset, .. } => Ok((*asset, record)),
                _ => Err(StoreError::Conflict),
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let mut assets = Vec::with_capacity(validated.item.assets.len());
        for asset in &validated.item.assets {
            let staged = staged_assets.get(asset).ok_or(StoreError::Conflict)?;
            let stored = self.objects.get(&staged.key).await.map_err(object_error)?;
            if stored.record != **staged || Sha256Digest::compute(&stored.bytes) != staged.sha256 {
                return Err(StoreError::Conflict);
            }
            let object = ObjectId::generate();
            let key = ObjectKey::published_problem_asset(
                scope,
                publication.problem,
                publication.version,
                *asset,
                object,
            );
            let candidate = PutObject {
                key: key.clone(),
                bytes: stored.bytes,
                media_type: staged.media_type.clone(),
                license: publication_license(draft),
                provenance: "QTI imported asset".to_string(),
                created_at: staged.created_at,
            };
            let (record, asset_publication, pending_source) = if scope == PublicationScope::Public {
                // The final CDN key stays absent until the catalog commit
                // has durably recorded both the target and its outbox job.
                (
                    ObjectRecord {
                        id: object,
                        bucket: key.bucket(),
                        key,
                        sha256: staged.sha256,
                        size_bytes: staged.size_bytes,
                        media_type: staged.media_type.clone(),
                        category: ObjectCategory::Asset,
                        version: Some(publication.version),
                        license: candidate.license,
                        provenance: candidate.provenance,
                        created_at: candidate.created_at,
                    },
                    learning_data_access::AssetPublication::Pending,
                    Some((*staged).clone()),
                )
            } else {
                (
                    self.objects.put(candidate).await.map_err(object_error)?,
                    learning_data_access::AssetPublication::Ready,
                    None,
                )
            };
            assets.push(AssetDeliveryRecord {
                id: AssetDeliveryId::from_asset(*asset),
                object: record,
                intrinsic_width: None,
                intrinsic_height: None,
                scope: AssetDeliveryScope::Catalog {
                    asset: *asset,
                    reference: publication,
                },
                publication: asset_publication,
                pending_source,
            });
        }

        Ok(PreparedQtiPublication {
            published_source: QuestionSource::Qti {
                item_id: validated.item.item_id,
                package_object: source_record.id,
                package_sha256: source_record.sha256.to_string(),
            },
            source_artifact: PublishedSourceArtifact {
                reference: publication,
                backend: QuestionBackend::Qti,
                object: source_record,
            },
            promotion: QtiPublicationPromotion {
                staging: validated.registry.reference,
                assets,
            },
        })
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

fn object_error(error: objects::ObjectStoreError) -> StoreError {
    match error {
        objects::ObjectStoreError::NotFound | objects::ObjectStoreError::ChecksumMismatch => {
            StoreError::NotFound
        }
        objects::ObjectStoreError::AlreadyExists
        | objects::ObjectStoreError::NotSignable
        | objects::ObjectStoreError::NumericOverflow => StoreError::Conflict,
        objects::ObjectStoreError::Unavailable(message) => StoreError::Unavailable(message),
    }
}

#[cfg(test)]
#[path = "qti_publication/tests.rs"]
mod tests;
