//! Crate-private PostgreSQL row bindings for iMathAS Question Backend Sessions.

use question_model::{
    AccountId, AssessmentId, CourseInstanceId, ImathasQuestionBackendBinding, ObjectId,
    SourceObjectChecksum, Timestamp,
};

use super::{
    ImathasGradingContext, ImathasLaunchBindingChecksum, ImathasQuestionBackendSession,
    ImathasQuestionBackendSessionAuthentication, ImathasQuestionBackendSessionChallenge,
    ImathasQuestionBackendSessionId, ImathasQuestionBackendStatePlaintext, ImathasResponseChecksum,
};

/// Exact server-only row facts used to create and reconstruct a Session.
#[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
pub(crate) struct ImathasQuestionBackendSessionStorageParts {
    pub(crate) session_id: ImathasQuestionBackendSessionId,
    pub(crate) account: AccountId,
    pub(crate) course: CourseInstanceId,
    pub(crate) assessment: AssessmentId,
    pub(crate) grading_context: ImathasGradingContext,
    pub(crate) imathas_question_backend_binding: ImathasQuestionBackendBinding,
    pub(crate) source_object: ObjectId,
    pub(crate) source_object_checksum: SourceObjectChecksum,
    pub(crate) response_checksum: ImathasResponseChecksum,
    pub(crate) challenge: ImathasQuestionBackendSessionChallenge,
    pub(crate) authentication: ImathasQuestionBackendSessionAuthentication,
    pub(crate) imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
    pub(crate) issued_at: Timestamp,
    pub(crate) expires_at: Timestamp,
}

/// Complete immutable restore binding carried within the server-side Store boundary.
#[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
pub(crate) struct ImathasQuestionBackendSessionRestoreParts {
    pub(crate) account: AccountId,
    pub(crate) course: CourseInstanceId,
    pub(crate) assessment: AssessmentId,
    pub(crate) grading_context: ImathasGradingContext,
    pub(crate) imathas_question_backend_binding: ImathasQuestionBackendBinding,
    pub(crate) source_object: ObjectId,
    pub(crate) source_object_checksum: SourceObjectChecksum,
    pub(crate) imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
    pub(crate) authentication: ImathasQuestionBackendSessionAuthentication,
}

/// Server-only create consumption result for the Store persistence boundary.
#[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
pub(crate) struct ImathasQuestionBackendSessionCreateParts {
    pub(crate) session: ImathasQuestionBackendSession,
    pub(crate) imathas_question_backend_state: ImathasQuestionBackendStatePlaintext,
}
