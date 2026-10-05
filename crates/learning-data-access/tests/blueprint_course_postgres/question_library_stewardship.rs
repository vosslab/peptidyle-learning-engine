//! Connected Library impact-notice delivery through the shipped Watch inbox.

use learning_data_access::postgres::{
    PostgresLibraryImpactNoticeStore, PostgresLibraryWatchNotificationStore,
    PostgresQuestionWatchStore,
};
use learning_data_access::{
    LibraryImpactNoticeStore, LibraryObjectTarget, LibraryWatchActivity, LibraryWatchInboxStore,
    QuestionWatchStore,
};
use question_model::LibraryObjectKind;

use super::*;

fn question_target() -> LibraryObjectTarget {
    LibraryObjectTarget {
        kind: LibraryObjectKind::Question,
        public_id: QUESTION.parse().expect("fixture Question ID"),
    }
}

async fn vet_seeded_instructor(admin: &sqlx::postgres::PgPool) {
    let mut transaction = admin.begin().await.expect("vetting transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("vetting private owner");
    let sysadmin_id: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('U00000009', 'sysadmin', pg_catalog.transaction_timestamp()) RETURNING account_id",
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("vetting Sysadmin");
    sqlx::query("INSERT INTO ple_private.instructor_profile (account_id, first_name, last_name, affiliation) VALUES ($1, 'Library', 'Instructor', 'Test University')")
        .bind(instructor_account_id())
        .execute(&mut *transaction)
        .await
        .expect("Instructor Profile");
    sqlx::query("SET LOCAL ROLE ple_audit_owner")
        .execute(&mut *transaction)
        .await
        .expect("vetting audit owner");
    sqlx::query("SELECT ple_audit.record_instructor_account_creation_event($1, $2)")
        .bind(instructor_account_id())
        .bind(&sysadmin_id)
        .execute(&mut *transaction)
        .await
        .expect("Instructor creation evidence");
    transaction
        .commit()
        .await
        .expect("Instructor fixture commit");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn question_library_impact_notice_reaches_a_watcher() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    super::blueprint_course_postgres_support::seed_if_needed(&admin).await;
    vet_seeded_instructor(&admin).await;

    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let application = lazy_pool(&application_url).expect("application pool");
    let target = question_target();
    let watch = PostgresQuestionWatchStore::new(application.clone());
    let projection = watch
        .set_current_question_watch(token(), &target.public_id, true)
        .await
        .expect("Question Watch");
    assert!(projection.watching);

    let notices = PostgresLibraryImpactNoticeStore::new(application.clone());
    let notice_id = notices
        .create_impact_notice(
            token(),
            &target,
            Some(1),
            "Revision 1 uses an outdated inhibitor concentration.",
        )
        .await
        .expect("impact notice");

    let inbox = PostgresLibraryWatchNotificationStore::new(application);
    let notifications = inbox
        .library_watch_notifications(token(), 20)
        .await
        .expect("Watch inbox");
    assert!(notifications.iter().any(|notification| {
        notification.target_public_id == target.public_id
            && matches!(
                notification.activity,
                LibraryWatchActivity::ImpactNotice {
                    impact_notice_id: delivered,
                    ..
                } if delivered == notice_id
            )
    }));
}
