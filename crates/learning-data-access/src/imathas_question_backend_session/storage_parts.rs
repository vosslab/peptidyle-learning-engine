//! Crate-private PostgreSQL row bindings for iMathAS Question Backend Sessions.

use objects::Sha256Checksum;
use question_model::{
    AccountId, AssignmentId, CourseId, ImathasQuestionBackendBinding, QuestionGradingRule,
    QuestionSubmissionId, SourceObjectChecksum, SourceObjectReference, Timestamp,
};
use uuid::Uuid;

use super::{
    ImathasGradingContext, ImathasQuestionBackendSession,
    ImathasQuestionBackendSessionAuthentication, ImathasQuestionBackendSessionChallenge,
    ImathasQuestionBackendSessionReference, ImathasQuestionBackendStatePlaintext,
    ImathasResponseChecksum, ImathasResult, ImathasResultChecksum,
    ImathasResultExchangeIdempotencyKey, ImathasResultTokenChecksum, JobId,
    QualifiedLaunchBindingDigest, QuestionSubmissionGradingId,
};

/// Exact server-only row facts used to create and reconstruct a Session.
#[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
pub(crate) struct ImathasQuestionBackendSessionStorageParts {
    pub(crate) reference: ImathasQuestionBackendSessionReference,
    pub(crate) account: AccountId,
    pub(crate) course: CourseId,
    pub(crate) assignment: AssignmentId,
    pub(crate) grading_context: ImathasGradingContext,
    pub(crate) question_grading_rule: QuestionGradingRule,
    pub(crate) imathas_question_backend_binding: ImathasQuestionBackendBinding,
    pub(crate) source_object: SourceObjectReference,
    pub(crate) source_object_checksum: SourceObjectChecksum,
    pub(crate) response_checksum: ImathasResponseChecksum,
    pub(crate) challenge: ImathasQuestionBackendSessionChallenge,
    pub(crate) authentication: ImathasQuestionBackendSessionAuthentication,
    pub(crate) qualified_launch_binding_digest: QualifiedLaunchBindingDigest,
    pub(crate) issued_at: Timestamp,
    pub(crate) expires_at: Timestamp,
    pub(crate) revoked_at: Option<Timestamp>,
    pub(crate) consumed_at: Option<Timestamp>,
    pub(crate) lease_expires_at: Option<Timestamp>,
    pub(crate) lease_active: bool,
}

/// Complete immutable restore binding carried within the server-side Store boundary.
#[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
pub(crate) struct ImathasQuestionBackendSessionRestoreParts {
    pub(crate) account: AccountId,
    pub(crate) course: CourseId,
    pub(crate) assignment: AssignmentId,
    pub(crate) grading_context: ImathasGradingContext,
    pub(crate) question_grading_rule: QuestionGradingRule,
    pub(crate) imathas_question_backend_binding: ImathasQuestionBackendBinding,
    pub(crate) source_object: SourceObjectReference,
    pub(crate) source_object_checksum: SourceObjectChecksum,
    pub(crate) qualified_launch_binding_digest: QualifiedLaunchBindingDigest,
    pub(crate) authentication: ImathasQuestionBackendSessionAuthentication,
}

/// Server-only create consumption result for the Store persistence boundary.
#[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
pub(crate) struct ImathasQuestionBackendSessionCreateParts {
    pub(crate) session: ImathasQuestionBackendSession,
    pub(crate) imathas_question_backend_state: ImathasQuestionBackendStatePlaintext,
}

/// Exact lease arguments retained for the stateless PostgreSQL Store.
#[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
pub(crate) struct ImathasQuestionBackendSessionLeaseParts {
    pub(crate) reference: ImathasQuestionBackendSessionReference,
    pub(crate) expires_at: Timestamp,
    pub(crate) capability_checksum: Sha256Checksum,
    pub(crate) restore: ImathasQuestionBackendSessionRestoreParts,
}

/// Exact verified iMathAS Result Exchange arguments carried to PostgreSQL.
#[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
pub(crate) struct StageVerifiedImathasResultParts {
    pub(crate) lease: ImathasQuestionBackendSessionLeaseParts,
    pub(crate) idempotency_key: ImathasResultExchangeIdempotencyKey,
    pub(crate) imathas_result_token_checksum: ImathasResultTokenChecksum,
    pub(crate) imathas_result: ImathasResult,
    pub(crate) imathas_result_checksum: ImathasResultChecksum,
    pub(crate) question_submission_id: QuestionSubmissionId,
    pub(crate) grading_job_id: JobId,
    pub(crate) question_submission_grading_id: QuestionSubmissionGradingId,
    pub(crate) transitioned_at: Timestamp,
}

/// Exact worker-job lease arguments carried to the PostgreSQL commit procedure.
#[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
pub(crate) struct ImathasGradingJobLeaseParts {
    pub(crate) job_id: JobId,
    pub(crate) lease_token: Uuid,
    pub(crate) expires_at: Timestamp,
}
