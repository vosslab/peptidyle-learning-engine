//! Authentication-session conformance for replica safety, secrecy, and expiry.

use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{SessionLifetime, SessionStore, SessionSubject, SessionTokenHash};
use question_model::{ActivityTimestamp, TenantId, UserId, UserRole};
use uuid::Uuid;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

async fn exercise_session_replicas(issuer: &dyn SessionStore, next_replica: &dyn SessionStore) {
    let token_hash = SessionTokenHash::compute(b"opaque replica test credential");
    let wrong_token_hash = SessionTokenHash::compute(b"different credential");
    let subject = SessionSubject::new(
        TenantId::from_uuid(uuid(101)),
        UserId::from_uuid(uuid(102)),
        "Replica Student",
        vec![UserRole::Student],
    )
    .expect("fixture identity should be valid");
    let lifetime = SessionLifetime::from_seconds(60).expect("positive lifetime");

    let issued = issuer
        .create_session(token_hash, subject.clone(), lifetime)
        .await
        .expect("first replica should issue a session");
    let resumed = next_replica
        .resolve_session(token_hash)
        .await
        .expect("second replica should resolve a session");

    assert_eq!(resumed, Some(issued));
    assert_eq!(
        next_replica.resolve_session(wrong_token_hash).await,
        Ok(None),
        "a different cookie must not reveal any session"
    );

    next_replica
        .revoke_session(token_hash)
        .await
        .expect("second replica should revoke the session");
    assert_eq!(issuer.resolve_session(token_hash).await, Ok(None));
    next_replica
        .revoke_session(token_hash)
        .await
        .expect("repeat revocation should be idempotent");
}

#[tokio::test]
async fn memory_sessions_are_replica_safe_and_revocable() {
    let issuer = MemoryStore::default();
    exercise_session_replicas(&issuer, &issuer.clone()).await;
}

#[tokio::test]
async fn memory_sessions_use_the_backend_clock_for_expiry() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("memory clock should be writable");
    let token_hash = SessionTokenHash::compute(b"expiring credential");
    let subject = SessionSubject::new(
        TenantId::from_uuid(uuid(201)),
        UserId::from_uuid(uuid(202)),
        "Expiring Student",
        vec![UserRole::Student],
    )
    .expect("fixture identity should be valid");
    store
        .create_session(
            token_hash,
            subject,
            SessionLifetime::from_seconds(1).expect("positive lifetime"),
        )
        .await
        .expect("session should be created");
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_000))
        .expect("memory clock should advance");

    assert_eq!(store.resolve_session(token_hash).await, Ok(None));
}
