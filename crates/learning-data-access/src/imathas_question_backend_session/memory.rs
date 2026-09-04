use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use async_trait::async_trait;
use question_model::{
    AccountId, CourseId, IssuedQuestion, QuestionAttemptId, QuestionSubmissionId, Timestamp,
};
use uuid::Uuid;

use crate::{ImathasQuestionBackendSessionStore, SessionTokenHash, StoreError};

use super::{
    AutomatedGradingReceipt, AutomatedGradingReceiptId, CommitStagedImathasResultGrading,
    GradingResultId, IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES, ImathasGradingJobLease,
    ImathasQuestionBackendSession, ImathasQuestionBackendSessionCreate,
    ImathasQuestionBackendSessionLease, ImathasQuestionBackendSessionReference,
    ImathasQuestionBackendSessionRestoreExpectation, ImathasQuestionBackendStateCipher,
    ImathasQuestionBackendStateKeyId, ImathasQuestionBackendStateKeyRing, ImathasResult,
    ImathasResultChecksum, ImathasResultTokenChecksum, JobId, LoadedImathasQuestionBackendSession,
    MAX_IMATHAS_GRADING_JOB_LEASE_MILLIS, QuestionSubmissionGradingId, StageVerifiedImathasResult,
    StagedImathasResultReceipt, automated_grading_receipt_checksum_v1,
    derive_imathas_question_backend_evaluation,
};

pub struct MemoryImathasQuestionBackendSessionStore {
    key_ring: ImathasQuestionBackendStateKeyRing,
    state: Mutex<MemoryState>,
}

struct MemoryState {
    now: Timestamp,
    authenticated_accounts: BTreeMap<SessionTokenHash, AccountId>,
    active_student_authorizations: BTreeSet<(AccountId, CourseId, QuestionAttemptId)>,
    issued_questions: BTreeMap<QuestionAttemptId, IssuedQuestion>,
    records:
        BTreeMap<ImathasQuestionBackendSessionReference, MemoryImathasQuestionBackendSessionRecord>,
    used_nonces: BTreeSet<(
        ImathasQuestionBackendStateKeyId,
        [u8; IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES],
    )>,
}

struct MemoryImathasQuestionBackendSessionRecord {
    session: ImathasQuestionBackendSession,
    cipher: ImathasQuestionBackendStateCipher,
    capability: Option<[u8; 32]>,
    grading_job_capability: Option<Uuid>,
    grading_job_lease_expires_at: Option<Timestamp>,
    committed_grading_job_capability: Option<Uuid>,
    grading_attempt_count: u8,
    instructor_attention: bool,
    grading_job_failed: bool,
    exchange: Option<MemoryImathasResultExchange>,
}

impl MemoryImathasQuestionBackendSessionStore {
    pub fn new(key_ring: ImathasQuestionBackendStateKeyRing, now: Timestamp) -> Self {
        Self {
            key_ring,
            state: Mutex::new(MemoryState {
                now,
                authenticated_accounts: BTreeMap::new(),
                active_student_authorizations: BTreeSet::new(),
                issued_questions: BTreeMap::new(),
                records: BTreeMap::new(),
                used_nonces: BTreeSet::new(),
            }),
        }
    }

    pub fn install_authenticated_session(&self, token: SessionTokenHash, account: AccountId) {
        self.state
            .lock()
            .expect("memory imathas-question-backend session store lock")
            .authenticated_accounts
            .insert(token, account);
    }

    pub fn install_active_student_authorization(
        &self,
        account: AccountId,
        course: CourseId,
        question_attempt: QuestionAttemptId,
    ) {
        self.state
            .lock()
            .expect("memory imathas-question-backend session store lock")
            .active_student_authorizations
            .insert((account, course, question_attempt));
    }

    /// Test-support resolver for the immutable Issued Question snapshot.
    pub fn install_issued_question_scoring_snapshot(
        &self,
        question_attempt: QuestionAttemptId,
        issued_question: IssuedQuestion,
    ) -> Result<(), StoreError> {
        let mut state = self.state.lock().map_err(|_| {
            StoreError::Unavailable(
                "memory imathas-question-backend session store lock unavailable".into(),
            )
        })?;
        match state.issued_questions.get(&question_attempt) {
            Some(existing) if existing == &issued_question => Ok(()),
            Some(_) => Err(StoreError::Conflict),
            None => {
                state
                    .issued_questions
                    .insert(question_attempt, issued_question);
                Ok(())
            }
        }
    }

    pub fn revoke_active_student_authorization(
        &self,
        account: AccountId,
        course: CourseId,
        question_attempt: QuestionAttemptId,
    ) {
        self.state
            .lock()
            .expect("memory imathas-question-backend session store lock")
            .active_student_authorizations
            .remove(&(account, course, question_attempt));
    }

    pub fn set_now(&self, now: Timestamp) {
        self.state
            .lock()
            .expect("memory imathas-question-backend session store lock")
            .now = now;
    }

    #[cfg(test)]
    pub(super) fn imathas_result_token_checksum(
        &self,
        reference: ImathasQuestionBackendSessionReference,
    ) -> Option<ImathasResultTokenChecksum> {
        self.state
            .lock()
            .expect("memory imathas-question-backend session store lock")
            .records
            .get(&reference)
            .and_then(|record| record.exchange.as_ref())
            .map(|exchange| exchange.imathas_result_token_checksum)
    }

    fn account(state: &MemoryState, token: SessionTokenHash) -> Result<AccountId, StoreError> {
        state
            .authenticated_accounts
            .get(&token)
            .copied()
            .ok_or(StoreError::Forbidden)
    }

    fn authorize(
        state: &MemoryState,
        token: SessionTokenHash,
        account: AccountId,
        course: CourseId,
        attempt: QuestionAttemptId,
    ) -> Result<(), StoreError> {
        if Self::account(state, token)? != account
            || !state
                .active_student_authorizations
                .contains(&(account, course, attempt))
        {
            return Err(StoreError::Forbidden);
        }
        Ok(())
    }
}

#[async_trait]
impl ImathasQuestionBackendSessionStore for MemoryImathasQuestionBackendSessionStore {
    async fn create_imathas_question_backend_session(
        &self,
        token: SessionTokenHash,
        create: ImathasQuestionBackendSessionCreate,
    ) -> Result<ImathasQuestionBackendSessionReference, StoreError> {
        let mut state = self.state.lock().map_err(|_| {
            StoreError::Unavailable(
                "memory imathas-question-backend session store lock unavailable".into(),
            )
        })?;
        Self::authorize(
            &state,
            token,
            create.account,
            create.course,
            create.grading_context.question_attempt(),
        )?;
        if !state
            .issued_questions
            .contains_key(&create.grading_context.question_attempt())
        {
            return Err(StoreError::NotFound);
        }
        if create.issued_at > state.now || create.expires_at <= state.now {
            return Err(StoreError::Conflict);
        }
        let reference = ImathasQuestionBackendSessionReference::generate()?;
        let (session, plaintext) = create.into_session(reference);
        let cipher = ImathasQuestionBackendStateCipher::seal(&self.key_ring, &session, &plaintext)?;
        if !state
            .used_nonces
            .insert((cipher.key_id().clone(), *cipher.nonce()))
        {
            return Err(StoreError::Conflict);
        }
        state.records.insert(
            reference,
            MemoryImathasQuestionBackendSessionRecord {
                session,
                cipher,
                capability: None,
                grading_job_capability: None,
                grading_job_lease_expires_at: None,
                committed_grading_job_capability: None,
                grading_attempt_count: 0,
                instructor_attention: false,
                grading_job_failed: false,
                exchange: None,
            },
        );
        Ok(reference)
    }

    async fn load_imathas_question_backend_session(
        &self,
        token: SessionTokenHash,
        reference: ImathasQuestionBackendSessionReference,
        expectation: ImathasQuestionBackendSessionRestoreExpectation,
    ) -> Result<LoadedImathasQuestionBackendSession, StoreError> {
        let state = self.state.lock().map_err(|_| {
            StoreError::Unavailable(
                "memory imathas-question-backend session store lock unavailable".into(),
            )
        })?;
        let record = state.records.get(&reference).ok_or(StoreError::NotFound)?;
        let session = &record.session;
        Self::authorize(
            &state,
            token,
            session.account,
            session.course,
            session.grading_context.question_attempt(),
        )?;
        if !expectation.matches(session) {
            return Err(StoreError::Forbidden);
        }
        session.active_at(state.now)?;
        Ok(LoadedImathasQuestionBackendSession {
            session: session.clone(),
            imathas_question_backend_state: record.cipher.open(&self.key_ring, session)?,
        })
    }

    async fn lease_imathas_question_backend_session(
        &self,
        token: SessionTokenHash,
        reference: ImathasQuestionBackendSessionReference,
        expectation: ImathasQuestionBackendSessionRestoreExpectation,
        lease_expires_at: Timestamp,
    ) -> Result<ImathasQuestionBackendSessionLease, StoreError> {
        let mut state = self.state.lock().map_err(|_| {
            StoreError::Unavailable(
                "memory imathas-question-backend session store lock unavailable".into(),
            )
        })?;
        let now = state.now;
        let (owner, course, attempt) = state
            .records
            .get(&reference)
            .map(|record| {
                (
                    record.session.account,
                    record.session.course,
                    record.session.grading_context.question_attempt(),
                )
            })
            .ok_or(StoreError::NotFound)?;
        Self::authorize(&state, token, owner, course, attempt)?;
        let record = state
            .records
            .get_mut(&reference)
            .ok_or(StoreError::NotFound)?;
        let session = &mut record.session;
        let active_capability = &mut record.capability;
        if !expectation.matches(session) {
            return Err(StoreError::Forbidden);
        }
        session.active_at(now)?;
        if record
            .exchange
            .as_ref()
            .is_some_and(|exchange| exchange.receipt.is_some())
            || lease_expires_at <= now
            || lease_expires_at.as_unix_millis() - now.as_unix_millis()
                > MAX_IMATHAS_GRADING_JOB_LEASE_MILLIS
            || lease_expires_at > session.expires_at
            || (session.lease_expires_at.is_some_and(|expiry| expiry > now)
                && active_capability.is_some())
        {
            return Err(StoreError::Conflict);
        }
        let mut capability = [0_u8; 32];
        getrandom::fill(&mut capability).map_err(|_| {
            StoreError::Unavailable("iMathAS Question Backend lease randomness unavailable".into())
        })?;
        session.lease_expires_at = Some(lease_expires_at);
        session.lease_active = true;
        *active_capability = Some(capability);
        Ok(ImathasQuestionBackendSessionLease::from_server_capability(
            reference,
            capability,
            lease_expires_at,
            expectation,
        ))
    }

    async fn stage_verified_imathas_result(
        &self,
        token: SessionTokenHash,
        transition: StageVerifiedImathasResult,
    ) -> Result<StagedImathasResultReceipt, StoreError> {
        let mut state = self.state.lock().map_err(|_| {
            StoreError::Unavailable(
                "memory imathas-question-backend session store lock unavailable".into(),
            )
        })?;
        let now = state.now;
        let (owner, course, attempt) = state
            .records
            .get(&transition.lease.reference)
            .map(|record| {
                (
                    record.session.account,
                    record.session.course,
                    record.session.grading_context.question_attempt(),
                )
            })
            .ok_or(StoreError::NotFound)?;
        Self::authorize(&state, token, owner, course, attempt)?;
        let record = state
            .records
            .get_mut(&transition.lease.reference)
            .ok_or(StoreError::NotFound)?;
        advance_memory_imathas_result_exchange_to_ready_to_commit(
            &mut record.session,
            &mut record.capability,
            &mut record.exchange,
            transition,
            now,
        )
    }

    async fn claim_imathas_result_grading_job(
        &self,
        grading_job_id: JobId,
        lease_expires_at: Timestamp,
    ) -> Result<ImathasGradingJobLease, StoreError> {
        let mut state = self.state.lock().map_err(|_| {
            StoreError::Unavailable(
                "memory imathas-question-backend session store lock unavailable".into(),
            )
        })?;
        let now = state.now;
        let record = state
            .records
            .values_mut()
            .find(|record| {
                record
                    .exchange
                    .as_ref()
                    .is_some_and(|exchange| exchange.grading_job_id == grading_job_id)
            })
            .ok_or(StoreError::NotFound)?;
        if record.grading_job_failed {
            return Err(StoreError::Conflict);
        }
        if record.grading_attempt_count >= 3
            && record
                .grading_job_lease_expires_at
                .is_some_and(|expiry| expiry <= now)
        {
            record.grading_job_capability = None;
            record.grading_job_lease_expires_at = None;
            record.grading_job_failed = true;
            record.instructor_attention = true;
            return Err(StoreError::Conflict);
        }
        if record
            .exchange
            .as_ref()
            .is_some_and(|exchange| exchange.receipt.is_some())
            || lease_expires_at <= now
            || lease_expires_at.as_unix_millis() - now.as_unix_millis()
                > MAX_IMATHAS_GRADING_JOB_LEASE_MILLIS
            || record
                .grading_job_lease_expires_at
                .is_some_and(|expiry| expiry > now)
        {
            return Err(StoreError::Conflict);
        }
        let capability = crate::random_uuid::random_uuid_v4(|_| {
            StoreError::Unavailable(
                "iMathAS Question Backend grading Job lease randomness unavailable".into(),
            )
        })?;
        record.grading_job_capability = Some(capability);
        record.grading_job_lease_expires_at = Some(lease_expires_at);
        record.grading_attempt_count += 1;
        Ok(ImathasGradingJobLease {
            grading_job_id,
            capability,
            expires_at: lease_expires_at,
        })
    }

    async fn commit_staged_imathas_result_grading(
        &self,
        command: CommitStagedImathasResultGrading,
    ) -> Result<AutomatedGradingReceipt, StoreError> {
        let mut state = self.state.lock().map_err(|_| {
            StoreError::Unavailable(
                "memory imathas-question-backend session store lock unavailable".into(),
            )
        })?;
        let now = state.now;
        let question_attempt = state
            .records
            .values()
            .find(|record| {
                record
                    .exchange
                    .as_ref()
                    .is_some_and(|exchange| exchange.grading_job_id == command.lease.grading_job_id)
            })
            .map(|record| record.session.grading_context.question_attempt())
            .ok_or(StoreError::NotFound)?;
        let issued_question = state
            .issued_questions
            .get(&question_attempt)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let record = state
            .records
            .values_mut()
            .find(|record| {
                record
                    .exchange
                    .as_ref()
                    .is_some_and(|exchange| exchange.grading_job_id == command.lease.grading_job_id)
            })
            .ok_or(StoreError::NotFound)?;
        let exchange = record.exchange.as_mut().ok_or(StoreError::NotFound)?;
        if let Some(receipt) = &exchange.receipt {
            return if record.committed_grading_job_capability == Some(command.lease.capability) {
                Ok(receipt.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        if command.committed_at != now
            || command.lease.expires_at <= now
            || record.grading_job_lease_expires_at != Some(command.lease.expires_at)
            || record.grading_job_capability != Some(command.lease.capability)
        {
            return Err(StoreError::Conflict);
        }
        let id = AutomatedGradingReceiptId::generate()?;
        let grading_result_id = GradingResultId::generate()?;
        let evaluation = derive_imathas_question_backend_evaluation(&exchange.imathas_result)?;
        let grading_result = question_model::GradingResult::from_issued_question_evaluation(
            &issued_question,
            evaluation,
        );
        let receipt = AutomatedGradingReceipt {
            id,
            checksum: automated_grading_receipt_checksum_v1(
                id,
                grading_result_id,
                exchange.question_submission_grading_id,
                exchange.question_submission_id,
                record.session.grading_context.question_attempt(),
                exchange.grading_job_id,
                record.session.reference(),
                exchange.imathas_result_token_checksum,
                exchange.imathas_result_checksum,
                grading_result,
                command.committed_at,
            ),
            grading_result,
        };
        exchange.receipt = Some(receipt.clone());
        record.committed_grading_job_capability = Some(command.lease.capability);
        record.grading_job_capability = None;
        record.grading_job_lease_expires_at = None;
        record.session.consumed_at = Some(command.committed_at);
        record.session.lease_expires_at = None;
        record.session.lease_active = false;
        Ok(receipt)
    }
}

struct MemoryImathasResultExchange {
    imathas_result_token_checksum: ImathasResultTokenChecksum,
    imathas_result: ImathasResult,
    imathas_result_checksum: ImathasResultChecksum,
    question_submission_id: QuestionSubmissionId,
    question_submission_grading_id: QuestionSubmissionGradingId,
    grading_job_id: JobId,
    receipt: Option<AutomatedGradingReceipt>,
}

fn advance_memory_imathas_result_exchange_to_ready_to_commit(
    session: &mut ImathasQuestionBackendSession,
    capability: &mut Option<[u8; 32]>,
    exchange: &mut Option<MemoryImathasResultExchange>,
    transition: StageVerifiedImathasResult,
    now: Timestamp,
) -> Result<StagedImathasResultReceipt, StoreError> {
    if let Some(existing) = exchange {
        return if existing.imathas_result_token_checksum == transition.imathas_result_token_checksum
            && existing.imathas_result == transition.imathas_result
        {
            Ok(StagedImathasResultReceipt {
                question_submission_id: existing.question_submission_id,
                question_submission_grading_id: existing.question_submission_grading_id,
                job_id: existing.grading_job_id,
            })
        } else {
            Err(StoreError::Conflict)
        };
    }
    if transition.transitioned_at != now
        || transition.lease.expires_at <= now
        || !transition.lease.expectation.matches(session)
        || *capability != Some(transition.lease.capability)
    {
        return Err(StoreError::Conflict);
    }
    session.active_at(now)?;
    let receipt = StagedImathasResultReceipt {
        question_submission_id: transition.question_submission_id,
        question_submission_grading_id: transition.question_submission_grading_id,
        job_id: transition.grading_job_id,
    };
    *exchange = Some(MemoryImathasResultExchange {
        imathas_result_token_checksum: transition.imathas_result_token_checksum,
        imathas_result_checksum: transition.imathas_result.checksum(),
        imathas_result: transition.imathas_result,
        question_submission_id: transition.question_submission_id,
        question_submission_grading_id: transition.question_submission_grading_id,
        grading_job_id: transition.grading_job_id,
        receipt: None,
    });
    *capability = None;
    session.consumed_at = Some(now);
    session.lease_expires_at = None;
    session.lease_active = false;
    Ok(receipt)
}
