use super::*;
use question_model::{
    AccountId, AssignmentId, CourseId, ImathasQuestionBackendBinding, SourceObjectChecksum,
    SourceObjectReference, Timestamp,
};

/// Server-only pre-imathas_question_backend-launch facts for one iMathAS Question Backend Session.
pub struct ImathasQuestionBackendSessionPreparationContext {
    account: AccountId,
    course: CourseId,
    assignment: AssignmentId,
    grading_context: ImathasGradingContext,
    imathas_question_backend_binding: ImathasQuestionBackendBinding,
    source_object: SourceObjectReference,
    source_object_checksum: SourceObjectChecksum,
    response_checksum: ImathasResponseChecksum,
    challenge: ImathasQuestionBackendSessionChallenge,
    authentication: ImathasQuestionBackendSessionAuthentication,
    issued_at: Timestamp,
    expires_at: Timestamp,
}

/// Server-only adapter facts available before iMathAS Launch Binding Checksum derivation and imathas_question_backend launch.
#[derive(Clone, PartialEq)]
pub struct ImathasQuestionBackendLaunchPreparationValidation {
    pub grading_context: ImathasGradingContext,
    pub imathas_question_backend_binding: ImathasQuestionBackendBinding,
    pub source_object: SourceObjectReference,
    pub source_object_checksum: SourceObjectChecksum,
    pub response_checksum: ImathasResponseChecksum,
    pub challenge: ImathasQuestionBackendSessionChallenge,
    pub authentication: ImathasQuestionBackendSessionAuthentication,
    pub expires_at: Timestamp,
}

impl ImathasQuestionBackendSessionPreparationContext {
    #[allow(clippy::too_many_arguments)] // Preparation gathers independent trusted launch facts.
    pub fn new(
        account: AccountId,
        course: CourseId,
        assignment: AssignmentId,
        grading_context: ImathasGradingContext,
        imathas_question_backend_binding: ImathasQuestionBackendBinding,
        source_object: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
        response_checksum: ImathasResponseChecksum,
        challenge: ImathasQuestionBackendSessionChallenge,
        authentication: ImathasQuestionBackendSessionAuthentication,
        issued_at: Timestamp,
        expires_at: Timestamp,
    ) -> Result<Self, StoreError> {
        if expires_at <= issued_at {
            return Err(StoreError::InvalidRecord(
                "iMathAS Question Backend Session expiry must follow issue time".into(),
            ));
        }
        Ok(Self {
            account,
            course,
            assignment,
            grading_context,
            imathas_question_backend_binding,
            source_object,
            source_object_checksum,
            response_checksum,
            challenge,
            authentication,
            issued_at,
            expires_at,
        })
    }

    pub fn preparation_validation(&self) -> ImathasQuestionBackendLaunchPreparationValidation {
        ImathasQuestionBackendLaunchPreparationValidation {
            grading_context: self.grading_context.clone(),
            imathas_question_backend_binding: self.imathas_question_backend_binding.clone(),
            source_object: self.source_object.clone(),
            source_object_checksum: self.source_object_checksum.clone(),
            response_checksum: self.response_checksum,
            challenge: self.challenge.clone(),
            authentication: self.authentication.clone(),
            expires_at: self.expires_at,
        }
    }

    pub fn complete(
        self,
        imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
        imathas_question_backend_state: ImathasQuestionBackendStatePlaintext,
    ) -> Result<ImathasQuestionBackendSessionCreate, StoreError> {
        ImathasQuestionBackendSessionCreate::new(
            self.account,
            self.course,
            self.assignment,
            self.grading_context,
            self.imathas_question_backend_binding,
            self.source_object,
            self.source_object_checksum,
            self.response_checksum,
            self.challenge,
            self.authentication,
            imathas_launch_binding_checksum,
            self.issued_at,
            self.expires_at,
            imathas_question_backend_state,
        )
    }
}

impl std::fmt::Debug for ImathasQuestionBackendSessionPreparationContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasQuestionBackendSessionPreparationContext([redacted])")
    }
}

impl std::fmt::Debug for ImathasQuestionBackendLaunchPreparationValidation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasQuestionBackendLaunchPreparationValidation([redacted])")
    }
}
