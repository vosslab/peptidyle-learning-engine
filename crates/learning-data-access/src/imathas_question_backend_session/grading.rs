#[derive(Clone, PartialEq, Eq)]
pub struct ImathasResultExchangeIdempotencyKey(String);

impl ImathasResultExchangeIdempotencyKey {
    pub fn parse(value: impl Into<String>) -> Result<Self, StoreError> {
        let value = value.into();
        if value.is_empty() || value.len() > 200 || !value.is_ascii() {
            return Err(StoreError::InvalidRecord(
                "iMathAS Result Exchange idempotency key is invalid".into(),
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ImathasResultExchangeIdempotencyKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasResultExchangeIdempotencyKey([redacted])")
    }
}

macro_rules! imathas_grading_identity {
    ($type:ident, $label:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $type(Uuid);

        impl $type {
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            pub(super) fn generate() -> Result<Self, StoreError> {
                crate::random_uuid::random_uuid_v4(|_| {
                    StoreError::Unavailable(concat!($label, " randomness unavailable").into())
                })
                .map(Self)
            }

            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }
    };
}

imathas_grading_identity!(JobId, "iMathAS Grading Job ID");
imathas_grading_identity!(
    QuestionSubmissionGradingId,
    "iMathAS Question Submission Grading ID"
);
imathas_grading_identity!(AutomatedGradingReceiptId, "Automated Grading Receipt ID");
imathas_grading_identity!(GradingResultId, "Grading Result ID");

#[derive(Clone, PartialEq)]
pub struct StageVerifiedImathasResult {
    pub(super) lease: ImathasQuestionBackendSessionLease,
    pub(super) idempotency_key: ImathasResultExchangeIdempotencyKey,
    pub(super) imathas_result_token_checksum: ImathasResultTokenChecksum,
    pub(super) imathas_result: ImathasResult,
    pub(super) question_submission_id: QuestionSubmissionId,
    pub(super) grading_job_id: JobId,
    pub(super) question_submission_grading_id: QuestionSubmissionGradingId,
    pub(super) transitioned_at: Timestamp,
}

impl StageVerifiedImathasResult {
    pub fn new(
        lease: ImathasQuestionBackendSessionLease,
        grading_context: ImathasGradingContext,
        session_authentication: ImathasQuestionBackendSessionAuthentication,
        idempotency_key: ImathasResultExchangeIdempotencyKey,
        imathas_result_token_checksum: ImathasResultTokenChecksum,
        imathas_result: ImathasResult,
        transitioned_at: Timestamp,
    ) -> Result<Self, StoreError> {
        if grading_context != lease.expectation.grading_context
            || session_authentication != lease.expectation.authentication
        {
            return Err(StoreError::Forbidden);
        }
        Ok(Self {
            lease,
            idempotency_key,
            imathas_result_token_checksum,
            imathas_result,
            question_submission_id: QuestionSubmissionId::from_uuid(
                crate::random_uuid::random_uuid_v4(|_| {
                    StoreError::Unavailable("Question Submission ID randomness unavailable".into())
                })?,
            ),
            grading_job_id: JobId::generate()?,
            question_submission_grading_id: QuestionSubmissionGradingId::generate()?,
            transitioned_at,
        })
    }

    #[allow(dead_code)] // Passed to the PostgreSQL Store consume binding.
    pub(crate) fn lease(&self) -> &ImathasQuestionBackendSessionLease {
        &self.lease
    }

    #[allow(dead_code)] // Passed to the PostgreSQL Store consume binding.
    pub(crate) fn transitioned_at(&self) -> Timestamp {
        self.transitioned_at
    }

    pub fn grading_job_id(&self) -> JobId {
        self.grading_job_id
    }

    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn storage_parts(&self) -> StageVerifiedImathasResultParts {
        StageVerifiedImathasResultParts {
            lease: self.lease.storage_parts(),
            idempotency_key: self.idempotency_key.clone(),
            imathas_result_token_checksum: self.imathas_result_token_checksum,
            imathas_result: self.imathas_result.clone(),
            imathas_result_checksum: self.imathas_result.checksum(),
            question_submission_id: self.question_submission_id,
            grading_job_id: self.grading_job_id,
            question_submission_grading_id: self.question_submission_grading_id,
            transitioned_at: self.transitioned_at,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ImathasGradingJobLease {
    pub(super) grading_job_id: JobId,
    pub(super) capability: Uuid,
    pub(super) expires_at: Timestamp,
}

pub const MAX_IMATHAS_GRADING_JOB_LEASE_MILLIS: i64 = 300_000;

#[derive(Clone, PartialEq, Eq)]
pub struct StagedImathasResultReceipt {
    pub(super) question_submission_id: QuestionSubmissionId,
    pub(super) question_submission_grading_id: QuestionSubmissionGradingId,
    pub(super) job_id: JobId,
}

impl StagedImathasResultReceipt {
    pub fn question_submission_id(&self) -> QuestionSubmissionId {
        self.question_submission_id
    }
    pub fn question_submission_grading_id(&self) -> QuestionSubmissionGradingId {
        self.question_submission_grading_id
    }
    pub fn job_id(&self) -> JobId {
        self.job_id
    }
    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn from_storage_parts(
        question_submission_id: QuestionSubmissionId,
        question_submission_grading_id: QuestionSubmissionGradingId,
        job_id: JobId,
    ) -> Self {
        Self {
            question_submission_id,
            question_submission_grading_id,
            job_id,
        }
    }
}

impl std::fmt::Debug for StagedImathasResultReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StagedImathasResultReceipt([redacted])")
    }
}

impl ImathasGradingJobLease {
    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn from_server_capability(
        grading_job_id: JobId,
        capability: Uuid,
        expires_at: Timestamp,
    ) -> Self {
        Self {
            grading_job_id,
            capability,
            expires_at,
        }
    }
    pub fn grading_job_id(&self) -> JobId {
        self.grading_job_id
    }
    pub fn expires_at(&self) -> Timestamp {
        self.expires_at
    }
    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn storage_parts(&self) -> ImathasGradingJobLeaseParts {
        ImathasGradingJobLeaseParts {
            job_id: self.grading_job_id,
            lease_token: self.capability,
            expires_at: self.expires_at,
        }
    }
}

impl std::fmt::Debug for ImathasGradingJobLease {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasGradingJobLease([redacted])")
    }
}

pub struct CommitStagedImathasResultGrading {
    pub(super) lease: ImathasGradingJobLease,
    pub(super) committed_at: Timestamp,
}

impl CommitStagedImathasResultGrading {
    pub fn new(lease: ImathasGradingJobLease, committed_at: Timestamp) -> Self {
        Self {
            lease,
            committed_at,
        }
    }
    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn storage_parts(&self) -> (ImathasGradingJobLeaseParts, Timestamp) {
        (self.lease.storage_parts(), self.committed_at)
    }
}

impl std::fmt::Debug for CommitStagedImathasResultGrading {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CommitStagedImathasResultGrading([redacted])")
    }
}

#[derive(Clone, PartialEq)]
pub struct AutomatedGradingReceipt {
    pub(super) id: AutomatedGradingReceiptId,
    pub(super) checksum: AutomatedGradingReceiptChecksum,
    pub(super) grading_result: question_model::GradingResult,
}

impl AutomatedGradingReceipt {
    pub fn id(&self) -> AutomatedGradingReceiptId {
        self.id
    }
    pub fn grading_result(&self) -> question_model::GradingResult {
        self.grading_result
    }
    pub fn checksum(&self) -> AutomatedGradingReceiptChecksum {
        self.checksum
    }
    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn from_storage_parts(
        id: AutomatedGradingReceiptId,
        checksum: AutomatedGradingReceiptChecksum,
        grading_result: question_model::GradingResult,
    ) -> Self {
        Self {
            id,
            checksum,
            grading_result,
        }
    }
}

#[allow(clippy::too_many_arguments)] // Fixed receipt checksum v1 has an explicit ordered lineage.
pub(crate) fn automated_grading_receipt_checksum_v1(
    receipt_id: AutomatedGradingReceiptId,
    grading_result_id: GradingResultId,
    grading_id: QuestionSubmissionGradingId,
    submission_id: QuestionSubmissionId,
    attempt_id: QuestionAttemptId,
    job_id: JobId,
    session_id: ImathasQuestionBackendSessionReference,
    token_checksum: ImathasResultTokenChecksum,
    result_checksum: ImathasResultChecksum,
    result: question_model::GradingResult,
    committed_at: Timestamp,
) -> AutomatedGradingReceiptChecksum {
    let mut bytes = b"ple:automated-grading-receipt:v1\0".to_vec();
    for id in [
        receipt_id.as_uuid(),
        grading_result_id.as_uuid(),
        grading_id.as_uuid(),
        submission_id.as_uuid(),
        attempt_id.as_uuid(),
        job_id.as_uuid(),
        session_id.as_uuid(),
    ] {
        bytes.extend_from_slice(id.as_bytes());
    }
    bytes.extend_from_slice(token_checksum.as_bytes());
    bytes.extend_from_slice(result_checksum.as_bytes());
    bytes.push(u8::from(result.correct));
    bytes.extend_from_slice(&result.points_earned.to_be_bytes());
    bytes.extend_from_slice(&result.points_possible.to_be_bytes());
    bytes.extend_from_slice(&committed_at.as_unix_millis().to_be_bytes());
    AutomatedGradingReceiptChecksum::from_bytes(*Sha256Checksum::compute(&bytes).as_bytes())
}

impl std::fmt::Debug for AutomatedGradingReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AutomatedGradingReceipt([redacted])")
    }
}

impl std::fmt::Debug for StageVerifiedImathasResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StageVerifiedImathasResult([redacted])")
    }
}

#[derive(Clone, PartialEq)]
pub struct LoadedImathasQuestionBackendSession {
    pub(super) session: ImathasQuestionBackendSession,
    pub(super) imathas_question_backend_state: ImathasQuestionBackendStatePlaintext,
}

impl LoadedImathasQuestionBackendSession {
    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn from_storage_parts(
        session: ImathasQuestionBackendSession,
        imathas_question_backend_state: ImathasQuestionBackendStatePlaintext,
    ) -> Self {
        Self {
            session,
            imathas_question_backend_state,
        }
    }

    pub fn session(&self) -> &ImathasQuestionBackendSession {
        &self.session
    }
    pub fn imathas_question_backend_state(&self) -> &ImathasQuestionBackendStatePlaintext {
        &self.imathas_question_backend_state
    }

    pub fn imathas_question_backend_validation(&self) -> ImathasQuestionBackendSessionValidation {
        self.session.imathas_question_backend_validation()
    }
}

impl std::fmt::Debug for LoadedImathasQuestionBackendSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(
            "LoadedImathasQuestionBackendSession([redacted iMathAS Question Backend state])",
        )
    }
}
use objects::Sha256Checksum;
use question_model::{QuestionAttemptId, QuestionSubmissionId, Timestamp};
use uuid::Uuid;

use crate::StoreError;

use super::{
    AutomatedGradingReceiptChecksum, ImathasGradingContext, ImathasGradingJobLeaseParts,
    ImathasQuestionBackendSession, ImathasQuestionBackendSessionAuthentication,
    ImathasQuestionBackendSessionLease, ImathasQuestionBackendSessionReference,
    ImathasQuestionBackendSessionValidation, ImathasQuestionBackendStatePlaintext, ImathasResult,
    ImathasResultChecksum, ImathasResultTokenChecksum, StageVerifiedImathasResultParts,
};
