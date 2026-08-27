//! Disposable PostgreSQL oracle for transaction retry semantics.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use tokio::sync::{Barrier, Notify};

use super::*;

fn acceptance_runtime() -> acceptance_runtime::AcceptanceRuntime {
    acceptance_runtime::AcceptanceRuntime::load()
        .unwrap_or_else(|error| panic!("acceptance runtime is required and invalid: {error}"))
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn concurrent_serialization_failure_is_retried_and_commits() {
    let runtime = acceptance_runtime();
    let database_url = runtime.admin_url().expose();
    let pool = pool_options(4)
        .connect(database_url)
        .await
        .expect("connect retry acceptance pool");
    sqlx::query("DROP TABLE IF EXISTS public.ple_transaction_retry_test")
        .execute(&pool)
        .await
        .expect("remove stale retry fixture");
    sqlx::query(
        "CREATE TABLE public.ple_transaction_retry_test (\
             id integer PRIMARY KEY, value integer NOT NULL\
         )",
    )
    .execute(&pool)
    .await
    .expect("create retry fixture");
    sqlx::query("INSERT INTO public.ple_transaction_retry_test (id, value) VALUES (1, 0), (2, 0)")
        .execute(&pool)
        .await
        .expect("seed retry fixture");

    let barrier = Arc::new(Barrier::new(2));
    let initial_commit_complete = Arc::new(AtomicBool::new(false));
    let initial_commit_notifier = Arc::new(Notify::new());
    let first_attempts = Arc::new(AtomicUsize::new(0));
    let second_attempts = Arc::new(AtomicUsize::new(0));
    let run = |own: i32,
               observed: i32,
               attempts: Arc<AtomicUsize>,
               barrier: Arc<Barrier>,
               initial_commit_complete: Arc<AtomicBool>,
               initial_commit_notifier: Arc<Notify>,
               pool: PgPool| async move {
        retry_transaction(|| {
            let pool = pool.clone();
            let barrier = barrier.clone();
            let initial_commit_complete = initial_commit_complete.clone();
            let initial_commit_notifier = initial_commit_notifier.clone();
            let first_attempt = attempts.fetch_add(1, Ordering::Relaxed) == 0;
            async move {
                if !first_attempt {
                    while !initial_commit_complete.load(Ordering::Acquire) {
                        let notified = initial_commit_notifier.notified();
                        if initial_commit_complete.load(Ordering::Acquire) {
                            break;
                        }
                        notified.await;
                    }
                }
                let mut transaction = pool.begin().await.map_err(map_sqlx_error)?;
                sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                let _: i32 = sqlx::query_scalar(
                    "SELECT value FROM public.ple_transaction_retry_test WHERE id = $1",
                )
                .bind(observed)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                if first_attempt {
                    barrier.wait().await;
                }
                sqlx::query(
                    "UPDATE public.ple_transaction_retry_test SET value = value + 1 \
                     WHERE id = $1",
                )
                .bind(own)
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                if first_attempt {
                    initial_commit_complete.store(true, Ordering::Release);
                    initial_commit_notifier.notify_waiters();
                }
                Ok(())
            }
        })
        .await
    };
    let (first, second) = tokio::join!(
        run(
            1,
            2,
            first_attempts.clone(),
            barrier.clone(),
            initial_commit_complete.clone(),
            initial_commit_notifier.clone(),
            pool.clone(),
        ),
        run(
            2,
            1,
            second_attempts.clone(),
            barrier,
            initial_commit_complete,
            initial_commit_notifier,
            pool.clone(),
        )
    );
    first.expect("first serializable operation commits");
    second.expect("second serializable operation commits after retry");
    assert_eq!(
        first_attempts.load(Ordering::Relaxed) + second_attempts.load(Ordering::Relaxed),
        3,
        "exactly one transaction must be retried"
    );
    let total: i64 =
        sqlx::query_scalar("SELECT sum(value)::bigint FROM public.ple_transaction_retry_test")
            .fetch_one(&pool)
            .await
            .expect("read retry fixture result");
    assert_eq!(total, 2);
    sqlx::query("DROP TABLE public.ple_transaction_retry_test")
        .execute(&pool)
        .await
        .expect("remove retry fixture");
    pool.close().await;
}
