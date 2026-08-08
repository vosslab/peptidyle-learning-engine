//! Server-only validation and bytes-first preparation for QTI publication.

use adapter_qti::{QtiImporter, qti_question_asset_checksums};
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::header::IF_MATCH;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use std::{collections::BTreeMap, sync::Arc};

use objects::{ObjectKey, ObjectStore, PutObject, Sha256Digest};
use question_model::{
    DraftQuestionDefinition, ObjectId, ProblemVersionRef, PublicationScope, QuestionBackend,
    QuestionSource, UserId, WorkspaceId, WorkspaceImportId,
};
use serde::Deserialize;
use store::{
    AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, CatalogStore, DraftRecord,
    PublishDraftCommand, PublishedSourceArtifact, QtiImportItem, QtiImportRegistry, QtiImportStore,
    QtiPublicationPromotion, SessionStore, Store, StoreError, TenantContext,
    WorkspaceDraftRevision,
};

use crate::auth::{auth_error_response, no_store, resolve_request_session};
use crate::catalog::{
    BackendRegistry, BackendRegistryError, PublicReviewGate, error_response, may_publish,
    mint_publication_reference, store_error_response,
};

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
    if !may_publish(authenticated.record.subject.roles(), request.scope) {
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
        Err(response) => return response,
    };
    if first.revision != expected_revision {
        return error_response(StatusCode::CONFLICT, "draft changed; reload it");
    }
    let _capabilities = match validate_qti_draft(state.backends.as_ref(), &first.record) {
        Ok(capabilities) => capabilities,
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
    let current = match load_qti_draft(
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
    let capabilities = match validate_qti_draft(state.backends.as_ref(), &current.record) {
        Ok(capabilities) => capabilities,
        Err(response) => return response,
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
    let publication = mint_publication_reference(current.record.revises);
    let prepared = match preparer
        .copy_candidates(&current.record, publication, validated)
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
        Err(error) => store_error_response(error),
    }
}

async fn load_qti_draft<S>(
    store: &S,
    context: TenantContext,
    actor: UserId,
    workspace: WorkspaceId,
) -> Result<store::WorkspaceDraft, Response>
where
    S: Store,
{
    match store.get_draft(context, actor, workspace).await {
        Ok(Some(draft)) => Ok(draft),
        Ok(None) => Err(error_response(StatusCode::NOT_FOUND, "draft not found")),
        Err(error) => Err(store_error_response(error)),
    }
}

#[allow(clippy::result_large_err)] // Route validation deliberately returns the exact HTTP refusal.
fn validate_qti_draft<B>(
    backends: &B,
    draft: &DraftRecord,
) -> Result<question_model::BackendCapabilities, Response>
where
    B: BackendRegistry,
{
    if draft.question.metadata.validate_title().is_err() {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "question title is invalid",
        ));
    }
    if !matches!(
        draft.question.source,
        question_model::DraftQuestionSource::Qti { .. }
    ) {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "draft is not a QTI import",
        ));
    }
    let capabilities = match backends.capabilities(&draft.question.source) {
        Ok(capabilities) => capabilities,
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
        Err(no_store(
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": "publication validation failed",
                    "violations": violations,
                })),
            )
                .into_response(),
        ))
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

    /// Copies only the exact validated source and item assets to fresh,
    /// published candidate keys. This is deliberately separate from Store
    /// promotion: object storage has no multi-object transaction, while the
    /// following Store transaction makes all catalog/grader bindings visible
    /// together.
    pub(crate) async fn copy_candidates(
        &self,
        draft: &DraftRecord,
        publication: ProblemVersionRef,
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
            let record = self
                .objects
                .put(PutObject {
                    key: ObjectKey::ProblemAsset {
                        problem: publication.problem,
                        version: publication.version,
                        asset: *asset,
                        object,
                    },
                    bytes: stored.bytes,
                    media_type: staged.media_type.clone(),
                    license: publication_license(draft),
                    provenance: "QTI imported asset".to_string(),
                    created_at: staged.created_at,
                })
                .await
                .map_err(object_error)?;
            assets.push(AssetDeliveryRecord {
                id: AssetDeliveryId::from_asset(*asset),
                object: record,
                scope: AssetDeliveryScope::Catalog {
                    asset: *asset,
                    reference: publication,
                },
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
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::Request;
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use objects::ObjectCategory;
    use objects::memory::MemoryObjectStore;
    use question_model::envelope::ContentBlock;
    use question_model::generation::RandomizationDefinition;
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        ActivityTimestamp, BackendCapabilities, Capability, GradingDefinition, QuestionMetadata,
        UserRole, WorkspaceId,
    };
    use store::memory::MemoryStore;
    use store::{
        CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, EnqueueJob, JobLeaseDuration,
        JobPayload, JobStore, PageRequest, PageSize, QtiImportStore, SessionLifetime,
        SessionSubject,
    };
    use tower::ServiceExt;

    use crate::qti_import::QtiImportHandler;
    use crate::worker::{JobExecution, JobHandler};

    use super::*;

    struct QtiRegistry;

    impl BackendRegistry for QtiRegistry {
        fn capabilities(
            &self,
            source: &question_model::DraftQuestionSource,
        ) -> Result<BackendCapabilities, BackendRegistryError> {
            if matches!(source, question_model::DraftQuestionSource::Qti { .. }) {
                Ok(BackendCapabilities::from_iter([
                    Capability::ServerGrading,
                    Capability::Hints,
                ]))
            } else {
                Err(BackendRegistryError::Unsupported)
            }
        }
    }

    struct RevisionChangingReview {
        store: Arc<MemoryStore>,
        actor: UserId,
    }

    #[async_trait]
    impl PublicReviewGate for RevisionChangingReview {
        async fn allows_publication(
            &self,
            context: TenantContext,
            _publisher: UserId,
            draft: &DraftRecord,
        ) -> Result<bool, crate::catalog::ReviewGateError> {
            let current = self
                .store
                .get_draft(context, self.actor, draft.question.workspace)
                .await
                .map_err(|error| crate::catalog::ReviewGateError(error.to_string()))?
                .ok_or_else(|| {
                    crate::catalog::ReviewGateError("fixture draft missing".to_string())
                })?;
            let mut changed = current.record;
            changed.question.metadata.title = "Changed during review".to_string();
            self.store
                .upsert_draft(context, self.actor, Some(current.revision), changed)
                .await
                .map_err(|error| crate::catalog::ReviewGateError(error.to_string()))?;
            Ok(true)
        }
    }

    const VALID_PACKAGE: &str = concat!(
        "UEsDBBQAAAAIAHS7B13yXbGdXwAAAIsAAAAPAAAAaW1zbWFuaWZlc3QueG1sVY5RDkAwEESv0uwBNHxXryLClg2l",
        "uku4vYoIfiYvM5PJGF9P5JBFUYuTkCOMJYShA2si8rzGBvnFX4sEPSg5Aib2vAhVl1XtftyKkIPqI7q7xvrSLCWg",
        "rdGfZf0csCdQSwMEFAAAAAgAdLsHXcJKi+S6AAAAiwEAAA4AAABpdGVtcy9pdGVtLnhtbH2QSw7CMAxErxLlAETs",
        "XUu0sOgGUDlBCEaN1CZVHH63J7QgKEXsrPEbe2zQzMTckotlpFbYQ6rs0VLIpE2CRAjEnXdMSzKNDjpa70ZYtdpt",
        "N+vdKqHGh0AmVk8Hwlk3J8I9qKEANSHUj/EIj9W5P9wQOixq75lErEk83UI7vlCYgerSztpbQ6WLFLTpw70mlr9C",
        "ilZfi97CmZynzGzbrqFBGt2lJS5Afbb/wHuJ+TesJtGS9r5MjV+Pd1BLAQIUAxQAAAAIAHS7B13yXbGdXwAAAIsA",
        "AAAPAAAAAAAAAAAAAACAAQAAAABpbXNtYW5pZmVzdC54bWxQSwECFAMUAAAACAB0uwddwkqL5LoAAACLAQAADgAA",
        "AAAAAAAAAAAAgAGMAAAAaXRlbXMvaXRlbS54bWxQSwUGAAAAAAIAAgB5AAAAcgEAAAAA",
    );

    fn id(value: u128) -> uuid::Uuid {
        uuid::Uuid::from_u128(value)
    }

    async fn issued_cookie(store: &MemoryStore, user: UserId, roles: Vec<UserRole>) -> String {
        let issued = crate::auth::issue_session(
            store,
            SessionSubject::new(
                question_model::TenantId::from_uuid(id(1)),
                user,
                "QTI route fixture",
                roles,
            )
            .expect("fixture identity"),
            crate::auth::SessionConfig::new(
                SessionLifetime::from_seconds(3_600).expect("valid fixture lifetime"),
                crate::auth::CookieTransport::LocalHttp,
            ),
        )
        .await
        .expect("fixture session");
        issued
            .set_cookie
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string()
    }

    struct Fixture {
        store: Arc<MemoryStore>,
        objects: Arc<MemoryObjectStore>,
        context: TenantContext,
        draft: DraftRecord,
        import: WorkspaceImportId,
    }

    async fn committed_fixture() -> Fixture {
        let tenant = question_model::TenantId::from_uuid(id(1));
        let workspace = WorkspaceId::from_uuid(id(2));
        let import = WorkspaceImportId::from_uuid(id(3));
        let source_object = ObjectId::from_uuid(id(4));
        let bytes = STANDARD
            .decode(VALID_PACKAGE.trim())
            .expect("fixture base64 must decode");
        let parsed = QtiImporter::default()
            .import(&bytes)
            .expect("fixture package must parse");
        let item = parsed.questions.first().expect("fixture item").clone();
        let store = Arc::new(MemoryStore::default());
        let objects = Arc::new(MemoryObjectStore::default());
        objects
            .put(PutObject {
                key: ObjectKey::WorkspaceSource {
                    tenant,
                    workspace,
                    import,
                    object: source_object,
                },
                bytes,
                media_type: "application/zip".to_string(),
                license: "private-workspace-import".to_string(),
                provenance: "QTI test source".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(1),
            })
            .await
            .expect("fixture source persists");
        let context = TenantContext::from_authenticated_session(tenant);
        let handler = QtiImportHandler::new(Arc::clone(&store), Arc::clone(&objects));
        handler
            .prepare(
                context,
                JobPayload::QtiImport {
                    workspace,
                    import,
                    source_object,
                },
                JobExecution::new(),
            )
            .await
            .expect("fixture QTI preparation");
        let job = store
            .enqueue_job(
                context,
                EnqueueJob {
                    tenant,
                    payload: JobPayload::QtiImport {
                        workspace,
                        import,
                        source_object,
                    },
                    max_attempts: 1,
                },
            )
            .await
            .expect("fixture QTI job");
        let claim = store
            .claim_next_job(JobLeaseDuration::from_seconds(60).expect("valid lease"))
            .await
            .expect("claim query")
            .expect("fixture claim");
        assert_eq!(
            store
                .commit_prepared_qti_import(
                    context,
                    CommitPreparedQtiImport {
                        job,
                        lease: claim.lease_token,
                        reference: store::QtiImportRef {
                            tenant,
                            workspace,
                            import,
                        },
                        source_object,
                    },
                )
                .await
                .expect("fixture import commit"),
            CommitPreparedQtiImportOutcome::Committed
        );
        Fixture {
            store,
            objects,
            context,
            draft: DraftRecord {
                tenant,
                question: question_model::DraftQuestionDefinition {
                    workspace,
                    source: question_model::DraftQuestionSource::Qti {
                        item_id: item.item_id,
                        import_id: import,
                    },
                    prompt: item.prompt,
                    response: item.response,
                    attempt_policy: AttemptPolicy {
                        max_attempts: None,
                        feedback: FeedbackDisclosure::ImmediateCorrectness,
                    },
                    timing_policy: TimingPolicy::Untimed,
                    randomization: RandomizationDefinition::Static,
                    grading: GradingDefinition::AllOrNothing { points: 1.0 },
                    metadata: QuestionMetadata {
                        title: "Imported QTI question".to_string(),
                        tags: Vec::new(),
                        taxonomy: Vec::new(),
                        license: License::CcBy,
                        language: "en-US".to_string(),
                    },
                },
                revises: None,
                derived_from: None,
            },
            import,
        }
    }

    #[tokio::test]
    async fn dedicated_qti_route_validates_then_copies_exact_source_bytes_first() {
        let fixture = committed_fixture().await;
        let preparer =
            QtiPublicationPreparer::new(Arc::clone(&fixture.store), Arc::clone(&fixture.objects));
        let question_model::DraftQuestionSource::Qti { item_id, .. } =
            &fixture.draft.question.source
        else {
            panic!("fixture must be QTI");
        };
        let validated = preparer
            .validate(
                fixture.context,
                &fixture.draft.question,
                fixture.import,
                item_id,
            )
            .await
            .expect("committed matching QTI validates before identity minting");
        let reference = ProblemVersionRef {
            problem: question_model::ProblemId::from_uuid(id(5)),
            version: question_model::VersionId::from_uuid(id(6)),
        };
        let prepared = preparer
            .copy_candidates(&fixture.draft, reference, validated)
            .await
            .expect("validated QTI copies candidate objects");

        let QuestionSource::Qti {
            item_id: published_item,
            package_object,
            package_sha256,
        } = &prepared.published_source
        else {
            panic!("prepared source must remain QTI");
        };
        assert_eq!(published_item, item_id);
        assert_eq!(package_object, &prepared.source_artifact.object.id);
        assert_eq!(
            package_sha256,
            &prepared.source_artifact.object.sha256.to_string()
        );
        assert_eq!(prepared.source_artifact.reference, reference);
        assert_eq!(
            prepared.source_artifact.object.category,
            ObjectCategory::Source
        );
        assert!(matches!(
            prepared.source_artifact.object.key,
            ObjectKey::ProblemSource { problem, version, .. }
                if problem == reference.problem && version == reference.version
        ));
        assert!(prepared.promotion.assets.is_empty());
        let candidate = fixture
            .objects
            .get(&prepared.source_artifact.object.key)
            .await
            .expect("candidate source is written before Store promotion");
        assert_eq!(candidate.record, prepared.source_artifact.object);
        assert_eq!(
            candidate.bytes,
            STANDARD
                .decode(VALID_PACKAGE.trim())
                .expect("fixture bytes")
        );
    }

    #[tokio::test]
    async fn dedicated_qti_route_refuses_changed_draft_before_candidate_copy() {
        let mut fixture = committed_fixture().await;
        fixture.draft.question.prompt.push(ContentBlock::Text {
            markdown: "browser substitution".to_string(),
        });
        let preparer =
            QtiPublicationPreparer::new(Arc::clone(&fixture.store), Arc::clone(&fixture.objects));
        let question_model::DraftQuestionSource::Qti { item_id, .. } =
            &fixture.draft.question.source
        else {
            panic!("fixture must be QTI");
        };
        assert_eq!(
            preparer
                .validate(
                    fixture.context,
                    &fixture.draft.question,
                    fixture.import,
                    item_id,
                )
                .await,
            Err(StoreError::Conflict)
        );
    }

    #[tokio::test]
    async fn dedicated_qti_route_refuses_foreign_tenant_before_object_lookup() {
        let fixture = committed_fixture().await;
        let preparer =
            QtiPublicationPreparer::new(Arc::clone(&fixture.store), Arc::clone(&fixture.objects));
        let question_model::DraftQuestionSource::Qti { item_id, .. } =
            &fixture.draft.question.source
        else {
            panic!("fixture must be QTI");
        };
        assert_eq!(
            preparer
                .validate(
                    TenantContext::from_authenticated_session(question_model::TenantId::from_uuid(
                        id(99)
                    )),
                    &fixture.draft.question,
                    fixture.import,
                    item_id,
                )
                .await,
            Err(StoreError::NotFound)
        );
    }

    #[tokio::test]
    async fn qti_publish_endpoint_is_the_only_route_that_promotes_committed_staging() {
        let fixture = committed_fixture().await;
        let publisher = UserId::from_uuid(id(8));
        let saved = fixture
            .store
            .upsert_draft(fixture.context, publisher, None, fixture.draft.clone())
            .await
            .expect("owner saves exact QTI draft");
        let cookie = issued_cookie(&fixture.store, publisher, vec![UserRole::Instructor]).await;
        let app = router(
            Arc::clone(&fixture.store),
            Arc::clone(&fixture.objects),
            Arc::new(QtiRegistry),
            Arc::new(crate::catalog::ReviewNotRequired),
        );
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/problems/{}/qti-publish",
                        fixture.draft.question.workspace
                    ))
                    .header("cookie", cookie)
                    .header("if-match", format!("\"{}\"", saved.revision.value()))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"institution"}"#))
                    .expect("QTI publish request"),
            )
            .await
            .expect("QTI publish response");
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("published response body");
        assert_eq!(
            status,
            StatusCode::CREATED,
            "{}",
            String::from_utf8_lossy(&body)
        );
        let published: serde_json::Value =
            serde_json::from_slice(&body).expect("published browser projection");
        assert_eq!(published["source"]["backend"], "qti");
        assert!(published["source"].get("importId").is_none());
        assert!(
            fixture
                .store
                .get_draft(fixture.context, publisher, fixture.draft.question.workspace)
                .await
                .expect("post-publish draft lookup")
                .is_none()
        );
    }

    #[tokio::test]
    async fn qti_publish_requires_one_current_strong_workspace_revision() {
        let fixture = committed_fixture().await;
        let publisher = UserId::from_uuid(id(18));
        let saved = fixture
            .store
            .upsert_draft(fixture.context, publisher, None, fixture.draft.clone())
            .await
            .expect("owner saves QTI draft");
        let cookie = issued_cookie(&fixture.store, publisher, vec![UserRole::Instructor]).await;
        let app = router(
            Arc::clone(&fixture.store),
            Arc::clone(&fixture.objects),
            Arc::new(QtiRegistry),
            Arc::new(crate::catalog::ReviewNotRequired),
        );
        for (header, expected) in [
            (None, StatusCode::PRECONDITION_REQUIRED),
            (Some("W/\"1\""), StatusCode::UNPROCESSABLE_ENTITY),
            (Some("\"0\""), StatusCode::UNPROCESSABLE_ENTITY),
            (
                Some("\"9223372036854775808\""),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (Some("\"999\""), StatusCode::CONFLICT),
        ] {
            let mut request = Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/problems/{}/qti-publish",
                    fixture.draft.question.workspace
                ))
                .header("cookie", &cookie)
                .header("content-type", "application/json");
            if let Some(header) = header {
                request = request.header("if-match", header);
            }
            let response = app
                .clone()
                .oneshot(
                    request
                        .body(Body::from(r#"{"scope":"institution"}"#))
                        .expect("QTI publish request"),
                )
                .await
                .expect("QTI publish response");
            assert_eq!(response.status(), expected);
        }
        assert_eq!(saved.revision.value(), 1);
        assert!(
            fixture
                .store
                .get_draft(fixture.context, publisher, fixture.draft.question.workspace)
                .await
                .expect("revision failures retain draft")
                .is_some()
        );
    }

    #[tokio::test]
    async fn qti_publish_rejects_review_time_draft_change_without_visible_version() {
        let fixture = committed_fixture().await;
        let publisher = UserId::from_uuid(id(28));
        let saved = fixture
            .store
            .upsert_draft(fixture.context, publisher, None, fixture.draft.clone())
            .await
            .expect("owner saves QTI draft");
        let cookie = issued_cookie(&fixture.store, publisher, vec![UserRole::Publisher]).await;
        let app = router(
            Arc::clone(&fixture.store),
            Arc::clone(&fixture.objects),
            Arc::new(QtiRegistry),
            Arc::new(RevisionChangingReview {
                store: Arc::clone(&fixture.store),
                actor: publisher,
            }),
        );
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/problems/{}/qti-publish",
                        fixture.draft.question.workspace
                    ))
                    .header("cookie", cookie)
                    .header("if-match", format!("\"{}\"", saved.revision.value()))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"scope":"public"}"#))
                    .expect("QTI publish request"),
            )
            .await
            .expect("QTI publish response");
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let draft = fixture
            .store
            .get_draft(fixture.context, publisher, fixture.draft.question.workspace)
            .await
            .expect("changed draft lookup")
            .expect("changed draft remains");
        assert_eq!(draft.revision.value(), saved.revision.value() + 1);
        assert_eq!(
            draft.record.question.metadata.title,
            "Changed during review"
        );
        let page = fixture
            .store
            .list_catalog(
                fixture.context,
                PageRequest::first(PageSize::new(10).expect("page size")),
            )
            .await
            .expect("catalog lookup");
        assert!(
            page.items.is_empty(),
            "stale publication must stay invisible"
        );
    }
}
