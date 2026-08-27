//! Shared server-only fixtures for authenticated persistence boundaries.

use std::sync::Arc;
use std::time::Duration;

use learning_data_access::{
    CourseCreationAuthority, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash,
    WorkerId,
};
use question_model::{CourseId, TenantId, UserId, UserRole};

use crate::accepted_submission_worker::AcceptedSubmissionExecutionWorker;
use crate::run::RunBackend;
use crate::worker::WorkerSettings;

/// Creates or reuses the deterministic Sysadmin session for one course fixture.
///
/// The course identity is part of the session token seed, so a fixture cannot
/// accidentally reuse another course's authority. The persisted session is
/// re-read and checked before the closed authority value is returned; course
/// creation still performs its real Memory or PostgreSQL authorization check.
pub(crate) async fn sysadmin_course_creation_authority<S>(
    store: &S,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> CourseCreationAuthority
where
    S: SessionStore,
{
    let token_seed = format!("ple-course-creation-sysadmin:{tenant}:{course}:{actor}");
    let session = SessionTokenHash::compute(token_seed.as_bytes());
    let display_name = format!("Course creation Sysadmin {course}");
    let subject = SessionSubject::new(tenant, actor, display_name, vec![UserRole::Sysadmin])
        .expect("course creation fixture session subject");

    if store
        .resolve_session(session)
        .await
        .expect("course creation fixture session lookup")
        .is_none()
    {
        match store
            .create_session(
                session,
                subject.clone(),
                SessionLifetime::from_seconds(86_400)
                    .expect("course creation fixture session lifetime"),
            )
            .await
        {
            Ok(_) | Err(learning_data_access::StoreError::AlreadyExists) => {}
            Err(error) => panic!("course creation fixture session creation: {error}"),
        }
    }

    let record = store
        .resolve_session(session)
        .await
        .expect("course creation fixture session validation")
        .expect("course creation fixture session is active");
    assert_eq!(record.subject, subject);

    CourseCreationAuthority::Sysadmin { actor, session }
}

/// Drains one accepted submission through the same sealed worker used by the
/// production composition and requires its durable commit to be acknowledged.
///
/// `MemoryStore` clones share state while retaining the worker's capability
/// boundary. Fixed bounded settings keep lifecycle tests deterministic and
/// free of polling or sleeps.
pub(crate) async fn drain_one_accepted_submission<B>(
    store: &Arc<learning_data_access::in_memory::MemoryStore>,
    backend: Arc<B>,
) where
    B: RunBackend + 'static,
{
    let settings = WorkerSettings::new(60, Duration::from_secs(5), 1)
        .expect("bounded accepted-submission worker settings");
    let worker = AcceptedSubmissionExecutionWorker::new(
        (**store).clone(),
        backend,
        WorkerId::from_uuid(uuid::Uuid::from_u128(70_001)),
        settings,
    )
    .expect("accepted-submission worker");
    let report = worker.drain_one().await.expect("accepted-submission drain");
    assert_eq!(
        report.committed, 1,
        "worker must commit one accepted execution"
    );
    assert_eq!(report.no_claim, 0);
    assert_eq!(report.rescheduled, 0);
    assert_eq!(report.terminal, 0);
    assert_eq!(report.stale_claim, 0);
    assert_eq!(report.outcome_unknown, 0);
}
