//! Active-Instructor command for forked private Draft Questions.
//!
//! The canonical path is the sole browser source selector. Its empty request
//! body deliberately cannot carry a Question ID, source bytes, attribution,
//! authorship, workspace, or Draft payload.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{
        HeaderMap, StatusCode,
        header::{COOKIE, HeaderName},
    },
    response::{IntoResponse, Response},
    routing::post,
};
use learning_data_access::{
    AuthoringDraftStore, ForkPublishedQuestionAssetInput, ForkPublishedQuestionError,
    ForkPublishedQuestionInput, PublishedQuestionForkAsset, PublishedQuestionLibraryEntry,
    QuestionForkStore, QuestionLibraryStore, SessionTokenHash, StoreError,
    postgres::{
        PostgresAuthoringDraftStore, PostgresQuestionForkStore, PostgresQuestionLibraryStore,
        PostgresSessionStore,
    },
};
use objects::{
    ObjectAddress, ObjectStore, ObjectStoreError, PutObject, ResolvedQuestionSource,
    image_validation::verify_still_image, s3::S3ObjectStore,
};
use question_model::{
    DraftQuestionReference, ObjectId, ProductRole, QuestionBackend, QuestionId,
    QuestionResponseFormat, QuestionRevisionNumber, QuestionRevisionReference, QuestionType,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    auth::{AuthError, resolve_session},
    question_publication::{QuestionIdIssuer, RandomQuestionIdIssuer},
};

const QUESTION_FORK_IDENTITY_ATTEMPTS: usize = 8;
const IDEMPOTENCY_KEY: HeaderName = HeaderName::from_static("idempotency-key");

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    forks: Arc<dyn QuestionForkStore>,
    library: PostgresQuestionLibraryStore,
    drafts: PostgresAuthoringDraftStore,
    objects: S3ObjectStore,
    question_id_issuer: Arc<dyn QuestionForkIdIssuer>,
}

/// Trusted issuance capability for a fork's future Question ID.
trait QuestionForkIdIssuer: Send + Sync {
    fn issue_question_id(
        &self,
    ) -> Result<QuestionId, crate::question_publication::QuestionIdIssuanceError>;
}

impl QuestionForkIdIssuer for RandomQuestionIdIssuer {
    fn issue_question_id(
        &self,
    ) -> Result<QuestionId, crate::question_publication::QuestionIdIssuanceError> {
        QuestionIdIssuer::issue_question_id(self)
    }
}

/// Registers the canonical active-Instructor fork command.
pub fn question_fork_router(
    sessions: Arc<PostgresSessionStore>,
    forks: PostgresQuestionForkStore,
    library: PostgresQuestionLibraryStore,
    drafts: PostgresAuthoringDraftStore,
    objects: S3ObjectStore,
    question_id_issuer: RandomQuestionIdIssuer,
) -> Router {
    question_fork_router_with_trusted_dependencies(
        sessions,
        Arc::new(forks),
        library,
        drafts,
        objects,
        Arc::new(question_id_issuer),
    )
}

fn question_fork_router_with_trusted_dependencies(
    sessions: Arc<PostgresSessionStore>,
    forks: Arc<dyn QuestionForkStore>,
    library: PostgresQuestionLibraryStore,
    drafts: PostgresAuthoringDraftStore,
    objects: S3ObjectStore,
    question_id_issuer: Arc<dyn QuestionForkIdIssuer>,
) -> Router {
    Router::new()
        .route(
            "/api/questions/by-id/{question_id}/revisions/{revision_number}/fork",
            post(fork_published_question),
        )
        .with_state(RouteState {
            sessions,
            forks,
            library,
            drafts,
            objects,
            question_id_issuer,
        })
}

/// Answer-free navigation result for one distinct private Draft.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ForkedQuestionResponse {
    draft_question: DraftQuestionReference,
}

async fn fork_published_question(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path((question_id, revision_number)): Path<(String, String)>,
    body: Bytes,
) -> Response {
    if !body.is_empty() {
        return route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Fork is invalid");
    }
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let idempotency_key = match idempotency_key(&headers) {
        Some(value) => value,
        None => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Question Fork retry key is invalid",
            );
        }
    };
    let source_question_revision = match canonical_source(&state, question_id, revision_number) {
        Some(value) => value,
        None => return concealed(),
    };
    let library_entry = match state
        .library
        .load_published_question_revision_library_entry(session, &source_question_revision)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let source = match ResolvedQuestionSource::resolve(
        &state.objects,
        source_question_revision.clone(),
        library_entry.source_object_reference.clone(),
        library_entry.source_object_checksum.clone(),
    )
    .await
    {
        Ok(value) if value.media_type() == library_entry.source_media_type => value,
        Ok(_) | Err(_) => return unavailable(),
    };
    let workspace = match state
        .drafts
        .ensure_own_authoring_workspace(session, Uuid::now_v7())
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let hotspot_asset = match load_hotspot_asset(
        &state,
        session,
        &library_entry,
        &source_question_revision,
        source.bytes(),
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return *response,
    };

    for _ in 0..QUESTION_FORK_IDENTITY_ATTEMPTS {
        let forked_question_id = match state.question_id_issuer.issue_question_id() {
            Ok(value) => value,
            Err(_) => {
                return route_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Question Fork is unavailable",
                );
            }
        };
        let proposed_draft_question_id = Uuid::now_v7();
        let target_source_record = match state
            .objects
            .put(PutObject {
                address: ObjectAddress::WorkspaceQuestionSource {
                    workspace,
                    object: ObjectId::generate(),
                },
                bytes: source.bytes().to_vec(),
                media_type: source.media_type().to_owned(),
                created_at: crate::authoring::now(),
            })
            .await
        {
            Ok(value) => value,
            Err(ObjectStoreError::AlreadyExists) => continue,
            Err(_) => return unavailable(),
        };
        let target_source_address = target_source_record.address.clone();
        let target_hotspot_asset = match copy_hotspot_asset(
            &state.objects,
            workspace,
            proposed_draft_question_id,
            hotspot_asset.as_ref(),
        )
        .await
        {
            Ok(value) => value,
            Err(ObjectStoreError::AlreadyExists) => {
                if state.objects.delete(&target_source_address).await.is_err() {
                    return unavailable();
                }
                continue;
            }
            Err(_) => return unavailable(),
        };
        let target_asset_address = target_hotspot_asset
            .as_ref()
            .map(|asset| asset.target_record.address.clone());
        let input = ForkPublishedQuestionInput {
            source_question_revision: source_question_revision.clone(),
            forked_question_id,
            workspace,
            proposed_draft_question_id,
            target_source_record,
            hotspot_asset: target_hotspot_asset,
            idempotency_key,
        };
        match state
            .forks
            .fork_published_question_to_draft(session, input)
            .await
        {
            Ok(fork) => {
                if !fork.created_new {
                    cleanup_replayed_candidates(
                        &state.objects,
                        &target_source_address,
                        target_asset_address.as_ref(),
                    )
                    .await;
                }
                return crate::auth::no_store(
                    (
                        StatusCode::CREATED,
                        Json(ForkedQuestionResponse {
                            draft_question: fork.draft_question,
                        }),
                    )
                        .into_response(),
                );
            }
            Err(ForkPublishedQuestionError::IdentityCollision) => {
                if cleanup_candidates(
                    &state.objects,
                    &target_source_address,
                    target_asset_address.as_ref(),
                )
                .await
                .is_err()
                {
                    return unavailable();
                }
                continue;
            }
            Err(ForkPublishedQuestionError::Store(error)) => return store_error_response(error),
        }
    }
    unavailable()
}

struct VerifiedForkHotspotAsset {
    evidence: PublishedQuestionForkAsset,
    bytes: Vec<u8>,
}

async fn load_hotspot_asset(
    state: &RouteState,
    session: SessionTokenHash,
    entry: &PublishedQuestionLibraryEntry,
    revision: &QuestionRevisionReference,
    source: &[u8],
) -> Result<Option<VerifiedForkHotspotAsset>, Box<Response>> {
    if entry.question_type != QuestionType::Hotspot {
        return Ok(None);
    }
    if entry.backend != QuestionBackend::Ple
        || entry.source_media_type != adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE
    {
        return Err(Box::new(unavailable()));
    }
    let document = adapter_ple::question_json::PleQuestionJsonDocument::parse(source)
        .map_err(|_| Box::new(unavailable()))?;
    let compiled = document.compile().map_err(|_| Box::new(unavailable()))?;
    let QuestionResponseFormat::Hotspot { surface, .. } = compiled.presentation().response() else {
        return Err(Box::new(unavailable()));
    };
    let evidence = state
        .forks
        .load_published_question_fork_asset(session, revision)
        .await
        .map_err(|_| Box::new(unavailable()))?
        .ok_or_else(|| Box::new(unavailable()))?;
    if evidence.asset_id != surface.question_asset
        || evidence.source_record.sha256.to_string() != surface.checksum
    {
        return Err(Box::new(unavailable()));
    }
    let stored = state
        .objects
        .get(&evidence.source_record.address)
        .await
        .map_err(|_| Box::new(unavailable()))?;
    if stored.record != evidence.source_record {
        return Err(Box::new(unavailable()));
    }
    let verified = verify_still_image(&stored.bytes).map_err(|_| Box::new(unavailable()))?;
    if verified.media_type.canonical_media_type() != evidence.source_record.media_type
        || verified.width != evidence.intrinsic_width
        || verified.height != evidence.intrinsic_height
    {
        return Err(Box::new(unavailable()));
    }
    Ok(Some(VerifiedForkHotspotAsset {
        evidence,
        bytes: stored.bytes,
    }))
}

async fn copy_hotspot_asset(
    objects: &S3ObjectStore,
    workspace: question_model::WorkspaceId,
    draft_question_uuid: Uuid,
    source: Option<&VerifiedForkHotspotAsset>,
) -> Result<Option<ForkPublishedQuestionAssetInput>, ObjectStoreError> {
    let Some(source) = source else {
        return Ok(None);
    };
    let target_record = objects
        .put(PutObject {
            address: ObjectAddress::DraftQuestionAsset {
                workspace,
                draft_question_uuid,
                asset: source.evidence.asset_id,
                object: ObjectId::generate(),
            },
            bytes: source.bytes.clone(),
            media_type: source.evidence.source_record.media_type.clone(),
            created_at: crate::authoring::now(),
        })
        .await?;
    Ok(Some(ForkPublishedQuestionAssetInput {
        asset_id: source.evidence.asset_id,
        target_record,
        intrinsic_width: source.evidence.intrinsic_width,
        intrinsic_height: source.evidence.intrinsic_height,
    }))
}

async fn cleanup_candidates(
    objects: &S3ObjectStore,
    source: &ObjectAddress,
    asset: Option<&ObjectAddress>,
) -> Result<(), ObjectStoreError> {
    let source_result = objects.delete(source).await;
    let asset_result = match asset {
        Some(asset) => objects.delete(asset).await,
        None => Ok(()),
    };
    source_result.and(asset_result)
}

async fn cleanup_replayed_candidates(
    objects: &S3ObjectStore,
    source: &ObjectAddress,
    asset: Option<&ObjectAddress>,
) {
    if let Err(error) = cleanup_candidates(objects, source, asset).await {
        // The immutable database receipt is authoritative. Cleanup remains a
        // diagnostic concern and must never turn a recovered retry into failure.
        tracing::error!(?error, "Question Fork replay candidate cleanup failed");
    }
}

fn canonical_source(
    state: &RouteState,
    question_id: String,
    revision_number: String,
) -> Option<QuestionRevisionReference> {
    let question_id = question_id.parse::<QuestionId>().ok()?;
    let revision_number = revision_number
        .parse::<u32>()
        .ok()
        .and_then(|value| QuestionRevisionNumber::new(value).ok())?;
    Some(QuestionRevisionReference {
        question_id,
        revision_number,
    })
}

fn idempotency_key(headers: &HeaderMap) -> Option<Uuid> {
    headers.get(&IDEMPOTENCY_KEY)?.to_str().ok()?.parse().ok()
}

async fn instructor_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.product_role == ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Fork authentication is unavailable",
        ))),
    }
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => {
            route_error(StatusCode::UNPROCESSABLE_ENTITY, "Question Fork is invalid")
        }
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Question Fork changed")
        }
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Question Fork conflicts")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Fork is unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Question Fork not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

fn unavailable() -> Response {
    route_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Question Fork is unavailable",
    )
}
