//! Existing initial/fork and successor publication routes share private capture.

use adapter_webwork::{WebworkAdapterError, renderer_contract::RendererFailure};
use question_model::{QuestionRevisionReason, generation::QuestionSeed};
use tokio::time::Instant;

use super::http::{
    existing_parent_published_question_revision_tuple, publication_error, question_authorship,
};
use super::{
    AuthoringDraft, AuthoringRouteState, HeaderMap, INITIAL_PUBLICATION_REASON, Json, Path,
    PublishDraftRequest, PublishRevisionDraftRequest, PublishedDraftResponse,
    PublishedRevisionDraftResponse, QuestionMetadata, Response, State, StatusCode,
    authoring_session_hash, canonical_classification_uuid, concealed, expected_edit_number, now,
    parse_draft_question_uuid, private_error, private_store_error,
};
use crate::authoring_source::{
    PublicationSourceBinding, PublicationSourceValidationError, load_verified_source,
    publication_source_binding, validated_native_publication_source,
};
use crate::question_publication::{
    DraftQuestionImageContext, ExistingQuestionRevisionPublicationCommand,
    ExistingQuestionRevisionPublisher, NewQuestionLineagePublicationCommand,
    NewQuestionLineagePublisher, PUBLICATION_DRAIN_BOUND, PUBLICATION_WORK_BOUND,
    PreparedPublicationThumbnail,
};
use axum::response::IntoResponse;
use learning_data_access::AuthoringDraftStore;

async fn validate_publication_source(
    state: &AuthoringRouteState,
    draft: &AuthoringDraft,
    bytes: &[u8],
) -> Result<Option<PreparedPublicationThumbnail>, PublicationSourceValidationError> {
    match publication_source_binding(draft)? {
        PublicationSourceBinding::Native => {
            validated_native_publication_source(draft, bytes)?;
            Ok(None)
        }
        PublicationSourceBinding::Webwork { pg_path, .. } => {
            std::str::from_utf8(bytes)
                .map_err(|_| PublicationSourceValidationError::InvalidSource)?;
            let document = state
                .webwork
                .preview_draft_document(QuestionSeed::new(0), bytes, pg_path)
                .await
                .map_err(|error| match error {
                    WebworkAdapterError::UnsupportedSource | WebworkAdapterError::InvalidPgPath => {
                        PublicationSourceValidationError::InvalidBinding
                    }
                    WebworkAdapterError::Renderer(RendererFailure::InvalidOutput(_)) => {
                        PublicationSourceValidationError::InvalidSource
                    }
                    WebworkAdapterError::SourceChecksumMismatch
                    | WebworkAdapterError::UntrustedSource
                    | WebworkAdapterError::ObjectStore(_)
                    | WebworkAdapterError::Renderer(_) => {
                        PublicationSourceValidationError::RendererUnavailable
                    }
                })?;
            let document = String::from_utf8(document)
                .map_err(|_| PublicationSourceValidationError::InvalidSource)?;
            state
                .thumbnail_capture
                .capture(document, draft.source_record.sha256)
                .await
                .map(Some)
                .map_err(|_| PublicationSourceValidationError::ThumbnailUnavailable)
        }
    }
}

fn publication_source_error(error: PublicationSourceValidationError) -> Response {
    match error {
        PublicationSourceValidationError::InvalidBinding => private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question source binding is invalid for publication",
        ),
        PublicationSourceValidationError::MissingWebworkType => private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "WebWork Draft Question requires a saved Question Type before publication",
        ),
        PublicationSourceValidationError::InvalidSource => private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question cannot be published",
        ),
        PublicationSourceValidationError::NativeTypeMismatch => private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question type declaration does not match its PLE source",
        ),
        PublicationSourceValidationError::ThumbnailUnavailable => private_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question thumbnail capture is unavailable; the Draft remains editable",
        ),
        PublicationSourceValidationError::RendererUnavailable => private_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "WebWork source validation is unavailable",
        ),
    }
}

pub(super) async fn publish_draft(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
    Json(request): Json<PublishDraftRequest>,
) -> Response {
    let deadline = Instant::now() + PUBLICATION_WORK_BOUND;
    match tokio::time::timeout_at(
        deadline + PUBLICATION_DRAIN_BOUND,
        publish_draft_inner(
            State(state),
            headers,
            Path(draft_question_id),
            Json(request),
            deadline,
        ),
    )
    .await
    {
        Ok(response) => response,
        Err(_) => private_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Publication did not complete",
        ),
    }
}

async fn publish_draft_inner(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
    Json(request): Json<PublishDraftRequest>,
    deadline: Instant,
) -> Response {
    // ASVS 2.2.1/2.2.2: canonical UUID input is checked at the trusted boundary.
    let classification = (|| {
        let discipline = canonical_classification_uuid(&request.discipline_uuid)?;
        let subject = canonical_classification_uuid(&request.subject_uuid)?;
        let topic = request
            .topic_uuid
            .as_deref()
            .map(canonical_classification_uuid)
            .transpose()?;
        let subtopic = request
            .subtopic_uuid
            .as_deref()
            .map(canonical_classification_uuid)
            .transpose()?;
        Ok::<_, ()>((discipline, subject, topic, subtopic))
    })();
    let Ok((discipline_uuid, subject_uuid, topic_uuid, subtopic_uuid)) = classification else {
        return private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Question classification is invalid",
        );
    };
    let draft_question_uuid = match parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(&headers) {
        Ok(number) => number,
        Err(response) => return *response,
    };
    let session_hash = match authoring_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let draft = match state
        .drafts
        .load_authoring_draft(session_hash, draft_question_uuid)
        .await
    {
        Ok(draft) => draft,
        Err(error) => return private_store_error(error),
    };
    // ASVS 2.2.1/2.2.2 and 8.2.3: a reviewed name position is the only
    // browser-selected link; the database supplies the current Account ID.
    if let Some(position) = request.self_author_position {
        if position >= request.authors.len() || i32::try_from(position).is_err() {
            return private_error(StatusCode::BAD_REQUEST, "Self author position is invalid");
        }
        if draft.parent_published_question_revision_tuple.is_some() {
            return private_error(
                StatusCode::BAD_REQUEST,
                "Question Fork Publication preserves source authorship",
            );
        }
    }
    let authorship = match question_authorship(request.authors) {
        Ok(authorship) => authorship,
        Err(()) => {
            return private_error(
                StatusCode::BAD_REQUEST,
                "Question Publication requires reviewed authors",
            );
        }
    };
    let bytes = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        load_verified_source(&state.publication_objects, &draft),
    )
    .await
    .unwrap_or(Err(()))
    {
        Ok(bytes) => bytes,
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring storage is unavailable",
            );
        }
    };
    let question_thumbnail = match tokio::time::timeout_at(
        deadline,
        validate_publication_source(&state, &draft, &bytes),
    )
    .await
    {
        Ok(Ok(thumbnail)) => thumbnail,
        Ok(Err(error)) => return publication_source_error(error),
        Err(_) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Question Publication preparation timed out",
            );
        }
    };
    let Some(question_license) = draft.metadata.question_license.clone() else {
        return private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question requires a Question License before publication",
        );
    };
    if draft.metadata.validate_question_title().is_err()
        || draft.metadata.validate_question_description().is_err()
    {
        return private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question requires a Title and Description before publication",
        );
    }
    let command = NewQuestionLineagePublicationCommand {
        question_thumbnail,
        draft_question_uuid: draft.draft_question_uuid,
        expected_draft_question_edit_number: expected_edit_number,
        workspace: draft.workspace,
        question_authorship: authorship,
        self_author_position: request.self_author_position,
        initial_shared_tags: draft.metadata.tags.clone(),
        discipline_uuid,
        subject_uuid,
        topic_uuid,
        subtopic_uuid,
        question_license,
        question_revision_reason: QuestionRevisionReason::new(
            INITIAL_PUBLICATION_REASON.to_string(),
        )
        .expect("fixed initial publication reason is valid"),
    };
    let publisher = NewQuestionLineagePublisher::new(
        state.publication_objects.clone(),
        state.publication.clone(),
        state.question_id_issuer,
        Some(DraftQuestionImageContext {
            store: state.draft_question_images.clone(),
            draft_question_uuid,
        }),
    );
    match publisher
        .publish(session_hash, command, now(), deadline)
        .await
    {
        Ok(revision) => crate::auth::no_store(
            Json(PublishedDraftResponse {
                question_id: revision.published_question_id.to_string(),
            })
            .into_response(),
        ),
        Err(error) => publication_error(error),
    }
}

pub(super) async fn publish_revision_draft(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
    Json(request): Json<PublishRevisionDraftRequest>,
) -> Response {
    let deadline = Instant::now() + PUBLICATION_WORK_BOUND;
    match tokio::time::timeout_at(
        deadline + PUBLICATION_DRAIN_BOUND,
        publish_revision_draft_inner(
            State(state),
            headers,
            Path(draft_question_id),
            Json(request),
            deadline,
        ),
    )
    .await
    {
        Ok(response) => response,
        Err(_) => private_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Question Publication did not complete",
        ),
    }
}

async fn publish_revision_draft_inner(
    State(state): State<AuthoringRouteState>,
    headers: HeaderMap,
    Path(draft_question_id): Path<String>,
    Json(request): Json<PublishRevisionDraftRequest>,
    deadline: Instant,
) -> Response {
    let draft_question_uuid = match parse_draft_question_uuid(&draft_question_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let expected_edit_number = match expected_edit_number(&headers) {
        Ok(number) => number,
        Err(response) => return *response,
    };
    let parent_published_question_revision_tuple =
        match existing_parent_published_question_revision_tuple(
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
    let session_hash = match authoring_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let draft = match state
        .drafts
        .load_authoring_draft(session_hash, draft_question_uuid)
        .await
    {
        Ok(draft) => draft,
        Err(error) => return private_store_error(error),
    };
    let bytes = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        load_verified_source(&state.publication_objects, &draft),
    )
    .await
    .unwrap_or(Err(()))
    {
        Ok(bytes) => bytes,
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring storage is unavailable",
            );
        }
    };
    let question_thumbnail = match tokio::time::timeout_at(
        deadline,
        validate_publication_source(&state, &draft, &bytes),
    )
    .await
    {
        Ok(Ok(thumbnail)) => thumbnail,
        Ok(Err(error)) => return publication_source_error(error),
        Err(_) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Question Publication preparation timed out",
            );
        }
    };
    if let Err(message) = validate_revision_publication_metadata(&draft.metadata) {
        return private_error(StatusCode::UNPROCESSABLE_ENTITY, message);
    }
    let publisher = ExistingQuestionRevisionPublisher::new(
        state.publication_objects.clone(),
        state.publication.clone(),
        Some(DraftQuestionImageContext {
            store: state.draft_question_images.clone(),
            draft_question_uuid,
        }),
    );
    match publisher
        .publish(
            session_hash,
            ExistingQuestionRevisionPublicationCommand {
                question_thumbnail,
                draft_question_uuid: draft.draft_question_uuid,
                expected_draft_question_edit_number: expected_edit_number,
                workspace: draft.workspace,
                parent_published_question_revision_tuple,
                question_revision_reason: revision_reason,
            },
            now(),
            deadline,
        )
        .await
    {
        Ok(published_question_revision_tuple) => crate::auth::no_store(
            Json(PublishedRevisionDraftResponse {
                published_question_revision_tuple,
            })
            .into_response(),
        ),
        Err(error) => publication_error(error),
    }
}

fn validate_revision_publication_metadata(metadata: &QuestionMetadata) -> Result<(), &'static str> {
    if metadata.question_license.is_none() {
        return Err("Draft Question requires a Question License before publication");
    }
    if metadata.validate_question_title().is_err()
        || metadata.validate_question_description().is_err()
    {
        return Err("Draft Question requires a Title and Description before publication");
    }
    Ok(())
}
