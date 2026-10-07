//! Ordinary Draft creation for the registered Native JSON, PG, and PGML sources.

use axum::{
    Json,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{MethodRouter, post},
};
use learning_data_access::{AuthoringDraftStore, CreateAuthoringDraftInput, DraftQuestionUuid};
use question_model::{QuestionBackend, QuestionFormat, QuestionMetadata, QuestionType};
use serde::{Deserialize, Deserializer, de::Visitor};
use uuid::Uuid;

use super::{
    AuthoringRouteState, CreatedDraftResponse, PLE_QUESTION_JSON_MEDIA_TYPE,
    authoring_session_hash, private_error, private_store_error,
};
use crate::authoring_source::{inspect_source, matches_source_content_type, put_workspace_source};

const MAX_CREATE_DRAFT_REQUEST_BYTES: usize = 2 * 1024 * 1024;
const WEBWORK_PG_MEDIA_TYPE: &str = "text/x-wework-pg";

/// Builds the bounded POST method for the ordinary Draft collection route.
pub(super) fn route() -> MethodRouter<AuthoringRouteState> {
    post(create_draft).layer(DefaultBodyLimit::max(MAX_CREATE_DRAFT_REQUEST_BYTES))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreateDraftRequest {
    metadata: QuestionMetadata,
    question_backend: QuestionBackend,
    question_format: QuestionFormat,
    webwork_pg_path: RequiredWebworkPgPath,
    source: String,
}

/// A nullable but required JSON member. `Option<String>` alone accepts an omitted key.
#[derive(Debug)]
struct RequiredWebworkPgPath(Option<String>);

impl<'de> Deserialize<'de> for RequiredWebworkPgPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RequiredWebworkPgPathVisitor;

        impl<'de> Visitor<'de> for RequiredWebworkPgPathVisitor {
            type Value = RequiredWebworkPgPath;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a string or null")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RequiredWebworkPgPath(None))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RequiredWebworkPgPath(None))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RequiredWebworkPgPath(Some(value.to_owned())))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RequiredWebworkPgPath(Some(value)))
            }
        }

        deserializer.deserialize_any(RequiredWebworkPgPathVisitor)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct PreparedCreateDraft {
    metadata: QuestionMetadata,
    source_bytes: Vec<u8>,
    source_media_type: &'static str,
    question_format: QuestionFormat,
    webwork_pg_path: Option<String>,
    question_type: Option<QuestionType>,
}

fn decode_request(body: &[u8]) -> Result<CreateDraftRequest, serde_json::Error> {
    serde_json::from_slice(body)
}

fn prepare_request(request: CreateDraftRequest) -> Result<PreparedCreateDraft, ()> {
    let webwork_pg_path = request.webwork_pg_path.0;
    let source_media_type = registered_source_media_type(
        request.question_backend,
        request.question_format,
        webwork_pg_path.as_deref(),
    )
    .ok_or(())?;
    let source_bytes = request.source.into_bytes();
    let question_type = inspect_source(&source_bytes, request.question_backend);

    Ok(PreparedCreateDraft {
        metadata: request.metadata,
        source_bytes,
        source_media_type,
        question_format: request.question_format,
        webwork_pg_path,
        question_type,
    })
}

fn registered_source_media_type(
    question_backend: QuestionBackend,
    question_format: QuestionFormat,
    webwork_pg_path: Option<&str>,
) -> Option<&'static str> {
    match (question_backend, question_format, webwork_pg_path) {
        (QuestionBackend::Ple, QuestionFormat::PleQuestionJson, None) => {
            Some(PLE_QUESTION_JSON_MEDIA_TYPE)
        }
        (
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPg | QuestionFormat::WebworkPgml,
            Some(path),
        ) if valid_webwork_pg_path(path) => Some(WEBWORK_PG_MEDIA_TYPE),
        _ => None,
    }
}

fn valid_webwork_pg_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1_024
        && !value.starts_with('/')
        && !value.contains(['\\', '\0'])
        && !value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
}

async fn create_draft(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !matches_source_content_type(&headers, "application/json") {
        return private_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Draft Question creation must use JSON",
        );
    }
    let session_hash = match authoring_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let request = match decode_request(&body) {
        Ok(request) => request,
        Err(_) => {
            return private_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Draft Question is invalid",
            );
        }
    };
    let prepared = match prepare_request(request) {
        Ok(prepared) => prepared,
        Err(()) => {
            return private_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Draft Question is invalid",
            );
        }
    };

    let workspace = match state
        .drafts
        .ensure_own_authoring_workspace(session_hash, Uuid::now_v7())
        .await
    {
        Ok(workspace) => workspace,
        Err(error) => return private_store_error(error),
    };
    let source_record = match put_workspace_source(
        &state.objects,
        workspace,
        prepared.source_bytes,
        prepared.source_media_type,
    )
    .await
    {
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
                question_format: prepared.question_format,
                webwork_pg_path: prepared.webwork_pg_path,
                question_type: prepared.question_type,
                metadata: prepared.metadata,
            },
        )
        .await;
    match created {
        Ok(draft) => crate::auth::no_store(
            (
                StatusCode::CREATED,
                Json(CreatedDraftResponse {
                    draft_question_id: draft.draft_question_uuid.as_uuid(),
                    draft_question_edit_number: draft.edit_number.as_postgres_bigint().to_string(),
                }),
            )
                .into_response(),
        ),
        Err(error) => private_store_error(error),
    }
}

#[cfg(test)]
#[path = "create/tests.rs"]
mod tests;
