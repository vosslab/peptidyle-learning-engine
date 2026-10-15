//! Authorized persistence for private Draft Question Source Bindings.
//!
//! Question Source bytes are recorded first through the Object Record Store.
//! This boundary then binds that immutable byte evidence to one existing
//! Draft Question at one expected Edit Number. It has no browser serialization path and never
//! accepts inline source data.

use std::{collections::BTreeSet, num::NonZeroU64};

use async_trait::async_trait;
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea};
use question_model::{
    DraftImathasQuestionBackendBinding, ObjectId, PublishedQuestionId,
    PublishedQuestionRevisionTuple, QuestionAuthorship, QuestionBackend, QuestionFormat,
    QuestionImageAssetId, QuestionLicense, QuestionRevisionNumber, QuestionRevisionReason,
    QuestionType, SourceObjectChecksum, Tag, WorkspaceId,
};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Server-only UUID identity for one private mutable Draft Question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DraftQuestionUuid(Uuid);

impl DraftQuestionUuid {
    /// Wraps a Draft Question UUID read from private persistence.
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the private persistence UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Positive optimistic-concurrency token for one mutable Draft Question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DraftQuestionEditNumber(NonZeroU64);

impl DraftQuestionEditNumber {
    /// Creates one positive Draft Question Edit Number representable by PostgreSQL `BIGINT`.
    pub fn new(value: u64) -> Result<Self, StoreError> {
        if value == 0 || value > i64::MAX as u64 {
            return Err(StoreError::InvalidRecord(
                "Draft Question Edit Number must fit a positive PostgreSQL bigint".to_string(),
            ));
        }
        Ok(Self(
            NonZeroU64::new(value).expect("positive value checked before construction"),
        ))
    }

    /// Returns the exact positive value for the PostgreSQL `BIGINT` parameter.
    pub const fn as_postgres_bigint(self) -> i64 {
        self.0.get() as i64
    }
}

/// Complete server-validated input for one Draft Question Source Binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestionSourceBindingInput {
    /// Existing private Draft Question that owns the source.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Exact saved Draft Question state required for this binding.
    pub expected_draft_question_edit_number: DraftQuestionEditNumber,
    /// Workspace authorizing that private authored state.
    pub workspace: WorkspaceId,
    /// Exact interpreter for the source bytes.
    pub question_backend: QuestionBackend,
    /// Exact source representation.
    pub question_format: QuestionFormat,
    /// Author-declared educational Question Type, never inferred from backend controls.
    pub question_type: QuestionType,
    /// WeBWorK PG Path for a WeBWorK Question Backend only.
    pub webwork_pg_path: Option<String>,
    /// iMathAS Deployment and Item IDs for an iMathAS Question Backend only.
    pub draft_imathas_question_backend_binding: Option<DraftImathasQuestionBackendBinding>,
    /// Immutable Object Record identifying the Question Source bytes.
    pub source_object_id: ObjectId,
    /// SHA-256 verification value for those exact bytes.
    pub source_object_checksum: SourceObjectChecksum,
}

impl DraftQuestionSourceBindingInput {
    /// Refuses incoherent backend and source-format combinations before a transaction starts.
    pub fn validate(&self) -> Result<(), StoreError> {
        if !self.question_backend.is_supported_for_production() {
            return Err(StoreError::InvalidRecord(
                "Question Backend is unavailable for new production work".to_string(),
            ));
        }
        let fields_match_backend = match self.question_backend {
            QuestionBackend::Ple => {
                self.webwork_pg_path.is_none()
                    && self.draft_imathas_question_backend_binding.is_none()
            }
            QuestionBackend::Webwork => {
                self.webwork_pg_path.is_some()
                    && self.draft_imathas_question_backend_binding.is_none()
            }
            QuestionBackend::Imathas => {
                self.webwork_pg_path.is_none()
                    && self.draft_imathas_question_backend_binding.is_some()
            }
        };
        if !fields_match_backend {
            return Err(StoreError::InvalidRecord(
                "Draft Question Source Binding must use exactly the fields for its Question Backend"
                    .to_string(),
            ));
        }
        let format_matches_backend = matches!(
            (self.question_backend, self.question_format),
            (QuestionBackend::Ple, QuestionFormat::PleQuestionJson)
                | (QuestionBackend::Webwork, QuestionFormat::WebworkPg)
                | (QuestionBackend::Webwork, QuestionFormat::WebworkPgml)
                | (QuestionBackend::Imathas, QuestionFormat::Imathas)
        );
        if !format_matches_backend {
            return Err(StoreError::InvalidRecord(
                "Question Format must be supported by its Question Backend".to_string(),
            ));
        }
        Ok(())
    }
}

/// Session-authorized persistence for Draft Question Source Bindings.
#[async_trait]
pub trait DraftQuestionSourceBindingStore: Send + Sync {
    /// Binds immutable source-byte evidence at the exact expected Edit Number,
    /// returning the committed Edit Number for a subsequent publication attempt.
    async fn bind_draft_question_source(
        &self,
        session_token_hash: SessionTokenHash,
        input: DraftQuestionSourceBindingInput,
    ) -> Result<DraftQuestionEditNumber, StoreError>;
}

/// Session-authorized resolution of the exact Draft Question Source selected
/// for a publication attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestionPublicationSource {
    /// Exact current private source Object Record.
    pub source_record: ObjectRecord,
    /// Public Question ID reserved at fork Draft creation; ordinary root Drafts have none.
    pub reserved_published_question_id: Option<PublishedQuestionId>,
}

#[async_trait]
pub trait DraftQuestionPublicationSourceStore: Send + Sync {
    /// Loads the current immutable Workspace Question Source Object Record only
    /// when the Draft Question, Edit Number, and workspace remain exact.
    async fn load_draft_question_publication_source(
        &self,
        session_token_hash: SessionTokenHash,
        draft_question_uuid: DraftQuestionUuid,
        expected_draft_question_edit_number: DraftQuestionEditNumber,
        workspace: WorkspaceId,
    ) -> Result<DraftQuestionPublicationSource, StoreError>;
}

/// Trusted bytes-first original raster facts for one exact target Revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedQuestionImagePublication {
    pub source_relative_name: Option<String>,
    pub question_image_asset_id: QuestionImageAssetId,
    pub restricted_source_record: ObjectRecord,
    pub public_object_id: ObjectId,
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
    pub delivery_id: Uuid,
    pub job_id: Uuid,
}

impl PreparedQuestionImagePublication {
    pub fn validate(
        &self,
        published_question_revision_tuple: &PublishedQuestionRevisionTuple,
    ) -> Result<(), StoreError> {
        if let Some(name) = &self.source_relative_name {
            crate::validate_source_relative_name(name)?;
        }
        let record = &self.restricted_source_record;
        let expected = ObjectAddress::RestrictedQuestionImage {
            published_question_revision_tuple: published_question_revision_tuple.clone(),
            question_image_asset_id: self.question_image_asset_id,
            object_id: record.id,
        };
        if record.address != expected
            || record.storage_area != ObjectStorageArea::PrivateContent
            || record.data_class != ObjectDataClass::QuestionImage
            || record.published_question_revision_tuple.as_ref()
                != Some(published_question_revision_tuple)
            || record.id == self.public_object_id
        {
            return Err(StoreError::InvalidRecord(
                "Publication image must belong to the exact target Revision".into(),
            ));
        }
        crate::draft_question_images::validate_raster_facts(
            record,
            self.intrinsic_width,
            self.intrinsic_height,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewQuestionLineagePublicationInput {
    /// Prepared rasters referenced by native source or its response surface.
    pub question_images: Vec<PreparedQuestionImagePublication>,
    /// Current private Draft Question selected for publication.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Exact saved Draft Question state validated by the server.
    pub expected_draft_question_edit_number: DraftQuestionEditNumber,
    /// Authoring Workspace that owns the Draft Question.
    pub workspace: WorkspaceId,
    /// Fresh server-minted Published Question lineage identity.
    pub question_id: PublishedQuestionId,
    /// Verified immutable target object created by the bytes-first copy.
    pub question_source_object_record: ObjectRecord,
    /// Reviewed ordered Question Authorship snapshot.
    pub question_authorship: QuestionAuthorship,
    /// Initial shared search tags for the new Published Question lineage.
    pub initial_shared_tags: Vec<Tag>,
    /// Required canonical lineage classification, independent of Revisions.
    pub discipline_uuid: Uuid,
    pub subject_uuid: Uuid,
    pub topic_uuid: Option<Uuid>,
    pub subtopic_uuid: Option<Uuid>,
    /// Compatible Question License for the immutable first revision.
    pub question_license: QuestionLicense,
    /// Reviewed Question Revision Reason recorded with first-revision acceptance.
    pub question_revision_reason: QuestionRevisionReason,
    /// Fresh immutable Question Ownership Event identity.
    pub question_ownership_event_id: Uuid,
    /// Fresh immutable Question Publication Event identity.
    pub question_publication_event_id: Uuid,
    /// Fresh initial Available-event identity.
    pub question_availability_event_id: Uuid,
}

impl NewQuestionLineagePublicationInput {
    /// Exact first Question Revision created by this publication.
    pub fn published_question_revision_tuple(&self) -> PublishedQuestionRevisionTuple {
        PublishedQuestionRevisionTuple {
            published_question_id: self.question_id.clone(),
            revision_number: QuestionRevisionNumber::new(1)
                .expect("first Question Revision Number is positive"),
        }
    }

    /// Refuses target object or acceptance facts that do not match this publication.
    pub fn validate(&self) -> Result<(), StoreError> {
        Self::validate_initial_shared_tags(&self.initial_shared_tags)?;
        let expected_revision_tuple = self.published_question_revision_tuple();
        for asset in &self.question_images {
            asset.validate(&expected_revision_tuple)?;
        }
        let ObjectAddress::QuestionSource {
            published_question_revision_tuple,
            object_id: object,
        } = &self.question_source_object_record.address
        else {
            return Err(StoreError::InvalidRecord(
                "Question Publication requires a Question Source Object Address".to_string(),
            ));
        };
        if published_question_revision_tuple != &expected_revision_tuple
            || *object != self.question_source_object_record.id
            || self.question_source_object_record.storage_area != ObjectStorageArea::PrivateContent
            || self.question_source_object_record.data_class != ObjectDataClass::QuestionSource
            || self
                .question_source_object_record
                .published_question_revision_tuple
                .as_ref()
                != Some(&expected_revision_tuple)
        {
            return Err(StoreError::InvalidRecord(
                "Question Publication Object Record must derive from its exact first Question Revision"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Refuses shared tags outside the Published Question metadata boundary.
    pub fn validate_initial_shared_tags(tags: &[Tag]) -> Result<(), StoreError> {
        // ASVS 2.2.1-2.2.2: the trusted service boundary enforces the
        // documented shared-metadata shape before object or database writes.
        let mut distinct = BTreeSet::new();
        if tags.iter().any(|tag| {
            let value = tag.as_str();
            value != value.trim()
                || !(1..=120).contains(&value.chars().count())
                || value.chars().any(char::is_control)
                || !distinct.insert(value)
        }) {
            return Err(StoreError::InvalidRecord(
                "Initial Published Question shared tags are invalid".to_string(),
            ));
        }
        Ok(())
    }
}

/// Session-authorized persistence for new-lineage Question Publication.
#[async_trait]
pub trait NewQuestionLineagePublicationStore: Send + Sync {
    /// Atomically records one complete new Published Question aggregate after
    /// the exact source bytes have been copied to immutable object storage.
    ///
    /// [`NewQuestionLineagePublicationError::IdentityCollision`] is reserved
    /// for a conclusively allocated Question ID. Every other persistence
    /// failure remains a [`StoreError`], so publication never retries or
    /// compensates an ambiguous outcome.
    async fn publish_new_question_lineage(
        &self,
        session_token_hash: SessionTokenHash,
        input: NewQuestionLineagePublicationInput,
    ) -> Result<PublishedQuestionRevisionTuple, NewQuestionLineagePublicationError>;
}

/// Result of registering one first Question Revision after its immutable
/// source object has been written.
///
/// The separate collision variant makes the only conclusive identity race
/// explicit at the persistence boundary. It prevents a coordinator from
/// treating a generic database uniqueness error as safe to delete and retry.
#[derive(Debug, Clone, PartialEq)]
pub enum NewQuestionLineagePublicationError {
    /// The candidate Question ID is already issued elsewhere.
    IdentityCollision,
    /// Any non-identity persistence failure is ambiguous to object storage.
    Store(StoreError),
}

/// Complete server-validated input for publishing one new immutable Question
/// Revision in an existing stable Question lineage.
///
/// The caller supplies the exact current parent revision selected by the
/// Instructor. PostgreSQL repeats that precondition while it locks the lineage
/// and records the successor, so a concurrent publication becomes an ordinary
/// optimistic-concurrency conflict rather than a reserved revision number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingQuestionRevisionPublicationInput {
    /// Prepared rasters referenced by native source or its response surface.
    pub question_images: Vec<PreparedQuestionImagePublication>,
    /// Current private Draft Question selected for publication.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Exact saved Draft Question state validated by the server.
    pub expected_draft_question_edit_number: DraftQuestionEditNumber,
    /// Authoring Workspace that owns the Draft Question.
    pub workspace: WorkspaceId,
    /// Exact immutable parent revision selected for this moderate edit.
    pub parent_published_question_revision_tuple: PublishedQuestionRevisionTuple,
    /// Verified immutable target object created by the bytes-first copy.
    pub question_source_object_record: ObjectRecord,
    /// Reviewed reason for accepting this revision. PostgreSQL copies the
    /// existing lineage's immutable authorship and license from the exact
    /// parent rather than accepting replacements at this boundary.
    pub question_revision_reason: QuestionRevisionReason,
    /// Fresh immutable Question Publication Event identity.
    pub question_publication_event_id: Uuid,
}

/// Failure from an existing-lineage publication after the target object has
/// been written. The dedicated stale category identifies the two named
/// PostgreSQL precondition failures whose rolled-back transaction leaves that
/// exact object unregistered and safe to remove.
#[derive(Debug, Clone, PartialEq)]
pub enum ExistingQuestionRevisionPublicationError {
    /// The exact Draft or parent Question Revision precondition is stale.
    Stale,
    /// Any other persistence outcome remains ambiguous to object storage.
    Store(StoreError),
}

impl ExistingQuestionRevisionPublicationInput {
    /// Returns the exact successor revision owned by this publication.
    pub fn published_question_revision_tuple(
        &self,
    ) -> Result<PublishedQuestionRevisionTuple, StoreError> {
        let revision_number = self
            .parent_published_question_revision_tuple
            .revision_number
            .get()
            .checked_add(1)
            .and_then(|value| QuestionRevisionNumber::new(value).ok())
            .ok_or_else(|| {
                StoreError::InvalidRecord(
                    "Question Revision Number cannot advance further".to_string(),
                )
            })?;
        Ok(PublishedQuestionRevisionTuple {
            published_question_id: self
                .parent_published_question_revision_tuple
                .published_question_id
                .clone(),
            revision_number,
        })
    }

    /// Refuses a target object that is not owned by the exact successor.
    pub fn validate(&self) -> Result<(), StoreError> {
        let expected_revision_tuple = self.published_question_revision_tuple()?;
        for asset in &self.question_images {
            asset.validate(&expected_revision_tuple)?;
        }
        let ObjectAddress::QuestionSource {
            published_question_revision_tuple,
            object_id: object,
        } = &self.question_source_object_record.address
        else {
            return Err(StoreError::InvalidRecord(
                "Question Revision Publication requires a Question Source Object Address"
                    .to_string(),
            ));
        };
        if published_question_revision_tuple != &expected_revision_tuple
            || *object != self.question_source_object_record.id
            || self.question_source_object_record.storage_area != ObjectStorageArea::PrivateContent
            || self.question_source_object_record.data_class != ObjectDataClass::QuestionSource
            || self
                .question_source_object_record
                .published_question_revision_tuple
                .as_ref()
                != Some(&expected_revision_tuple)
        {
            return Err(StoreError::InvalidRecord(
                "Question Revision Publication Object Record must derive from its exact successor"
                    .to_string(),
            ));
        }
        Ok(())
    }
}

/// Session-authorized persistence for existing-lineage Question Revision publication.
#[async_trait]
pub trait ExistingQuestionRevisionPublicationStore: Send + Sync {
    /// Atomically verifies current ownership and the exact parent revision,
    /// then records one immutable successor without changing lineage
    /// availability.
    async fn publish_question_revision(
        &self,
        session_token_hash: SessionTokenHash,
        input: ExistingQuestionRevisionPublicationInput,
    ) -> Result<PublishedQuestionRevisionTuple, ExistingQuestionRevisionPublicationError>;
}
