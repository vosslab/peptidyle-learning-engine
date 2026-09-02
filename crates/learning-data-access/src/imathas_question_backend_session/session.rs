use super::*;
use objects::Sha256Checksum;
use question_model::{
    AccountId, AssignmentId, CourseId, ImathasQuestionBackendBinding, QuestionGradingRule,
    SourceObjectChecksum, SourceObjectReference, Timestamp,
};

#[derive(Clone, PartialEq)]

pub struct ImathasQuestionBackendSession {
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
    pub(crate) imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
    pub(crate) issued_at: Timestamp,
    pub(crate) expires_at: Timestamp,
    pub(crate) revoked_at: Option<Timestamp>,
    pub(crate) consumed_at: Option<Timestamp>,
    pub(crate) lease_expires_at: Option<Timestamp>,
    pub(crate) lease_active: bool,
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
    pub fn question_grading_rule(&self) -> &QuestionGradingRule {
        &self.question_grading_rule
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
            assignment: self.assignment,
            grading_context: self.grading_context.clone(),
            question_grading_rule: self.question_grading_rule.clone(),
            imathas_question_backend_binding: self.imathas_question_backend_binding.clone(),
            source_object: self.source_object.clone(),
            source_object_checksum: self.source_object_checksum.clone(),
            response_checksum: self.response_checksum,
            challenge: self.challenge.clone(),
            authentication: self.authentication.clone(),
            imathas_launch_binding_checksum: self.imathas_launch_binding_checksum.clone(),
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            revoked_at: self.revoked_at,
            consumed_at: self.consumed_at,
            lease_expires_at: self.lease_expires_at,
            lease_active: self.lease_active,
        }
    }

    pub(crate) fn active_at(&self, now: Timestamp) -> Result<(), StoreError> {
        if now < self.issued_at
            || self.revoked_at.is_some()
            || self.consumed_at.is_some()
            || now >= self.expires_at
        {
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
        assignment: AssignmentId,
        grading_context: ImathasGradingContext,
        question_grading_rule: QuestionGradingRule,
        imathas_question_backend_binding: ImathasQuestionBackendBinding,
        source_object: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
        response_checksum: ImathasResponseChecksum,
        challenge: ImathasQuestionBackendSessionChallenge,
        authentication: ImathasQuestionBackendSessionAuthentication,
        imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
        issued_at: Timestamp,
        expires_at: Timestamp,
        revoked_at: Option<Timestamp>,
        consumed_at: Option<Timestamp>,
        lease_expires_at: Option<Timestamp>,
        lease_active: bool,
    ) -> Result<Self, StoreError> {
        validate_question_grading_rule(&question_grading_rule)?;
        if expires_at <= issued_at
            || revoked_at.is_some_and(|time| time < issued_at)
            || consumed_at.is_some_and(|time| time < issued_at)
            || lease_active != lease_expires_at.is_some()
            || lease_expires_at.is_some_and(|time| time <= issued_at || time > expires_at)
            || (revoked_at.is_some() && consumed_at.is_some())
            || ((revoked_at.is_some() || consumed_at.is_some()) && lease_active)
        {
            return Err(StoreError::InvalidRecord(
                "iMathAS Question Backend Session storage facts are invalid".into(),
            ));
        }
        Ok(Self {
            reference,
            account,
            course,
            assignment,
            grading_context,
            question_grading_rule,
            imathas_question_backend_binding,
            source_object,
            source_object_checksum,
            response_checksum,
            challenge,
            authentication,
            imathas_launch_binding_checksum,
            issued_at,
            expires_at,
            revoked_at,
            consumed_at,
            lease_expires_at,
            lease_active,
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
            parts.assignment,
            parts.grading_context,
            parts.question_grading_rule,
            parts.imathas_question_backend_binding,
            parts.source_object,
            parts.source_object_checksum,
            parts.response_checksum,
            parts.challenge,
            parts.authentication,
            parts.imathas_launch_binding_checksum,
            parts.issued_at,
            parts.expires_at,
            parts.revoked_at,
            parts.consumed_at,
            parts.lease_expires_at,
            parts.lease_active,
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
            .field("assignment", &self.assignment)
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
            .field("revoked_at", &self.revoked_at)
            .field("consumed_at", &self.consumed_at)
            .field("lease_expires_at", &self.lease_expires_at)
            .field("lease_active", &self.lease_active)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub struct ImathasQuestionBackendSessionRestoreExpectation {
    pub(crate) account: AccountId,
    pub(crate) course: CourseId,
    pub(crate) assignment: AssignmentId,
    pub(crate) grading_context: ImathasGradingContext,
    pub(crate) question_grading_rule: QuestionGradingRule,
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
        assignment: AssignmentId,
        grading_context: ImathasGradingContext,
        question_grading_rule: QuestionGradingRule,
        imathas_question_backend_binding: ImathasQuestionBackendBinding,
        source_object: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
        imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
        authentication: ImathasQuestionBackendSessionAuthentication,
    ) -> Self {
        Self {
            account,
            course,
            assignment,
            grading_context,
            question_grading_rule,
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
            && self.assignment == session.assignment
            && self.grading_context == session.grading_context
            && self.question_grading_rule == session.question_grading_rule
            && self.imathas_question_backend_binding == session.imathas_question_backend_binding
            && self.source_object == session.source_object
            && self.source_object_checksum == session.source_object_checksum
            && self.imathas_launch_binding_checksum == session.imathas_launch_binding_checksum
            && self.authentication == session.authentication
    }

    #[allow(dead_code)] // Passed to PostgreSQL Store lease and consume bindings.
    pub(crate) fn store_predicate(&self) -> ImathasQuestionBackendSessionStorePredicate {
        ImathasQuestionBackendSessionStorePredicate {
            course: self.course,
            assignment: self.assignment,
            grading_context: self.grading_context.clone(),
            question_grading_rule: self.question_grading_rule.clone(),
            imathas_question_backend_binding: self.imathas_question_backend_binding.clone(),
            source_object: self.source_object.clone(),
            source_object_checksum: self.source_object_checksum.clone(),
            imathas_launch_binding_checksum: self.imathas_launch_binding_checksum.clone(),
            authentication: self.authentication.clone(),
        }
    }

    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn storage_parts(&self) -> ImathasQuestionBackendSessionRestoreParts {
        ImathasQuestionBackendSessionRestoreParts {
            account: self.account,
            course: self.course,
            assignment: self.assignment,
            grading_context: self.grading_context.clone(),
            question_grading_rule: self.question_grading_rule.clone(),
            imathas_question_backend_binding: self.imathas_question_backend_binding.clone(),
            source_object: self.source_object.clone(),
            source_object_checksum: self.source_object_checksum.clone(),
            imathas_launch_binding_checksum: self.imathas_launch_binding_checksum.clone(),
            authentication: self.authentication.clone(),
        }
    }
}

/// Exact immutable Store predicate facts carried only within the server-side boundary.
#[allow(dead_code)] // Passed to PostgreSQL Store lease and consume bindings.
#[derive(Clone, PartialEq)]
pub(crate) struct ImathasQuestionBackendSessionStorePredicate {
    pub(crate) course: CourseId,
    pub(crate) assignment: AssignmentId,
    pub(crate) grading_context: ImathasGradingContext,
    pub(crate) question_grading_rule: QuestionGradingRule,
    pub(crate) imathas_question_backend_binding: ImathasQuestionBackendBinding,
    pub(crate) source_object: SourceObjectReference,
    pub(crate) source_object_checksum: SourceObjectChecksum,
    pub(crate) imathas_launch_binding_checksum: ImathasLaunchBindingChecksum,
    pub(crate) authentication: ImathasQuestionBackendSessionAuthentication,
}

/// Server-only facts an adapter needs after Store authorization and AEAD restoration.
#[derive(Clone, PartialEq)]
pub struct ImathasQuestionBackendSessionValidation {
    pub grading_context: ImathasGradingContext,
    pub question_grading_rule: QuestionGradingRule,
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
            question_grading_rule: self.question_grading_rule.clone(),
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
    pub(crate) assignment: AssignmentId,
    pub(crate) grading_context: ImathasGradingContext,
    pub(crate) question_grading_rule: QuestionGradingRule,
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
        assignment: AssignmentId,
        grading_context: ImathasGradingContext,
        question_grading_rule: QuestionGradingRule,
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
        validate_question_grading_rule(&question_grading_rule)?;
        Ok(Self {
            account,
            course,
            assignment,
            grading_context,
            question_grading_rule,
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
            assignment: self.assignment,
            grading_context: self.grading_context,
            question_grading_rule: self.question_grading_rule,
            imathas_question_backend_binding: self.imathas_question_backend_binding,
            source_object: self.source_object,
            source_object_checksum: self.source_object_checksum,
            response_checksum: self.response_checksum,
            challenge: self.challenge,
            authentication: self.authentication,
            imathas_launch_binding_checksum: self.imathas_launch_binding_checksum,
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            revoked_at: None,
            consumed_at: None,
            lease_expires_at: None,
            lease_active: false,
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
pub struct ImathasQuestionBackendSessionLease {
    pub(crate) reference: ImathasQuestionBackendSessionReference,
    pub(crate) capability: [u8; 32],
    pub(crate) expires_at: Timestamp,
    pub(crate) expectation: ImathasQuestionBackendSessionRestoreExpectation,
}

impl ImathasQuestionBackendSessionLease {
    /// Returns the exact server-only grading context bound to this lease.
    pub fn grading_context(&self) -> ImathasGradingContext {
        self.expectation.grading_context.clone()
    }

    /// Returns the exact server-only Session authentication bound to this lease.
    pub fn launch_session_authentication(&self) -> ImathasQuestionBackendSessionAuthentication {
        self.expectation.authentication.clone()
    }

    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn from_server_capability(
        reference: ImathasQuestionBackendSessionReference,
        capability: [u8; 32],
        expires_at: Timestamp,
        expectation: ImathasQuestionBackendSessionRestoreExpectation,
    ) -> Self {
        Self {
            reference,
            capability,
            expires_at,
            expectation,
        }
    }

    #[allow(dead_code)] // Passed to the PostgreSQL Store lease binding.
    pub(crate) fn capability_checksum(&self) -> Sha256Checksum {
        Sha256Checksum::compute(&self.capability)
    }

    #[allow(dead_code)] // Passed to PostgreSQL Store lease and consume bindings.
    pub(crate) fn store_predicate(&self) -> ImathasQuestionBackendSessionStorePredicate {
        self.expectation.store_predicate()
    }

    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn storage_parts(&self) -> ImathasQuestionBackendSessionLeaseParts {
        ImathasQuestionBackendSessionLeaseParts {
            reference: self.reference,
            expires_at: self.expires_at,
            capability_checksum: self.capability_checksum(),
            restore: self.expectation.storage_parts(),
        }
    }

    pub fn reference(&self) -> ImathasQuestionBackendSessionReference {
        self.reference
    }
    pub fn expires_at(&self) -> Timestamp {
        self.expires_at
    }
}

impl std::fmt::Debug for ImathasQuestionBackendSessionLease {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasQuestionBackendSessionLease([redacted])")
    }
}
