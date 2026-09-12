//! Private Draft Question authoring and first-publication Server Routes.
//!
//! Only an authorized Draft Question Reference and its Edit Number cross the
//! browser boundary. Workspace UUIDs, Draft Question UUIDs, source object
//! addresses, and publication copy inputs remain server-only.

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CONTENT_TYPE, COOKIE, ETAG, IF_MATCH},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use learning_data_access::{
    AuthoringDraft, AuthoringDraftStore, CreateAuthoringDraftInput, DraftQuestionEditNumber,
    DraftQuestionUuid, SaveAuthoringDraftInput, SessionTokenHash, StoreError,
    postgres::{
        PostgresAuthoringDraftStore, PostgresDraftQuestionSourceBindingStore, PostgresSessionStore,
    },
};
use objects::{ObjectAddress, ObjectStore, PutObject, s3::S3ObjectStore};
use question_model::{
    DraftQuestionReference, ObjectId, QuestionAuthor, QuestionAuthorDisplayName,
    QuestionAuthorship, QuestionId, QuestionLicense, QuestionRevisionNumber,
    QuestionRevisionReason, QuestionRevisionReference, Timestamp,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{AuthError, resolve_session},
    question_publication::{
        ExistingQuestionRevisionPublicationCommand, ExistingQuestionRevisionPublisher,
        HmacQuestionIdIssuer, NewQuestionLineagePublicationCommand, NewQuestionLineagePublisher,
    },
};

const PLE_QUESTION_JSON_MEDIA_TYPE: &str = "application/vnd.peptidyle.question+json";
const INITIAL_PUBLICATION_REASON: &str = "Initial publication from Authoring Workspace";

#[derive(Clone)]
struct AuthoringRouteState {
    sessions: Arc<PostgresSessionStore>,
    drafts: PostgresAuthoringDraftStore,
    publication: PostgresDraftQuestionSourceBindingStore,
    objects: S3ObjectStore,
    question_id_issuer: HmacQuestionIdIssuer,
}

/// Registers private Authoring Workspace routes and the initial publication operation.
pub fn authoring_router(
    sessions: Arc<PostgresSessionStore>,
    drafts: PostgresAuthoringDraftStore,
    publication: PostgresDraftQuestionSourceBindingStore,
    objects: S3ObjectStore,
    question_id_issuer: HmacQuestionIdIssuer,
) -> Router {
    Router::new()
        .route("/api/authoring/drafts", get(list_drafts).post(create_draft))
        .route(
            "/api/authoring/drafts/{reference}/source",
            get(load_source).put(save_source),
        )
        .route(
            "/api/authoring/drafts/{reference}/publish",
            post(publish_draft),
        )
        .route(
            "/api/authoring/drafts/{reference}/publish-revision",
            post(publish_revision_draft),
        )
        .with_state(AuthoringRouteState {
            sessions,
            drafts,
            publication,
            objects,
            question_id_issuer,
        })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DraftSummaryResponse {
    draft_question: DraftQuestionReference,
    edit_number: u64,
    question_title: String,
    question_description: String,
}

#[derive(Debug, Serialize)]
struct DraftListResponse {
    items: Vec<DraftSummaryResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreatedDraftResponse {
    draft_question: DraftQuestionReference,
    edit_number: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublishDraftRequest {
    authors: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublishedDraftResponse {
    question_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PublishRevisionDraftRequest {
    question_id: String,
    parent_revision_number: u32,
    reason_for_edit: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublishedRevisionDraftResponse {
    question_revision: QuestionRevisionReference,
}

async fn list_drafts(State(state): State<AuthoringRouteState>, headers: HeaderMap) -> Response {
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.drafts.list_authoring_drafts(session_hash).await {
        Ok(drafts) => crate::auth::no_store(
            Json(DraftListResponse {
                items: drafts
                    .into_iter()
                    .map(|draft| DraftSummaryResponse {
                        draft_question: draft.reference,
                        edit_number: draft.edit_number.as_postgres_bigint() as u64,
                        question_title: draft.title,
                        question_description: draft.description,
                    })
                    .collect(),
            })
            .into_response(),
        ),
        Err(error) => private_store_error(error),
    }
}

async fn create_draft(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !is_ple_question_json_request(&headers) {
        return private_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Draft Question source media type is required",
        );
    }
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let source = match validated_source(&body) {
        Ok(source) => source,
        Err(response) => return *response,
    };
    let workspace = match state
        .drafts
        .ensure_own_authoring_workspace(session_hash, Uuid::now_v7())
        .await
    {
        Ok(workspace) => workspace,
        Err(error) => return private_store_error(error),
    };
    let source_record =
        match put_workspace_source(&state.objects, workspace, source.bytes.clone()).await {
            Ok(record) => record,
            Err(()) => {
                return private_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Authoring storage is unavailable",
                );
            }
        };
    let created = state
        .drafts
        .create_authoring_draft(
            session_hash,
            workspace,
            CreateAuthoringDraftInput {
                draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::now_v7()),
                source_record,
                webwork_pg_path: None,
                title: source.title,
                description: source.description,
                language: source.language,
            },
        )
        .await;
    match created {
        Ok(draft) => crate::auth::no_store(
            (
                StatusCode::CREATED,
                Json(CreatedDraftResponse {
                    draft_question: draft.reference,
                    edit_number: draft.edit_number.as_postgres_bigint() as u64,
                }),
            )
                .into_response(),
        ),
        Err(error) => private_store_error(error),
    }
}

async fn load_source(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let reference = match parse_reference(&reference) {
        Ok(reference) => reference,
        Err(response) => return *response,
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let draft = match state
        .drafts
        .load_authoring_draft(session_hash, reference)
        .await
    {
        Ok(draft) => draft,
        Err(error) => return private_store_error(error),
    };
    let source = match load_verified_source(&state.objects, &draft).await {
        Ok(bytes) => bytes,
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring storage is unavailable",
            );
        }
    };
    let mut response = crate::auth::no_store(source.into_response());
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static(PLE_QUESTION_JSON_MEDIA_TYPE),
    );
    match etag(draft.edit_number) {
        Ok(value) => response.headers_mut().insert(ETAG, value),
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring draft is unavailable",
            );
        }
    };
    response
}

async fn save_source(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    body: Bytes,
) -> Response {
    if !is_ple_question_json_request(&headers) {
        return private_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Draft Question source media type is required",
        );
    }
    let reference = match parse_reference(&reference) {
        Ok(reference) => reference,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(&headers) {
        Ok(number) => number,
        Err(response) => return *response,
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let source = match validated_source(&body) {
        Ok(source) => source,
        Err(response) => return *response,
    };
    let current = match state
        .drafts
        .load_authoring_draft(session_hash, reference)
        .await
    {
        Ok(draft) => draft,
        Err(error) => return private_store_error(error),
    };
    let source_record =
        match put_workspace_source(&state.objects, current.workspace, source.bytes.clone()).await {
            Ok(record) => record,
            Err(()) => {
                return private_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Authoring storage is unavailable",
                );
            }
        };
    match state
        .drafts
        .save_authoring_draft(
            session_hash,
            SaveAuthoringDraftInput {
                reference,
                expected_edit_number,
                source_record,
                title: source.title,
                description: source.description,
                language: source.language,
            },
        )
        .await
    {
        Ok(draft) => edit_number_response(draft.edit_number),
        Err(StoreError::Conflict | StoreError::RetryableTransaction) => private_error(
            StatusCode::PRECONDITION_FAILED,
            "Draft Question changed before this save",
        ),
        Err(StoreError::LifecycleConflict) => {
            private_error(StatusCode::CONFLICT, "Draft Question lifecycle conflict")
        }
        Err(error) => private_store_error(error),
    }
}

async fn publish_draft(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    Json(request): Json<PublishDraftRequest>,
) -> Response {
    let reference = match parse_reference(&reference) {
        Ok(reference) => reference,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(&headers) {
        Ok(number) => number,
        Err(response) => return *response,
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let draft = match state
        .drafts
        .load_authoring_draft(session_hash, reference)
        .await
    {
        Ok(draft) => draft,
        Err(error) => return private_store_error(error),
    };
    let bytes = match load_verified_source(&state.objects, &draft).await {
        Ok(bytes) => bytes,
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring storage is unavailable",
            );
        }
    };
    let source = match validated_source(&bytes) {
        Ok(source) => source,
        Err(_) => {
            return private_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Draft Question cannot be published",
            );
        }
    };
    let authorship = match question_authorship(request.authors) {
        Ok(authorship) => authorship,
        Err(()) => {
            return private_error(
                StatusCode::BAD_REQUEST,
                "Question Publication requires reviewed authors",
            );
        }
    };
    let Some(question_license) = source.license else {
        return private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question requires a Question License before publication",
        );
    };
    let command = NewQuestionLineagePublicationCommand {
        draft_question_uuid: draft.draft_question_uuid,
        expected_draft_question_edit_number: expected_edit_number,
        workspace: draft.workspace,
        question_authorship: authorship,
        question_license,
        question_revision_reason: QuestionRevisionReason::new(
            INITIAL_PUBLICATION_REASON.to_string(),
        )
        .expect("fixed initial publication reason is valid"),
    };
    let publisher = NewQuestionLineagePublisher::new(
        state.objects.clone(),
        state.publication.clone(),
        state.question_id_issuer.clone(),
    );
    match publisher.publish(session_hash, command, now()).await {
        Ok(revision) => crate::auth::no_store(
            Json(PublishedDraftResponse {
                question_id: revision.question_id.to_string(),
            })
            .into_response(),
        ),
        Err(error) => publication_error(error),
    }
}

async fn publish_revision_draft(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    Json(request): Json<PublishRevisionDraftRequest>,
) -> Response {
    let reference = match parse_reference(&reference) {
        Ok(reference) => reference,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(&headers) {
        Ok(number) => number,
        Err(response) => return *response,
    };
    let parent_question_revision = match existing_parent_question_revision(
        &state.question_id_issuer,
        request.question_id,
        request.parent_revision_number,
    ) {
        Ok(revision) => revision,
        Err(()) => return concealed(),
    };
    let revision_reason = match QuestionRevisionReason::new(request.reason_for_edit) {
        Ok(reason) => reason,
        Err(_) => {
            return private_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Question Revision requires a reviewed reason for edit",
            );
        }
    };
    let session_hash = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let draft = match state
        .drafts
        .load_authoring_draft(session_hash, reference)
        .await
    {
        Ok(draft) => draft,
        Err(error) => return private_store_error(error),
    };
    let bytes = match load_verified_source(&state.objects, &draft).await {
        Ok(bytes) => bytes,
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring storage is unavailable",
            );
        }
    };
    if validated_source(&bytes).is_err() {
        return private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question cannot be published",
        );
    }
    let publisher =
        ExistingQuestionRevisionPublisher::new(state.objects.clone(), state.publication.clone());
    match publisher
        .publish(
            session_hash,
            ExistingQuestionRevisionPublicationCommand {
                draft_question_uuid: draft.draft_question_uuid,
                expected_draft_question_edit_number: expected_edit_number,
                workspace: draft.workspace,
                parent_question_revision,
                question_revision_reason: revision_reason,
            },
            now(),
        )
        .await
    {
        Ok(question_revision) => crate::auth::no_store(
            Json(PublishedRevisionDraftResponse { question_revision }).into_response(),
        ),
        Err(error) => publication_error(error),
    }
}

fn existing_parent_question_revision(
    question_id_issuer: &HmacQuestionIdIssuer,
    question_id: String,
    parent_revision_number: u32,
) -> Result<QuestionRevisionReference, ()> {
    let question_id = question_id.parse::<QuestionId>().map_err(|_| ())?;
    if !question_id_issuer.validates_question_id(&question_id) {
        return Err(());
    }
    let revision_number = QuestionRevisionNumber::new(parent_revision_number).map_err(|_| ())?;
    Ok(QuestionRevisionReference {
        question_id,
        revision_number,
    })
}

struct ValidatedSource {
    bytes: Vec<u8>,
    title: String,
    description: String,
    language: String,
    license: Option<QuestionLicense>,
}

fn validated_source(bytes: &[u8]) -> Result<ValidatedSource, Box<Response>> {
    let document =
        adapter_ple::question_json::PleQuestionJsonDocument::parse(bytes).map_err(|_| {
            Box::new(private_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Draft Question source is invalid",
            ))
        })?;
    let compiled = document.compile().map_err(|_| {
        Box::new(private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question source is invalid",
        ))
    })?;
    let metadata = compiled.presentation().metadata();
    let bytes = document.canonical_bytes().map_err(|_| {
        Box::new(private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question source is invalid",
        ))
    })?;
    Ok(ValidatedSource {
        bytes,
        title: metadata.question_title.clone(),
        description: metadata.question_description.clone(),
        language: metadata.language.clone(),
        license: metadata.question_license.clone(),
    })
}

async fn put_workspace_source(
    objects: &S3ObjectStore,
    workspace: question_model::WorkspaceId,
    bytes: Vec<u8>,
) -> Result<objects::ObjectRecord, ()> {
    objects
        .put(PutObject {
            address: ObjectAddress::WorkspaceQuestionSource {
                workspace,
                object: ObjectId::generate(),
            },
            bytes,
            media_type: PLE_QUESTION_JSON_MEDIA_TYPE.to_string(),
            created_at: now(),
        })
        .await
        .map_err(|_| ())
}

async fn load_verified_source(
    objects: &S3ObjectStore,
    draft: &AuthoringDraft,
) -> Result<Bytes, ()> {
    let source = objects
        .get(&draft.source_record.address)
        .await
        .map_err(|_| ())?;
    if source.record != draft.source_record
        || source.record.media_type != PLE_QUESTION_JSON_MEDIA_TYPE
    {
        return Err(());
    }
    Ok(Bytes::from(source.bytes))
}

fn question_authorship(authors: Vec<String>) -> Result<QuestionAuthorship, ()> {
    let authors = authors
        .into_iter()
        .map(|name| {
            QuestionAuthorDisplayName::new(name).map(|display_name| QuestionAuthor { display_name })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ())?;
    QuestionAuthorship::new(authors).map_err(|_| ())
}

fn parse_reference(value: &str) -> Result<DraftQuestionReference, Box<Response>> {
    value.parse().map_err(|_| Box::new(concealed()))
}

fn expected_edit_number(headers: &HeaderMap) -> Result<DraftQuestionEditNumber, Box<Response>> {
    let Some(value) = headers.get(IF_MATCH).and_then(|value| value.to_str().ok()) else {
        return Err(Box::new(private_error(
            StatusCode::PRECONDITION_REQUIRED,
            "Draft Question Edit Number is required",
        )));
    };
    let Some(number) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(Box::new(private_error(
            StatusCode::BAD_REQUEST,
            "Draft Question Edit Number is invalid",
        )));
    };
    number
        .parse::<u64>()
        .ok()
        .and_then(|number| DraftQuestionEditNumber::new(number).ok())
        .ok_or_else(|| {
            Box::new(private_error(
                StatusCode::BAD_REQUEST,
                "Draft Question Edit Number is invalid",
            ))
        })
}

fn etag(edit_number: DraftQuestionEditNumber) -> Result<HeaderValue, ()> {
    HeaderValue::from_str(&format!("\"{}\"", edit_number.as_postgres_bigint())).map_err(|_| ())
}

fn edit_number_response(edit_number: DraftQuestionEditNumber) -> Response {
    let mut response = crate::auth::no_store(StatusCode::NO_CONTENT.into_response());
    match etag(edit_number) {
        Ok(value) => response.headers_mut().insert(ETAG, value),
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring draft is unavailable",
            );
        }
    };
    response
}

fn is_ple_question_json_request(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim() == PLE_QUESTION_JSON_MEDIA_TYPE)
        })
}

async fn instructor_session_hash(
    state: &AuthoringRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.product_role == question_model::ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(private_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Authoring authentication is unavailable",
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

fn private_store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => private_error(
            StatusCode::PRECONDITION_FAILED,
            "Draft Question changed before this operation",
        ),
        StoreError::LifecycleConflict => {
            private_error(StatusCode::CONFLICT, "Draft Question lifecycle conflict")
        }
        StoreError::InvalidRecord(_) => private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question is invalid",
        ),
        StoreError::AlreadyExists => {
            private_error(StatusCode::CONFLICT, "Draft Question operation conflicts")
        }
        StoreError::Unavailable(_) | StoreError::AssignmentActivity(_) | StoreError::TimedOut => {
            private_error(StatusCode::SERVICE_UNAVAILABLE, "Authoring is unavailable")
        }
    }
}

fn publication_error(error: crate::question_publication::QuestionPublicationError) -> Response {
    match error {
        crate::question_publication::QuestionPublicationError::StaleQuestionRevision => {
            private_error(
                StatusCode::PRECONDITION_FAILED,
                "Question Revision changed before publication",
            )
        }
        crate::question_publication::QuestionPublicationError::Store(
            StoreError::Conflict | StoreError::RetryableTransaction,
        ) => private_error(
            StatusCode::PRECONDITION_FAILED,
            "Draft Question changed before publication",
        ),
        crate::question_publication::QuestionPublicationError::Store(
            StoreError::LifecycleConflict,
        ) => private_error(
            StatusCode::CONFLICT,
            "Question publication lifecycle conflict",
        ),
        crate::question_publication::QuestionPublicationError::Store(error) => {
            private_store_error(error)
        }
        crate::question_publication::QuestionPublicationError::IdentityCollisions
        | crate::question_publication::QuestionPublicationError::ObjectStore(_)
        | crate::question_publication::QuestionPublicationError::SourceObjectRecordMismatch
        | crate::question_publication::QuestionPublicationError::QuestionIdIssuance(_) => {
            private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Question Publication is unavailable",
            )
        }
    }
}

fn concealed() -> Response {
    crate::auth::no_store(
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Not found" })),
        )
            .into_response(),
    )
}

fn private_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, Json(serde_json::json!({ "error": message }))).into_response())
}

fn now() -> Timestamp {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(milliseconds).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::question_publication::{QuestionIdIssuer, QuestionIdSecret};

    #[test]
    fn same_lineage_publication_requires_a_server_validated_positive_parent_revision() {
        let issuer = HmacQuestionIdIssuer::new(QuestionIdSecret::from_bytes([9; 32]));
        let question_id = issuer.issue_question_id().expect("issued Question ID");

        assert_eq!(
            existing_parent_question_revision(&issuer, question_id.to_string(), 1),
            Ok(QuestionRevisionReference {
                question_id: question_id.clone(),
                revision_number: QuestionRevisionNumber::new(1)
                    .expect("positive Question Revision Number"),
            })
        );
        assert!(existing_parent_question_revision(&issuer, question_id.to_string(), 0).is_err());
        assert!(existing_parent_question_revision(&issuer, "000-0000".to_string(), 1).is_err());
    }
}
