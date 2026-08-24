//! Closed, per-generation execution artifacts for rehearsal delivery.
//!
//! A frozen question is source material, not an issued variant.  This module
//! makes that distinction durable: only the sealed execution facade can turn
//! frozen material into an exact generated envelope and commit it.

use question_model::presentation::reproduce_presentation_v1;
use question_model::{AttemptProvenance, PresentationBindingV1, QuestionEnvelope};
use serde::{Deserialize, Serialize};

use super::SealedRehearsalDeliveryExecution;
use crate::{
    IssuedQuestionFamilyWitnessV1, IssuedQuestionSnapshotV1, PrefetchedPrivateExecutionV1,
    ReceiptPresentationSnapshot, RehearsalDeliveryExecutionDescriptorV1, RehearsalOperationDigest,
    RehearsalOperationId, StoreError,
};

pub const MAX_REHEARSAL_ISSUED_EXECUTION_ARTIFACT_BYTES: usize = 512 * 1024;
const SCHEMA_VERSION: u8 = 1;

/// Test-only persisted-field mutations used to prove that a canonical but
/// tampered artifact is rejected by the sealed read boundary.
#[cfg(feature = "test-support")]
#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub enum RehearsalIssuedExecutionTestTampering {
    OperationBinding,
    GenerationBinding,
    Presentation,
    Provenance,
    Envelope,
}

/// The two family shapes the rehearsal execution coordinator may issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RehearsalIssuedExecutionFamilyV1 {
    Native,
    Flat,
}

/// Trusted executor input for a dispatched generation.  It cannot be created
/// by routes or browsers because the commit capability is private.
pub struct SealedRehearsalDeliveryIssueWork {
    operation: RehearsalOperationId,
    descriptor: RehearsalDeliveryExecutionDescriptorV1,
    issued_snapshot: IssuedQuestionSnapshotV1,
    private_execution: PrefetchedPrivateExecutionV1,
    private_digest: RehearsalOperationDigest,
    capability: RehearsalIssuedExecutionCommitCapability,
}

impl SealedRehearsalDeliveryIssueWork {
    pub(crate) fn new(
        operation: RehearsalOperationId,
        descriptor: RehearsalDeliveryExecutionDescriptorV1,
        issued_snapshot: IssuedQuestionSnapshotV1,
        private_execution: PrefetchedPrivateExecutionV1,
        private_digest: RehearsalOperationDigest,
    ) -> Self {
        Self {
            operation,
            descriptor,
            issued_snapshot,
            private_execution,
            private_digest,
            capability: RehearsalIssuedExecutionCommitCapability(()),
        }
    }
    pub fn operation(&self) -> RehearsalOperationId {
        self.operation
    }
    pub fn descriptor(&self) -> &RehearsalDeliveryExecutionDescriptorV1 {
        &self.descriptor
    }
    pub fn issued_snapshot(&self) -> &IssuedQuestionSnapshotV1 {
        &self.issued_snapshot
    }
    pub fn private_execution(&self) -> &PrefetchedPrivateExecutionV1 {
        &self.private_execution
    }
    pub fn commit_capability(&self) -> &RehearsalIssuedExecutionCommitCapability {
        &self.capability
    }
    #[cfg(feature = "postgres")]
    #[allow(dead_code)] // consumed by the PostgreSQL sealed broker feature.
    pub(crate) fn private_digest(&self) -> RehearsalOperationDigest {
        self.private_digest
    }
}

impl std::fmt::Debug for SealedRehearsalDeliveryIssueWork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SealedRehearsalDeliveryIssueWork([REDACTED])")
    }
}

/// Opaque proof pairing a constructed artifact with its issued work.
pub struct RehearsalIssuedExecutionCommitCapability(());
impl std::fmt::Debug for RehearsalIssuedExecutionCommitCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RehearsalIssuedExecutionCommitCapability([OPAQUE])")
    }
}

/// Result of an idempotent sealed preparation.  Existing bytes are never
/// exposed to an ordinary Store or browser-facing route.
pub enum SealedRehearsalDeliveryIssuePreparation {
    IssueWork(Box<SealedRehearsalDeliveryIssueWork>),
    ExistingArtifact(Box<SealedRehearsalDeliveryExecution>),
}

#[derive(Clone)]
pub struct RehearsalIssuedExecutionArtifactV1 {
    bytes: Vec<u8>,
}

impl RehearsalIssuedExecutionArtifactV1 {
    /// Restores bounded canonical bytes at a sealed persistence boundary.
    /// Generation-specific material is deliberately verified later by
    /// [`Self::decode_for_work`], after the frozen siblings are hydrated.
    pub(crate) fn from_persisted_bytes(bytes: Vec<u8>) -> Result<Self, StoreError> {
        if bytes.len() > MAX_REHEARSAL_ISSUED_EXECUTION_ARTIFACT_BYTES {
            return Err(StoreError::Unavailable(
                "issued rehearsal artifact exceeds its byte limit".into(),
            ));
        }
        let value: PersistedArtifactV1 = serde_json::from_slice(&bytes)
            .map_err(|_| StoreError::Unavailable("issued rehearsal artifact is invalid".into()))?;
        if serde_json::to_vec(&value).ok().as_deref() != Some(bytes.as_slice()) {
            return Err(StoreError::Unavailable(
                "issued rehearsal artifact is not canonical".into(),
            ));
        }
        if value.schema_version != SCHEMA_VERSION {
            return Err(StoreError::Unavailable(
                "issued rehearsal artifact schema is unsupported".into(),
            ));
        }
        Ok(Self { bytes })
    }
    /// Builds the closed V1 artifact from the exact result of one trusted
    /// family issue call.  The result is bounded and revalidated immediately.
    pub fn from_issue_work(
        work: &SealedRehearsalDeliveryIssueWork,
        envelope: QuestionEnvelope,
        parameter_hash: String,
        provenance: AttemptProvenance,
        presentation_binding: PresentationBindingV1,
        presentation_snapshot: ReceiptPresentationSnapshot,
    ) -> Result<Self, StoreError> {
        let family = family_for_snapshot(work.issued_snapshot())?;
        let (_, snapshot_digest) = work.issued_snapshot.canonical_payload_bytes()?;
        let value = PersistedArtifactV1 {
            schema_version: SCHEMA_VERSION,
            family,
            operation_id: work.operation.as_uuid(),
            snapshot_digest: *snapshot_digest.as_bytes(),
            private_digest: work.private_digest.as_bytes(),
            deterministic_seed: work.descriptor.deterministic_seed(),
            frozen_content_digest: work.descriptor.frozen_content_digest().as_bytes(),
            parameter_hash,
            provenance,
            envelope,
            presentation_binding,
            presentation_snapshot,
        };
        let bytes = serde_json::to_vec(&value).map_err(|error| {
            StoreError::InvalidRecord(format!("issued rehearsal artifact encode failed: {error}"))
        })?;
        if bytes.len() > MAX_REHEARSAL_ISSUED_EXECUTION_ARTIFACT_BYTES {
            return Err(StoreError::InvalidRecord(
                "issued rehearsal artifact exceeds its byte limit".into(),
            ));
        }
        let artifact = Self { bytes };
        artifact.decode_for_work(work)?;
        Ok(artifact)
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Return canonical bytes with one persisted field changed. This helper
    /// exists only for corruption conformance; it does not expose the private
    /// persisted representation or create a commit capability.
    #[cfg(feature = "test-support")]
    pub(crate) fn canonical_test_tamper(
        &self,
        tampering: RehearsalIssuedExecutionTestTampering,
    ) -> Result<Self, StoreError> {
        let mut document: PersistedArtifactV1 = serde_json::from_slice(&self.bytes)
            .map_err(|_| StoreError::Unavailable("issued rehearsal artifact is invalid".into()))?;
        match tampering {
            RehearsalIssuedExecutionTestTampering::OperationBinding => {
                document.operation_id = uuid::Uuid::from_u128(0xA501)
            }
            RehearsalIssuedExecutionTestTampering::GenerationBinding => {
                document.frozen_content_digest = [0xA5; 32]
            }
            RehearsalIssuedExecutionTestTampering::Presentation => {
                document.presentation_snapshot.envelope.title = "tampered presentation".into()
            }
            RehearsalIssuedExecutionTestTampering::Provenance => {
                document.provenance.rendered_question_sha256 = "00".repeat(32)
            }
            RehearsalIssuedExecutionTestTampering::Envelope => {
                document.envelope.title = "tampered envelope".into()
            }
        }
        let bytes = serde_json::to_vec(&document).map_err(|_| {
            StoreError::Unavailable("issued rehearsal artifact tamper encode failed".into())
        })?;
        Self::from_persisted_bytes(bytes)
    }
    #[allow(dead_code)] // consumed by the sealed completion facade.
    pub(crate) fn presentation_snapshot(&self) -> Result<ReceiptPresentationSnapshot, StoreError> {
        let checked = Self::from_persisted_bytes(self.bytes.clone())?;
        serde_json::from_slice::<PersistedArtifactV1>(&checked.bytes)
            .map(|value| value.presentation_snapshot)
            .map_err(|_| StoreError::Unavailable("issued rehearsal artifact is invalid".into()))
    }
    pub(crate) fn active_screen(
        &self,
    ) -> Result<question_model::RehearsalActiveScreenV1, StoreError> {
        question_model::rehearsal_active_screen_from_issued_presentation_v1(
            &self.presentation_snapshot()?.envelope,
        )
        .map_err(|_| {
            StoreError::InvalidRecord(
                "issued rehearsal artifact cannot produce an active screen".into(),
            )
        })
    }
    pub(crate) fn grading_fields(
        &self,
        response: &question_model::StudentResponse,
    ) -> Result<RehearsalIssuedArtifactGradingFields, StoreError> {
        let checked = Self::from_persisted_bytes(self.bytes.clone())?;
        let value: PersistedArtifactV1 = serde_json::from_slice(&checked.bytes)
            .map_err(|_| StoreError::Unavailable("issued rehearsal artifact is invalid".into()))?;
        let presentation = reproduce_presentation_v1(
            &value.envelope,
            &value.presentation_snapshot.asset_bindings,
            value.presentation_binding,
        )
        .map_err(|_| StoreError::Conflict)?;
        if presentation.envelope != value.presentation_snapshot.envelope
            || presentation.asset_bindings != value.presentation_snapshot.asset_bindings
        {
            return Err(StoreError::Conflict);
        }
        let response =
            question_model::presentation::translate_rendered_response_v1(response, &presentation)
                .map_err(|_| StoreError::Conflict)?;
        Ok(RehearsalIssuedArtifactGradingFields {
            family: value.family,
            envelope: value.envelope,
            parameter_hash: value.parameter_hash,
            provenance: value.provenance,
            response,
        })
    }
    pub(crate) fn decode_for_work(
        &self,
        work: &SealedRehearsalDeliveryIssueWork,
    ) -> Result<SealedRehearsalDeliveryExecution, StoreError> {
        let checked = Self::from_persisted_bytes(self.bytes.clone())?;
        let value: PersistedArtifactV1 = serde_json::from_slice(&checked.bytes)
            .map_err(|_| StoreError::Unavailable("issued rehearsal artifact is invalid".into()))?;
        if value.schema_version != SCHEMA_VERSION
            || value.operation_id != work.operation.as_uuid()
            || value.private_digest != work.private_digest.as_bytes()
            || value.deterministic_seed != work.descriptor.deterministic_seed()
            || value.frozen_content_digest != work.descriptor.frozen_content_digest().as_bytes()
        {
            return Err(StoreError::Unavailable(
                "issued rehearsal artifact disagrees with its generation".into(),
            ));
        }
        let (_, source_digest) = work.issued_snapshot().canonical_payload_bytes()?;
        if *source_digest.as_bytes() != value.snapshot_digest
            || family_for_snapshot(work.issued_snapshot())? != value.family
            || work.issued_snapshot().question().problem != work.descriptor.problem().problem
            || work.issued_snapshot().question().version != work.descriptor.problem().version
            || work.issued_snapshot().question().response != *work.descriptor.response_definition()
        {
            return Err(StoreError::Unavailable(
                "issued rehearsal artifact snapshot disagrees with its generation".into(),
            ));
        }
        validate_issued_fields(work, &value)?;
        Ok(SealedRehearsalDeliveryExecution::with_issued_artifact(
            work.issued_snapshot().clone(),
            work.private_execution().clone(),
            self.clone(),
        ))
    }
}
impl std::fmt::Debug for RehearsalIssuedExecutionArtifactV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RehearsalIssuedExecutionArtifactV1([REDACTED])")
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedArtifactV1 {
    schema_version: u8,
    family: RehearsalIssuedExecutionFamilyV1,
    operation_id: uuid::Uuid,
    snapshot_digest: [u8; 32],
    private_digest: [u8; 32],
    deterministic_seed: u64,
    frozen_content_digest: [u8; 32],
    parameter_hash: String,
    provenance: AttemptProvenance,
    envelope: QuestionEnvelope,
    presentation_binding: PresentationBindingV1,
    presentation_snapshot: ReceiptPresentationSnapshot,
}

pub(crate) struct RehearsalIssuedArtifactGradingFields {
    pub(crate) family: RehearsalIssuedExecutionFamilyV1,
    pub(crate) envelope: QuestionEnvelope,
    pub(crate) parameter_hash: String,
    pub(crate) provenance: AttemptProvenance,
    pub(crate) response: question_model::StudentResponse,
}

fn family_for_snapshot(
    snapshot: &IssuedQuestionSnapshotV1,
) -> Result<RehearsalIssuedExecutionFamilyV1, StoreError> {
    match snapshot.family_witness() {
        IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings,
        } if physical_asset_bindings.is_empty() => Ok(RehearsalIssuedExecutionFamilyV1::Native),
        IssuedQuestionFamilyWitnessV1::Flat {} => Ok(RehearsalIssuedExecutionFamilyV1::Flat),
        _ => Err(StoreError::Unavailable(
            "rehearsal issued execution supports only no-asset native or flat families".into(),
        )),
    }
}
fn validate_issued_fields(
    work: &SealedRehearsalDeliveryIssueWork,
    value: &PersistedArtifactV1,
) -> Result<(), StoreError> {
    let snapshot = work.issued_snapshot();
    if value.envelope.version != snapshot.question().version
        || value.envelope.seed.value() != value.deterministic_seed
        || value.envelope.response != snapshot.question().response
        || !is_lower_sha256(&value.parameter_hash)
        || !is_lower_sha256(&value.provenance.rendered_question_sha256)
        || !value.provenance.asset_objects.is_empty()
    {
        return Err(StoreError::Unavailable(
            "issued rehearsal artifact generated fields are invalid".into(),
        ));
    }
    if value.provenance.source_artifact.is_some()
        || envelope_sha256(&value.envelope)? != value.provenance.rendered_question_sha256
    {
        return Err(StoreError::Unavailable(
            "issued rehearsal artifact has invalid provenance".into(),
        ));
    }
    let reproduced = reproduce_presentation_v1(
        &value.envelope,
        &value.presentation_snapshot.asset_bindings,
        value.presentation_binding,
    )
    .map_err(|_| {
        StoreError::Unavailable("issued rehearsal artifact presentation is invalid".into())
    })?;
    if reproduced.envelope != value.presentation_snapshot.envelope
        || reproduced.asset_bindings != value.presentation_snapshot.asset_bindings
        || !value.presentation_snapshot.asset_bindings.is_empty()
    {
        return Err(StoreError::Unavailable(
            "issued rehearsal artifact presentation disagrees with its envelope".into(),
        ));
    }
    match value.family {
        RehearsalIssuedExecutionFamilyV1::Native
            if work.private_execution().flat_grading.is_none()
                && work.private_execution().webwork_replay.is_none()
                && work.private_execution().webwork_grading.is_none()
                && work.private_execution().qti_grading.is_none() => {}
        RehearsalIssuedExecutionFamilyV1::Flat => {
            let flat = work
                .private_execution()
                .flat_grading
                .as_ref()
                .ok_or_else(|| {
                    StoreError::Unavailable(
                        "flat rehearsal issued execution lacks its private contract".into(),
                    )
                })?;
            flat.grading().validate_for_question(snapshot.question())?;
            if flat.question() != snapshot.question()
                || work.private_execution().webwork_replay.is_some()
                || work.private_execution().webwork_grading.is_some()
                || work.private_execution().qti_grading.is_some()
            {
                return Err(StoreError::Unavailable(
                    "flat rehearsal private execution disagrees with its snapshot".into(),
                ));
            }
        }
        _ => {
            return Err(StoreError::Unavailable(
                "native rehearsal private execution disagrees with its snapshot".into(),
            ));
        }
    }
    Ok(())
}
fn envelope_sha256(envelope: &QuestionEnvelope) -> Result<String, StoreError> {
    let bytes = serde_json::to_vec(envelope).map_err(|_| {
        StoreError::Unavailable("issued rehearsal envelope cannot be encoded".into())
    })?;
    Ok(objects::Sha256Digest::compute(&bytes).to_string())
}
fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
