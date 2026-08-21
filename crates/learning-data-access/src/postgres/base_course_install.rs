//! Host-side lifecycle coordination for the installed live-demo Base Course.
//!
//! The guard owns one PostgreSQL session while holding a session advisory
//! lock.  It deliberately exposes only the lifecycle transitions required by
//! the host installer, so application stores cannot use it as a generic
//! database-write capability.

use std::collections::BTreeSet;

use question_model::{TenantId, UserId, UserRole};
use serde_json::Value;
use sqlx::pool::PoolConnection;
use sqlx::{AssertSqlSafe, Connection, PgConnection, Postgres, Row, Transaction};

use crate::{AuthenticationEmail, StoreError, validated_account_display_name};

use super::{PgPool, map_sqlx_error};

/// Fixed deployment-wide PostgreSQL session advisory-lock key for Base Course
/// installation.
pub const BASE_COURSE_INSTALL_ADVISORY_LOCK_KEY: i64 = 0x504c_4542_4153_4501;

const MAX_BASE_COURSE_INSTALL_ACCOUNTS: usize = 16;

/// Exact platform-role state accepted by Base Course account provisioning.
///
/// Student and Instructor authority comes from ordinary course memberships,
/// so the host installer can create only an account with no platform role or
/// the one operator-controlled Sysadmin role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseCourseAccountPlatformRoles {
    /// The account has no platform-wide role.
    None,
    /// The account has exactly the Sysadmin platform role.
    Sysadmin,
}

impl BaseCourseAccountPlatformRoles {
    /// Returns the exact persisted role list represented by this value.
    pub fn as_slice(self) -> &'static [UserRole] {
        match self {
            Self::None => &[],
            Self::Sysadmin => &[UserRole::Sysadmin],
        }
    }
}

/// One exact ordinary PLE account required by the installed Base Course.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseCourseAccountRecipe {
    user: UserId,
    email: AuthenticationEmail,
    display_name: String,
    platform_roles: BaseCourseAccountPlatformRoles,
}

impl BaseCourseAccountRecipe {
    /// Creates a recipe after validating its user-visible account label.
    pub fn new(
        user: UserId,
        email: AuthenticationEmail,
        display_name: impl Into<String>,
        platform_roles: BaseCourseAccountPlatformRoles,
    ) -> Result<Self, StoreError> {
        let display_name = display_name.into();
        let display_name = validated_account_display_name(&display_name)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        Ok(Self {
            user,
            email,
            display_name,
            platform_roles,
        })
    }

    /// Returns the stable account identity.
    pub fn user(&self) -> UserId {
        self.user
    }

    /// Returns the validated authentication email.
    pub fn email(&self) -> &AuthenticationEmail {
        &self.email
    }

    /// Returns the validated user-visible account label.
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Returns the exact persisted platform-role list.
    pub fn platform_roles(&self) -> &'static [UserRole] {
        self.platform_roles.as_slice()
    }
}

/// The durable state of the deployment's seeded Base Course.
#[derive(Debug, Clone, PartialEq)]
pub enum BaseCourseInstallState {
    /// Installation has claimed a tenant and can resume only with identical
    /// baseline inputs.
    Installing {
        tenant_id: TenantId,
        baseline_version: String,
        installation_generation: uuid::Uuid,
        object_manifest: Value,
    },
    /// The named tenant contains the completed baseline.
    Complete {
        tenant_id: TenantId,
        baseline_version: String,
        installation_generation: uuid::Uuid,
        object_manifest: Value,
        storage_receipt_sha256: String,
    },
}

/// Exclusive host installer capability backed by a checked-out PostgreSQL
/// connection. The connection stays checked out across work performed through
/// other pools, preserving the session advisory lock.
pub struct BaseCourseInstallLock {
    connection: Option<PoolConnection<Postgres>>,
}

impl BaseCourseInstallLock {
    /// Atomically inserts or exactly verifies a bounded set of ordinary PLE
    /// accounts on the privileged installer session.
    ///
    /// The operation rejects duplicate account identities and normalized
    /// emails before opening its transaction. Existing rows must match every
    /// recipe field, including delivery-email spelling and platform roles.
    pub async fn provision_accounts(
        &mut self,
        accounts: &[BaseCourseAccountRecipe],
    ) -> Result<(), StoreError> {
        validate_account_batch(accounts)?;
        let connection = self.connection_mut()?;
        let mut transaction = connection.begin().await.map_err(map_sqlx_error)?;
        for account in accounts {
            provision_account(&mut transaction, account).await?;
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    /// Returns the current durable lifecycle marker without changing it.
    pub async fn read_state(&mut self) -> Result<Option<BaseCourseInstallState>, StoreError> {
        let connection = self.connection_mut()?;
        let row = sqlx::query(
            "SELECT state, baseline_version, tenant_id, installation_generation, object_manifest, \
             storage_receipt_sha256 \
             FROM public.live_demo_install_state WHERE singleton = true",
        )
        .fetch_optional(&mut **connection)
        .await
        .map_err(map_sqlx_error)?;
        row.map(decode_state).transpose()
    }

    /// Atomically creates the installing marker, or resumes the same marker.
    ///
    /// A complete marker returns before callers inspect object storage. An
    /// installing marker keeps its generated installation generation across
    /// retries. A conflicting tenant or baseline returns [`StoreError::Conflict`].
    pub async fn prepare(
        &mut self,
        tenant_id: TenantId,
        baseline_version: &str,
        object_manifest: &Value,
    ) -> Result<BaseCourseInstallState, StoreError> {
        validate_baseline_inputs(baseline_version, object_manifest)?;
        let connection = self.connection_mut()?;
        let mut transaction = connection.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("LOCK TABLE public.live_demo_install_state IN SHARE ROW EXCLUSIVE MODE")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let state = sqlx::query(
            "SELECT state, baseline_version, tenant_id, installation_generation, object_manifest, \
             storage_receipt_sha256 \
             FROM public.live_demo_install_state WHERE singleton = true FOR UPDATE",
        )
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .map(decode_state)
        .transpose()?;

        match state {
            None => {
                ensure_unmarked_install_is_fresh(&mut transaction).await?;
                let row = sqlx::query(
                    "INSERT INTO public.live_demo_install_state \
                     (singleton, state, baseline_version, tenant_id, installation_generation, object_manifest) \
                     VALUES (true, 'installing', $1, $2, gen_random_uuid(), $3) \
                     RETURNING installation_generation",
                )
                .bind(baseline_version)
                .bind(tenant_id.as_uuid())
                .bind(sqlx::types::Json(object_manifest))
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                let installation_generation: uuid::Uuid = row
                    .try_get("installation_generation")
                    .map_err(map_sqlx_error)?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(BaseCourseInstallState::Installing {
                    tenant_id,
                    baseline_version: baseline_version.to_owned(),
                    installation_generation,
                    object_manifest: object_manifest.clone(),
                })
            }
            Some(
                state @ (BaseCourseInstallState::Installing { .. }
                | BaseCourseInstallState::Complete { .. }),
            ) => {
                ensure_matching_install(&state, tenant_id, baseline_version, object_manifest)?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(state)
            }
        }
    }

    /// Makes an identical installing marker terminal after successful seeding.
    pub async fn mark_complete(
        &mut self,
        tenant_id: TenantId,
        baseline_version: &str,
        installation_generation: uuid::Uuid,
        object_manifest: &Value,
        storage_receipt_sha256: &str,
    ) -> Result<(), StoreError> {
        validate_baseline_inputs(baseline_version, object_manifest)?;
        validate_storage_receipt_sha256(storage_receipt_sha256)?;
        let connection = self.connection_mut()?;
        let result = sqlx::query(
            "UPDATE public.live_demo_install_state \
                     SET state = 'complete', storage_receipt_sha256 = $4, \
                         completed_at = transaction_timestamp() \
             WHERE singleton = true AND state = 'installing' \
               AND tenant_id = $1 AND baseline_version = $2 \
               AND installation_generation = $3 AND object_manifest = $5",
        )
        .bind(tenant_id.as_uuid())
        .bind(baseline_version)
        .bind(installation_generation)
        .bind(storage_receipt_sha256)
        .bind(sqlx::types::Json(object_manifest))
        .execute(&mut **connection)
        .await
        .map_err(map_sqlx_error)?;
        if result.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        Ok(())
    }

    /// Explicitly unlocks and returns the checked-out session to the pool.
    pub async fn release(mut self) -> Result<(), StoreError> {
        let mut connection = self.take_connection()?;
        let unlock = sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(BASE_COURSE_INSTALL_ADVISORY_LOCK_KEY)
            .execute(&mut *connection)
            .await;
        match unlock {
            Ok(_) => {
                drop(connection);
                Ok(())
            }
            Err(error) => {
                // An unlock error leaves the session's lock ownership unknown.
                // Detach and close instead of placing it back in the pool.
                let _ = close_locked_connection(connection).await;
                Err(map_sqlx_error(error))
            }
        }
    }

    /// Closes the locked session after a failed installation attempt.
    ///
    /// Closing rather than returning it to the pool guarantees the
    /// session-scoped lock cannot leak to a future pool borrower.
    pub async fn abort(mut self) -> Result<(), StoreError> {
        let connection = self.take_connection()?;
        close_locked_connection(connection).await
    }

    fn connection_mut(&mut self) -> Result<&mut PoolConnection<Postgres>, StoreError> {
        self.connection.as_mut().ok_or_else(|| {
            StoreError::Unavailable("Base Course installation lock is closed".to_string())
        })
    }

    fn take_connection(&mut self) -> Result<PoolConnection<Postgres>, StoreError> {
        self.connection.take().ok_or_else(|| {
            StoreError::Unavailable("Base Course installation lock is closed".to_string())
        })
    }
}

/// Refuses a first Base Course install after ordinary application state exists.
///
/// The caller already holds the installation advisory lock. The table locks
/// make the catalog snapshot authoritative against regular application writers
/// until the installing marker commits. The marker table itself is locked by
/// `prepare`; SQLx's migration ledger is schema metadata, and the one exact
/// unconsumed question namespace is required migration-seeded state.
async fn ensure_unmarked_install_is_fresh(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), StoreError> {
    sqlx::query("LOCK TABLE public.question_id_namespace IN SHARE MODE")
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    let namespace: (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE singleton AND issued_count = 0) \
         FROM public.question_id_namespace",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if namespace != (1, 1) {
        return Err(StoreError::InvalidRecord(
            "live-demo baseline requires an unconsumed question ID namespace; regenerate both stores before Base Course installation".to_string(),
        ));
    }

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT relation.relname \
         FROM pg_catalog.pg_class AS relation \
         JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation.relnamespace \
         WHERE namespace.nspname = 'public' \
           AND relation.relkind IN ('r', 'p') \
           AND relation.relname NOT IN ('_sqlx_migrations', 'question_id_namespace', \
                                        'live_demo_install_state') \
         ORDER BY relation.relname",
    )
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    for table in tables {
        let identifier = quoted_identifier(&table);
        // `table` comes only from PostgreSQL's public catalog and is quoted
        // before this intentionally dynamic identifier use.
        sqlx::query(AssertSqlSafe(format!(
            "LOCK TABLE public.{identifier} IN SHARE MODE"
        )))
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        let has_rows: bool = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT EXISTS (SELECT 1 FROM public.{identifier} LIMIT 1)"
        )))
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        if has_rows {
            return Err(StoreError::InvalidRecord(format!(
                "live-demo baseline requires an empty public application schema; table public.{table} contains live rows; regenerate both stores before Base Course installation"
            )));
        }
    }
    Ok(())
}

fn quoted_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn validate_account_batch(accounts: &[BaseCourseAccountRecipe]) -> Result<(), StoreError> {
    if accounts.is_empty() || accounts.len() > MAX_BASE_COURSE_INSTALL_ACCOUNTS {
        return Err(StoreError::InvalidRecord(format!(
            "Base Course account batch must contain 1 to {MAX_BASE_COURSE_INSTALL_ACCOUNTS} accounts"
        )));
    }
    let mut users = BTreeSet::new();
    let mut emails = BTreeSet::new();
    for account in accounts {
        if !users.insert(account.user) || !emails.insert(account.email.normalized()) {
            return Err(StoreError::InvalidRecord(
                "Base Course account batch contains duplicate identities or emails".to_string(),
            ));
        }
    }
    Ok(())
}

async fn provision_account(
    transaction: &mut Transaction<'_, Postgres>,
    account: &BaseCourseAccountRecipe,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO public.ple_account (\
             user_id, normalized_email, delivery_email, display_name, platform_roles\
         ) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
    )
    .bind(account.user.as_uuid())
    .bind(account.email.normalized())
    .bind(account.email.delivery())
    .bind(&account.display_name)
    .bind(sqlx::types::Json(account.platform_roles()))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

    let row = sqlx::query(
        "SELECT normalized_email, delivery_email, display_name, platform_roles \
         FROM public.ple_account WHERE user_id = $1 FOR UPDATE",
    )
    .bind(account.user.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::Conflict)?;
    let normalized_email: String = row.try_get("normalized_email").map_err(map_sqlx_error)?;
    let delivery_email: String = row.try_get("delivery_email").map_err(map_sqlx_error)?;
    let display_name: String = row.try_get("display_name").map_err(map_sqlx_error)?;
    let sqlx::types::Json(platform_roles): sqlx::types::Json<Vec<UserRole>> =
        row.try_get("platform_roles").map_err(map_sqlx_error)?;
    if normalized_email != account.email.normalized()
        || delivery_email != account.email.delivery()
        || display_name != account.display_name
        || platform_roles != account.platform_roles()
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

impl Drop for BaseCourseInstallLock {
    fn drop(&mut self) {
        // A caller that forgets `release` or `abort` must not return a session
        // carrying the advisory lock to the pool. Detaching closes it on drop.
        if let Some(connection) = self.connection.take() {
            drop(connection.detach());
        }
    }
}

/// Acquires the deployment-wide Base Course installation lock on one checked
/// out PostgreSQL session. A concurrent caller waits on the database lock.
pub async fn acquire_base_course_install_lock(
    pool: &PgPool,
) -> Result<BaseCourseInstallLock, StoreError> {
    let mut connection = pool.acquire().await.map_err(map_sqlx_error)?;
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(BASE_COURSE_INSTALL_ADVISORY_LOCK_KEY)
        .execute(&mut *connection)
        .await
        .map_err(map_sqlx_error)?;
    Ok(BaseCourseInstallLock {
        connection: Some(connection),
    })
}

async fn close_locked_connection(connection: PoolConnection<Postgres>) -> Result<(), StoreError> {
    let connection: PgConnection = connection.detach();
    connection.close().await.map_err(map_sqlx_error)
}

fn decode_state(row: sqlx::postgres::PgRow) -> Result<BaseCourseInstallState, StoreError> {
    let state: String = row.try_get("state").map_err(map_sqlx_error)?;
    let baseline_version: String = row.try_get("baseline_version").map_err(map_sqlx_error)?;
    let installation_generation: uuid::Uuid = row
        .try_get("installation_generation")
        .map_err(map_sqlx_error)?;
    let object_manifest: sqlx::types::Json<Value> =
        row.try_get("object_manifest").map_err(map_sqlx_error)?;
    let tenant_id: Option<uuid::Uuid> = row.try_get("tenant_id").map_err(map_sqlx_error)?;
    let storage_receipt_sha256: Option<String> = row
        .try_get("storage_receipt_sha256")
        .map_err(map_sqlx_error)?;
    match (state.as_str(), tenant_id, storage_receipt_sha256) {
        ("installing", Some(tenant_id), None) => Ok(BaseCourseInstallState::Installing {
            tenant_id: TenantId::from_uuid(tenant_id),
            baseline_version,
            installation_generation,
            object_manifest: object_manifest.0,
        }),
        ("complete", Some(tenant_id), Some(storage_receipt_sha256)) => {
            Ok(BaseCourseInstallState::Complete {
                tenant_id: TenantId::from_uuid(tenant_id),
                baseline_version,
                installation_generation,
                object_manifest: object_manifest.0,
                storage_receipt_sha256,
            })
        }
        _ => Err(StoreError::Unavailable(
            "live-demo install state violates its lifecycle invariant".to_string(),
        )),
    }
}

fn validate_baseline_inputs(
    baseline_version: &str,
    object_manifest: &Value,
) -> Result<(), StoreError> {
    if baseline_version != "base-course-v1" || object_manifest != &Value::Array(Vec::new()) {
        return Err(StoreError::InvalidRecord(
            "live-demo install inputs do not match the supported baseline".to_string(),
        ));
    }
    Ok(())
}

fn validate_storage_receipt_sha256(value: &str) -> Result<(), StoreError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(StoreError::InvalidRecord(
            "live-demo storage receipt hash must be a lowercase SHA-256 hex digest".to_string(),
        ));
    }
    Ok(())
}

fn ensure_matching_install(
    state: &BaseCourseInstallState,
    tenant_id: TenantId,
    baseline_version: &str,
    object_manifest: &Value,
) -> Result<(), StoreError> {
    let (stored_tenant, stored_version, stored_manifest) = match state {
        BaseCourseInstallState::Installing {
            tenant_id,
            baseline_version,
            object_manifest,
            ..
        }
        | BaseCourseInstallState::Complete {
            tenant_id,
            baseline_version,
            object_manifest,
            ..
        } => (tenant_id, baseline_version, object_manifest),
    };
    if stored_tenant != &tenant_id
        || stored_version != baseline_version
        || stored_manifest != object_manifest
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use question_model::UserRole;
    use serde_json::json;
    use uuid::Uuid;

    use super::*;

    fn account_recipe(
        user: u128,
        email: &str,
        platform_roles: BaseCourseAccountPlatformRoles,
    ) -> BaseCourseAccountRecipe {
        BaseCourseAccountRecipe::new(
            UserId::from_uuid(Uuid::from_u128(user)),
            AuthenticationEmail::parse(email).expect("test email should be valid"),
            format!("Account {user}"),
            platform_roles,
        )
        .expect("test account recipe should be valid")
    }

    #[test]
    fn account_recipe_exposes_only_supported_platform_role_states() {
        let ordinary = account_recipe(
            1,
            "ordinary@example.invalid",
            BaseCourseAccountPlatformRoles::None,
        );
        let sysadmin = account_recipe(
            2,
            "sysadmin@example.invalid",
            BaseCourseAccountPlatformRoles::Sysadmin,
        );

        assert_eq!(ordinary.platform_roles(), &[]);
        assert_eq!(sysadmin.platform_roles(), &[UserRole::Sysadmin]);
    }

    #[test]
    fn account_batch_rejects_duplicate_identity_or_normalized_email() {
        let first = account_recipe(
            1,
            "first@example.invalid",
            BaseCourseAccountPlatformRoles::None,
        );
        let duplicate_user = account_recipe(
            1,
            "second@example.invalid",
            BaseCourseAccountPlatformRoles::None,
        );
        let duplicate_email = account_recipe(
            2,
            "FIRST@example.invalid",
            BaseCourseAccountPlatformRoles::None,
        );

        assert!(matches!(
            validate_account_batch(&[first.clone(), duplicate_user]),
            Err(StoreError::InvalidRecord(_))
        ));
        assert!(matches!(
            validate_account_batch(&[first, duplicate_email]),
            Err(StoreError::InvalidRecord(_))
        ));
    }

    #[test]
    fn account_batch_requires_a_bounded_nonempty_set() {
        assert!(matches!(
            validate_account_batch(&[]),
            Err(StoreError::InvalidRecord(_))
        ));
        let accounts = (0..=MAX_BASE_COURSE_INSTALL_ACCOUNTS)
            .map(|index| {
                account_recipe(
                    index as u128 + 1,
                    &format!("account-{index}@example.invalid"),
                    BaseCourseAccountPlatformRoles::None,
                )
            })
            .collect::<Vec<_>>();
        assert!(matches!(
            validate_account_batch(&accounts),
            Err(StoreError::InvalidRecord(_))
        ));
    }

    #[test]
    fn only_the_current_baseline_inputs_are_accepted() {
        assert!(validate_baseline_inputs("base-course-v1", &json!([])).is_ok());
        assert!(matches!(
            validate_baseline_inputs("base-course-v2", &json!([])),
            Err(StoreError::InvalidRecord(_))
        ));
        assert!(matches!(
            validate_baseline_inputs("base-course-v1", &json!(["object"])),
            Err(StoreError::InvalidRecord(_))
        ));
    }

    #[test]
    fn installing_inputs_must_match_exactly_to_resume() {
        let tenant = TenantId::from_uuid(Uuid::from_u128(1));
        let state = BaseCourseInstallState::Installing {
            tenant_id: tenant,
            baseline_version: "base-course-v1".to_string(),
            installation_generation: Uuid::from_u128(3),
            object_manifest: json!([]),
        };
        assert!(ensure_matching_install(&state, tenant, "base-course-v1", &json!([])).is_ok());
        assert_eq!(
            ensure_matching_install(
                &state,
                TenantId::from_uuid(Uuid::from_u128(2)),
                "base-course-v1",
                &json!([]),
            ),
            Err(StoreError::Conflict)
        );
    }

    #[test]
    fn only_lowercase_sha256_receipt_hashes_are_accepted() {
        assert!(validate_storage_receipt_sha256(&"a".repeat(64)).is_ok());
        assert!(validate_storage_receipt_sha256(&"A".repeat(64)).is_err());
        assert!(validate_storage_receipt_sha256("not-a-sha256").is_err());
    }
}
