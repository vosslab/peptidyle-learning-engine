use super::super::*;

use crate::{
    ProblemCollectionReplacementTarget, ProblemCurationStore, ReplaceProblemCollectionCommand,
    SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, StoreError, TenantContext,
};
use question_model::{
    ActivityTimestamp, ProblemCollectionReference, ProblemCollectionRevision,
    ProblemCollectionSummaryView, ProblemCollectionVisibility, QuestionId, TenantId, UserId,
    UserRole,
};
use uuid::Uuid;

pub(super) const ELENA: u128 = 930_001;
pub(super) const MORGAN: u128 = 930_002;
pub(super) const ADA: u128 = 930_003;
pub(super) const OTHER_TENANT: u128 = 930_004;

pub(super) struct Fixture {
    pub(super) store: MemoryStore,
    pub(super) context: TenantContext,
    pub(super) elena: SessionTokenHash,
    pub(super) morgan: SessionTokenHash,
    pub(super) ada: SessionTokenHash,
    pub(super) question_ids: Vec<QuestionId>,
}

impl Fixture {
    pub(super) async fn new(question_count: u128) -> Self {
        let store = MemoryStore::default();
        let tenant = tenant(ELENA);
        let context = TenantContext::from_authenticated_session(tenant);
        let elena = token("d2-elena");
        let morgan = token("d2-morgan");
        let ada = token("d2-ada");
        let mut question_ids = Vec::new();
        {
            let mut state = store.write_state().expect("fixture state");
            for number in 1..=question_count {
                let record = super::super::catalog_search_tests::record(ELENA + number + 100);
                question_ids.push(record.question_id.clone());
                state
                    .published
                    .insert((record.problem, record.version), record);
            }
            approve(&mut state, user(ELENA));
            approve(&mut state, user(ADA));
        }
        for (session, actor, roles, label) in [
            (elena, user(ELENA), vec![UserRole::Instructor], "Elena"),
            (morgan, user(MORGAN), vec![UserRole::Sysadmin], "Morgan"),
            (ada, user(ADA), vec![UserRole::Instructor], "Ada"),
        ] {
            store
                .create_session(
                    session,
                    SessionSubject::new(tenant, actor, label, roles).expect("valid session"),
                    SessionLifetime::from_seconds(600).expect("positive lifetime"),
                )
                .await
                .expect("fixture session");
        }
        Self {
            store,
            context,
            elena,
            morgan,
            ada,
            question_ids,
        }
    }

    pub(super) fn context_for(&self, tenant: TenantId) -> TenantContext {
        TenantContext::from_authenticated_session(tenant)
    }

    pub(super) async fn named(
        &self,
        session: SessionTokenHash,
        title: &str,
        visibility: ProblemCollectionVisibility,
        question_ids: Vec<QuestionId>,
    ) -> Result<ProblemCollectionSummaryView, StoreError> {
        self.store
            .replace_problem_collection(
                self.context,
                session,
                ReplaceProblemCollectionCommand {
                    target: ProblemCollectionReplacementTarget::NewNamed,
                    expected_revision: None,
                    title: Some(title.to_string()),
                    visibility: Some(visibility),
                    question_ids,
                },
            )
            .await
    }

    pub(super) async fn replace_named(
        &self,
        session: SessionTokenHash,
        reference: ProblemCollectionReference,
        revision: ProblemCollectionRevision,
        title: &str,
        visibility: ProblemCollectionVisibility,
        question_ids: Vec<QuestionId>,
    ) -> Result<ProblemCollectionSummaryView, StoreError> {
        self.store
            .replace_problem_collection(
                self.context,
                session,
                ReplaceProblemCollectionCommand {
                    target: ProblemCollectionReplacementTarget::Existing(reference),
                    expected_revision: Some(revision),
                    title: Some(title.to_string()),
                    visibility: Some(visibility),
                    question_ids,
                },
            )
            .await
    }
}

pub(super) fn tenant(number: u128) -> TenantId {
    TenantId::from_uuid(Uuid::from_u128(number))
}
pub(super) fn user(number: u128) -> UserId {
    UserId::from_uuid(Uuid::from_u128(number))
}
pub(super) fn token(label: &str) -> SessionTokenHash {
    SessionTokenHash::compute(label.as_bytes())
}

fn approve(state: &mut State, actor: UserId) {
    state.instructor_approvals.insert(
        actor,
        crate::StoredInstructorApproval {
            approval: question_model::InstructorApproval {
                user: actor,
                approved_by: actor,
                approved_at: ActivityTimestamp::from_unix_millis(0),
                revoked_at: None,
            },
            revision: crate::InstructorApprovalRevision::INITIAL,
        },
    );
}
