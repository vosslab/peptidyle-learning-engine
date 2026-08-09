//! PostgreSQL pool construction and portable error classification.

use std::future::Future;
use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::StoreError;

const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const MAX_LIFETIME: Duration = Duration::from_secs(30 * 60);
const TRANSACTION_ATTEMPTS: u8 = 3;

fn pool_options(max_connections: u32) -> PgPoolOptions {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .idle_timeout(Some(IDLE_TIMEOUT))
        .max_lifetime(Some(MAX_LIFETIME))
}

/// Builds the bounded lazy application pool.
pub fn lazy_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    pool_options(8).connect_lazy(database_url)
}

/// Connects the bounded dedicated QTI grader pool.
pub(super) async fn connect_grader_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    pool_options(4).connect(database_url).await
}

pub(super) fn is_connection_error(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Io(_)
            | sqlx::Error::Tls(_)
            | sqlx::Error::Protocol(_)
            | sqlx::Error::PoolTimedOut
            | sqlx::Error::PoolClosed
            | sqlx::Error::WorkerCrashed
            | sqlx::Error::BeginFailed
    ) || matches!(
        error,
        sqlx::Error::Database(database_error)
            if matches!(database_error.code().as_deref(), Some(code) if code.starts_with("08") || matches!(code, "57P01" | "57P02" | "57P03" | "53300"))
    )
}

pub(super) fn map_sqlx_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database_error) = &error
        && let Some(mapped) = map_database_error(
            database_error.code().as_deref(),
            database_error.constraint(),
        )
    {
        return mapped;
    }
    StoreError::Unavailable("database operation failed".to_string())
}

fn map_database_error(code: Option<&str>, constraint: Option<&str>) -> Option<StoreError> {
    match code {
        Some("40001") | Some("40P01") => Some(StoreError::RetryableTransaction),
        Some("23505") if constraint == Some("problem_version_linear_chain_idx") => {
            Some(StoreError::Conflict)
        }
        Some("23505") => Some(StoreError::AlreadyExists),
        Some("23503") => Some(StoreError::InvalidRecord(constraint_message(
            constraint,
            "foreign key",
        ))),
        Some("23514") => Some(StoreError::InvalidRecord(constraint_message(
            constraint, "check",
        ))),
        Some("22023") => Some(StoreError::InvalidRecord(
            "database capability arguments are invalid".to_string(),
        )),
        Some("55000") => Some(StoreError::Conflict),
        Some("42501") => Some(StoreError::Forbidden),
        _ => None,
    }
}

/// Replays a complete operation only after PostgreSQL aborts its transaction.
///
/// The operation must begin and finish its transaction inside the returned
/// future. Retrying a statement on an already-aborted transaction is invalid,
/// and retrying connection failures could duplicate an ambiguously committed
/// operation, so neither is permitted here.
pub(super) async fn retry_transaction<T, F, Fut>(mut operation: F) -> Result<T, StoreError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, StoreError>>,
{
    for attempt in 1..=TRANSACTION_ATTEMPTS {
        match operation().await {
            Err(StoreError::RetryableTransaction) if attempt < TRANSACTION_ATTEMPTS => continue,
            result => return result,
        }
    }
    unreachable!("the bounded transaction retry loop always returns")
}

fn constraint_message(constraint: Option<&str>, kind: &str) -> String {
    match constraint {
        Some(name) => format!("database {kind} constraint {name} was violated"),
        None => format!("database {kind} constraint was violated"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tokio::sync::Barrier;

    use super::*;

    #[test]
    fn constraint_messages_never_depend_on_database_error_text() {
        assert_eq!(
            constraint_message(Some("attempt_credit_check"), "check"),
            "database check constraint attempt_credit_check was violated"
        );
        assert_eq!(
            constraint_message(None, "foreign key"),
            "database foreign key constraint was violated"
        );
    }

    #[test]
    fn invalid_parameter_value_is_a_safe_invalid_record() {
        assert_eq!(
            map_database_error(Some("22023"), Some("private_capability_detail")),
            Some(StoreError::InvalidRecord(
                "database capability arguments are invalid".to_string(),
            ))
        );
    }

    #[test]
    fn connection_class_errors_are_not_schema_incompatibilities() {
        assert!(is_connection_error(&sqlx::Error::PoolTimedOut));
        assert!(is_connection_error(&sqlx::Error::PoolClosed));
        let options = pool_options(8);
        assert_eq!(options.get_max_connections(), 8);
        assert_eq!(options.get_acquire_timeout(), Duration::from_secs(5));
        assert_eq!(
            options.get_idle_timeout(),
            Some(Duration::from_secs(10 * 60))
        );
        assert_eq!(
            options.get_max_lifetime(),
            Some(Duration::from_secs(30 * 60))
        );
    }

    #[tokio::test]
    async fn retry_transaction_stops_after_three_aborted_attempts() {
        let attempts = AtomicUsize::new(0);
        let result = retry_transaction(|| {
            attempts.fetch_add(1, Ordering::Relaxed);
            std::future::ready(Err::<(), _>(StoreError::RetryableTransaction))
        })
        .await;
        assert_eq!(result, Err(StoreError::RetryableTransaction));
        assert_eq!(attempts.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    #[ignore = "requires the disposable PostgreSQL acceptance database"]
    async fn concurrent_serialization_failure_is_retried_and_commits() {
        let database_url = std::env::var("PLE_TEST_DATABASE_URL")
            .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
        let pool = pool_options(4)
            .connect(&database_url)
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
        sqlx::query(
            "INSERT INTO public.ple_transaction_retry_test (id, value) VALUES (1, 0), (2, 0)",
        )
        .execute(&pool)
        .await
        .expect("seed retry fixture");

        let barrier = Arc::new(Barrier::new(2));
        let first_attempts = Arc::new(AtomicUsize::new(0));
        let second_attempts = Arc::new(AtomicUsize::new(0));
        let run = |own: i32,
                   observed: i32,
                   attempts: Arc<AtomicUsize>,
                   barrier: Arc<Barrier>,
                   pool: PgPool| async move {
            retry_transaction(|| {
                let pool = pool.clone();
                let barrier = barrier.clone();
                let first_attempt = attempts.fetch_add(1, Ordering::Relaxed) == 0;
                async move {
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
                    transaction.commit().await.map_err(map_sqlx_error)
                }
            })
            .await
        };
        let (first, second) = tokio::join!(
            run(1, 2, first_attempts.clone(), barrier.clone(), pool.clone()),
            run(2, 1, second_attempts.clone(), barrier, pool.clone())
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
}
