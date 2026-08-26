use learning_data_access::{CurriculumAdoptionStore, StoreError};
use sqlx::Row;

use super::fixture::AdoptionFixture;

pub(super) async fn assert_broker_boundary(fixture: &AdoptionFixture) {
    fixture
        .store
        .preflight_curriculum_adoption(fixture.context, fixture.instructor_session)
        .await
        .expect("approved Instructor reaches the B2 Store capability");
    fixture
        .store
        .preflight_curriculum_adoption(fixture.foreign_context, fixture.foreign_instructor_session)
        .await
        .expect("approved Instructor reaches B2 capability in that Instructor's tenant");
    assert_ne!(
        fixture.tenant, fixture.foreign_tenant,
        "the source and destination fixtures exercise separate tenants"
    );
    assert_ne!(
        fixture.instructor, fixture.foreign_instructor,
        "each tenant has an independent Instructor authority witness"
    );
    assert_eq!(
        fixture
            .store
            .preflight_curriculum_adoption(fixture.context, fixture.learner_session)
            .await,
        Err(StoreError::Forbidden),
        "learner cannot discover or operate the adoption capability"
    );
    assert_eq!(
        fixture
            .store
            .preflight_curriculum_adoption(fixture.context, fixture.sysadmin_session)
            .await,
        Err(StoreError::Forbidden),
        "unrelated Sysadmin cannot substitute for the destination Instructor"
    );

    let broker = sqlx::query(
        "SELECT rolcanlogin, rolinherit, rolbypassrls \
         FROM pg_roles WHERE rolname='ple_curriculum_adoption_broker'",
    )
    .fetch_one(&fixture.pool)
    .await
    .expect("B2 broker role");
    assert!(!broker.try_get::<bool, _>("rolcanlogin").expect("NOLOGIN"));
    assert!(!broker.try_get::<bool, _>("rolinherit").expect("NOINHERIT"));
    assert!(
        !broker
            .try_get::<bool, _>("rolbypassrls")
            .expect("NOBYPASSRLS")
    );

    let mut transaction = fixture
        .pool
        .begin()
        .await
        .expect("application probe transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("application role");
    assert!(
        sqlx::query("SELECT * FROM public.curriculum_adoption_receipt")
            .fetch_all(&mut *transaction)
            .await
            .is_err(),
        "application cannot read B2 receipt evidence directly"
    );
    assert!(
        sqlx::query("DELETE FROM public.curriculum_adoption_receipt")
            .execute(&mut *transaction)
            .await
            .is_err(),
        "application cannot mutate B2 receipt evidence directly"
    );
    transaction.rollback().await.expect("probe rollback");
}
