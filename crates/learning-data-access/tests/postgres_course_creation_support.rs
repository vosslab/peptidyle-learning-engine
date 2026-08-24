//! Shared PostgreSQL course-creation authority fixtures.

use learning_data_access::{
    CourseCreationAuthority, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash,
};
use question_model::{CourseId, TenantId, UserId, UserRole};

/// Return a deterministic, persisted Sysadmin authority for one course.
///
/// The token hash is scoped to the tenant, course, and actor so a fixture
/// cannot accidentally reuse authority across any of those boundaries. The
/// SessionStore round trip validates the complete persisted identity before
/// the course broker receives it (ASVS 2.1.2, 2.3.1).
pub async fn sysadmin_course_creation_authority<S>(
    store: &S,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> CourseCreationAuthority
where
    S: SessionStore + ?Sized,
{
    let mut token_material = b"postgres-course-creation-sysadmin-v1".to_vec();
    token_material.extend_from_slice(tenant.as_uuid().as_bytes());
    token_material.extend_from_slice(course.as_uuid().as_bytes());
    token_material.extend_from_slice(actor.as_uuid().as_bytes());
    let session = SessionTokenHash::compute(&token_material);
    let subject = SessionSubject::new(
        tenant,
        actor,
        "PostgreSQL fixture Sysadmin",
        vec![UserRole::Sysadmin],
    )
    .expect("fixture Sysadmin subject");

    let record = match store
        .resolve_session(session)
        .await
        .expect("resolve deterministic course-creation session")
    {
        Some(record) => record,
        None => {
            store
                .create_session(
                    session,
                    subject.clone(),
                    SessionLifetime::from_seconds(3_600).expect("positive session lifetime"),
                )
                .await
                .expect("create deterministic course-creation session");
            store
                .resolve_session(session)
                .await
                .expect("resolve created course-creation session")
                .expect("created course-creation session remains active")
        }
    };

    assert_eq!(record.token_hash, session);
    assert_eq!(record.subject, subject);
    assert!(record.expires_at > record.created_at);
    CourseCreationAuthority::Sysadmin { actor, session }
}
