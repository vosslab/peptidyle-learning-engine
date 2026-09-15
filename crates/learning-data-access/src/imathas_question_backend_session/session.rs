use super::*;
use question_model::{
    AccountId, AssessmentId, CourseId, ImathasQuestionBackendBinding, SourceObjectChecksum,
    SourceObjectReference, Timestamp,
};

#[derive(Clone, PartialEq)]

pub struct ImathasQuestionBackendSession {
    pub(crate) reference: ImathasQuestionBackendSessionReference,
    pub(crate) account: AccountId,
    pub(crate) course: CourseId,
    pub(crate) assessment: AssessmentId,
    pub(crate) grading_context: ImathasGradingContext,
    pub(crate) imathas_question_backend_binding: ImathasQuestionBackendBinding,
    pub(crate) source_object: SourceObjectReference,
    pub(crate) source_object_checksum: SourceObjectChecksum,
    pub(crate) response_checksum: ImathasResponseChecksum,
    pub(crate) challenge: ImathasQuestionBackendSessionChallenge,
    pub(crate) authentication: ImathasQuestionBackendSessionAuthentication,
    pub(crate) imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
    pub(crate) issued_at: Timestamp,
    pub(crate) expires_at: Timestamp,
}

impl ImathasQuestionBackendSession {
    pub fn reference(&self) -> ImathasQuestionBackendSessionReference {
        self.reference
    }
    pub fn account(&self) -> AccountId {
        self.account
    }
    pub fn grading_context(&self) -> &ImathasGradingContext {
        &self.grading_context
    }
    pub fn expires_at(&self) -> Timestamp {
        self.expires_at
    }

    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn storage_parts(&self) -> ImathasQuestionBackendSessionStorageParts {
        ImathasQuestionBackendSessionStorageParts {
            reference: self.reference,
            account: self.account,
            course: self.course,
            assessment: self.assessment,
            grading_context: self.grading_context.clone(),
            imathas_question_backend_binding: self.imathas_question_backend_binding.clone(),
            source_object: self.source_object.clone(),
            source_object_checksum: self.source_object_checksum.clone(),
            response_checksum: self.response_checksum,
            challenge: self.challenge.clone(),
            authentication: self.authentication.clone(),
            imathas_launch_binding_checksum: self.imathas_launch_binding_checksum.clone(),
            issued_at: self.issued_at,
            expires_at: self.expires_at,
        }
    }

    pub(crate) fn active_at(&self, now: Timestamp) -> Result<(), StoreError> {
        if now < self.issued_at || now >= self.expires_at {
            return Err(StoreError::Conflict);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn from_storage_parts(
        reference: ImathasQuestionBackendSessionReference,
        account: AccountId,
        course: CourseId,
        assessment: AssessmentId,
        grading_context: ImathasGradingContext,
        imathas_question_backend_binding: ImathasQuestionBackendBinding,
        source_object: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
        response_checksum: ImathasResponseChecksum,
        challenge: ImathasQuestionBackendSessionChallenge,
        authentication: ImathasQuestionBackendSessionAuthentication,
        imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
        issued_at: Timestamp,
        expires_at: Timestamp,
    ) -> Result<Self, StoreError> {
        if expires_at <= issued_at {
            return Err(StoreError::InvalidRecord(
                "iMathAS Question Backend Session storage facts are invalid".into(),
            ));
        }
        Ok(Self {
            reference,
            account,
            course,
            assessment,
            grading_context,
            imathas_question_backend_binding,
            source_object,
            source_object_checksum,
            response_checksum,
            challenge,
            authentication,
            imathas_launch_binding_checksum,
            issued_at,
            expires_at,
        })
    }

    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn from_row_parts(
        parts: ImathasQuestionBackendSessionStorageParts,
    ) -> Result<Self, StoreError> {
        Self::from_storage_parts(
            parts.reference,
            parts.account,
            parts.course,
            parts.assessment,
            parts.grading_context,
            parts.imathas_question_backend_binding,
            parts.source_object,
            parts.source_object_checksum,
            parts.response_checksum,
            parts.challenge,
            parts.authentication,
            parts.imathas_launch_binding_checksum,
            parts.issued_at,
            parts.expires_at,
        )
    }
}

impl std::fmt::Debug for ImathasQuestionBackendSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImathasQuestionBackendSession")
            .field("reference", &self.reference)
            .field("account", &self.account)
            .field("course", &self.course)
            .field("assessment", &self.assessment)
            .field("grading_context", &self.grading_context)
            .field("imathas_question_backend_binding", &"[redacted]")
            .field("source_object", &self.source_object)
            .field("source_object_checksum", &"[redacted]")
            .field("response_checksum", &"[redacted]")
            .field("challenge", &"[redacted]")
            .field("authentication", &"[redacted]")
            .field("imathas_launch_binding_checksum", &"[redacted]")
            .field("issued_at", &self.issued_at)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub struct ImathasQuestionBackendSessionRestoreExpectation {
    pub(crate) account: AccountId,
    pub(crate) course: CourseId,
    pub(crate) assessment: AssessmentId,
    pub(crate) grading_context: ImathasGradingContext,
    pub(crate) imathas_question_backend_binding: ImathasQuestionBackendBinding,
    pub(crate) source_object: SourceObjectReference,
    pub(crate) source_object_checksum: SourceObjectChecksum,
    pub(crate) imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
    pub(crate) authentication: ImathasQuestionBackendSessionAuthentication,
}

impl ImathasQuestionBackendSessionRestoreExpectation {
    #[allow(clippy::too_many_arguments)] // Remaining facts are separate persistence predicates.
    pub fn new(
        account: AccountId,
        course: CourseId,
        assessment: AssessmentId,
        grading_context: ImathasGradingContext,
        imathas_question_backend_binding: ImathasQuestionBackendBinding,
        source_object: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
        imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
        authentication: ImathasQuestionBackendSessionAuthentication,
    ) -> Self {
        Self {
            account,
            course,
            assessment,
            grading_context,
            imathas_question_backend_binding,
            source_object,
            source_object_checksum,
            imathas_launch_binding_checksum,
            authentication,
        }
    }

    pub(crate) fn matches(&self, session: &ImathasQuestionBackendSession) -> bool {
        self.account == session.account
            && self.course == session.course
            && self.assessment == session.assessment
            && self.grading_context == session.grading_context
            && self.imathas_question_backend_binding == session.imathas_question_backend_binding
            && self.source_object == session.source_object
            && self.source_object_checksum == session.source_object_checksum
            && self.imathas_launch_binding_checksum == session.imathas_launch_binding_checksum
            && self.authentication == session.authentication
    }

    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn storage_parts(&self) -> ImathasQuestionBackendSessionRestoreParts {
        ImathasQuestionBackendSessionRestoreParts {
            account: self.account,
            course: self.course,
            assessment: self.assessment,
            grading_context: self.grading_context.clone(),
            imathas_question_backend_binding: self.imathas_question_backend_binding.clone(),
            source_object: self.source_object.clone(),
            source_object_checksum: self.source_object_checksum.clone(),
            imathas_launch_binding_checksum: self.imathas_launch_binding_checksum.clone(),
            authentication: self.authentication.clone(),
        }
    }
}

/// Server-only facts an adapter needs after Store authorization and AEAD restoration.
#[derive(Clone, PartialEq)]
pub struct ImathasQuestionBackendSessionValidation {
    pub grading_context: ImathasGradingContext,
    pub imathas_question_backend_binding: ImathasQuestionBackendBinding,
    pub source_object: SourceObjectReference,
    pub source_object_checksum: SourceObjectChecksum,
    pub response_checksum: ImathasResponseChecksum,
    pub challenge: ImathasQuestionBackendSessionChallenge,
    pub authentication: ImathasQuestionBackendSessionAuthentication,
    pub imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
    pub expires_at: Timestamp,
}

impl ImathasQuestionBackendSession {
    pub fn imathas_question_backend_validation(&self) -> ImathasQuestionBackendSessionValidation {
        ImathasQuestionBackendSessionValidation {
            grading_context: self.grading_context.clone(),
            imathas_question_backend_binding: self.imathas_question_backend_binding.clone(),
            source_object: self.source_object.clone(),
            source_object_checksum: self.source_object_checksum.clone(),
            response_checksum: self.response_checksum,
            challenge: self.challenge.clone(),
            authentication: self.authentication.clone(),
            imathas_launch_binding_checksum: self.imathas_launch_binding_checksum.clone(),
            expires_at: self.expires_at,
        }
    }
}

impl std::fmt::Debug for ImathasQuestionBackendSessionValidation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasQuestionBackendSessionValidation([redacted])")
    }
}

impl std::fmt::Debug for ImathasQuestionBackendSessionRestoreExpectation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasQuestionBackendSessionRestoreExpectation([redacted])")
    }
}

pub struct ImathasQuestionBackendSessionCreate {
    pub(crate) account: AccountId,
    pub(crate) course: CourseId,
    pub(crate) assessment: AssessmentId,
    pub(crate) grading_context: ImathasGradingContext,
    pub(crate) imathas_question_backend_binding: ImathasQuestionBackendBinding,
    pub(crate) source_object: SourceObjectReference,
    pub(crate) source_object_checksum: SourceObjectChecksum,
    pub(crate) response_checksum: ImathasResponseChecksum,
    pub(crate) challenge: ImathasQuestionBackendSessionChallenge,
    pub(crate) authentication: ImathasQuestionBackendSessionAuthentication,
    pub(crate) imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
    pub(crate) issued_at: Timestamp,
    pub(crate) expires_at: Timestamp,
    pub(crate) imathas_question_backend_state: ImathasQuestionBackendStatePlaintext,
}

impl ImathasQuestionBackendSessionCreate {
    #[allow(clippy::too_many_arguments)] // Preparation owns the bounded construction boundary.
    pub(super) fn new(
        account: AccountId,
        course: CourseId,
        assessment: AssessmentId,
        grading_context: ImathasGradingContext,
        imathas_question_backend_binding: ImathasQuestionBackendBinding,
        source_object: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
        response_checksum: ImathasResponseChecksum,
        challenge: ImathasQuestionBackendSessionChallenge,
        authentication: ImathasQuestionBackendSessionAuthentication,
        imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
        issued_at: Timestamp,
        expires_at: Timestamp,
        imathas_question_backend_state: ImathasQuestionBackendStatePlaintext,
    ) -> Result<Self, StoreError> {
        if expires_at <= issued_at {
            return Err(StoreError::InvalidRecord(
                "iMathAS Question Backend Session expiry must follow issue time".into(),
            ));
        }
        Ok(Self {
            account,
            course,
            assessment,
            grading_context,
            imathas_question_backend_binding,
            source_object,
            source_object_checksum,
            response_checksum,
            challenge,
            authentication,
            imathas_launch_binding_checksum,
            issued_at,
            expires_at,
            imathas_question_backend_state,
        })
    }

    pub(crate) fn into_session(
        self,
        reference: ImathasQuestionBackendSessionReference,
    ) -> (
        ImathasQuestionBackendSession,
        ImathasQuestionBackendStatePlaintext,
    ) {
        let session = ImathasQuestionBackendSession {
            reference,
            account: self.account,
            course: self.course,
            assessment: self.assessment,
            grading_context: self.grading_context,
            imathas_question_backend_binding: self.imathas_question_backend_binding,
            source_object: self.source_object,
            source_object_checksum: self.source_object_checksum,
            response_checksum: self.response_checksum,
            challenge: self.challenge,
            authentication: self.authentication,
            imathas_launch_binding_checksum: self.imathas_launch_binding_checksum,
            issued_at: self.issued_at,
            expires_at: self.expires_at,
        };
        (session, self.imathas_question_backend_state)
    }

    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn into_storage_parts(
        self,
        reference: ImathasQuestionBackendSessionReference,
    ) -> ImathasQuestionBackendSessionCreateParts {
        let (session, imathas_question_backend_state) = self.into_session(reference);
        ImathasQuestionBackendSessionCreateParts {
            session,
            imathas_question_backend_state,
        }
    }
}

impl std::fmt::Debug for ImathasQuestionBackendSessionCreate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(
            "ImathasQuestionBackendSessionCreate([redacted imathas_question_backend state])",
        )
    }
}

#[derive(Clone, PartialEq)]
pub struct LoadedImathasQuestionBackendSession {
    pub(super) session: ImathasQuestionBackendSession,
    pub(super) imathas_question_backend_state: ImathasQuestionBackendStatePlaintext,
}

impl LoadedImathasQuestionBackendSession {
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
