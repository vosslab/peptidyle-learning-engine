//! Native source validation and raw private Draft source access.

use axum::{
    body::Bytes,
    http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::Response,
};
use learning_data_access::AuthoringDraft;
use objects::{ObjectAddress, ObjectStore, PutObject, Sha256Checksum};
use question_model::{ObjectId, QuestionBackend, QuestionFormat, QuestionType, WorkspaceId};
use serde::Serialize;

use crate::authoring::{now, private_error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublicationSourceBinding<'a> {
    Native,
    Webwork {
        question_format: QuestionFormat,
        pg_path: &'a str,
        question_type: QuestionType,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublicationSourceValidationError {
    InvalidBinding,
    MissingWebworkType,
    InvalidSource,
    NativeTypeMismatch,
    RendererUnavailable,
    ThumbnailUnavailable,
}

/// Selects publication behavior only from the persisted source binding and Draft Type.
pub(crate) fn publication_source_binding(
    draft: &AuthoringDraft,
) -> Result<PublicationSourceBinding<'_>, PublicationSourceValidationError> {
    if draft.source_media_type() != Some(draft.source_record.media_type.as_str()) {
        return Err(PublicationSourceValidationError::InvalidBinding);
    }
    match (
        draft.question_backend,
        draft.question_format,
        draft.webwork_pg_path.as_deref(),
    ) {
        (QuestionBackend::Ple, QuestionFormat::PleQuestionJson, None) => {
            Ok(PublicationSourceBinding::Native)
        }
        (
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPg | QuestionFormat::WebworkPgml,
            Some(pg_path),
        ) => draft
            .question_type
            .map(|question_type| PublicationSourceBinding::Webwork {
                question_format: draft.question_format,
                pg_path,
                question_type,
            })
            .ok_or(PublicationSourceValidationError::MissingWebworkType),
        _ => Err(PublicationSourceValidationError::InvalidBinding),
    }
}

pub(crate) fn validated_native_publication_source(
    draft: &AuthoringDraft,
    bytes: &[u8],
) -> Result<QuestionType, PublicationSourceValidationError> {
    if publication_source_binding(draft)? != PublicationSourceBinding::Native {
        return Err(PublicationSourceValidationError::InvalidBinding);
    }
    let question_type =
        validated_source(bytes).map_err(|_| PublicationSourceValidationError::InvalidSource)?;
    if Some(question_type) != draft.question_type {
        return Err(PublicationSourceValidationError::NativeTypeMismatch);
    }
    Ok(question_type)
}

pub(crate) fn validated_source(bytes: &[u8]) -> Result<QuestionType, Box<Response>> {
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
    document.canonical_bytes().map_err(|_| {
        Box::new(private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question source is invalid",
        ))
    })?;
    Ok(compiled.presentation().question_type())
}

/// Reads the source-derived Native Type when the current bytes compile.
/// Incomplete or broken source has no derived metadata and remains valid Draft content.
pub(crate) fn inspect_source(bytes: &[u8], backend: QuestionBackend) -> Option<QuestionType> {
    if backend != QuestionBackend::Ple {
        return None;
    }
    let Ok(document) = adapter_ple::question_json::PleQuestionJsonDocument::parse(bytes) else {
        return None;
    };
    let Ok(compiled) = document.compile() else {
        return None;
    };
    Some(compiled.presentation().question_type())
}

pub(crate) fn matches_source_content_type(headers: &HeaderMap, expected_media_type: &str) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(expected_media_type))
}

/// Adds explicit source-binding headers while keeping the response body as raw source bytes.
pub(crate) fn source_response_headers(draft: &AuthoringDraft) -> Result<HeaderMap, ()> {
    let media_type = draft.source_media_type().ok_or(())?;
    if draft.source_record.media_type != media_type {
        return Err(());
    }
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(media_type));
    insert_serialized_header(
        &mut headers,
        "x-ple-question-backend",
        &draft.question_backend,
    )?;
    insert_serialized_header(
        &mut headers,
        "x-ple-question-format",
        &draft.question_format,
    )?;
    let path = draft
        .webwork_pg_path
        .as_deref()
        .map(percent_encode_header_value)
        .unwrap_or_default();
    headers.insert(
        "x-ple-webwork-pg-path",
        HeaderValue::from_str(&path).map_err(|_| ())?,
    );
    Ok(headers)
}

fn insert_serialized_header<T: Serialize>(
    headers: &mut HeaderMap,
    name: &'static str,
    value: &T,
) -> Result<(), ()> {
    let value = serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or(())?;
    headers.insert(name, HeaderValue::from_str(&value).map_err(|_| ())?);
    Ok(())
}

fn percent_encode_header_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(b"0123456789ABCDEF"[usize::from(byte >> 4)]));
            encoded.push(char::from(b"0123456789ABCDEF"[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

pub(crate) async fn put_workspace_source(
    objects: &impl ObjectStore,
    workspace: WorkspaceId,
    bytes: Vec<u8>,
    media_type: &str,
) -> Result<objects::ObjectRecord, ()> {
    objects
        .put(PutObject {
            address: ObjectAddress::WorkspaceQuestionSource {
                workspace_id: workspace,
                object_id: ObjectId::generate(),
            },
            bytes,
            media_type: media_type.to_owned(),
            created_at: now(),
        })
        .await
        .map_err(|_| ())
}

pub(crate) async fn load_verified_source(
    objects: &impl ObjectStore,
    draft: &AuthoringDraft,
) -> Result<Bytes, ()> {
    let media_type = draft.source_media_type().ok_or(())?;
    let source = objects
        .get(&draft.source_record.address)
        .await
        .map_err(|_| ())?;
    if source.record != draft.source_record
        || source.record.media_type != media_type
        || source.bytes.len() as u64 != source.record.size_bytes
        || Sha256Checksum::compute(&source.bytes) != source.record.sha256
    {
        return Err(());
    }
    Ok(Bytes::from(source.bytes))
}
