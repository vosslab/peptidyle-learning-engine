//! Server-only Question Publication coordination.
//!
//! This boundary resolves an authorized Draft Question Source Object Record,
//! verifies and copies its bytes to the immutable Question Revision Object
//! Address, then asks persistence to commit the complete publication.

use hmac::{Hmac, KeyInit, Mac};
use learning_data_access::{
    DraftQuestionEditNumber, DraftQuestionPublicationSourceStore, DraftQuestionUuid,
    NewQuestionLineagePublicationInput, NewQuestionLineagePublicationStore, SessionTokenHash,
    StoreError, validate_workspace_question_source_object_record,
};
use objects::{ObjectAddress, ObjectStore, ObjectStoreError, PutObject};
use question_model::{
    ObjectId, QUESTION_ID_ALPHABET, QUESTION_ID_IDENTIFIER_LENGTH, QuestionAuthorship, QuestionId,
    QuestionLicense, QuestionRevisionNumber, QuestionRevisionReason, QuestionRevisionReference,
    Timestamp, WorkspaceId,
};
use sha2::Sha256;
use uuid::Uuid;

const PUBLICATION_IDENTITY_ATTEMPTS: usize = 8;

/// Server-held HMAC-SHA-256 key for Question ID validation characters.
#[derive(Clone)]
pub struct QuestionIdSecret([u8; 32]);

impl QuestionIdSecret {
    /// Wraps the exact 256-bit secret supplied by deployment secret storage.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl std::fmt::Debug for QuestionIdSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("QuestionIdSecret([redacted])")
    }
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

/// Server-only source of fresh HMAC-validated Question IDs.
pub trait QuestionIdIssuer: Send + Sync {
    /// Mints one fresh candidate for a new Published Question lineage.
    fn issue_question_id(&self) -> Result<QuestionId, QuestionIdIssuanceError>;
}

/// HMAC-SHA-256 Question ID issuer backed by operating-system randomness.
#[derive(Clone)]
pub struct HmacQuestionIdIssuer {
    secret: QuestionIdSecret,
}

impl HmacQuestionIdIssuer {
    /// Binds issuance to the deployment-owned Question ID secret.
    pub const fn new(secret: QuestionIdSecret) -> Self {
        Self { secret }
    }
}

impl QuestionIdIssuer for HmacQuestionIdIssuer {
    fn issue_question_id(&self) -> Result<QuestionId, QuestionIdIssuanceError> {
        let mut random = [0_u8; 4];
        // ASVS 11.2.1 and 11.4.1: the documented HMAC-SHA-256 construction
        // uses vetted RustCrypto primitives. The public identifier is not a
        // credential, but its six-character candidate still uses the OS CSPRNG.
        getrandom::fill(&mut random).map_err(|_| QuestionIdIssuanceError)?;
        Ok(question_id_from_random_bytes(random, &self.secret))
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
    /// Compatible Question License for the immutable first revision.
    pub question_license: QuestionLicense,
    /// Reviewed reason for accepting the first Question Revision.
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
}

impl<O, S, I> NewQuestionLineagePublisher<O, S, I>
where
    O: ObjectStore,
    S: DraftQuestionPublicationSourceStore + NewQuestionLineagePublicationStore,
    I: QuestionIdIssuer,
{
    /// Creates the server-only coordinator from its exact three owners.
    pub const fn new(object_store: O, publication_store: S, question_id_issuer: I) -> Self {
        Self {
            object_store,
            publication_store,
            question_id_issuer,
        }
    }

    /// Publishes one exact current Draft Question as revision 1 of a new lineage.
    pub async fn publish(
        &self,
        session_token_hash: SessionTokenHash,
        command: NewQuestionLineagePublicationCommand,
        stored_at: Timestamp,
    ) -> Result<QuestionRevisionReference, QuestionPublicationError> {
        let source_record = self
            .publication_store
            .load_draft_question_publication_source(
                session_token_hash,
                command.draft_question_uuid,
                command.expected_draft_question_edit_number,
                command.workspace,
            )
            .await
            .map_err(QuestionPublicationError::Store)?;
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

        for _ in 0..PUBLICATION_IDENTITY_ATTEMPTS {
            let question_id = self
                .question_id_issuer
                .issue_question_id()
                .map_err(QuestionPublicationError::QuestionIdIssuance)?;
            let revision = QuestionRevisionReference {
                question_id: question_id.clone(),
                revision_number: QuestionRevisionNumber::new(1)
                    .expect("first Question Revision Number is positive"),
            };
            let target_address = ObjectAddress::QuestionSource {
                question_revision: revision,
                object: ObjectId::generate(),
            };
            // ASVS 5.3.2, 8.2.2, 14.2.4, and 15.4.2: typed server-created
            // addresses select storage; verified bytes are copied before the
            // final database transaction rechecks authorization and source state.
            let target_record = match self
                .object_store
                .put(PutObject {
                    address: target_address,
                    bytes: source.bytes.clone(),
                    media_type: source_record.media_type.clone(),
                    created_at: stored_at,
                })
                .await
            {
                Ok(record) => record,
                Err(ObjectStoreError::AlreadyExists) => continue,
                Err(error) => return Err(QuestionPublicationError::ObjectStore(error)),
            };
            let input = NewQuestionLineagePublicationInput {
                draft_question_uuid: command.draft_question_uuid,
                expected_draft_question_edit_number: command.expected_draft_question_edit_number,
                workspace: command.workspace,
                question_id,
                question_source_object_record: target_record,
                question_authorship: command.question_authorship.clone(),
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
                Ok(reference) => return Ok(reference),
                Err(StoreError::AlreadyExists) => continue,
                Err(error) => return Err(QuestionPublicationError::Store(error)),
            }
        }
        Err(QuestionPublicationError::IdentityCollisions)
    }
}

fn question_id_from_random_bytes(random: [u8; 4], secret: &QuestionIdSecret) -> QuestionId {
    let value = u32::from_be_bytes(random) >> 2;
    let identifier: String = (0..QUESTION_ID_IDENTIFIER_LENGTH)
        .map(|position| {
            let shift = (QUESTION_ID_IDENTIFIER_LENGTH - position - 1) * 5;
            QUESTION_ID_ALPHABET[((value >> shift) & 0x1f) as usize] as char
        })
        .collect();
    let mut hmac = Hmac::<Sha256>::new_from_slice(&secret.0)
        .expect("HMAC-SHA-256 accepts the fixed 256-bit Question ID secret");
    hmac.update(identifier.as_bytes());
    let validation = QUESTION_ID_ALPHABET[(hmac.finalize().into_bytes()[0] >> 3) as usize] as char;
    QuestionId::from_canonical_parts(&identifier, validation)
        .expect("generated Question ID components use the canonical alphabet")
}

#[cfg(test)]
mod tests;
