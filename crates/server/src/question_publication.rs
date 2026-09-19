//! Server-only Question Publication coordination.
//!
//! This boundary resolves an authorized Draft Question Source Object Record,
//! verifies and copies its bytes to the immutable Question Revision Object
//! Address, then asks persistence to commit the complete publication.

use std::sync::Arc;

use learning_data_access::{
    AuthoringAssetsStore, DraftQuestionEditNumber, DraftQuestionPublicationSourceStore,
    DraftQuestionUuid, ExistingQuestionRevisionPublicationError,
    ExistingQuestionRevisionPublicationInput, ExistingQuestionRevisionPublicationStore,
    NewQuestionLineagePublicationError, NewQuestionLineagePublicationInput,
    NewQuestionLineagePublicationStore, SessionTokenHash, StoreError,
    validate_workspace_question_source_object_record,
};
use objects::{ObjectAddress, ObjectStore, ObjectStoreError, PutObject};
use question_model::{
    ObjectId, QUESTION_ID_ALPHABET, QUESTION_ID_IDENTIFIER_LENGTH, QuestionAuthorship, QuestionId,
    QuestionLicense, QuestionRevisionNumber, QuestionRevisionReason, QuestionRevisionTuple, Tag,
    Timestamp, WorkspaceId,
};
use uuid::Uuid;

const PUBLICATION_IDENTITY_ATTEMPTS: usize = 8;

/// Explicit current private Authoring boundary for native image publication.
/// Non-image server publication workflows have no Draft image context.
pub struct AuthoringAssetContext {
    pub store: Arc<dyn AuthoringAssetsStore>,
    pub draft_question_uuid: DraftQuestionUuid,
}

/// Failure to mint one server-authenticated Question ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionIdIssuanceError;

impl std::fmt::Display for QuestionIdIssuanceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Question ID issuance is unavailable")
    }
}

impl std::error::Error for QuestionIdIssuanceError {}

/// Server-only source of fresh canonical Question IDs.
pub trait QuestionIdIssuer: Send + Sync {
    /// Mints one fresh candidate for a new Published Question lineage.
    fn issue_question_id(&self) -> Result<QuestionId, QuestionIdIssuanceError>;
}

/// Stateless operating-system-random issuer for canonical Question IDs.
///
/// [`QuestionId`] itself validates the public SHA-256 checksum at every parse
/// boundary, so no deployment secret or issuer-specific validation exists.
#[derive(Clone, Copy, Default)]
pub struct RandomQuestionIdIssuer;

impl RandomQuestionIdIssuer {
    /// Builds the stateless issuer.
    pub const fn new() -> Self {
        Self
    }
}

impl QuestionIdIssuer for RandomQuestionIdIssuer {
    fn issue_question_id(&self) -> Result<QuestionId, QuestionIdIssuanceError> {
        let mut random = [0_u8; QUESTION_ID_IDENTIFIER_LENGTH];
        // ASVS 2.2.1 and 2.2.2: issuance uses OS CSPRNG output, then the
        // shared model constructs the one exact checksum-bearing public form.
        getrandom::fill(&mut random).map_err(|_| QuestionIdIssuanceError)?;
        let identifier: String = random
            .into_iter()
            .map(|byte| QUESTION_ID_ALPHABET[(byte & 0x1f) as usize] as char)
            .collect();
        QuestionId::from_random_identifier(&identifier).map_err(|_| QuestionIdIssuanceError)
    }
}

/// Complete server-held request to publish a Draft Question as a new lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewQuestionLineagePublicationCommand {
    /// Current private Draft Question selected for publication.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Exact saved Draft Question state reviewed by the Instructor.
    pub expected_draft_question_edit_number: DraftQuestionEditNumber,
    /// Authoring Workspace that owns the Draft Question.
    pub workspace: WorkspaceId,
    /// Reviewed ordered Question Authorship snapshot.
    pub question_authorship: QuestionAuthorship,
    /// Initial shared search tags derived from the reviewed canonical source.
    pub initial_shared_tags: Vec<Tag>,
    /// Reviewed canonical lineage classification; PostgreSQL validates its parents.
    pub discipline_uuid: Uuid,
    pub subject_uuid: Uuid,
    pub topic_uuid: Option<Uuid>,
    pub subtopic_uuid: Option<Uuid>,
    /// Compatible Question License for the immutable first revision.
    pub question_license: QuestionLicense,
    /// Reviewed reason for accepting the first Question Revision.
    pub question_revision_reason: QuestionRevisionReason,
}

/// Complete server-held request to publish changed Draft Question content as
/// the immediate successor of an existing Question lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingQuestionRevisionPublicationCommand {
    /// Current private Draft Question selected for publication.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Exact saved Draft Question state reviewed by the Instructor.
    pub expected_draft_question_edit_number: DraftQuestionEditNumber,
    /// Authoring Workspace that owns the Draft Question.
    pub workspace: WorkspaceId,
    /// Exact current immutable revision the Instructor is editing from.
    pub parent_question_revision_tuple: QuestionRevisionTuple,
    /// Reviewed reason for accepting the successor. The database copies the
    /// parent revision's immutable authorship and compatible license.
    pub question_revision_reason: QuestionRevisionReason,
}

/// Safe failure categories for server-only Question Publication coordination.
#[derive(Debug, Clone, PartialEq)]
pub enum QuestionPublicationError {
    /// Authenticated persistence refused or could not complete the operation.
    Store(StoreError),
    /// Object storage could not verify, copy, or retain the exact source bytes.
    ObjectStore(ObjectStoreError),
    /// Database and object storage disagreed about the source Object Record.
    SourceObjectRecordMismatch,
    /// The exact Draft or selected parent revision changed before the database
    /// could register this successor.
    StaleQuestionRevision,
    /// Bounded fresh identity attempts all collided.
    IdentityCollisions,
    /// The operating system could not issue a Question ID candidate.
    QuestionIdIssuance(QuestionIdIssuanceError),
}

impl std::fmt::Display for QuestionPublicationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(error) => write!(formatter, "Question Publication Store failed: {error}"),
            Self::ObjectStore(error) => {
                write!(
                    formatter,
                    "Question Publication Object Store failed: {error}"
                )
            }
            Self::SourceObjectRecordMismatch => {
                formatter.write_str("Question Publication source evidence does not match")
            }
            Self::StaleQuestionRevision => {
                formatter.write_str("Question Revision changed before publication")
            }
            Self::IdentityCollisions => {
                formatter.write_str("Question Publication identity allocation did not complete")
            }
            Self::QuestionIdIssuance(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for QuestionPublicationError {}

/// Coordinates verified bytes-first publication without exposing source bytes
/// or persistence identifiers to a browser contract.
pub struct NewQuestionLineagePublisher<O, S, I> {
    object_store: O,
    publication_store: S,
    question_id_issuer: I,
    authoring_assets: Option<AuthoringAssetContext>,
}

impl<O, S, I> NewQuestionLineagePublisher<O, S, I>
where
    O: ObjectStore,
    S: DraftQuestionPublicationSourceStore + NewQuestionLineagePublicationStore,
    I: QuestionIdIssuer,
{
    /// Creates the server-only coordinator from its exact three owners.
    pub const fn new(
        object_store: O,
        publication_store: S,
        question_id_issuer: I,
        authoring_assets: Option<AuthoringAssetContext>,
    ) -> Self {
        Self {
            object_store,
            publication_store,
            question_id_issuer,
            authoring_assets,
        }
    }

    /// Publishes one exact current Draft Question as revision 1 of a new lineage.
    pub async fn publish(
        &self,
        session_token_hash: SessionTokenHash,
        command: NewQuestionLineagePublicationCommand,
        stored_at: Timestamp,
    ) -> Result<QuestionRevisionTuple, QuestionPublicationError> {
        NewQuestionLineagePublicationInput::validate_initial_shared_tags(
            &command.initial_shared_tags,
        )
        .map_err(QuestionPublicationError::Store)?;
        let publication_source = self
            .publication_store
            .load_draft_question_publication_source(
                session_token_hash,
                command.draft_question_uuid,
                command.expected_draft_question_edit_number,
                command.workspace,
            )
            .await
            .map_err(QuestionPublicationError::Store)?;
        let source_record = publication_source.source_record;
        validate_workspace_question_source_object_record(command.workspace, &source_record)
            .map_err(QuestionPublicationError::Store)?;
        let source = self
            .object_store
            .get(&source_record.address)
            .await
            .map_err(QuestionPublicationError::ObjectStore)?;
        if source.record != source_record {
            return Err(QuestionPublicationError::SourceObjectRecordMismatch);
        }
        let hotspot_asset = crate::question_publication_assets::load_hotspot_asset(
            &self.object_store,
            self.authoring_assets.as_ref(),
            session_token_hash,
            command.workspace,
            command.draft_question_uuid,
            &source.bytes,
            &source_record.media_type,
        )
        .await?;
        for _ in 0..PUBLICATION_IDENTITY_ATTEMPTS {
            let question_id = self
                .question_id_issuer
                .issue_question_id()
                .map_err(QuestionPublicationError::QuestionIdIssuance)?;
            let revision = QuestionRevisionTuple {
                question_id: question_id.clone(),
                revision_number: QuestionRevisionNumber::new(1)
                    .expect("first Question Revision Number is positive"),
            };
            let target_address = ObjectAddress::QuestionSource {
                question_revision_tuple: revision.clone(),
                object: ObjectId::generate(),
            };
            // ASVS 5.3.2, 8.2.2, 14.2.4, and 15.4.2: typed server-created
            // addresses select storage; verified bytes are copied before the
            // final database transaction rechecks authorization and source state.
            let target_record = self
                .object_store
                .put(PutObject {
                    address: target_address,
                    bytes: source.bytes.clone(),
                    media_type: source_record.media_type.clone(),
                    created_at: stored_at,
                })
                .await
                .map_err(QuestionPublicationError::ObjectStore)?;
            let target_object_address = target_record.address.clone();
            let prepared_asset = crate::question_publication_assets::prepare_hotspot_asset(
                &self.object_store,
                hotspot_asset.as_ref(),
                &revision,
                stored_at,
            )
            .await?;
            let target_image_address = prepared_asset
                .as_ref()
                .map(|asset| asset.restricted_source_record.address.clone());
            let input = NewQuestionLineagePublicationInput {
                draft_question_uuid: command.draft_question_uuid,
                expected_draft_question_edit_number: command.expected_draft_question_edit_number,
                workspace: command.workspace,
                question_id,
                question_source_object_record: target_record,
                hotspot_asset: prepared_asset,
                question_authorship: command.question_authorship.clone(),
                initial_shared_tags: command.initial_shared_tags.clone(),
                discipline_uuid: command.discipline_uuid,
                subject_uuid: command.subject_uuid,
                topic_uuid: command.topic_uuid,
                subtopic_uuid: command.subtopic_uuid,
                question_license: command.question_license.clone(),
                question_revision_reason: command.question_revision_reason.clone(),
                question_ownership_event_id: Uuid::now_v7(),
                question_publication_event_id: Uuid::now_v7(),
                question_availability_event_id: Uuid::now_v7(),
            };
            match self
                .publication_store
                .publish_new_question_lineage(session_token_hash, input)
                .await
            {
                Ok(question_revision_tuple) => return Ok(question_revision_tuple),
                // PostgreSQL returns IdentityCollision only after its locked
                // allocation check (with the primary key retained as a legacy
                // backstop). That conclusive rollback leaves this request's
                // just-written target unregistered, so delete that exact
                // target before retrying. Any other store outcome is ambiguous
                // and retains its object.
                Err(NewQuestionLineagePublicationError::IdentityCollision) => {
                    crate::question_publication_assets::cleanup_targets(
                        &self.object_store,
                        &target_object_address,
                        target_image_address.as_ref(),
                    )
                    .await?;
                    continue;
                }
                Err(NewQuestionLineagePublicationError::Store(error)) => {
                    return Err(QuestionPublicationError::Store(error));
                }
            }
        }
        Err(QuestionPublicationError::IdentityCollisions)
    }
}

/// Coordinates one ordinary same-lineage publication. The selected parent
/// revision determines the immutable target address; PostgreSQL repeats that
/// exact parent precondition under the lineage lock before it registers the
/// successor.
pub struct ExistingQuestionRevisionPublisher<O, S> {
    object_store: O,
    publication_store: S,
    authoring_assets: Option<AuthoringAssetContext>,
}

impl<O, S> ExistingQuestionRevisionPublisher<O, S>
where
    O: ObjectStore,
    S: DraftQuestionPublicationSourceStore + ExistingQuestionRevisionPublicationStore,
{
    /// Creates the server-only coordinator from object storage and persistence.
    pub const fn new(
        object_store: O,
        publication_store: S,
        authoring_assets: Option<AuthoringAssetContext>,
    ) -> Self {
        Self {
            object_store,
            publication_store,
            authoring_assets,
        }
    }

    /// Publishes the changed exact Draft Question Source as the selected
    /// parent's successor. A stale parent is conclusive: PostgreSQL rolls back
    /// without registering this random object identity, so its exact object is
    /// removed before the caller receives the ordinary precondition failure.
    pub async fn publish(
        &self,
        session_token_hash: SessionTokenHash,
        command: ExistingQuestionRevisionPublicationCommand,
        stored_at: Timestamp,
    ) -> Result<QuestionRevisionTuple, QuestionPublicationError> {
        let successor_revision = successor_revision(&command.parent_question_revision_tuple)
            .map_err(QuestionPublicationError::Store)?;
        let publication_source = self
            .publication_store
            .load_draft_question_publication_source(
                session_token_hash,
                command.draft_question_uuid,
                command.expected_draft_question_edit_number,
                command.workspace,
            )
            .await
            .map_err(QuestionPublicationError::Store)?;
        let source_record = publication_source.source_record;
        validate_workspace_question_source_object_record(command.workspace, &source_record)
            .map_err(QuestionPublicationError::Store)?;
        let source = self
            .object_store
            .get(&source_record.address)
            .await
            .map_err(QuestionPublicationError::ObjectStore)?;
        if source.record != source_record {
            return Err(QuestionPublicationError::SourceObjectRecordMismatch);
        }
        let hotspot_asset = crate::question_publication_assets::load_hotspot_asset(
            &self.object_store,
            self.authoring_assets.as_ref(),
            session_token_hash,
            command.workspace,
            command.draft_question_uuid,
            &source.bytes,
            &source_record.media_type,
        )
        .await?;
        let target_address = ObjectAddress::QuestionSource {
            question_revision_tuple: successor_revision.clone(),
            object: ObjectId::generate(),
        };
        let target_record = self
            .object_store
            .put(PutObject {
                address: target_address,
                bytes: source.bytes,
                media_type: source_record.media_type,
                created_at: stored_at,
            })
            .await
            .map_err(QuestionPublicationError::ObjectStore)?;
        let target_address = target_record.address.clone();
        let prepared_asset = crate::question_publication_assets::prepare_hotspot_asset(
            &self.object_store,
            hotspot_asset.as_ref(),
            &successor_revision,
            stored_at,
        )
        .await?;
        let target_image_address = prepared_asset
            .as_ref()
            .map(|asset| asset.restricted_source_record.address.clone());
        let input = ExistingQuestionRevisionPublicationInput {
            draft_question_uuid: command.draft_question_uuid,
            expected_draft_question_edit_number: command.expected_draft_question_edit_number,
            workspace: command.workspace,
            parent_question_revision_tuple: command.parent_question_revision_tuple,
            question_source_object_record: target_record,
            hotspot_asset: prepared_asset,
            question_revision_reason: command.question_revision_reason,
            question_publication_event_id: Uuid::now_v7(),
        };
        match self
            .publication_store
            .publish_question_revision(session_token_hash, input)
            .await
        {
            Ok(revision) => Ok(revision),
            // A 40001 parent/Draft precondition failure proves this database
            // transaction did not register the unique target object address.
            // ASVS 2.3.1: clean up only that conclusive outcome; an ambiguous
            // store failure retains bytes for investigation rather than risking
            // deletion of evidence from a committed operation.
            Err(ExistingQuestionRevisionPublicationError::Stale) => {
                crate::question_publication_assets::cleanup_targets(
                    &self.object_store,
                    &target_address,
                    target_image_address.as_ref(),
                )
                .await?;
                Err(QuestionPublicationError::StaleQuestionRevision)
            }
            Err(ExistingQuestionRevisionPublicationError::Store(error)) => {
                Err(QuestionPublicationError::Store(error))
            }
        }
    }
}

fn successor_revision(
    parent_question_revision_tuple: &QuestionRevisionTuple,
) -> Result<QuestionRevisionTuple, StoreError> {
    let revision_number = parent_question_revision_tuple
        .revision_number
        .get()
        .checked_add(1)
        .and_then(|value| QuestionRevisionNumber::new(value).ok())
        .ok_or_else(|| {
            StoreError::InvalidRecord("Question Revision Number cannot advance further".to_string())
        })?;
    Ok(QuestionRevisionTuple {
        question_id: parent_question_revision_tuple.question_id.clone(),
        revision_number,
    })
}

#[cfg(test)]
mod tests;
