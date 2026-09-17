use async_trait::async_trait;
use learning_data_access::{
    BloomClassificationPreparationStore, BloomPreparationReceiptId, PrepareBloomClassificationInput,
};
use question_model::{BloomClassification, BloomCognitiveProcess, BloomKnowledgeDimension};

use super::*;

#[derive(Clone, Default)]
struct RecordingBloomPreparationStore {
    preparations: Arc<Mutex<Vec<PrepareBloomClassificationInput>>>,
}

#[derive(Default)]
struct RecordingBloomClassifier {
    classifications: Arc<Mutex<usize>>,
}

#[derive(Clone)]
struct PutCountingObjectStore {
    memory: MemoryObjectStore,
    puts: Arc<Mutex<usize>>,
}

#[async_trait]
impl crate::bloom_classification::BloomClassifier for RecordingBloomClassifier {
    async fn classify(
        &self,
        _candidate: &crate::bloom_classification::BloomClassificationCandidate,
    ) -> Result<
        crate::bloom_classification::BloomClassifierOutput,
        crate::bloom_classification::BloomClassificationError,
    > {
        *self
            .classifications
            .lock()
            .expect("classification capture lock") += 1;
        Ok(crate::bloom_classification::BloomClassifierOutput::new(
            BloomClassification {
                cognitive_process: BloomCognitiveProcess::Apply,
                knowledge_dimension: BloomKnowledgeDimension::ConceptualKnowledge,
            },
            crate::bloom_classification::BloomClassifierProvenance::new(
                "test".to_owned(),
                "fixed".to_owned(),
                "test-v1".to_owned(),
                std::time::Duration::ZERO,
            )
            .expect("safe fixed provenance"),
        ))
    }
}

#[async_trait]
impl BloomClassificationPreparationStore for RecordingBloomPreparationStore {
    async fn prepare_bloom_classification(
        &self,
        input: PrepareBloomClassificationInput,
    ) -> Result<BloomPreparationReceiptId, StoreError> {
        let mut preparations = self.preparations.lock().expect("preparation capture lock");
        preparations.push(input);
        Ok(BloomPreparationReceiptId::from_uuid(Uuid::from_u128(
            preparations.len() as u128,
        )))
    }
}

#[async_trait]
impl ObjectStore for PutCountingObjectStore {
    async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
        *self.puts.lock().expect("put capture lock") += 1;
        self.memory.put(request).await
    }

    async fn get(&self, address: &ObjectAddress) -> Result<StoredObject, ObjectStoreError> {
        self.memory.get(address).await
    }

    async fn delete(&self, address: &ObjectAddress) -> Result<(), ObjectStoreError> {
        self.memory.delete(address).await
    }

    async fn signed_url(
        &self,
        address: &ObjectAddress,
        now: Timestamp,
    ) -> Result<SignedUrl, ObjectStoreError> {
        self.memory.signed_url(address, now).await
    }
}

pub(super) fn bloom_preparation() -> Arc<crate::bloom_classification::BloomPublicationPreparation> {
    bloom_preparation_with_recorders().0
}

fn bloom_preparation_with_recorders() -> (
    Arc<crate::bloom_classification::BloomPublicationPreparation>,
    Arc<RecordingBloomPreparationStore>,
    Arc<RecordingBloomClassifier>,
) {
    let receipts = Arc::new(RecordingBloomPreparationStore::default());
    let classifier = Arc::new(RecordingBloomClassifier::default());
    (
        Arc::new(
            crate::bloom_classification::BloomPublicationPreparation::new(
                classifier.clone(),
                receipts.clone(),
            ),
        ),
        receipts,
        classifier,
    )
}

fn unconfigured_bloom_preparation() -> Arc<crate::bloom_classification::BloomPublicationPreparation>
{
    Arc::new(
        crate::bloom_classification::BloomPublicationPreparation::new(
            Arc::new(crate::bloom_classification::NotConfiguredBloomClassifier),
            Arc::new(RecordingBloomPreparationStore::default()),
        ),
    )
}

async fn source_fixture_with(
    object_store: &MemoryObjectStore,
    workspace: WorkspaceId,
    bytes: Vec<u8>,
    media_type: &str,
) -> ObjectRecord {
    let object = ObjectId::from_uuid(Uuid::from_u128(3));
    object_store
        .put(PutObject {
            address: ObjectAddress::WorkspaceQuestionSource { workspace, object },
            bytes,
            media_type: media_type.to_owned(),
            created_at: Timestamp::from_unix_millis(1_000),
        })
        .await
        .expect("source object")
}

async fn assert_unsupported_visual_evidence_stops_before_side_effects(
    source: Vec<u8>,
    media_type: &str,
) {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let memory = MemoryObjectStore::default();
    let source_record = source_fixture_with(&memory, workspace, source, media_type).await;
    let publications = Arc::new(Mutex::new(Vec::new()));
    let puts = Arc::new(Mutex::new(0));
    let (preparation, receipts, classifier) = bloom_preparation_with_recorders();
    let publisher = NewQuestionLineagePublisher::new(
        PutCountingObjectStore {
            memory,
            puts: Arc::clone(&puts),
        },
        RecordingPublicationStore {
            source_record,
            publications: Arc::clone(&publications),
        },
        fixed_issuer(&["0000000"]),
        preparation,
        None,
    );

    assert_eq!(
        publisher
            .publish(
                SessionTokenHash::compute(b"session"),
                command(workspace),
                Timestamp::from_unix_millis(2_000),
            )
            .await,
        Err(QuestionPublicationError::BloomClassification(
            crate::bloom_classification::BloomClassificationError::UnsupportedEvidence,
        ))
    );
    assert_eq!(*puts.lock().expect("put capture lock"), 0);
    assert!(
        publications
            .lock()
            .expect("publication capture lock")
            .is_empty()
    );
    assert!(
        receipts
            .preparations
            .lock()
            .expect("preparation capture lock")
            .is_empty()
    );
    assert_eq!(
        *classifier
            .classifications
            .lock()
            .expect("classification capture lock"),
        0
    );
}

#[tokio::test]
async fn native_external_image_is_rejected_before_classification_receipt_or_object_copy() {
    let source = serde_json::to_vec(&serde_json::json!({
        "format": "pleQuestionJson",
        "questionTitle": "Interpret the figure",
        "questionDescription": "A native visual question.",
        "prompt": "Use the referenced figure.",
        "response": {
            "kind": "singleChoice",
            "choices": [{"id": "a", "text": "A"}, {"id": "b", "text": "B"}],
            "correctChoice": "a"
        },
        "externalResources": [{"url": "https://example.edu/figure", "kind": "image"}],
        "language": "en"
    }))
    .expect("native source");

    assert_unsupported_visual_evidence_stops_before_side_effects(
        source,
        adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE,
    )
    .await;
}

#[tokio::test]
async fn webwork_image_dependency_is_rejected_before_classification_receipt_or_object_copy() {
    assert_unsupported_visual_evidence_stops_before_side_effects(
        br#"DOCUMENT(); BEGIN_TEXT @{ image("diagram.png") } END_TEXT ENDDOCUMENT();"#.to_vec(),
        "text/x-wework-pg",
    )
    .await;
}

#[tokio::test]
async fn webwork_inline_svg_is_rejected_case_insensitively() {
    assert_unsupported_visual_evidence_stops_before_side_effects(
        br#"DOCUMENT(); BEGIN_TEXT <SvG viewBox="0 0 10 10"></SvG> END_TEXT ENDDOCUMENT();"#
            .to_vec(),
        "text/x-wework-pg",
    )
    .await;
}

#[tokio::test]
async fn webwork_image_call_with_tab_or_newline_is_rejected() {
    for source in [
        b"DOCUMENT(); BEGIN_TEXT @{ IMAGE\t(\"figure\") } END_TEXT ENDDOCUMENT();".as_slice(),
        b"DOCUMENT(); BEGIN_TEXT @{ image\n (\"figure\") } END_TEXT ENDDOCUMENT();".as_slice(),
    ] {
        assert_unsupported_visual_evidence_stops_before_side_effects(
            source.to_vec(),
            "text/x-wework-pg",
        )
        .await;
    }
}

#[tokio::test]
async fn source_contained_nonvisual_webwork_generation_is_supported() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let object_store = MemoryObjectStore::default();
    let source_record = source_fixture_with(
        &object_store,
        workspace,
        br#"DOCUMENT(); $value = random(2, 9, 1); BEGIN_PGML What is [$value] + 1? END_PGML ENDDOCUMENT();"#
            .to_vec(),
        "text/x-wework-pg",
    )
    .await;
    let publications = Arc::new(Mutex::new(Vec::new()));
    let (preparation, receipts, classifier) = bloom_preparation_with_recorders();
    let publisher = NewQuestionLineagePublisher::new(
        object_store,
        RecordingPublicationStore {
            source_record,
            publications: Arc::clone(&publications),
        },
        fixed_issuer(&["0000000"]),
        preparation,
        None,
    );

    publisher
        .publish(
            SessionTokenHash::compute(b"session"),
            command(workspace),
            Timestamp::from_unix_millis(2_000),
        )
        .await
        .expect("nonvisual generated PG publishes without source execution");
    assert_eq!(
        publications.lock().expect("publication capture lock").len(),
        1
    );
    assert_eq!(
        receipts
            .preparations
            .lock()
            .expect("preparation capture lock")
            .len(),
        1
    );
    assert_eq!(
        *classifier
            .classifications
            .lock()
            .expect("classification capture lock"),
        1
    );
}

#[tokio::test]
async fn new_lineage_publication_refuses_an_unconfigured_classifier_before_copying_source() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let object_store = MemoryObjectStore::default();
    let source_record = source_fixture(&object_store, workspace).await;
    let publications = Arc::new(Mutex::new(Vec::new()));
    let publisher = NewQuestionLineagePublisher::new(
        object_store,
        RecordingPublicationStore {
            source_record,
            publications: Arc::clone(&publications),
        },
        RandomQuestionIdIssuer::new(),
        unconfigured_bloom_preparation(),
        None,
    );

    assert_eq!(
        publisher
            .publish(
                SessionTokenHash::compute(b"session"),
                command(workspace),
                Timestamp::from_unix_millis(2_000),
            )
            .await,
        Err(QuestionPublicationError::BloomClassification(
            crate::bloom_classification::BloomClassificationError::NotConfigured,
        ))
    );
    assert!(
        publications
            .lock()
            .expect("publication capture lock")
            .is_empty()
    );
}

#[tokio::test]
async fn revision_publication_refuses_an_unconfigured_classifier_before_copying_source() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let object_store = MemoryObjectStore::default();
    let source_record = source_fixture(&object_store, workspace).await;
    let publications = Arc::new(Mutex::new(Vec::new()));
    let publisher = ExistingQuestionRevisionPublisher::new(
        object_store,
        ExistingRevisionRecordingStore {
            source_record,
            publications: Arc::clone(&publications),
            outcome: Ok(()),
        },
        unconfigured_bloom_preparation(),
        None,
    );

    assert_eq!(
        publisher
            .publish(
                SessionTokenHash::compute(b"session"),
                existing_command(workspace),
                Timestamp::from_unix_millis(2_000),
            )
            .await,
        Err(QuestionPublicationError::BloomClassification(
            crate::bloom_classification::BloomClassificationError::NotConfigured,
        ))
    );
    assert!(
        publications
            .lock()
            .expect("publication capture lock")
            .is_empty()
    );
}

#[tokio::test]
async fn identity_collision_prepares_a_fresh_receipt_without_reclassifying() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let object_store = MemoryObjectStore::default();
    let source_record = source_fixture(&object_store, workspace).await;
    let (publication_store, publications) = scripted_store(
        source_record,
        [
            Err(NewQuestionLineagePublicationError::IdentityCollision),
            Ok(()),
        ],
    );
    let (preparation, receipt_recorder, classifier_recorder) = bloom_preparation_with_recorders();
    let publisher = NewQuestionLineagePublisher::new(
        object_store.clone(),
        publication_store,
        fixed_issuer(&["0000000", "0000001"]),
        preparation,
        None,
    );

    let published = publisher
        .publish(
            SessionTokenHash::compute(b"session"),
            command(workspace),
            Timestamp::from_unix_millis(2_000),
        )
        .await
        .expect("second exact Question ID should publish");
    let publications = publications
        .lock()
        .expect("publication capture lock")
        .clone();

    assert_eq!(publications.len(), 2);
    assert_eq!(published, publications[1].question_revision());
    assert_ne!(
        publications[0].bloom_preparation_receipt_id,
        publications[1].bloom_preparation_receipt_id
    );
    let preparations = receipt_recorder
        .preparations
        .lock()
        .expect("preparation capture lock")
        .clone();
    assert_eq!(preparations.len(), 2);
    assert_eq!(
        *classifier_recorder
            .classifications
            .lock()
            .expect("classification capture lock"),
        1,
        "identity collision must not call the model again"
    );
    assert!(preparations.iter().all(|preparation| {
        matches!(
            &preparation.candidate,
            learning_data_access::BloomPreparationCandidate::Question { source_checksum }
                if source_checksum == &Sha256Checksum::compute(b"complete Question Source")
        )
    }));
    assert_eq!(
        object_store
            .get(&publications[0].question_source_object_record.address)
            .await,
        Err(ObjectStoreError::NotFound)
    );
    assert!(
        object_store
            .get(&publications[1].question_source_object_record.address)
            .await
            .is_ok()
    );
}
