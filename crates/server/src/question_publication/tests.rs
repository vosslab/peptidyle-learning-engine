use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use learning_data_access::{
    DraftQuestionPublicationSourceStore, NewQuestionLineagePublicationInput,
    NewQuestionLineagePublicationStore,
};
use objects::{ObjectRecord, ObjectStore, PutObject, Sha256Checksum};
use question_model::{QuestionAuthor, QuestionAuthorDisplayName, QuestionRevisionReason};

use super::*;
use objects::memory::MemoryObjectStore;

#[derive(Clone)]
struct RecordingPublicationStore {
    source_record: ObjectRecord,
    publications: Arc<Mutex<Vec<NewQuestionLineagePublicationInput>>>,
}

#[async_trait]
impl DraftQuestionPublicationSourceStore for RecordingPublicationStore {
    async fn load_draft_question_publication_source(
        &self,
        _session_token_hash: SessionTokenHash,
        _draft_question_uuid: DraftQuestionUuid,
        _expected_draft_question_edit_number: DraftQuestionEditNumber,
        _workspace: WorkspaceId,
    ) -> Result<ObjectRecord, StoreError> {
        Ok(self.source_record.clone())
    }
}

#[async_trait]
impl NewQuestionLineagePublicationStore for RecordingPublicationStore {
    async fn publish_new_question_lineage(
        &self,
        _session_token_hash: SessionTokenHash,
        input: NewQuestionLineagePublicationInput,
    ) -> Result<QuestionRevisionReference, StoreError> {
        let result = input.question_revision();
        self.publications
            .lock()
            .expect("publication capture lock")
            .push(input);
        Ok(result)
    }
}

fn command(workspace: WorkspaceId) -> NewQuestionLineagePublicationCommand {
    NewQuestionLineagePublicationCommand {
        draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::from_u128(2)),
        expected_draft_question_edit_number: DraftQuestionEditNumber::new(3)
            .expect("positive Draft Question Edit Number"),
        workspace,
        question_authorship: QuestionAuthorship::new(vec![QuestionAuthor {
            display_name: QuestionAuthorDisplayName::new("Ada Lovelace".to_string())
                .expect("reviewed Question Author"),
        }])
        .expect("bounded Question Authorship"),
        question_license: QuestionLicense::CcBy4_0,
        question_revision_reason: QuestionRevisionReason::new(
            "Initial reviewed publication".to_string(),
        )
        .expect("reviewed Question Revision Reason"),
    }
}

async fn source_fixture(object_store: &MemoryObjectStore, workspace: WorkspaceId) -> ObjectRecord {
    let object = ObjectId::from_uuid(Uuid::from_u128(3));
    object_store
        .put(PutObject {
            address: ObjectAddress::WorkspaceQuestionSource { workspace, object },
            bytes: b"complete Question Source".to_vec(),
            media_type: "application/json".to_string(),
            created_at: Timestamp::from_unix_millis(1_000),
        })
        .await
        .expect("source object")
}

#[tokio::test]
async fn publication_copies_verified_source_before_committing_its_exact_revision() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let object_store = MemoryObjectStore::default();
    let source_record = source_fixture(&object_store, workspace).await;
    let publications = Arc::new(Mutex::new(Vec::new()));
    let publication_store = RecordingPublicationStore {
        source_record,
        publications: Arc::clone(&publications),
    };
    let issuer = HmacQuestionIdIssuer::new(QuestionIdSecret::from_bytes([7; 32]));
    let publisher =
        NewQuestionLineagePublisher::new(object_store.clone(), publication_store, issuer);

    let published = publisher
        .publish(
            SessionTokenHash::compute(b"session"),
            command(workspace),
            Timestamp::from_unix_millis(2_000),
        )
        .await
        .expect("new-lineage Question Publication");
    let input = publications
        .lock()
        .expect("publication capture lock")
        .first()
        .cloned()
        .expect("captured publication");
    let stored = object_store
        .get(&input.question_source_object_record.address)
        .await
        .expect("published source object");

    assert_eq!(input.question_revision(), published);
    assert_eq!(stored.bytes, b"complete Question Source");
}

#[tokio::test]
async fn publication_refuses_database_and_object_store_source_disagreement() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let object_store = MemoryObjectStore::default();
    let mut source_record = source_fixture(&object_store, workspace).await;
    source_record.sha256 = Sha256Checksum::compute(b"different bytes");
    let publication_store = RecordingPublicationStore {
        source_record,
        publications: Arc::new(Mutex::new(Vec::new())),
    };
    let publisher = NewQuestionLineagePublisher::new(
        object_store,
        publication_store,
        HmacQuestionIdIssuer::new(QuestionIdSecret::from_bytes([7; 32])),
    );

    let result = publisher
        .publish(
            SessionTokenHash::compute(b"session"),
            command(workspace),
            Timestamp::from_unix_millis(2_000),
        )
        .await;

    assert_eq!(
        result,
        Err(QuestionPublicationError::SourceObjectRecordMismatch)
    );
}

#[test]
fn question_id_uses_the_documented_hmac_sha256_validation_character() {
    let secret = QuestionIdSecret::from_bytes(std::array::from_fn(|index| index as u8));

    assert_eq!(
        question_id_from_random_bytes([0; 4], &secret).to_string(),
        "000-000N"
    );
    assert_eq!(format!("{secret:?}"), "QuestionIdSecret([redacted])");
}
