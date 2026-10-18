//! Exact candidate ownership before the atomic publication transaction begins.

use axum::body::Bytes;
use learning_data_access::{
    OwnedDraftQuestionImage, PreparedQuestionImagePublication, PreparedQuestionThumbnailPublication,
};
use objects::{
    ObjectAddress, ObjectRecord, ObjectStore, ObjectStoreError, PutObject, StoredObject,
};
use question_model::{ObjectId, PublishedQuestionRevisionTuple, Timestamp};
use tokio::time::Instant;

use crate::question_publication::{
    PreparedPublicationThumbnail, PublicationResponseSender, QuestionPublicationError,
    publication_active,
};

pub(crate) struct PreparedPublicationObjects {
    pub(crate) source: ObjectRecord,
    pub(crate) images: Vec<PreparedQuestionImagePublication>,
    pub(crate) thumbnail: Option<PreparedQuestionThumbnailPublication>,
    pub(crate) targets: Vec<ObjectAddress>,
}

/// Every address enters the candidate set before its put, including a put whose
/// transport response is lost. Registration has not begun, so cleanup is conclusive.
#[allow(clippy::too_many_arguments)] // exact source/artifact facts and request lifetime stay explicit
pub(crate) async fn prepare<O: ObjectStore>(
    objects: &O,
    source: &StoredObject,
    images: &[(OwnedDraftQuestionImage, Bytes)],
    thumbnail: Option<&PreparedPublicationThumbnail>,
    revision: &PublishedQuestionRevisionTuple,
    stored_at: Timestamp,
    response: &PublicationResponseSender,
    deadline: Instant,
) -> Result<PreparedPublicationObjects, QuestionPublicationError> {
    let mut identities = std::collections::BTreeSet::new();
    if images
        .iter()
        .any(|(image, _)| !identities.insert(image.question_image_asset_id))
        || thumbnail.is_some_and(|image| image.source_checksum != source.record.sha256)
    {
        return Err(QuestionPublicationError::SourceObjectRecordMismatch);
    }
    let mut targets = Vec::new();
    let prepared = async {
        publication_active(response, deadline, std::time::Duration::ZERO)?;
        let address = ObjectAddress::QuestionSource {
            published_question_revision_tuple: revision.clone(),
            object_id: ObjectId::generate(),
        };
        let record = put_candidate(
            objects,
            &mut targets,
            PutObject {
                address,
                bytes: source.bytes.clone(),
                media_type: source.record.media_type.clone(),
                created_at: stored_at,
            },
        )
        .await?;
        let mut prepared_images = Vec::with_capacity(images.len());
        for image in images {
            publication_active(response, deadline, std::time::Duration::ZERO)?;
            prepared_images.extend(
                crate::question_publication_images::prepare_question_images(
                    objects,
                    std::slice::from_ref(image),
                    revision,
                    stored_at,
                    &mut targets,
                )
                .await?,
            );
        }
        publication_active(response, deadline, std::time::Duration::ZERO)?;
        let prepared_thumbnail = if let Some(thumbnail) = thumbnail {
            let record = put_candidate(
                objects,
                &mut targets,
                PutObject {
                    address: ObjectAddress::QuestionThumbnail {
                        published_question_revision_tuple: revision.clone(),
                        object_id: ObjectId::generate(),
                    },
                    bytes: thumbnail.thumbnail.bytes().to_vec(),
                    media_type: "image/webp".into(),
                    created_at: stored_at,
                },
            )
            .await?;
            Some(PreparedQuestionThumbnailPublication {
                object_record: record,
                source_checksum: thumbnail.source_checksum,
                intrinsic_width: thumbnail.thumbnail.width(),
                intrinsic_height: thumbnail.thumbnail.height(),
            })
        } else {
            None
        };
        publication_active(
            response,
            deadline,
            crate::question_publication::REGISTRATION_BOUND,
        )?;
        Ok((record, prepared_images, prepared_thumbnail))
    }
    .await;
    match prepared {
        Ok((source, images, thumbnail)) => Ok(PreparedPublicationObjects {
            source,
            images,
            thumbnail,
            targets,
        }),
        Err(error) => {
            crate::question_publication_images::cleanup_targets(objects, &targets).await?;
            Err(error)
        }
    }
}

pub(crate) async fn put_candidate<O: ObjectStore>(
    objects: &O,
    targets: &mut Vec<ObjectAddress>,
    request: PutObject,
) -> Result<ObjectRecord, QuestionPublicationError> {
    targets.push(request.address.clone());
    let result = objects.put(request).await;
    if matches!(result, Err(ObjectStoreError::AlreadyExists)) {
        // An existing immutable object was never owned by this put.
        targets.pop();
    }
    result.map_err(QuestionPublicationError::ObjectStore)
}
