use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use async_trait::async_trait;
use question_model::{AccountId, CourseInstanceId, QuestionAttemptId, Timestamp};

use crate::{ImathasQuestionBackendSessionStore, SessionTokenHash, StoreError};

use super::{
    IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES, ImathasQuestionBackendSession,
    ImathasQuestionBackendSessionCreate, ImathasQuestionBackendSessionId,
    ImathasQuestionBackendSessionRestoreExpectation, ImathasQuestionBackendStateCipher,
    ImathasQuestionBackendStateKeyId, ImathasQuestionBackendStateKeyRing,
    LoadedImathasQuestionBackendSession,
};

/// In-memory server-only iMathAS launch-session persistence.
///
/// Submission and scoring use the ordinary Assessment path. iMathAS session storage owns only
/// authorization and encrypted backend state required to restore the backend-owned launch.
pub struct MemoryImathasQuestionBackendSessionStore {
    key_ring: ImathasQuestionBackendStateKeyRing,
    state: Mutex<MemoryState>,
}

struct MemoryState {
    now: Timestamp,
    authenticated_accounts: BTreeMap<SessionTokenHash, AccountId>,
    active_student_authorizations: BTreeSet<(AccountId, CourseInstanceId, QuestionAttemptId)>,
    records: BTreeMap<ImathasQuestionBackendSessionId, MemoryImathasQuestionBackendSessionRecord>,
    used_nonces: BTreeSet<(
        ImathasQuestionBackendStateKeyId,
        [u8; IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES],
    )>,
}

struct MemoryImathasQuestionBackendSessionRecord {
    session: ImathasQuestionBackendSession,
    cipher: ImathasQuestionBackendStateCipher,
}

impl MemoryImathasQuestionBackendSessionStore {
    pub fn new(key_ring: ImathasQuestionBackendStateKeyRing, now: Timestamp) -> Self {
        Self {
            key_ring,
            state: Mutex::new(MemoryState {
                now,
                authenticated_accounts: BTreeMap::new(),
                active_student_authorizations: BTreeSet::new(),
                records: BTreeMap::new(),
                used_nonces: BTreeSet::new(),
            }),
        }
    }

    pub fn install_authenticated_session(&self, token: SessionTokenHash, account: AccountId) {
        self.state
            .lock()
            .expect("memory iMathAS Question Backend Session store lock")
            .authenticated_accounts
            .insert(token, account);
    }

    pub fn install_active_student_authorization(
        &self,
        account: AccountId,
        course: CourseInstanceId,
        question_attempt: QuestionAttemptId,
    ) {
        self.state
            .lock()
            .expect("memory iMathAS Question Backend Session store lock")
            .active_student_authorizations
            .insert((account, course, question_attempt));
    }

    pub fn revoke_active_student_authorization(
        &self,
        account: AccountId,
        course: CourseInstanceId,
        question_attempt: QuestionAttemptId,
    ) {
        self.state
            .lock()
            .expect("memory iMathAS Question Backend Session store lock")
            .active_student_authorizations
            .remove(&(account, course, question_attempt));
    }

    fn account(state: &MemoryState, token: SessionTokenHash) -> Result<AccountId, StoreError> {
        state
            .authenticated_accounts
            .get(&token)
            .cloned()
            .ok_or(StoreError::Forbidden)
    }

    fn authorize(
        state: &MemoryState,
        token: SessionTokenHash,
        account: AccountId,
        course: CourseInstanceId,
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
    ) -> Result<ImathasQuestionBackendSessionId, StoreError> {
        let mut state = self.state.lock().map_err(|_| {
            StoreError::Unavailable(
                "memory iMathAS Question Backend Session store lock unavailable".into(),
            )
        })?;
        Self::authorize(
            &state,
            token,
            create.account.clone(),
            create.course.clone(),
            create.grading_context.question_attempt(),
        )?;
        if create.issued_at > state.now || create.expires_at <= state.now {
            return Err(StoreError::Conflict);
        }
        let session_id = ImathasQuestionBackendSessionId::generate()?;
        let (session, plaintext) = create.into_session(session_id);
        let cipher = ImathasQuestionBackendStateCipher::seal(&self.key_ring, &session, &plaintext)?;
        if !state
            .used_nonces
            .insert((cipher.key_id().clone(), *cipher.nonce()))
        {
            return Err(StoreError::Conflict);
        }
        state.records.insert(
            session_id,
            MemoryImathasQuestionBackendSessionRecord { session, cipher },
        );
        Ok(session_id)
    }

    async fn load_imathas_question_backend_session(
        &self,
        token: SessionTokenHash,
        session_id: ImathasQuestionBackendSessionId,
        expectation: ImathasQuestionBackendSessionRestoreExpectation,
    ) -> Result<LoadedImathasQuestionBackendSession, StoreError> {
        let state = self.state.lock().map_err(|_| {
            StoreError::Unavailable(
                "memory iMathAS Question Backend Session store lock unavailable".into(),
            )
        })?;
        let record = state.records.get(&session_id).ok_or(StoreError::NotFound)?;
        let session = &record.session;
        Self::authorize(
            &state,
            token,
            session.account.clone(),
            session.course.clone(),
            session.grading_context.question_attempt(),
        )?;
        if !expectation.matches(session) {
            return Err(StoreError::Forbidden);
        }
        session.active_at(state.now)?;
        Ok(LoadedImathasQuestionBackendSession::from_storage_parts(
            session.clone(),
            record.cipher.open(&self.key_ring, session)?,
        ))
    }
}
