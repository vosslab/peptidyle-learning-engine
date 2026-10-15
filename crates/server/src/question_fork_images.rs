//! Exact image evidence and object copying shared by fork and correction Drafts.

use learning_data_access::{
    ForkPublishedQuestionImageInput, PublishedQuestionForkImage, QuestionForkStore,
    SessionTokenHash,
};
use objects::{
    ObjectAddress, ObjectStore, ObjectStoreError, PutObject, image_validation::verify_still_image,
};
use question_model::{
    ObjectId, PublishedQuestionRevisionTuple, QuestionBackend, QuestionHtmlImageBinding,
    QuestionImageAssetTuple, QuestionResponseFormat, WorkspaceId,
};
use uuid::Uuid;

pub(crate) struct VerifiedForkImage {
    evidence: PublishedQuestionForkImage,
    bytes: Vec<u8>,
}

pub(crate) async fn load_question_images<O: ObjectStore>(
    forks: &dyn QuestionForkStore,
    objects: &O,
    session: SessionTokenHash,
    backend: QuestionBackend,
    revision: &PublishedQuestionRevisionTuple,
    source: &[u8],
) -> Result<Vec<VerifiedForkImage>, ()> {
    if backend != QuestionBackend::Ple {
        return Ok(Vec::new());
    }
    let evidence = forks
        .load_published_question_fork_assets(session, revision)
        .await
        .map_err(|_| ())?;
    let bindings = evidence
        .iter()
        .filter_map(|image| {
            image
                .source_relative_name
                .as_ref()
                .map(|name| QuestionHtmlImageBinding {
                    source_relative_name: name.clone(),
                    question_image_asset_tuple: QuestionImageAssetTuple {
                        question_image_asset_id: image.question_image_asset_id,
                        checksum: image.source_record.sha256.to_string(),
                    },
                })
        })
        .collect::<Vec<_>>();
    let document =
        adapter_ple::question_json::PleQuestionJsonDocument::parse(source).map_err(|_| ())?;
    let compiled = document.compile_with_images(&bindings).map_err(|_| ())?;
    if let QuestionResponseFormat::Hotspot {
        question_image_asset_tuple,
        ..
    } = compiled.presentation().response()
        && !evidence.iter().any(|image| {
            image.question_image_asset_id == question_image_asset_tuple.question_image_asset_id
                && image.source_record.sha256.to_string() == question_image_asset_tuple.checksum
        })
    {
        return Err(());
    }
    let mut images = Vec::with_capacity(evidence.len());
    for image in evidence {
        let stored = objects
            .get(&image.source_record.address)
            .await
            .map_err(|_| ())?;
        // ASVS 8.3.1: trusted evidence selects immutable objects; source HTML never selects storage.
        if stored.record != image.source_record {
            return Err(());
        }
        let verified = verify_still_image(&stored.bytes).map_err(|_| ())?;
        if verified.media_type.canonical_media_type() != image.source_record.media_type
            || verified.width != image.intrinsic_width
            || verified.height != image.intrinsic_height
        {
            return Err(());
        }
        images.push(VerifiedForkImage {
            evidence: image,
            bytes: stored.bytes,
        });
    }
    Ok(images)
}

pub(crate) async fn copy_question_images<O: ObjectStore>(
    objects: &O,
    workspace: WorkspaceId,
    draft: Uuid,
    images: &[VerifiedForkImage],
) -> Result<Vec<ForkPublishedQuestionImageInput>, ObjectStoreError> {
    let mut copied: Vec<ForkPublishedQuestionImageInput> = Vec::with_capacity(images.len());
    for image in images {
        let record = objects
            .put(PutObject {
                address: ObjectAddress::DraftQuestionImage {
                    workspace_id: workspace,
                    draft_question_id: draft,
                    question_image_asset_id: image.evidence.question_image_asset_id,
                    object_id: ObjectId::generate(),
                },
                bytes: image.bytes.clone(),
                media_type: image.evidence.source_record.media_type.clone(),
                created_at: crate::authoring::now(),
            })
            .await;
        match record {
            Ok(target_record) => copied.push(ForkPublishedQuestionImageInput {
                question_image_asset_id: image.evidence.question_image_asset_id,
                source_relative_name: image.evidence.source_relative_name.clone(),
                target_record,
                intrinsic_width: image.evidence.intrinsic_width,
                intrinsic_height: image.evidence.intrinsic_height,
            }),
            Err(error) => {
                for image in &copied {
                    let _ = objects.delete(&image.target_record.address).await;
                }
                return Err(error);
            }
        }
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use learning_data_access::{
        ForkPublishedQuestionInput, ForkedPublishedQuestionDraft, StoreError,
    };
    use objects::{ObjectRecord, SignedUrl, StoredObject, memory::MemoryObjectStore};
    use question_model::{QuestionImageAssetId, QuestionRevisionNumber, Timestamp};
    use std::sync::Mutex;

    struct ForkEvidence(Vec<PublishedQuestionForkImage>);

    #[async_trait]
    impl QuestionForkStore for ForkEvidence {
        async fn load_published_question_fork_assets(
            &self,
            _: SessionTokenHash,
            _: &PublishedQuestionRevisionTuple,
        ) -> Result<Vec<PublishedQuestionForkImage>, StoreError> {
            Ok(self.0.clone())
        }
        async fn fork_published_question_to_draft(
            &self,
            _: SessionTokenHash,
            _: ForkPublishedQuestionInput,
        ) -> Result<ForkedPublishedQuestionDraft, StoreError> {
            unreachable!("image evidence test never creates a Draft")
        }
    }

    fn revision() -> PublishedQuestionRevisionTuple {
        PublishedQuestionRevisionTuple {
            published_question_id: question_model::PublishedQuestionId::from_random_identifier(
                "1234567",
            )
            .unwrap(),
            revision_number: QuestionRevisionNumber::new(1).unwrap(),
        }
    }

    async fn evidence(objects: &MemoryObjectStore, name: &str) -> PublishedQuestionForkImage {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 3)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let question_image_asset_id = QuestionImageAssetId::from_uuid(Uuid::now_v7());
        let source_record = objects
            .put(PutObject {
                address: ObjectAddress::RestrictedQuestionImage {
                    published_question_revision_tuple: revision(),
                    question_image_asset_id,
                    object_id: ObjectId::generate(),
                },
                bytes: bytes.into_inner(),
                media_type: "image/png".into(),
                created_at: Timestamp::from_unix_millis(1),
            })
            .await
            .unwrap();
        PublishedQuestionForkImage {
            question_image_asset_id,
            source_relative_name: Some(name.into()),
            source_record,
            intrinsic_width: 2,
            intrinsic_height: 3,
        }
    }

    #[tokio::test]
    async fn inline_images_keep_source_names_tuples_and_bytes_in_new_draft() {
        let objects = MemoryObjectStore::default();
        let images = vec![
            evidence(&objects, "images/prompt.png").await,
            evidence(&objects, "choice.png").await,
        ];
        let forks = ForkEvidence(images.clone());
        let source = br#"{"format":"pleQuestionJson","prompt":"<img src='images/prompt.png' alt='Prompt'>","response":{"kind":"singleChoice","choices":[{"id":"a","text":"<img src='choice.png' alt='Choice'>","feedback":null},{"id":"b","text":"B","feedback":null}],"correctChoice":"a","randomizeChoices":false}}"#;
        let token = SessionTokenHash::compute(&[1; 32]);
        let verified = load_question_images(
            &forks,
            &objects,
            token,
            QuestionBackend::Ple,
            &revision(),
            source,
        )
        .await
        .unwrap();
        let workspace = WorkspaceId::from_uuid(Uuid::now_v7());
        let draft = Uuid::now_v7();
        let copies = copy_question_images(&objects, workspace, draft, &verified)
            .await
            .unwrap();
        assert_eq!(copies.len(), 2);
        for (copied, original) in copies.iter().zip(&images) {
            assert_eq!(
                copied.question_image_asset_id,
                original.question_image_asset_id
            );
            assert_eq!(copied.source_relative_name, original.source_relative_name);
            assert_eq!(copied.target_record.sha256, original.source_record.sha256);
            assert_ne!(copied.target_record.id, original.source_record.id);
            assert_eq!(
                copied.target_record.address,
                ObjectAddress::DraftQuestionImage {
                    workspace_id: workspace,
                    draft_question_id: draft,
                    question_image_asset_id: original.question_image_asset_id,
                    object_id: copied.target_record.id,
                }
            );
            assert_eq!(
                objects
                    .get(&copied.target_record.address)
                    .await
                    .unwrap()
                    .bytes,
                objects
                    .get(&original.source_record.address)
                    .await
                    .unwrap()
                    .bytes
            );
        }
        let target_source_record = objects
            .put(PutObject {
                address: ObjectAddress::WorkspaceQuestionSource {
                    workspace_id: workspace,
                    object_id: ObjectId::generate(),
                },
                bytes: source.to_vec(),
                media_type: adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE.into(),
                created_at: Timestamp::from_unix_millis(2),
            })
            .await
            .unwrap();
        let mut input = ForkPublishedQuestionInput {
            source_published_question_revision_tuple: revision(),
            workspace,
            proposed_draft_question_id: draft,
            target_source_record,
            question_images: copies,
            idempotency_key: Uuid::now_v7(),
        };
        input.validate().unwrap();
        input.question_images[1].target_record.address = ObjectAddress::DraftQuestionImage {
            workspace_id: WorkspaceId::from_uuid(Uuid::now_v7()),
            draft_question_id: draft,
            question_image_asset_id: input.question_images[1].question_image_asset_id,
            object_id: input.question_images[1].target_record.id,
        };
        assert!(input.validate().is_err());
        let wrong_bindings = ForkEvidence(vec![images[0].clone()]);
        assert!(
            load_question_images(
                &wrong_bindings,
                &objects,
                token,
                QuestionBackend::Ple,
                &revision(),
                source
            )
            .await
            .is_err()
        );
    }

    struct FailSecondCopy {
        inner: MemoryObjectStore,
        written: Mutex<Vec<ObjectAddress>>,
    }

    #[async_trait]
    impl ObjectStore for FailSecondCopy {
        async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
            {
                let written = self.written.lock().unwrap();
                if !written.is_empty() {
                    return Err(ObjectStoreError::Unavailable("injected failure".into()));
                }
            }
            let record = self.inner.put(request).await?;
            self.written.lock().unwrap().push(record.address.clone());
            Ok(record)
        }
        async fn get(&self, address: &ObjectAddress) -> Result<StoredObject, ObjectStoreError> {
            self.inner.get(address).await
        }
        async fn delete(&self, address: &ObjectAddress) -> Result<(), ObjectStoreError> {
            self.inner.delete(address).await
        }
        async fn signed_url(
            &self,
            address: &ObjectAddress,
            now: Timestamp,
        ) -> Result<SignedUrl, ObjectStoreError> {
            self.inner.signed_url(address, now).await
        }
    }

    #[tokio::test]
    async fn failed_copy_removes_prior_candidates_and_keeps_source_objects() {
        let inner = MemoryObjectStore::default();
        let images = vec![
            evidence(&inner, "one.png").await,
            evidence(&inner, "two.png").await,
        ];
        let mut verified = Vec::new();
        for image in &images {
            verified.push(VerifiedForkImage {
                evidence: image.clone(),
                bytes: inner.get(&image.source_record.address).await.unwrap().bytes,
            });
        }
        let objects = FailSecondCopy {
            inner,
            written: Mutex::new(Vec::new()),
        };
        assert!(
            copy_question_images(
                &objects,
                WorkspaceId::from_uuid(Uuid::now_v7()),
                Uuid::now_v7(),
                &verified
            )
            .await
            .is_err()
        );
        let address = objects.written.lock().unwrap()[0].clone();
        assert_eq!(objects.get(&address).await, Err(ObjectStoreError::NotFound));
        for image in images {
            assert!(objects.get(&image.source_record.address).await.is_ok());
        }
    }
}
