//! Connected GitHub-like Library stewardship through the shipped Watch inbox.

use learning_data_access::postgres::{
    PostgresLibraryDiscussionStore, PostgresLibraryWatchNotificationStore,
    PostgresQuestionWatchStore,
};
use learning_data_access::{
    LibraryDiscussionStore, LibraryDiscussionTarget, LibraryWatchActivity, LibraryWatchInboxStore,
    QuestionWatchStore,
};
use question_model::LibraryObjectKind;

use super::*;

fn question_target() -> LibraryDiscussionTarget {
    LibraryDiscussionTarget {
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
    sqlx::query("SET LOCAL ROLE ple_audit_owner")
        .execute(&mut *transaction)
        .await
        .expect("vetting audit owner");
    let decision_id: Uuid = sqlx::query_scalar(
        "SELECT ple_audit.record_completed_instructor_identity_vetting_decision(\
             'library-stewardship-instructor@example.test', 'Library Stewardship Instructor', $1)",
    )
    .bind(&sysadmin_id)
    .fetch_one(&mut *transaction)
    .await
    .expect("Instructor vetting decision");
    sqlx::query("SELECT ple_audit.record_instructor_account_creation_event($1, $2, $3)")
        .bind(instructor_account_id())
        .bind(&sysadmin_id)
        .bind(decision_id)
        .execute(&mut *transaction)
        .await
        .expect("Instructor creation evidence");
    transaction.commit().await.expect("vetting commit");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn question_library_stewardship_delivers_threads_and_notices_to_a_watcher() {
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

    let discussion = PostgresLibraryDiscussionStore::new(application.clone());
    let thread_id = discussion
        .create_improvement_thread(
            token(),
            &target,
            "The prompt should name the enzyme and the inhibitor.",
        )
        .await
        .expect("improvement thread");
    let notice_id = discussion
        .create_impact_notice(
            token(),
            &target,
            Some(1),
            "Revision 1 uses an outdated inhibitor concentration.",
        )
        .await
        .expect("impact notice");
    let view = discussion
        .library_discussion_view(token(), &target)
        .await
        .expect("discussion view");
    assert!(view.threads.iter().any(|thread| {
        thread.thread_id == thread_id
            && thread
                .posts
                .iter()
                .any(|post| post.body == "The prompt should name the enzyme and the inhibitor.")
    }));
    assert!(view.impact_notices.iter().any(|notice| {
        notice.impact_notice_id == notice_id
            && notice.body == "Revision 1 uses an outdated inhibitor concentration."
    }));

    let inbox = PostgresLibraryWatchNotificationStore::new(application);
    let notifications = inbox
        .library_watch_notifications(token(), 20)
        .await
        .expect("Watch inbox");
    assert!(notifications.iter().any(|notification| {
        notification.target_public_id == target.public_id
            && matches!(
                notification.activity,
                LibraryWatchActivity::ImprovementThread {
                    thread_id: delivered,
                    ..
                } if delivered == thread_id
            )
    }));
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
