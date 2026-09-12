use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use learning_data_access::{
    DraftQuestionPublicationSourceStore, ExistingQuestionRevisionPublicationError,
    ExistingQuestionRevisionPublicationInput, ExistingQuestionRevisionPublicationStore,
    NewQuestionLineagePublicationInput, NewQuestionLineagePublicationStore,
};
use objects::{
    ObjectRecord, ObjectStore, ObjectStoreError, PutObject, Sha256Checksum, SignedUrl, StoredObject,
};
use question_model::{QuestionAuthor, QuestionAuthorDisplayName, QuestionRevisionReason};

use super::*;
use objects::memory::MemoryObjectStore;

#[derive(Clone)]
struct RecordingPublicationStore {
    source_record: ObjectRecord,
    publications: Arc<Mutex<Vec<NewQuestionLineagePublicationInput>>>,
}

#[derive(Clone)]
struct ScriptedPublicationStore {
    source_record: ObjectRecord,
    publications: Arc<Mutex<Vec<NewQuestionLineagePublicationInput>>>,
    outcomes: Arc<Mutex<VecDeque<Result<(), StoreError>>>>,
}

#[derive(Clone)]
struct ExistingRevisionRecordingStore {
    source_record: ObjectRecord,
    publications: Arc<Mutex<Vec<ExistingQuestionRevisionPublicationInput>>>,
    outcome: Result<(), ExistingQuestionRevisionPublicationError>,
}

#[derive(Clone)]
struct FixedQuestionIdIssuer {
    question_ids: Arc<Mutex<VecDeque<QuestionId>>>,
}

#[derive(Clone)]
struct DeleteFailObjectStore {
    memory: MemoryObjectStore,
}

#[derive(Clone)]
struct PutAlreadyExistsObjectStore {
    memory: MemoryObjectStore,
    delete_attempts: Arc<Mutex<usize>>,
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

#[async_trait]
impl DraftQuestionPublicationSourceStore for ScriptedPublicationStore {
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
impl NewQuestionLineagePublicationStore for ScriptedPublicationStore {
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
        match self
            .outcomes
            .lock()
            .expect("publication outcome lock")
            .pop_front()
            .expect("scripted publication outcome")
        {
            Ok(()) => Ok(result),
            Err(error) => Err(error),
        }
    }
}

#[async_trait]
impl DraftQuestionPublicationSourceStore for ExistingRevisionRecordingStore {
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
impl ExistingQuestionRevisionPublicationStore for ExistingRevisionRecordingStore {
    async fn publish_question_revision(
        &self,
        _session_token_hash: SessionTokenHash,
        input: ExistingQuestionRevisionPublicationInput,
    ) -> Result<QuestionRevisionReference, ExistingQuestionRevisionPublicationError> {
        let revision = input
            .question_revision()
            .map_err(ExistingQuestionRevisionPublicationError::Store)?;
        self.publications
            .lock()
            .expect("publication capture lock")
            .push(input);
        self.outcome.clone().map(|()| revision)
    }
}

impl QuestionIdIssuer for FixedQuestionIdIssuer {
    fn issue_question_id(&self) -> Result<QuestionId, QuestionIdIssuanceError> {
        self.question_ids
            .lock()
            .expect("Question ID lock")
            .pop_front()
            .ok_or(QuestionIdIssuanceError)
    }
}

#[async_trait]
impl ObjectStore for DeleteFailObjectStore {
    async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
        self.memory.put(request).await
    }

    async fn get(&self, address: &ObjectAddress) -> Result<StoredObject, ObjectStoreError> {
        self.memory.get(address).await
    }

    async fn delete(&self, _address: &ObjectAddress) -> Result<(), ObjectStoreError> {
        Err(ObjectStoreError::Unavailable(
            "injected delete failure".to_string(),
        ))
    }

    async fn signed_url(
        &self,
        address: &ObjectAddress,
        now: Timestamp,
    ) -> Result<SignedUrl, ObjectStoreError> {
        self.memory.signed_url(address, now).await
    }
}

#[async_trait]
impl ObjectStore for PutAlreadyExistsObjectStore {
    async fn put(&self, _request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
        Err(ObjectStoreError::AlreadyExists)
    }

    async fn get(&self, address: &ObjectAddress) -> Result<StoredObject, ObjectStoreError> {
        self.memory.get(address).await
    }

    async fn delete(&self, _address: &ObjectAddress) -> Result<(), ObjectStoreError> {
        *self
            .delete_attempts
            .lock()
            .expect("delete attempt capture lock") += 1;
        Ok(())
    }

    async fn signed_url(
        &self,
        address: &ObjectAddress,
        now: Timestamp,
    ) -> Result<SignedUrl, ObjectStoreError> {
        self.memory.signed_url(address, now).await
    }
}

fn fixed_question_id(identifier: &str) -> QuestionId {
    QuestionId::from_canonical_parts(identifier, '0')
        .expect("fixed Question ID uses canonical Crockford characters")
}

fn scripted_store(
    source_record: ObjectRecord,
    outcomes: impl IntoIterator<Item = Result<(), StoreError>>,
) -> (
    ScriptedPublicationStore,
    Arc<Mutex<Vec<NewQuestionLineagePublicationInput>>>,
) {
    let publications = Arc::new(Mutex::new(Vec::new()));
    (
        ScriptedPublicationStore {
            source_record,
            publications: Arc::clone(&publications),
            outcomes: Arc::new(Mutex::new(outcomes.into_iter().collect())),
        },
        publications,
    )
}

fn fixed_issuer(ids: &[&str]) -> FixedQuestionIdIssuer {
    FixedQuestionIdIssuer {
        question_ids: Arc::new(Mutex::new(
            ids.iter()
                .map(|identifier| fixed_question_id(identifier))
                .collect(),
        )),
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

fn existing_command(workspace: WorkspaceId) -> ExistingQuestionRevisionPublicationCommand {
    ExistingQuestionRevisionPublicationCommand {
        draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::from_u128(2)),
        expected_draft_question_edit_number: DraftQuestionEditNumber::new(3)
            .expect("positive Draft Question Edit Number"),
        workspace,
        parent_question_revision: QuestionRevisionReference {
            question_id: fixed_question_id("000000"),
            revision_number: QuestionRevisionNumber::new(1)
                .expect("positive Question Revision Number"),
        },
        question_revision_reason: QuestionRevisionReason::new(
            "Correct the amino-acid charge".to_string(),
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
    let issuer = HmacQuestionIdIssuer::new(secret.clone());
    let issued = question_id_from_random_bytes([0; 4], &secret);

    assert_eq!(issued.to_string(), "000-000N");
    assert!(issuer.validates_question_id(&issued));
    assert!(issuer.validates_question_id(&"000000n".parse().expect("syntax only ID")));
    assert!(!issuer.validates_question_id(&"000-000P".parse().expect("syntax only ID")));
    assert_eq!(format!("{secret:?}"), "QuestionIdSecret([redacted])");
}

#[tokio::test]
async fn exact_question_id_collision_deletes_this_candidates_object_before_retrying() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let object_store = MemoryObjectStore::default();
    let source_record = source_fixture(&object_store, workspace).await;
    let (publication_store, publications) =
        scripted_store(source_record, [Err(StoreError::AlreadyExists), Ok(())]);
    let publisher = NewQuestionLineagePublisher::new(
        object_store.clone(),
        publication_store,
        fixed_issuer(&["000000", "000001"]),
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

#[tokio::test]
async fn conditional_object_already_exists_is_reported_without_retry_or_delete() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let memory = MemoryObjectStore::default();
    let source_record = source_fixture(&memory, workspace).await;
    let (publication_store, publications) = scripted_store(source_record, [Ok(())]);
    let delete_attempts = Arc::new(Mutex::new(0));
    let publisher = NewQuestionLineagePublisher::new(
        PutAlreadyExistsObjectStore {
            memory,
            delete_attempts: Arc::clone(&delete_attempts),
        },
        publication_store,
        fixed_issuer(&["000000"]),
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
        Err(QuestionPublicationError::ObjectStore(
            ObjectStoreError::AlreadyExists
        ))
    );
    assert!(
        publications
            .lock()
            .expect("publication capture lock")
            .is_empty()
    );
    assert_eq!(
        *delete_attempts.lock().expect("delete attempt capture lock"),
        0
    );
}

#[tokio::test]
async fn a_noncollision_store_rejection_is_not_retried_or_compensated() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let object_store = MemoryObjectStore::default();
    let source_record = source_fixture(&object_store, workspace).await;
    let (publication_store, publications) = scripted_store(
        source_record,
        [Err(StoreError::InvalidRecord(
            "unrelated unique invariant".to_string(),
        ))],
    );
    let publisher = NewQuestionLineagePublisher::new(
        object_store.clone(),
        publication_store,
        fixed_issuer(&["000000"]),
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
        Err(QuestionPublicationError::Store(StoreError::InvalidRecord(
            "unrelated unique invariant".to_string()
        )))
    );
    let publications = publications
        .lock()
        .expect("publication capture lock")
        .clone();
    assert_eq!(publications.len(), 1);
    assert!(
        object_store
            .get(&publications[0].question_source_object_record.address)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn failed_collision_cleanup_fails_closed_without_another_publication_attempt() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let memory = MemoryObjectStore::default();
    let source_record = source_fixture(&memory, workspace).await;
    let (publication_store, publications) =
        scripted_store(source_record, [Err(StoreError::AlreadyExists)]);
    let publisher = NewQuestionLineagePublisher::new(
        DeleteFailObjectStore {
            memory: memory.clone(),
        },
        publication_store,
        fixed_issuer(&["000000", "000001"]),
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
        Err(QuestionPublicationError::ObjectStore(
            ObjectStoreError::Unavailable("injected delete failure".to_string())
        ))
    );
    assert_eq!(
        publications.lock().expect("publication capture lock").len(),
        1
    );
}

#[tokio::test]
async fn same_lineage_publication_copies_to_the_exact_successor_revision() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let object_store = MemoryObjectStore::default();
    let source_record = source_fixture(&object_store, workspace).await;
    let publications = Arc::new(Mutex::new(Vec::new()));
    let publisher = ExistingQuestionRevisionPublisher::new(
        object_store.clone(),
        ExistingRevisionRecordingStore {
            source_record,
            publications: Arc::clone(&publications),
            outcome: Ok(()),
        },
    );

    let published = publisher
        .publish(
            SessionTokenHash::compute(b"session"),
            existing_command(workspace),
            Timestamp::from_unix_millis(2_000),
        )
        .await
        .expect("same-lineage Question Publication");
    let input = publications
        .lock()
        .expect("publication capture lock")
        .first()
        .cloned()
        .expect("captured publication");
    assert_eq!(
        published,
        input.question_revision().expect("successor revision")
    );
    assert_eq!(published.revision_number.get(), 2);
    assert_eq!(
        object_store
            .get(&input.question_source_object_record.address)
            .await
            .expect("successor source object")
            .bytes,
        b"complete Question Source"
    );
}

#[tokio::test]
async fn stale_same_lineage_publication_removes_only_its_unregistered_target() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let object_store = MemoryObjectStore::default();
    let source_record = source_fixture(&object_store, workspace).await;
    let publications = Arc::new(Mutex::new(Vec::new()));
    let publisher = ExistingQuestionRevisionPublisher::new(
        object_store.clone(),
        ExistingRevisionRecordingStore {
            source_record,
            publications: Arc::clone(&publications),
            outcome: Err(ExistingQuestionRevisionPublicationError::Stale),
        },
    );

    let result = publisher
        .publish(
            SessionTokenHash::compute(b"session"),
            existing_command(workspace),
            Timestamp::from_unix_millis(2_000),
        )
        .await;
    assert_eq!(result, Err(QuestionPublicationError::StaleQuestionRevision));
    let input = publications
        .lock()
        .expect("publication capture lock")
        .first()
        .cloned()
        .expect("captured publication");
    assert_eq!(
        object_store
            .get(&input.question_source_object_record.address)
            .await,
        Err(ObjectStoreError::NotFound)
    );
}

#[tokio::test]
async fn ambiguous_same_lineage_failure_retains_its_target_evidence() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(1));
    let object_store = MemoryObjectStore::default();
    let source_record = source_fixture(&object_store, workspace).await;
    let publications = Arc::new(Mutex::new(Vec::new()));
    let publisher = ExistingQuestionRevisionPublisher::new(
        object_store.clone(),
        ExistingRevisionRecordingStore {
            source_record,
            publications: Arc::clone(&publications),
            outcome: Err(ExistingQuestionRevisionPublicationError::Store(
                StoreError::RetryableTransaction,
            )),
        },
    );

    let result = publisher
        .publish(
            SessionTokenHash::compute(b"session"),
            existing_command(workspace),
            Timestamp::from_unix_millis(2_000),
        )
        .await;
    assert_eq!(
        result,
        Err(QuestionPublicationError::Store(
            StoreError::RetryableTransaction
        ))
    );
    let input = publications
        .lock()
        .expect("publication capture lock")
        .first()
        .cloned()
        .expect("captured publication");
    assert!(
        object_store
            .get(&input.question_source_object_record.address)
            .await
            .is_ok()
    );
}
