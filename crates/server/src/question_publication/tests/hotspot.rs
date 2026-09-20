//! The real compiler, raster verifier, typed objects, and coordinator rollback boundary.

use std::io::Cursor;

use image::{ImageBuffer, ImageFormat, Rgba};
use learning_data_access::{OwnedDraftQuestionAsset, RegisterDraftQuestionAssetInput};
use question_model::QuestionAssetId;

use super::*;

struct DraftImageStore(OwnedDraftQuestionAsset);

#[async_trait]
impl AuthoringAssetsStore for DraftImageStore {
    async fn register_draft_question_asset(
        &self,
        _: SessionTokenHash,
        _: RegisterDraftQuestionAssetInput,
    ) -> Result<OwnedDraftQuestionAsset, StoreError> {
        Err(StoreError::Forbidden)
    }

    async fn load_draft_question_asset(
        &self,
        session: SessionTokenHash,
        draft_question_uuid: DraftQuestionUuid,
        asset: QuestionAssetId,
    ) -> Result<OwnedDraftQuestionAsset, StoreError> {
        if session != SessionTokenHash::compute(b"session")
            || draft_question_uuid.as_uuid() != Uuid::from_u128(2)
            || asset != self.0.asset_id
        {
            return Err(StoreError::NotFound);
        }
        Ok(self.0.clone())
    }
}

async fn fixture(
    objects: &MemoryObjectStore,
    workspace: WorkspaceId,
) -> (ObjectRecord, OwnedDraftQuestionAsset, Vec<u8>) {
    let mut image = ImageBuffer::from_pixel(10, 10, Rgba([255_u8, 255, 255, 255]));
    image.put_pixel(5, 5, Rgba([0, 0, 0, 255]));
    let mut png = Cursor::new(Vec::new());
    image.write_to(&mut png, ImageFormat::Png).expect("dot PNG");
    let png = png.into_inner();
    let asset_id = QuestionAssetId::generate();
    let image_record = objects
        .put(PutObject {
            address: ObjectAddress::DraftQuestionAsset {
                workspace_id: workspace,
                draft_question_id: Uuid::from_u128(2),
                question_asset_id: asset_id,
                object_id: ObjectId::generate(),
            },
            bytes: png.clone(),
            media_type: "image/png".into(),
            created_at: Timestamp::from_unix_millis(1_000),
        })
        .await
        .expect("private Draft dot image");
    let source = serde_json::to_vec(&serde_json::json!({
        "format": "pleQuestionJson", "questionTitle": "Click the dot",
        "questionDescription": "Select the dot in the image.", "prompt": "Click the dot.",
        "language": "en", "response": { "kind": "hotspot",
            "surface": { "questionAssetId": asset_id, "checksum": image_record.sha256.to_string(),
                "description": "One black dot on white" },
            "regions": [{ "id": "dot", "label": "Dot", "x": 5000, "y": 5000,
                "width": 1000, "height": 1000 }], "correctRegions": ["dot"] }
    }))
    .expect("native source");
    let document = adapter_ple::question_json::PleQuestionJsonDocument::parse(&source)
        .expect("dot native source parses");
    let bytes = document
        .canonical_bytes()
        .expect("saved native source bytes");
    let source_record = objects
        .put(PutObject {
            address: ObjectAddress::WorkspaceQuestionSource {
                workspace_id: workspace,
                object_id: ObjectId::generate(),
            },
            bytes,
            media_type: crate::authoring::PLE_QUESTION_JSON_MEDIA_TYPE.into(),
            created_at: Timestamp::from_unix_millis(1_000),
        })
        .await
        .expect("saved Draft source");
    (
        source_record,
        OwnedDraftQuestionAsset {
            asset_id,
            source_record: image_record,
            intrinsic_width: 10,
            intrinsic_height: 10,
        },
        png,
    )
}

fn context(asset: OwnedDraftQuestionAsset) -> AuthoringAssetContext {
    AuthoringAssetContext {
        store: Arc::new(DraftImageStore(asset)),
        draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::from_u128(2)),
    }
}

#[tokio::test]
async fn hotspot_collision_retries_exact_bytes_and_cleans_both_rolled_back_targets() {
    let objects = MemoryObjectStore::default();
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let (source, asset, png) = fixture(&objects, workspace).await;
    let saved = objects
        .get(&source.address)
        .await
        .expect("saved source")
        .bytes;
    let draft_image_address = asset.source_record.address.clone();
    let (store, publications) = scripted_store(
        source,
        [
            Err(NewQuestionLineagePublicationError::IdentityCollision),
            Ok(()),
        ],
    );
    NewQuestionLineagePublisher::new(
        objects.clone(),
        store,
        fixed_issuer(&["0000000", "0000001"]),
        Some(context(asset)),
    )
    .publish(
        SessionTokenHash::compute(b"session"),
        command(workspace),
        Timestamp::from_unix_millis(2_000),
    )
    .await
    .expect("publish dot after collision");
    let publications = publications.lock().expect("captured transactions").clone();
    let failed = &publications[0];
    let accepted = &publications[1];
    let failed_image = failed
        .hotspot_asset
        .as_ref()
        .expect("prepared rolled-back image");
    let accepted_image = accepted
        .hotspot_asset
        .as_ref()
        .expect("prepared accepted image");
    assert_eq!(
        objects
            .get(&failed.question_source_object_record.address)
            .await,
        Err(ObjectStoreError::NotFound)
    );
    assert_eq!(
        objects
            .get(&failed_image.restricted_source_record.address)
            .await,
        Err(ObjectStoreError::NotFound)
    );
    assert_eq!(
        objects
            .get(&accepted.question_source_object_record.address)
            .await
            .expect("immutable accepted source")
            .bytes,
        saved
    );
    assert_eq!(
        objects
            .get(&accepted_image.restricted_source_record.address)
            .await
            .expect("restricted exact original image")
            .bytes,
        png
    );
    assert_eq!(
        objects
            .get(&draft_image_address)
            .await
            .expect("Draft image retained")
            .bytes,
        png
    );
    assert_ne!(
        failed_image.restricted_source_record.id,
        accepted_image.restricted_source_record.id
    );
    assert_eq!(failed_image.asset_id, accepted_image.asset_id);
    assert_eq!(accepted_image.intrinsic_width, 10);
    assert_eq!(accepted_image.intrinsic_height, 10);
    assert_eq!(
        objects
            .get(&ObjectAddress::QuestionAsset {
                question_revision_tuple: accepted.question_revision_tuple(),
                question_asset_id: accepted_image.asset_id,
                object_id: accepted_image.public_object_id
            })
            .await,
        Err(ObjectStoreError::NotFound)
    );
}

#[tokio::test]
async fn hotspot_successor_stale_removes_source_and_image_but_retains_private_draft_image() {
    let objects = MemoryObjectStore::default();
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let (source, asset, png) = fixture(&objects, workspace).await;
    let draft_image_address = asset.source_record.address.clone();
    let publications = Arc::new(Mutex::new(Vec::new()));
    let publisher = ExistingQuestionRevisionPublisher::new(
        objects.clone(),
        ExistingRevisionRecordingStore {
            source_record: source,
            publications: publications.clone(),
            outcome: Err(ExistingQuestionRevisionPublicationError::Stale),
        },
        Some(context(asset)),
    );
    assert_eq!(
        publisher
            .publish(
                SessionTokenHash::compute(b"session"),
                existing_command(workspace),
                Timestamp::from_unix_millis(2_000)
            )
            .await,
        Err(QuestionPublicationError::StaleQuestionRevision)
    );
    let publications = publications.lock().expect("captured transactions").clone();
    let failed = &publications[0];
    assert_eq!(
        objects
            .get(&failed.question_source_object_record.address)
            .await,
        Err(ObjectStoreError::NotFound)
    );
    assert_eq!(
        objects
            .get(
                &failed
                    .hotspot_asset
                    .as_ref()
                    .expect("prepared image")
                    .restricted_source_record
                    .address
            )
            .await,
        Err(ObjectStoreError::NotFound)
    );
    assert_eq!(
        objects
            .get(&draft_image_address)
            .await
            .expect("private original retained")
            .bytes,
        png
    );
}

#[tokio::test]
async fn native_hotspot_without_a_real_authoring_context_fails_before_any_publication() {
    let objects = MemoryObjectStore::default();
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let (source, _, _) = fixture(&objects, workspace).await;
    let (store, publications) = scripted_store(source, [Ok(())]);
    let result = NewQuestionLineagePublisher::new(objects, store, fixed_issuer(&["0000000"]), None)
        .publish(
            SessionTokenHash::compute(b"session"),
            command(workspace),
            Timestamp::from_unix_millis(2_000),
        )
        .await;
    assert!(matches!(
        result,
        Err(QuestionPublicationError::Store(StoreError::InvalidRecord(
            _
        )))
    ));
    assert!(
        publications
            .lock()
            .expect("captured transactions")
            .is_empty()
    );
}
