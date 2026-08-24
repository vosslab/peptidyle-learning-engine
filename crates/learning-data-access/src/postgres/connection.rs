//! PostgreSQL pool construction and portable error classification.

use std::future::Future;
use std::str::FromStr;
use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgConnection, PgPool, PgPoolOptions, PgSslMode};
use sqlx::types::Json;
use sqlx::{Executor, Row};

use crate::StoreError;

#[path = "connection_contract.rs"]
mod connection_contract;
use connection_contract::*;

const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const MAX_LIFETIME: Duration = Duration::from_secs(30 * 60);
const STANDARD_POOL_MAX_CONNECTIONS: u32 = 8;
const BASE_COURSE_POOL_MAX_CONNECTIONS: u32 = 1;
const GRADER_POOL_MAX_CONNECTIONS: u32 = 4;
const TRANSACTION_ATTEMPTS: u8 = 3;

/// Fixed least-privilege identities accepted by production process pools.
///
/// These login roles are deployment-owned. Schema migrations continue to own
/// only the NOLOGIN capabilities that the process assumes transaction-locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionLoginProfile {
    /// Browser/API process: tenant data plus passwordless account sessions.
    Api,
    /// Background worker: tenant data only; never account authentication.
    Worker,
    /// Invitation delivery dispatcher: only the function-only outbox capability.
    InvitationDeliveryWorker,
    /// Public asset publisher: no tenant/application tables, only its queue capability.
    Publisher,
}

/// An attested pool reserved for the short-lived Base Course installer.
///
/// The inner pool remains private so callers cannot accidentally pass an API,
/// migration, or ordinary application pool to the installer facade.
#[derive(Clone)]
pub struct BaseCourseInstallerPool(PgPool);

impl BaseCourseInstallerPool {
    pub(super) fn acquire_pool(&self) -> &PgPool {
        &self.0
    }
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

/// Builds a local-development pool while preserving the exact process-login
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

/// Builds the only production installer pool accepted by the Base Course
/// installer facade.
pub fn base_course_installer_pool(
    database_url: &str,
) -> Result<BaseCourseInstallerPool, sqlx::Error> {
    let contract = LoginContract::BaseCourseInstaller;
    let options = verified_connect_options(database_url, contract)?;
    Ok(BaseCourseInstallerPool(attested_pool(
        options,
        contract,
        BASE_COURSE_POOL_MAX_CONNECTIONS,
    )))
}

/// Builds the only disposable-stack installer pool accepted by the Base Course
/// installer facade while retaining the exact installer-login attestation.
pub fn local_base_course_installer_pool(
    database_url: &str,
) -> Result<BaseCourseInstallerPool, sqlx::Error> {
    let contract = LoginContract::BaseCourseInstaller;
    let options = local_connect_options(database_url, contract)?;
    Ok(BaseCourseInstallerPool(attested_pool(
        options,
        contract,
        BASE_COURSE_POOL_MAX_CONNECTIONS,
    )))
}

/// Builds the dedicated one-connection production application pool used only
/// by Base Course convergence.
pub fn base_course_application_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let contract = LoginContract::BaseCourseApplication;
    let options = verified_connect_options(database_url, contract)?;
    Ok(attested_pool(
        options,
        contract,
        BASE_COURSE_POOL_MAX_CONNECTIONS,
    ))
}

/// Builds the dedicated one-connection disposable-stack application pool used
/// only by Base Course convergence.
pub fn local_base_course_application_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let contract = LoginContract::BaseCourseApplication;
    let options = local_connect_options(database_url, contract)?;
    Ok(attested_pool(
        options,
        contract,
        BASE_COURSE_POOL_MAX_CONNECTIONS,
    ))
}

fn attested_pool(
    options: PgConnectOptions,
    contract: LoginContract,
    max_connections: u32,
) -> PgPool {
    pool_options(max_connections)
        .after_connect(move |connection, _metadata| {
            Box::pin(async move { verify_login_authority(connection, contract).await })
        })
        .connect_lazy_with(options)
}

/// Connects the bounded dedicated QTI grader pool.
pub(super) async fn connect_grader_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let contract = LoginContract::Grader;
    let options = verified_connect_options(database_url, contract)?;
    pool_options(GRADER_POOL_MAX_CONNECTIONS)
        .after_connect(move |connection, _metadata| {
            Box::pin(async move { verify_login_authority(connection, contract).await })
        })
        .connect_with(options)
        .await
}

/// Connects the local-development grader pool without requiring TLS while
/// retaining the exact grader login and authority contract.
pub(super) async fn connect_local_grader_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let contract = LoginContract::Grader;
    let options = local_connect_options(database_url, contract)?;
    pool_options(GRADER_POOL_MAX_CONNECTIONS)
        .after_connect(move |connection, _metadata| {
            Box::pin(async move { verify_login_authority(connection, contract).await })
        })
        .connect_with(options)
        .await
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
        && authority.direct_memberships.len() == contract.expected_memberships().len()
        && authority
            .direct_memberships
            .iter()
            .zip(contract.expected_memberships())
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
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use tokio::sync::{Barrier, Notify};

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
        let options = pool_options(STANDARD_POOL_MAX_CONNECTIONS);
        assert_eq!(options.get_max_connections(), STANDARD_POOL_MAX_CONNECTIONS);
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
    async fn named_pool_factories_preserve_their_resource_contracts() {
        let lazy = lazy_pool("postgres://ignored:secret@db.example/ple").unwrap();
        assert_eq!(
            lazy.options().get_max_connections(),
            STANDARD_POOL_MAX_CONNECTIONS
        );

        let production = production_pool(
            "postgres://ple_api_login:secret@db.example/ple?sslmode=verify-full",
            ProductionLoginProfile::Api,
        )
        .unwrap();
        let local = local_development_pool(
            "postgres://ple_worker_login:secret@db.example/ple",
            ProductionLoginProfile::Worker,
        )
        .unwrap();
        for pool in [production, local] {
            assert_eq!(
                pool.options().get_max_connections(),
                STANDARD_POOL_MAX_CONNECTIONS
            );
        }

        let production_installer = base_course_installer_pool(
            "postgres://ple_base_course_installer_login:secret@db.example/ple?sslmode=verify-full",
        )
        .unwrap();
        let local_installer = local_base_course_installer_pool(
            "postgres://ple_base_course_installer_login:secret@db.example/ple",
        )
        .unwrap();
        let production_application = base_course_application_pool(
            "postgres://ple_base_course_app_login:secret@db.example/ple?sslmode=verify-full",
        )
        .unwrap();
        let local_application = local_base_course_application_pool(
            "postgres://ple_base_course_app_login:secret@db.example/ple",
        )
        .unwrap();
        for pool in [
            production_installer.0,
            local_installer.0,
            production_application,
            local_application,
        ] {
            assert_eq!(
                pool.options().get_max_connections(),
                BASE_COURSE_POOL_MAX_CONNECTIONS
            );
        }

        assert_eq!(
            pool_options(GRADER_POOL_MAX_CONNECTIONS).get_max_connections(),
            GRADER_POOL_MAX_CONNECTIONS
        );
    }

    fn authority(contract: LoginContract) -> LoginAuthority {
        LoginAuthority {
            current_user: contract.expected_login().to_string(),
            session_user: contract.expected_login().to_string(),
            superuser: false,
            create_database: false,
            create_role: false,
            inherit: false,
            replication: false,
            bypass_rls: false,
            can_login: true,
            direct_memberships: contract
                .expected_memberships()
                .iter()
                .map(|expected| DirectMembership {
                    role_name: expected.role_name.to_string(),
                    admin_option: false,
                    inherit_option: false,
                    set_option: expected.set_option,
                })
                .collect(),
        }
    }

    #[test]
    fn grader_contract_attests_only_its_settable_capability_role() {
        let expected = [ExpectedMembership {
            role_name: "ple_grader",
            set_option: true,
        }];
        assert_eq!(LoginContract::Grader.expected_memberships(), expected);
        assert_eq!(LoginContract::Grader.expected_capabilities(), expected);
    }

    fn capability_authority(role_name: &str) -> CapabilityAuthority {
        CapabilityAuthority {
            role_name: role_name.to_string(),
            superuser: false,
            create_database: false,
            create_role: false,
            inherit: false,
            replication: false,
            bypass_rls: false,
            can_login: false,
            direct_memberships: Vec::new(),
        }
    }

    #[test]
    fn production_urls_require_verified_tls_and_fixed_process_identity() {
        let api = LoginContract::Production(ProductionLoginProfile::Api);
        assert!(
            verified_connect_options(
                "postgres://ple_api_login:secret@db.example/ple?sslmode=verify-full",
                api,
            )
            .is_ok()
        );
        for url in [
            "postgres://ple_api_login:secret@db.example/ple",
            "postgres://ple_api_login:secret@db.example/ple?sslmode=require",
            "postgres://postgres:secret@db.example/ple?sslmode=verify-full",
        ] {
            assert!(
                verified_connect_options(url, api).is_err(),
                "accepted {url}"
            );
        }
        let publisher = LoginContract::Production(ProductionLoginProfile::Publisher);
        assert!(
            verified_connect_options(
                "postgres://ple_publisher_login:secret@db.example/ple?sslmode=verify-full",
                publisher,
            )
            .is_ok()
        );
        assert!(
            verified_connect_options(
                "postgres://ple_worker_login:secret@db.example/ple?sslmode=verify-full",
                publisher,
            )
            .is_err()
        );
        let delivery_worker =
            LoginContract::Production(ProductionLoginProfile::InvitationDeliveryWorker);
        assert!(
            verified_connect_options(
                "postgres://ple_invitation_delivery_worker_login:secret@db.example/ple?sslmode=verify-full",
                delivery_worker,
            )
            .is_ok()
        );
        let base_course_application = LoginContract::BaseCourseApplication;
        assert!(
            verified_connect_options(
                "postgres://ple_base_course_app_login:secret@db.example/ple?sslmode=verify-full",
                base_course_application,
            )
            .is_ok()
        );
        let base_course_installer = LoginContract::BaseCourseInstaller;
        assert!(
            verified_connect_options(
                "postgres://ple_base_course_installer_login:secret@db.example/ple?sslmode=verify-full",
                base_course_installer,
            )
            .is_ok()
        );
        assert!(
            verified_connect_options(
                "postgres://ple_base_course_app_login:secret@db.example/ple?sslmode=verify-full",
                base_course_installer,
            )
            .is_err()
        );
    }

    #[test]
    fn process_authority_contract_rejects_each_privilege_widening() {
        for contract in [
            LoginContract::Production(ProductionLoginProfile::Api),
            LoginContract::Production(ProductionLoginProfile::Worker),
            LoginContract::Production(ProductionLoginProfile::InvitationDeliveryWorker),
            LoginContract::Production(ProductionLoginProfile::Publisher),
            LoginContract::BaseCourseApplication,
            LoginContract::BaseCourseInstaller,
            LoginContract::Grader,
        ] {
            assert!(login_authority_matches(&authority(contract), contract));

            let mut widened = authority(contract);
            widened.bypass_rls = true;
            assert!(!login_authority_matches(&widened, contract));

            let mut widened = authority(contract);
            widened.superuser = true;
            assert!(!login_authority_matches(&widened, contract));

            let mut widened = authority(contract);
            widened.inherit = true;
            assert!(!login_authority_matches(&widened, contract));

            let mut widened = authority(contract);
            widened.direct_memberships.push(DirectMembership {
                role_name: "ple_grader".to_string(),
                admin_option: false,
                inherit_option: false,
                set_option: true,
            });
            assert!(!login_authority_matches(&widened, contract));
        }
    }

    #[test]
    fn process_authority_contract_rejects_delegable_or_unscoped_memberships() {
        for contract in [
            LoginContract::Production(ProductionLoginProfile::Api),
            LoginContract::Production(ProductionLoginProfile::Worker),
            LoginContract::Production(ProductionLoginProfile::InvitationDeliveryWorker),
            LoginContract::Production(ProductionLoginProfile::Publisher),
            LoginContract::BaseCourseApplication,
            LoginContract::BaseCourseInstaller,
            LoginContract::Grader,
        ] {
            let mut missing = authority(contract);
            missing.direct_memberships.clear();
            assert!(
                !login_authority_matches(&missing, contract),
                "{contract:?} must retain its exact capability membership"
            );

            let mut delegable = authority(contract);
            delegable.direct_memberships[0].admin_option = true;
            assert!(
                !login_authority_matches(&delegable, contract),
                "{contract:?} must not delegate a capability role"
            );

            let mut inherited = authority(contract);
            inherited.direct_memberships[0].inherit_option = true;
            assert!(
                !login_authority_matches(&inherited, contract),
                "{contract:?} must not gain a capability outside SET LOCAL ROLE"
            );

            let mut cannot_enter_expected_role = authority(contract);
            cannot_enter_expected_role.direct_memberships[0].set_option = false;
            assert!(
                !login_authority_matches(&cannot_enter_expected_role, contract),
                "{contract:?} must retain only its attested SET LOCAL ROLE path"
            );

            let mut unscoped = authority(contract);
            unscoped.direct_memberships[0]
                .role_name
                .push_str("_unscoped");
            assert!(
                !login_authority_matches(&unscoped, contract),
                "{contract:?} must not accept an unscoped capability role"
            );
        }
    }

    #[test]
    fn effective_capability_roles_have_closed_exact_authority() {
        for contract in [
            LoginContract::Production(ProductionLoginProfile::Api),
            LoginContract::Production(ProductionLoginProfile::Worker),
            LoginContract::Production(ProductionLoginProfile::InvitationDeliveryWorker),
            LoginContract::Production(ProductionLoginProfile::Publisher),
            LoginContract::BaseCourseApplication,
            LoginContract::BaseCourseInstaller,
            LoginContract::Grader,
        ] {
            for expected in contract.expected_capabilities() {
                assert!(capability_authority_matches(
                    &capability_authority(expected.role_name),
                    expected
                ));
            }
        }
        assert!(login_authority_matches(
            &authority(LoginContract::Grader),
            LoginContract::Grader
        ));
    }

    #[test]
    fn effective_capability_roles_reject_privilege_and_nested_role_widening() {
        for contract in [
            LoginContract::Production(ProductionLoginProfile::Api),
            LoginContract::Production(ProductionLoginProfile::Worker),
            LoginContract::Production(ProductionLoginProfile::InvitationDeliveryWorker),
            LoginContract::Production(ProductionLoginProfile::Publisher),
            LoginContract::BaseCourseApplication,
            LoginContract::BaseCourseInstaller,
            LoginContract::Grader,
        ] {
            for expected in contract.expected_capabilities() {
                let mut widened = capability_authority(expected.role_name);
                widened.bypass_rls = true;
                assert!(!capability_authority_matches(&widened, expected));

                let mut widened = capability_authority(expected.role_name);
                widened.can_login = true;
                assert!(!capability_authority_matches(&widened, expected));

                let mut widened = capability_authority(expected.role_name);
                widened.create_role = true;
                assert!(!capability_authority_matches(&widened, expected));

                for (admin_option, inherit_option, set_option) in [
                    (true, false, false),
                    (false, true, false),
                    (false, false, true),
                ] {
                    let mut widened = capability_authority(expected.role_name);
                    widened.direct_memberships.push(DirectMembership {
                        role_name: "ple_catalog_ownership_broker".to_string(),
                        admin_option,
                        inherit_option,
                        set_option,
                    });
                    assert!(
                        !capability_authority_matches(&widened, expected),
                        "{} must not gain a nested capability role",
                        expected.role_name
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn permanently_aborted_transaction_returns_without_success_effects() {
        let committed = AtomicBool::new(false);
        let post_commit_effect = AtomicBool::new(false);
        let result = retry_transaction(|| {
            let committed = &committed;
            let post_commit_effect = &post_commit_effect;
            committed.store(false, Ordering::Relaxed);
            post_commit_effect.store(false, Ordering::Relaxed);
            std::future::ready(Err::<(), _>(StoreError::RetryableTransaction))
        })
        .await;
        assert_eq!(result, Err(StoreError::RetryableTransaction));
        assert!(!committed.load(Ordering::Relaxed));
        assert!(!post_commit_effect.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn transient_transaction_abort_restarts_a_fresh_operation() {
        let first_attempt = AtomicBool::new(true);
        let committed = AtomicBool::new(false);
        let result = retry_transaction(|| {
            if first_attempt.swap(false, Ordering::Relaxed) {
                return std::future::ready(Err(StoreError::RetryableTransaction));
            }
            committed.store(true, Ordering::Relaxed);
            std::future::ready(Ok(()))
        })
        .await;
        assert_eq!(result, Ok(()));
        assert!(committed.load(Ordering::Relaxed));
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
}
