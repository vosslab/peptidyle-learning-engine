//! Authoritative native source validation and private source object access.

use axum::{body::Bytes, http::StatusCode, response::Response};
use learning_data_access::AuthoringDraft;
use objects::{ObjectAddress, ObjectStore, PutObject, s3::S3ObjectStore};
use question_model::{ObjectId, QuestionImageAssetTuple, QuestionLicense, QuestionType, Tag};

use crate::authoring::{PLE_QUESTION_JSON_MEDIA_TYPE, now, private_error};

pub(crate) struct ValidatedSource {
    pub bytes: Vec<u8>,
    pub title: String,
    pub description: String,
    pub language: String,
    pub license: Option<QuestionLicense>,
    pub tags: Vec<Tag>,
    pub question_type: QuestionType,
    pub hotspot_surface: Option<QuestionImageAssetTuple>,
}

pub(crate) fn validated_source(bytes: &[u8]) -> Result<ValidatedSource, Box<Response>> {
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
        tags: metadata.tags.clone(),
        question_type: compiled.presentation().question_type(),
        hotspot_surface: match compiled.presentation().response() {
            question_model::QuestionResponseFormat::Hotspot {
                question_image_asset_tuple,
                ..
            } => Some(question_image_asset_tuple.clone()),
            _ => None,
        },
    })
}

pub(crate) async fn put_workspace_source(
    objects: &S3ObjectStore,
    workspace: question_model::WorkspaceId,
    bytes: Vec<u8>,
) -> Result<objects::ObjectRecord, ()> {
    objects
        .put(PutObject {
            address: ObjectAddress::WorkspaceQuestionSource {
                workspace_id: workspace,
                object_id: ObjectId::generate(),
            },
            bytes,
            media_type: PLE_QUESTION_JSON_MEDIA_TYPE.to_string(),
            created_at: now(),
        })
        .await
        .map_err(|_| ())
}

pub(crate) async fn load_verified_source(
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
