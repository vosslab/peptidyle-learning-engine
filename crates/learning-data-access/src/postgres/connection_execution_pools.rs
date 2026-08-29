//! Sealed PostgreSQL pools for accepted-submission execution capabilities.

use sqlx::postgres::PgPool;
use sqlx::{query, query_scalar};

use super::{
    LoginContract, connect_attested_pool, local_connect_options, verified_connect_options,
};

const EXECUTION_POOL_MAX_CONNECTIONS: u32 = 4;

const GENERIC_CLAIM_SIGNATURE: &str =
    "public.ple_claim_accepted_submission_execution_v1(uuid,uuid,integer)";
const EXACT_CLAIM_SIGNATURE: &str = "public.ple_claim_exact_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,uuid,uuid,integer)";
const INTERNAL_CLAIM_SIGNATURE: &str = "public.ple_claim_accepted_submission_execution_transition_v1(uuid,uuid,uuid,uuid,uuid,uuid,integer)";
const LOAD_SIGNATURE: &str =
    "public.ple_load_accepted_submission_execution_v2(uuid,uuid,uuid,uuid,bigint,uuid)";
const LOCK_SIGNATURE: &str =
    "public.ple_lock_accepted_submission_completion_v1(uuid,uuid,uuid,uuid,bigint,uuid)";
const COMMIT_SIGNATURE: &str = "public.ple_commit_accepted_submission_completion_v2(uuid,uuid,uuid,uuid,bigint,uuid,smallint,text,text,character,text,jsonb,character,text,character,text,jsonb,character,text,character,bigint,bigint,uuid,uuid,text,jsonb,character,text,jsonb,character,boolean,uuid,jsonb,bigint,uuid,integer)";
const FAIL_SIGNATURE: &str =
    "public.ple_fail_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,bigint,uuid,text,text)";

const PREFLIGHT_SQL: &str = "
    SELECT
        (
            SELECT count(*) = cardinality($1::text[])
                AND bool_and(procedure_id IS NOT NULL)
                AND bool_and(
                    pg_catalog.has_function_privilege(
                        current_user, procedure_id, 'EXECUTE'
                    )
                )
              FROM unnest($1::text[]) AS allowed(signature)
              CROSS JOIN LATERAL pg_catalog.to_regprocedure(allowed.signature)
                  AS procedure_id
        )
        AND NOT EXISTS (
            SELECT 1
              FROM unnest($2::text[]) AS denied(signature)
              CROSS JOIN LATERAL pg_catalog.to_regprocedure(denied.signature)
                  AS procedure_id
             WHERE procedure_id IS NULL
                OR pg_catalog.has_function_privilege(
                    current_user, procedure_id, 'EXECUTE'
                )
        )
    ";

#[derive(Clone, Copy)]
struct ExecutionPoolPreflight {
    set_role_sql: &'static str,
    allowed_signatures: &'static [&'static str],
    denied_signatures: &'static [&'static str],
}

const RECOVERY_PREFLIGHT: ExecutionPoolPreflight = ExecutionPoolPreflight {
    set_role_sql: "SET LOCAL ROLE ple_accepted_submission_execution",
    allowed_signatures: &[
        GENERIC_CLAIM_SIGNATURE,
        LOAD_SIGNATURE,
        LOCK_SIGNATURE,
        COMMIT_SIGNATURE,
        FAIL_SIGNATURE,
    ],
    denied_signatures: &[EXACT_CLAIM_SIGNATURE, INTERNAL_CLAIM_SIGNATURE],
};

const FAST_PATH_PREFLIGHT: ExecutionPoolPreflight = ExecutionPoolPreflight {
    set_role_sql: "SET LOCAL ROLE ple_accepted_submission_execution_fast_path",
    allowed_signatures: &[
        EXACT_CLAIM_SIGNATURE,
        LOAD_SIGNATURE,
        LOCK_SIGNATURE,
        COMMIT_SIGNATURE,
        FAIL_SIGNATURE,
    ],
    denied_signatures: &[GENERIC_CLAIM_SIGNATURE, INTERNAL_CLAIM_SIGNATURE],
};

/// An attested pool reserved for generic accepted-submission recovery claims.
///
/// Its private inner pool keeps recovery authority out of general
/// application-store construction (ASVS 8.2.1 and 13.2.2).
#[derive(Clone)]
pub struct AcceptedSubmissionRecoveryPool(PgPool);

impl AcceptedSubmissionRecoveryPool {
    pub(in crate::postgres) fn into_pool(self) -> PgPool {
        self.0
    }
}

/// An attested pool reserved for exact accepted-submission fast-path claims.
///
/// Its private inner pool keeps exact-target authority out of general
/// application-store construction (ASVS 8.2.1 and 13.2.2).
#[derive(Clone)]
pub struct AcceptedSubmissionFastPathPool(PgPool);

impl AcceptedSubmissionFastPathPool {
    pub(in crate::postgres) fn into_pool(self) -> PgPool {
        self.0
    }
}

/// Connects the production pool that can assume only generic recovery authority.
pub async fn accepted_submission_recovery_pool(
    database_url: &str,
) -> Result<AcceptedSubmissionRecoveryPool, sqlx::Error> {
    connect_execution_pool(
        database_url,
        LoginContract::AcceptedSubmissionRecovery,
        RECOVERY_PREFLIGHT,
        verified_connect_options,
    )
    .await
    .map(AcceptedSubmissionRecoveryPool)
}

/// Connects the disposable-stack pool with generic recovery authority.
pub async fn local_accepted_submission_recovery_pool(
    database_url: &str,
) -> Result<AcceptedSubmissionRecoveryPool, sqlx::Error> {
    connect_execution_pool(
        database_url,
        LoginContract::AcceptedSubmissionRecovery,
        RECOVERY_PREFLIGHT,
        local_connect_options,
    )
    .await
    .map(AcceptedSubmissionRecoveryPool)
}

/// Connects the production pool that can assume only exact-target claim authority.
pub async fn accepted_submission_fast_path_pool(
    database_url: &str,
) -> Result<AcceptedSubmissionFastPathPool, sqlx::Error> {
    connect_execution_pool(
        database_url,
        LoginContract::AcceptedSubmissionFastPath,
        FAST_PATH_PREFLIGHT,
        verified_connect_options,
    )
    .await
    .map(AcceptedSubmissionFastPathPool)
}

/// Connects the production Base Course pool under its host-only exact-target identity.
pub async fn base_course_accepted_submission_fast_path_pool(
    database_url: &str,
) -> Result<AcceptedSubmissionFastPathPool, sqlx::Error> {
    connect_execution_pool(
        database_url,
        LoginContract::BaseCourseAcceptedSubmissionFastPath,
        FAST_PATH_PREFLIGHT,
        verified_connect_options,
    )
    .await
    .map(AcceptedSubmissionFastPathPool)
}

/// Connects the disposable-stack pool with exact-target claim authority.
pub async fn local_accepted_submission_fast_path_pool(
    database_url: &str,
) -> Result<AcceptedSubmissionFastPathPool, sqlx::Error> {
    connect_execution_pool(
        database_url,
        LoginContract::AcceptedSubmissionFastPath,
        FAST_PATH_PREFLIGHT,
        local_connect_options,
    )
    .await
    .map(AcceptedSubmissionFastPathPool)
}

/// Connects the disposable Base Course pool under its host-only exact-target identity.
pub async fn local_base_course_accepted_submission_fast_path_pool(
    database_url: &str,
) -> Result<AcceptedSubmissionFastPathPool, sqlx::Error> {
    connect_execution_pool(
        database_url,
        LoginContract::BaseCourseAcceptedSubmissionFastPath,
        FAST_PATH_PREFLIGHT,
        local_connect_options,
    )
    .await
    .map(AcceptedSubmissionFastPathPool)
}

async fn connect_execution_pool(
    database_url: &str,
    contract: LoginContract,
    preflight: ExecutionPoolPreflight,
    connect_options: fn(
        &str,
        LoginContract,
    ) -> Result<sqlx::postgres::PgConnectOptions, sqlx::Error>,
) -> Result<PgPool, sqlx::Error> {
    let options = connect_options(database_url, contract)?;
    let pool = connect_attested_pool(options, contract, EXECUTION_POOL_MAX_CONNECTIONS).await?;
    preflight_execution_capability(&pool, preflight).await?;
    Ok(pool)
}

async fn preflight_execution_capability(
    pool: &PgPool,
    preflight: ExecutionPoolPreflight,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    query("SET TRANSACTION READ ONLY")
        .execute(&mut *transaction)
        .await?;
    query(preflight.set_role_sql)
        .execute(&mut *transaction)
        .await?;
    let is_attested: bool = query_scalar(PREFLIGHT_SQL)
        .bind(preflight.allowed_signatures)
        .bind(preflight.denied_signatures)
        .fetch_one(&mut *transaction)
        .await?;
    if !is_attested {
        return Err(sqlx::Error::Protocol(
            "accepted-submission execution capability preflight failed".to_string(),
        ));
    }
    transaction.commit().await
}

#[cfg(test)]
pub(super) fn execution_pool_max_connections() -> u32 {
    EXECUTION_POOL_MAX_CONNECTIONS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_surfaces_are_closed_role_specific_function_contracts() {
        assert_eq!(
            RECOVERY_PREFLIGHT.set_role_sql,
            "SET LOCAL ROLE ple_accepted_submission_execution"
        );
        assert_eq!(
            FAST_PATH_PREFLIGHT.set_role_sql,
            "SET LOCAL ROLE ple_accepted_submission_execution_fast_path"
        );
        assert_eq!(
            RECOVERY_PREFLIGHT.allowed_signatures,
            [
                GENERIC_CLAIM_SIGNATURE,
                LOAD_SIGNATURE,
                LOCK_SIGNATURE,
                COMMIT_SIGNATURE,
                FAIL_SIGNATURE,
            ]
        );
        assert_eq!(
            FAST_PATH_PREFLIGHT.allowed_signatures,
            [
                EXACT_CLAIM_SIGNATURE,
                LOAD_SIGNATURE,
                LOCK_SIGNATURE,
                COMMIT_SIGNATURE,
                FAIL_SIGNATURE,
            ]
        );
        assert_eq!(
            RECOVERY_PREFLIGHT.denied_signatures,
            [EXACT_CLAIM_SIGNATURE, INTERNAL_CLAIM_SIGNATURE]
        );
        assert_eq!(
            FAST_PATH_PREFLIGHT.denied_signatures,
            [GENERIC_CLAIM_SIGNATURE, INTERNAL_CLAIM_SIGNATURE]
        );
    }
}
