//! PostgreSQL pool construction and portable error classification.

use std::str::FromStr;
use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgConnection, PgPool, PgPoolOptions, PgSslMode};
use sqlx::types::Json;
use sqlx::{Executor, Postgres, Row, Transaction};

use question_model::{AccountId, AssessmentId, CourseInstanceId};

use crate::{SessionTokenHash, StoreError};

/// Opens an ordinary application transaction under the originating session.
pub(super) async fn begin_authenticated_transaction(
    pool: &PgPool,
    token: SessionTokenHash,
) -> Result<Transaction<'_, Postgres>, StoreError> {
    let mut transaction = pool.begin().await.map_err(map_sqlx_error)?;
    install_application_session(&mut transaction, token).await?;
    Ok(transaction)
}

/// Installs session authority after the caller sets any transaction isolation.
pub(super) async fn install_application_session(
    transaction: &mut Transaction<'_, Postgres>,
    token: SessionTokenHash,
) -> Result<(), StoreError> {
    // ASVS 1.2.4 and 8.3.3: bind the originating session; roles stay transaction-local.
    sqlx::query("SET LOCAL ROLE ple_auth")
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    let session = sqlx::query(
        "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
    )
    .bind(token.to_string())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if session.is_none() {
        return Err(StoreError::Forbidden);
    }
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    Ok(())
}

#[path = "connection_contract.rs"]
mod connection_contract;
use connection_contract::*;

const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const MAX_LIFETIME: Duration = Duration::from_secs(30 * 60);
const STANDARD_POOL_MAX_CONNECTIONS: u32 = 8;

/// Fixed least-privilege identities accepted by production process pools.
///
/// These login roles are deployment-owned. Schema migrations continue to own
/// only the NOLOGIN capabilities that the process assumes transaction-locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionLoginProfile {
    /// Browser/API process: course data plus passwordless account sessions.
    Api,
    /// Assessment Attempt expiry worker: preparation and commit procedures only.
    AssessmentAttemptExpiryWorker,
    /// Course-retention executor: stored due-action reads and one-way Course
    /// mark-inactive, Student-record archive, and deletion procedures only.
    CourseRetentionExecutor,
    /// Retention notifier: three direct SECURITY DEFINER receipt procedures;
    /// it has no capability-role membership or raw-table authority.
    CourseRetentionNotifier,
    /// Dedicated immutable public Question Image Asset publisher: its exact private
    /// source-to-public rendition claim and activation procedures only.
    PublicAssetPublisher,
}

fn pool_options(max_connections: u32) -> PgPoolOptions {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .idle_timeout(Some(IDLE_TIMEOUT))
        .max_lifetime(Some(MAX_LIFETIME))
}

/// Builds the bounded lazy application pool.
pub fn lazy_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    pool_options(STANDARD_POOL_MAX_CONNECTIONS).connect_lazy(database_url)
}

/// Builds a lazy production pool with a verified transport and per-connection
/// least-privilege login enforcement.
///
/// The callback runs for every replacement connection, so a credential or
/// server-side role change cannot silently widen a long-running process.
pub fn production_pool(
    database_url: &str,
    profile: ProductionLoginProfile,
) -> Result<PgPool, sqlx::Error> {
    let contract = LoginContract::Production(profile);
    let options = verified_connect_options(database_url, contract)?;
    Ok(attested_pool(
        options,
        contract,
        STANDARD_POOL_MAX_CONNECTIONS,
    ))
}

/// Builds a local-development pool while preserving the exact service-login
/// and effective-capability attestation. The caller chooses this only for the
/// disposable plaintext stack; production always uses [`production_pool`].
pub fn local_development_pool(
    database_url: &str,
    profile: ProductionLoginProfile,
) -> Result<PgPool, sqlx::Error> {
    let contract = LoginContract::Production(profile);
    let options = local_connect_options(database_url, contract)?;
    Ok(attested_pool(
        options,
        contract,
        STANDARD_POOL_MAX_CONNECTIONS,
    ))
}

fn attested_pool(
    options: PgConnectOptions,
    contract: LoginContract,
    max_connections: u32,
) -> PgPool {
    attested_pool_options(contract, max_connections).connect_lazy_with(options)
}

fn attested_pool_options(contract: LoginContract, max_connections: u32) -> PgPoolOptions {
    pool_options(max_connections).after_connect(move |connection, _metadata| {
        Box::pin(async move { verify_login_authority(connection, contract).await })
    })
}

fn local_connect_options(
    database_url: &str,
    contract: LoginContract,
) -> Result<PgConnectOptions, sqlx::Error> {
    let options = PgConnectOptions::from_str(database_url)
        .map_err(|_| sqlx::Error::Configuration("local database URL is invalid".into()))?;
    if options.get_username() != contract.expected_login() {
        return Err(sqlx::Error::Configuration(
            format!(
                "local database URL must use the {} login",
                contract.expected_login()
            )
            .into(),
        ));
    }
    Ok(options)
}

fn verified_connect_options(
    database_url: &str,
    contract: LoginContract,
) -> Result<PgConnectOptions, sqlx::Error> {
    let options = PgConnectOptions::from_str(database_url)
        .map_err(|_| sqlx::Error::Configuration("production database URL is invalid".into()))?;
    if !matches!(options.get_ssl_mode(), PgSslMode::VerifyFull) {
        return Err(sqlx::Error::Configuration(
            "production database URL must set sslmode=verify-full".into(),
        ));
    }
    if options.get_username() != contract.expected_login() {
        return Err(sqlx::Error::Configuration(
            format!(
                "production database URL must use the {} login",
                contract.expected_login()
            )
            .into(),
        ));
    }
    Ok(options)
}

async fn verify_login_authority(
    connection: &mut PgConnection,
    contract: LoginContract,
) -> Result<(), sqlx::Error> {
    let row = connection
        .fetch_one(
            "SELECT current_user AS current_user, session_user AS session_user, \
             r.rolsuper, r.rolcreatedb, r.rolcreaterole, r.rolinherit, \
             r.rolreplication, r.rolbypassrls, r.rolcanlogin, \
             COALESCE(( \
                 SELECT jsonb_agg(jsonb_build_object( \
                     'role_name', granted.rolname, \
                     'admin_option', membership.admin_option, \
                     'inherit_option', membership.inherit_option, \
                     'set_option', membership.set_option \
                 ) ORDER BY granted.rolname) \
                 FROM pg_catalog.pg_auth_members AS membership \
                 JOIN pg_catalog.pg_roles AS granted \
                   ON granted.oid = membership.roleid \
                 WHERE membership.member = r.oid \
             ), '[]'::jsonb) AS direct_memberships \
             FROM pg_catalog.pg_roles AS r WHERE r.rolname = current_user",
        )
        .await?;
    let authority = LoginAuthority {
        current_user: row.try_get("current_user")?,
        session_user: row.try_get("session_user")?,
        superuser: row.try_get("rolsuper")?,
        create_database: row.try_get("rolcreatedb")?,
        create_role: row.try_get("rolcreaterole")?,
        inherit: row.try_get("rolinherit")?,
        replication: row.try_get("rolreplication")?,
        bypass_rls: row.try_get("rolbypassrls")?,
        can_login: row.try_get("rolcanlogin")?,
        direct_memberships: {
            let Json(memberships): Json<Vec<DirectMembership>> =
                row.try_get("direct_memberships")?;
            memberships
        },
    };
    if !login_authority_matches(&authority, contract) {
        Err(sqlx::Error::Protocol(
            "database login violates the process authority contract".to_string(),
        ))?;
    }

    for expected in contract.expected_capabilities() {
        let authority = fetch_capability_authority(connection, expected.role_name).await?;
        if !capability_authority_matches(&authority, expected) {
            return Err(sqlx::Error::Protocol(
                "database capability role violates the process authority contract".to_string(),
            ));
        }
    }
    let expected_functions = contract.expected_effective_functions();
    if !expected_functions.is_empty() {
        let set_role = contract.set_function_inventory_role_sql();
        if let Some(set_role) = set_role {
            sqlx::query(set_role).execute(&mut *connection).await?;
        }
        let actual_functions = sqlx::query_scalar::<_, String>(
            "SELECT p.oid::regprocedure::text \
             FROM pg_catalog.pg_proc AS p \
             JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = p.pronamespace \
             WHERE namespace.nspname LIKE 'ple\\_%' ESCAPE '\\' \
               AND has_function_privilege(current_user, p.oid, 'EXECUTE') \
             ORDER BY p.oid::regprocedure::text",
        )
        .fetch_all(&mut *connection)
        .await?;
        if set_role.is_some() {
            sqlx::query("RESET ROLE").execute(&mut *connection).await?;
        }
        let mut expected_functions = expected_functions
            .iter()
            .map(|function| (*function).to_string())
            .collect::<Vec<_>>();
        expected_functions.sort_unstable();
        if actual_functions != expected_functions {
            return Err(sqlx::Error::Protocol(
                "database login effective function authority violates the process contract"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

async fn fetch_capability_authority(
    connection: &mut PgConnection,
    expected_role: &str,
) -> Result<CapabilityAuthority, sqlx::Error> {
    let row = sqlx::query(
        "SELECT r.rolname AS role_name, r.rolsuper, r.rolcreatedb, r.rolcreaterole, \
             r.rolinherit, r.rolreplication, r.rolbypassrls, r.rolcanlogin, \
             COALESCE(( \
                 SELECT jsonb_agg(jsonb_build_object( \
                     'role_name', granted.rolname, \
                     'admin_option', membership.admin_option, \
                     'inherit_option', membership.inherit_option, \
                     'set_option', membership.set_option \
                 ) ORDER BY granted.rolname) \
                 FROM pg_catalog.pg_auth_members AS membership \
                 JOIN pg_catalog.pg_roles AS granted \
                   ON granted.oid = membership.roleid \
                 WHERE membership.member = r.oid \
             ), '[]'::jsonb) AS direct_memberships \
             FROM pg_catalog.pg_roles AS r WHERE r.rolname = $1",
    )
    .bind(expected_role)
    .fetch_optional(connection)
    .await?
    .ok_or_else(|| {
        sqlx::Error::Protocol(
            "database capability role violates the process authority contract".to_string(),
        )
    })?;
    Ok(CapabilityAuthority {
        role_name: row.try_get("role_name")?,
        superuser: row.try_get("rolsuper")?,
        create_database: row.try_get("rolcreatedb")?,
        create_role: row.try_get("rolcreaterole")?,
        inherit: row.try_get("rolinherit")?,
        replication: row.try_get("rolreplication")?,
        bypass_rls: row.try_get("rolbypassrls")?,
        can_login: row.try_get("rolcanlogin")?,
        direct_memberships: {
            let Json(memberships): Json<Vec<DirectMembership>> =
                row.try_get("direct_memberships")?;
            memberships
        },
    })
}

fn login_authority_matches(authority: &LoginAuthority, contract: LoginContract) -> bool {
    authority.current_user == contract.expected_login()
        && authority.session_user == contract.expected_login()
        && !authority.superuser
        && !authority.create_database
        && !authority.create_role
        && !authority.inherit
        && !authority.replication
        && !authority.bypass_rls
        && authority.can_login
        && direct_memberships_match(
            &authority.direct_memberships,
            contract.expected_memberships(),
        )
}

/// Compare PostgreSQL catalog memberships as one exact, metadata-bearing set.
///
/// PostgreSQL emits the member roles in catalog order. The contract owns no
/// ordering requirement, so sorting prevents a legitimate replacement
/// connection from failing while duplicate names still fail closed.
fn direct_memberships_match(actual: &[DirectMembership], expected: &[ExpectedMembership]) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    let mut actual_by_name = actual.iter().collect::<Vec<_>>();
    actual_by_name.sort_unstable_by(|left, right| left.role_name.cmp(&right.role_name));
    if actual_by_name
        .windows(2)
        .any(|pair| pair[0].role_name == pair[1].role_name)
    {
        return false;
    }
    let mut expected_by_name = expected.iter().collect::<Vec<_>>();
    expected_by_name.sort_unstable_by(|left, right| left.role_name.cmp(right.role_name));
    if expected_by_name
        .windows(2)
        .any(|pair| pair[0].role_name == pair[1].role_name)
    {
        return false;
    }
    actual_by_name
        .iter()
        .zip(expected_by_name)
        .all(|(actual, expected)| {
            actual.role_name == expected.role_name
                && !actual.admin_option
                && !actual.inherit_option
                && actual.set_option == expected.set_option
        })
}

fn capability_authority_matches(
    authority: &CapabilityAuthority,
    expected: &ExpectedMembership,
) -> bool {
    authority.role_name == expected.role_name
        && !authority.superuser
        && !authority.create_database
        && !authority.create_role
        && !authority.inherit
        && !authority.replication
        && !authority.bypass_rls
        && !authority.can_login
        // A capability role cannot itself have a grant path: otherwise SET
        // LOCAL ROLE makes any of its nested roles effective as well.  This
        // catches ADMIN, INHERIT, and SET delegation metadata too because the
        // expected direct-membership set is exactly empty.
        && authority.direct_memberships.is_empty()
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

pub(super) fn parse_account_id(value: String) -> Result<AccountId, StoreError> {
    AccountId::new(value).map_err(|message| StoreError::InvalidRecord(message.to_string()))
}

pub(super) fn parse_course_instance_id(value: String) -> Result<CourseInstanceId, StoreError> {
    CourseInstanceId::new(value).map_err(|message| StoreError::InvalidRecord(message.to_string()))
}

pub(super) fn parse_assessment_id(value: String) -> Result<AssessmentId, StoreError> {
    AssessmentId::new(value).map_err(|message| StoreError::InvalidRecord(message.to_string()))
}

pub(super) fn map_sqlx_error(error: sqlx::Error) -> StoreError {
    // Log driver categories and schema metadata, never source text or row values.
    let error_kind = match &error {
        sqlx::Error::Configuration(_) => "configuration",
        sqlx::Error::InvalidArgument(_) => "invalid_argument",
        sqlx::Error::Database(_) => "database",
        sqlx::Error::Io(_) => "io",
        sqlx::Error::Tls(_) => "tls",
        sqlx::Error::Protocol(_) => "protocol",
        sqlx::Error::RowNotFound => "row_not_found",
        sqlx::Error::TypeNotFound { .. } => "type_not_found",
        sqlx::Error::ColumnIndexOutOfBounds { .. } => "column_index_out_of_bounds",
        sqlx::Error::ColumnNotFound(_) => "column_not_found",
        sqlx::Error::ColumnDecode { .. } => "column_decode",
        sqlx::Error::Encode(_) => "encode",
        sqlx::Error::Decode(_) => "decode",
        sqlx::Error::AnyDriverError(_) => "any_driver",
        sqlx::Error::PoolTimedOut => "pool_timed_out",
        sqlx::Error::PoolClosed => "pool_closed",
        sqlx::Error::WorkerCrashed => "worker_crashed",
        sqlx::Error::BeginFailed => "begin_failed",
        sqlx::Error::InvalidSavePointStatement => "invalid_savepoint_statement",
        _ => "other",
    };
    let column = match &error {
        sqlx::Error::ColumnDecode { index, .. } | sqlx::Error::ColumnNotFound(index) => {
            Some(index.as_str())
        }
        _ => None,
    };
    let column_index = match &error {
        sqlx::Error::ColumnIndexOutOfBounds { index, .. } => Some(*index),
        _ => None,
    };
    let unexpected_null = match &error {
        sqlx::Error::ColumnDecode { source, .. } | sqlx::Error::Decode(source) => {
            source.is::<sqlx::error::UnexpectedNullError>()
        }
        _ => false,
    };
    let database_error = error.as_database_error();
    tracing::error!(
        event = "database_operation_failed",
        error_kind,
        column,
        ?column_index,
        unexpected_null,
        sqlstate = database_error.and_then(|error| error.code()).as_deref(),
        constraint = database_error.and_then(|error| error.constraint()),
    );
    if let sqlx::Error::Database(database_error) = &error
        && let Some(mapped) = map_database_error(
            database_error.code().as_deref(),
            database_error.constraint(),
        )
    {
        return mapped;
    }
    StoreError::Unavailable(format!("database operation failed: {error}"))
}

fn map_database_error(code: Option<&str>, constraint: Option<&str>) -> Option<StoreError> {
    match code {
        Some("40001") | Some("40P01") => Some(StoreError::RetryableTransaction),
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
        Some("55000") => Some(StoreError::LifecycleConflict),
        Some("42501") => Some(StoreError::Forbidden),
        _ => None,
    }
}

fn constraint_message(constraint: Option<&str>, kind: &str) -> String {
    match constraint {
        Some(name) => format!("database {kind} constraint {name} was violated"),
        None => format!("database {kind} constraint was violated"),
    }
}
