//! Shared server-only fixtures for authenticated persistence boundaries.

use learning_data_access::{
    CourseCreationAuthority, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash,
};
use question_model::{CourseId, TenantId, UserId, UserRole};

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
