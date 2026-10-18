//! Verified Draft image loading and immutable revision preparation.

use std::collections::BTreeSet;

use axum::body::Bytes;
use learning_data_access::{
    DraftQuestionUuid, OwnedDraftQuestionImage, PreparedQuestionImagePublication, StoreError,
};
use objects::{ObjectAddress, ObjectStore, PutObject};
use question_model::{
    ObjectId, PublishedQuestionRevisionTuple, QuestionHtmlImageBinding, QuestionImageAssetTuple,
    QuestionResponseFormat, Timestamp, WorkspaceId,
};
use uuid::Uuid;

use crate::{
    draft_question_images::{require_surface, verified_question_image_bytes},
    question_publication::{DraftQuestionImageContext, QuestionPublicationError},
};

/// Resolve every native image through the authoritative compiler and Draft owner.
pub(crate) async fn load_question_images<O: ObjectStore>(
    objects: &O,
    context: Option<&DraftQuestionImageContext>,
    session_hash: learning_data_access::SessionTokenHash,
    workspace: WorkspaceId,
    draft: DraftQuestionUuid,
    source: &[u8],
    media_type: &str,
) -> Result<Vec<(OwnedDraftQuestionImage, Bytes)>, QuestionPublicationError> {
    if media_type != "application/vnd.peptidyle.question+json" {
        return Ok(Vec::new());
    }
    let document = adapter_ple::question_json::PleQuestionJsonDocument::parse(source)
        .map_err(|_| invalid_source())?;
    let names = document
        .relative_image_names()
        .map_err(|_| invalid_source())?;
    let mut assets = Vec::new();
    if !names.is_empty() {
        let context = exact_context(context, draft)?;
        let named = context
            .store
            .load_named_draft_question_images(session_hash, draft)
            .await
            .map_err(QuestionPublicationError::Store)?;
        for name in &names {
            let asset = named
                .iter()
                .find(|asset| asset.source_relative_name.as_ref() == Some(name))
                .ok_or_else(invalid_source)?;
            assets.push(asset.clone());
        }
    }
    let bindings = assets
        .iter()
        .map(|asset| QuestionHtmlImageBinding {
            source_relative_name: asset
                .source_relative_name
                .clone()
                .expect("named asset matched above"),
            question_image_asset_tuple: QuestionImageAssetTuple {
                question_image_asset_id: asset.question_image_asset_id,
                checksum: asset.source_record.sha256.to_string(),
            },
        })
        .collect::<Vec<_>>();
    let compiled = document
        .compile_with_images(&bindings)
        .map_err(|_| invalid_source())?;
    if let QuestionResponseFormat::Hotspot {
        question_image_asset_tuple,
        ..
    } = compiled.presentation().response()
    {
        let context = exact_context(context, draft)?;
        let asset = require_surface(
            context.store.as_ref(),
            session_hash,
            draft,
            question_image_asset_tuple,
        )
        .await
        .map_err(QuestionPublicationError::Store)?;
        if !assets
            .iter()
            .any(|item| item.question_image_asset_id == asset.question_image_asset_id)
        {
            assets.push(asset);
        }
    }
    let mut verified = Vec::with_capacity(assets.len());
    for asset in assets {
        // ASVS 8.2.2/8.3.1: each logical asset and physical object share this exact owner.
        if !matches!(&asset.source_record.address,
            ObjectAddress::DraftQuestionImage { workspace_id: owner, draft_question_id: id, question_image_asset_id: image, object_id: object }
            if *owner == workspace && *id == draft.as_uuid() && *image == asset.question_image_asset_id
                && *object == asset.source_record.id)
        {
            return Err(QuestionPublicationError::SourceObjectRecordMismatch);
        }
        // ASVS 5.2.2/5.2.6: repeat full raster and checksum verification at publication.
        let bytes = verified_question_image_bytes(objects, &asset)
            .await
            .map_err(|()| QuestionPublicationError::SourceObjectRecordMismatch)?;
        verified.push((asset, bytes));
    }
    Ok(verified)
}

fn exact_context(
    context: Option<&DraftQuestionImageContext>,
    draft: DraftQuestionUuid,
) -> Result<&DraftQuestionImageContext, QuestionPublicationError> {
    context
        .filter(|context| context.draft_question_uuid == draft)
        .ok_or_else(invalid_source)
}

fn invalid_source() -> QuestionPublicationError {
    QuestionPublicationError::Store(StoreError::InvalidRecord(
        "Native Question source or Draft-owned image is invalid".into(),
    ))
}

pub(crate) async fn prepare_question_images<O: ObjectStore>(
    objects: &O,
    assets: &[(OwnedDraftQuestionImage, Bytes)],
    published_question_revision_tuple: &PublishedQuestionRevisionTuple,
    stored_at: Timestamp,
    targets: &mut Vec<ObjectAddress>,
) -> Result<Vec<PreparedQuestionImagePublication>, QuestionPublicationError> {
    let mut prepared = Vec::with_capacity(assets.len());
    let mut identities = BTreeSet::new();
    for (asset, bytes) in assets {
        if !identities.insert(asset.question_image_asset_id) {
            return Err(invalid_source());
        }
        // ASVS 5.3.2: new physical identity; stable logical image UUID and exact bytes.
        let record = crate::question_publication_objects::put_candidate(
            objects,
            targets,
            PutObject {
                address: ObjectAddress::RestrictedQuestionImage {
                    published_question_revision_tuple: published_question_revision_tuple.clone(),
                    question_image_asset_id: asset.question_image_asset_id,
                    object_id: ObjectId::generate(),
                },
                bytes: bytes.to_vec(),
                media_type: asset.source_record.media_type.clone(),
                created_at: stored_at,
            },
        )
        .await?;
        prepared.push(PreparedQuestionImagePublication {
            source_relative_name: asset.source_relative_name.clone(),
            question_image_asset_id: asset.question_image_asset_id,
            restricted_source_record: record,
            public_object_id: ObjectId::generate(),
            intrinsic_width: asset.intrinsic_width,
            intrinsic_height: asset.intrinsic_height,
            delivery_id: Uuid::now_v7(),
            job_id: Uuid::now_v7(),
        });
    }
    Ok(prepared)
}

/// Only call for conclusive transaction rollback, never an ambiguous store outcome.
pub(crate) async fn cleanup_targets<O: ObjectStore>(
    objects: &O,
    targets: &[ObjectAddress],
) -> Result<(), QuestionPublicationError> {
    // ASVS 2.3.3: cleanup is bounded and names only this request's candidates.
    let cleanup = async {
        let mut result = Ok(());
        for target in targets {
            let deleted = match objects.delete(target).await {
                Err(objects::ObjectStoreError::NotFound) => Ok(()),
                other => other,
            };
            result = result.and(deleted);
        }
        result.map_err(QuestionPublicationError::ObjectStore)
    };
    tokio::time::timeout(std::time::Duration::from_secs(3), cleanup)
        .await
        .map_err(|_| {
            QuestionPublicationError::ObjectStore(objects::ObjectStoreError::Unavailable(
                "Publication candidate cleanup timed out".into(),
            ))
        })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use async_trait::async_trait;
    use learning_data_access::{
        DraftQuestionImageStore, RegisterDraftQuestionImageInput, SessionTokenHash,
    };
    use objects::memory::MemoryObjectStore;
    use question_model::{QuestionImageAssetId, QuestionRevisionNumber};

    struct DraftImages(Vec<OwnedDraftQuestionImage>);

    #[async_trait]
    impl DraftQuestionImageStore for DraftImages {
        async fn register_draft_question_image(
            &self,
            _: SessionTokenHash,
            _: RegisterDraftQuestionImageInput,
        ) -> Result<OwnedDraftQuestionImage, StoreError> {
            unreachable!("publication never uploads")
        }
        async fn load_named_draft_question_images(
            &self,
            _: SessionTokenHash,
            _: DraftQuestionUuid,
        ) -> Result<Vec<OwnedDraftQuestionImage>, StoreError> {
            Ok(self.0.clone())
        }
        async fn load_draft_question_image(
            &self,
            _: SessionTokenHash,
            _: DraftQuestionUuid,
            id: QuestionImageAssetId,
        ) -> Result<OwnedDraftQuestionImage, StoreError> {
            self.0
                .iter()
                .find(|image| image.question_image_asset_id == id)
                .cloned()
                .ok_or(StoreError::NotFound)
        }
    }

    async fn image(
        objects: &MemoryObjectStore,
        workspace: WorkspaceId,
        draft: DraftQuestionUuid,
        name: &str,
    ) -> OwnedDraftQuestionImage {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 3)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let question_image_asset_id = QuestionImageAssetId::generate();
        let source_record = objects
            .put(PutObject {
                address: ObjectAddress::DraftQuestionImage {
                    workspace_id: workspace,
                    draft_question_id: draft.as_uuid(),
                    question_image_asset_id,
                    object_id: ObjectId::generate(),
                },
                bytes: bytes.into_inner(),
                media_type: "image/png".into(),
                created_at: Timestamp::from_unix_millis(1),
            })
            .await
            .unwrap();
        OwnedDraftQuestionImage {
            question_image_asset_id,
            source_relative_name: Some(name.into()),
            source_record,
            intrinsic_width: 2,
            intrinsic_height: 3,
        }
    }

    const SOURCE: &[u8] = br#"{"format":"pleQuestionJson","prompt":"<img src='images/diagram.png' alt='Diagram'>","response":{"kind":"singleChoice","choices":[{"id":"a","text":"A","feedback":null},{"id":"b","text":"B","feedback":null}],"correctChoice":"a","randomizeChoices":false}}"#;
    const MEDIA: &str = "application/vnd.peptidyle.question+json";

    #[tokio::test]
    async fn named_publication_copies_only_referenced_images_with_exact_name_and_bytes() {
        let objects = MemoryObjectStore::default();
        let workspace = WorkspaceId::from_uuid(Uuid::now_v7());
        let draft = DraftQuestionUuid::from_uuid(Uuid::now_v7());
        let referenced = image(&objects, workspace, draft, "images/diagram.png").await;
        let unused = image(&objects, workspace, draft, "unused.png").await;
        let context = DraftQuestionImageContext {
            store: Arc::new(DraftImages(vec![referenced.clone(), unused])),
            draft_question_uuid: draft,
        };
        let loaded = load_question_images(
            &objects,
            Some(&context),
            SessionTokenHash::compute(&[1; 32]),
            workspace,
            draft,
            SOURCE,
            MEDIA,
        )
        .await
        .unwrap();
        assert_eq!(loaded.len(), 1);
        let revision = PublishedQuestionRevisionTuple {
            published_question_id: question_model::PublishedQuestionId::from_random_identifier(
                "1234567",
            )
            .unwrap(),
            revision_number: QuestionRevisionNumber::new(1).unwrap(),
        };
        let prepared = prepare_question_images(
            &objects,
            &loaded,
            &revision,
            Timestamp::from_unix_millis(2),
            &mut Vec::new(),
        )
        .await
        .unwrap();
        assert_eq!(prepared.len(), 1);
        assert_eq!(
            prepared[0].source_relative_name,
            referenced.source_relative_name
        );
        assert_eq!(
            prepared[0].question_image_asset_id,
            referenced.question_image_asset_id
        );
        assert_eq!(
            prepared[0].restricted_source_record.sha256,
            referenced.source_record.sha256
        );
        assert_eq!(
            objects
                .get(&prepared[0].restricted_source_record.address)
                .await
                .unwrap()
                .bytes,
            loaded[0].1.to_vec()
        );
        assert_ne!(
            prepared[0].restricted_source_record.id,
            referenced.source_record.id
        );
    }

    #[tokio::test]
    async fn named_publication_rejects_other_draft_owner_and_incorrect_image_evidence() {
        let objects = MemoryObjectStore::default();
        let workspace = WorkspaceId::from_uuid(Uuid::now_v7());
        let draft = DraftQuestionUuid::from_uuid(Uuid::now_v7());
        let other = DraftQuestionUuid::from_uuid(Uuid::now_v7());
        let wrong_draft = image(&objects, workspace, other, "images/diagram.png").await;
        let wrong_workspace = image(
            &objects,
            WorkspaceId::from_uuid(Uuid::now_v7()),
            draft,
            "images/diagram.png",
        )
        .await;
        let mut wrong_facts = image(&objects, workspace, draft, "images/diagram.png").await;
        wrong_facts.intrinsic_width = 3;
        for asset in [wrong_draft, wrong_workspace, wrong_facts] {
            let context = DraftQuestionImageContext {
                store: Arc::new(DraftImages(vec![asset])),
                draft_question_uuid: draft,
            };
            assert!(matches!(
                load_question_images(
                    &objects,
                    Some(&context),
                    SessionTokenHash::compute(&[1; 32]),
                    workspace,
                    draft,
                    SOURCE,
                    MEDIA
                )
                .await,
                Err(QuestionPublicationError::SourceObjectRecordMismatch)
            ));
        }
        let context = DraftQuestionImageContext {
            store: Arc::new(DraftImages(Vec::new())),
            draft_question_uuid: draft,
        };
        assert!(
            load_question_images(
                &objects,
                Some(&context),
                SessionTokenHash::compute(&[1; 32]),
                workspace,
                draft,
                SOURCE,
                MEDIA
            )
            .await
            .is_err()
        );
    }
}
