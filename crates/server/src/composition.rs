//! Explicit assembly for the current Account and Authenticated Session server.

use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result, bail};
use axum::{Router, http::StatusCode, routing::get};
use learning_data_access::{
    SessionLifetime,
    postgres::{
        PostgresSessionStore, ProductionLoginProfile, local_development_pool, production_pool,
    },
};
use question_model::AccountId;

use crate::auth::{
    CookieTransport, ProductionBrowserBoundary, SeededDemoAccount, SeededDemoConfig,
    SeededDemoPersona, SessionConfig, live_demo_router, session_router,
};

const LIVE_DEMO_ACCOUNT_ID_ENV: [(SeededDemoPersona, &str, &str); 5] = [
    (
        SeededDemoPersona::ElenaInstructor,
        "PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID",
        "Elena Instructor",
    ),
    (
        SeededDemoPersona::MaryStudent,
        "PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID",
        "Mary Student",
    ),
    (
        SeededDemoPersona::JackStudent,
        "PLE_LIVE_DEMO_JACK_STUDENT_ACCOUNT_ID",
        "Jack Student",
    ),
    (
        SeededDemoPersona::AveryStudent,
        "PLE_LIVE_DEMO_AVERY_STUDENT_ACCOUNT_ID",
        "Avery Student",
    ),
    (
        SeededDemoPersona::MorganSysadmin,
        "PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID",
        "Morgan Sysadmin",
    ),
];

/// Builds the current route surface from explicit environment configuration.
pub async fn production_router_from_env() -> Result<Router> {
    let database_url = required_env("DATABASE_URL")?;
    let pool = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local")
    {
        local_development_pool(&database_url, ProductionLoginProfile::Api)
    } else {
        production_pool(&database_url, ProductionLoginProfile::Api)
    }
    .context("could not construct the attested API database pool")?;
    pool.acquire()
        .await
        .context("the attested API database pool could not connect")?;

    let sessions = Arc::new(PostgresSessionStore::new(pool));
    let session_config = production_session_config();
    let router = Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .merge(session_router(Arc::clone(&sessions), session_config))
        .merge(live_demo_router(
            sessions,
            live_demo_config_from_env()?,
            session_config,
        ));
    let browser_boundary =
        ProductionBrowserBoundary::new(Arc::from(required_env("PLE_BROWSER_ORIGIN")?))
            .map_err(anyhow::Error::msg)?;
    Ok(crate::http_security::apply_api_security_headers(
        router.layer(axum::middleware::from_fn_with_state(
            browser_boundary,
            crate::auth::production_cookie_boundary,
        )),
    ))
}

/// The address the binary binds, parsed once at startup.
pub fn bind_address_from_env() -> Result<SocketAddr> {
    std::env::var("PLE_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .context("PLE_BIND_ADDR must be a socket address")
}

fn production_session_config() -> SessionConfig {
    SessionConfig::new(
        SessionLifetime::from_seconds(8 * 60 * 60).expect("positive session lifetime"),
        CookieTransport::FirstPartyHttps,
    )
}

fn live_demo_config_from_env() -> Result<Option<SeededDemoConfig>> {
    live_demo_config_from_environment_values(|name| std::env::var(name).ok())
}

fn live_demo_config_from_environment_values(
    mut value_for: impl FnMut(&str) -> Option<String>,
) -> Result<Option<SeededDemoConfig>> {
    // ASVS 2.2.1 and 16.5.2: each trusted deployment value is independently
    // allow-listed as one fixed persona and malformed values affect only it.
    let configured = LIVE_DEMO_ACCOUNT_ID_ENV
        .iter()
        .filter_map(|(persona, name, display_name)| {
            let value = value_for(name)?;
            let id = uuid::Uuid::parse_str(&value).ok()?;
            Some((*persona, AccountId::from_uuid(id), *display_name))
        })
        .collect::<Vec<_>>();
    live_demo_config_from_account_mappings(configured)
}

fn live_demo_config_from_account_mappings(
    configured: Vec<(SeededDemoPersona, AccountId, &'static str)>,
) -> Result<Option<SeededDemoConfig>> {
    let duplicate_accounts = configured.iter().fold(
        std::collections::BTreeMap::<AccountId, usize>::new(),
        |mut counts, (_, account, _)| {
            *counts.entry(*account).or_default() += 1;
            counts
        },
    );
    let accounts = configured
        .into_iter()
        // ASVS 8.2.1-8.2.3: never infer authority from a duplicate mapping.
        .filter(|(_, account, _)| duplicate_accounts[account] == 1)
        .map(|(persona, account, display_name)| {
            SeededDemoAccount::new(persona, account, display_name).map_err(anyhow::Error::msg)
        })
        .collect::<Result<Vec<_>>>()?;
    (!accounts.is_empty())
        .then(|| SeededDemoConfig::new(accounts).map_err(anyhow::Error::msg))
        .transpose()
}

fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} must be set"))?;
    if value.is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use learning_data_access::{
        SessionId, SessionLifetime, SessionRecord, SessionStore, SessionTokenHash, StoreError,
    };
    use question_model::{ProductRole, Timestamp};
    use std::{collections::BTreeMap, sync::Mutex};
    use tower::ServiceExt;

    fn account(value: u128) -> AccountId {
        AccountId::from_uuid(uuid::Uuid::from_u128(value))
    }

    #[test]
    fn duplicate_account_mappings_are_all_omitted_while_valid_mappings_remain() {
        let config = live_demo_config_from_account_mappings(vec![
            (
                SeededDemoPersona::ElenaInstructor,
                account(1),
                "Elena Instructor",
            ),
            (SeededDemoPersona::MaryStudent, account(1), "Mary Student"),
            (
                SeededDemoPersona::MorganSysadmin,
                account(2),
                "Morgan Sysadmin",
            ),
        ])
        .expect("trusted mappings should parse")
        .expect("one unambiguous mapping remains");

        assert_eq!(config.unavailable_account_count(), 4);
    }

    #[test]
    fn only_conflicting_mappings_leave_the_demo_capability_absent() {
        let config = live_demo_config_from_account_mappings(vec![
            (
                SeededDemoPersona::ElenaInstructor,
                account(1),
                "Elena Instructor",
            ),
            (SeededDemoPersona::MaryStudent, account(1), "Mary Student"),
        ])
        .expect("trusted mappings should parse");

        assert!(config.is_none());
    }

    #[test]
    fn missing_or_malformed_persona_values_do_not_remove_valid_demo_personas() {
        let values = BTreeMap::from([
            (
                "PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID",
                uuid::Uuid::from_u128(1).to_string(),
            ),
            (
                "PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID",
                "not-an-account-id".to_string(),
            ),
            (
                "PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID",
                uuid::Uuid::from_u128(2).to_string(),
            ),
        ]);

        let config = live_demo_config_from_environment_values(|name| values.get(name).cloned())
            .expect("independent deployment values should parse")
            .expect("the two valid persona mappings remain available");

        assert_eq!(config.unavailable_account_count(), 3);
    }

    #[derive(Clone, Default)]
    struct MemorySessionStore(Arc<Mutex<BTreeMap<SessionTokenHash, SessionRecord>>>);

    #[async_trait]
    impl SessionStore for MemorySessionStore {
        async fn create_session(
            &self,
            token_hash: SessionTokenHash,
            account: AccountId,
            lifetime: SessionLifetime,
        ) -> Result<SessionRecord, StoreError> {
            let record = SessionRecord {
                id: SessionId::generate()?,
                token_hash,
                account,
                product_role: ProductRole::Student,
                created_at: Timestamp::from_unix_millis(0),
                expires_at: Timestamp::from_unix_millis(i64::from(lifetime.as_seconds()) * 1_000),
            };
            self.0
                .lock()
                .expect("test store lock")
                .insert(token_hash, record.clone());
            Ok(record)
        }

        async fn resolve_session(
            &self,
            token_hash: SessionTokenHash,
        ) -> Result<Option<SessionRecord>, StoreError> {
            Ok(self
                .0
                .lock()
                .expect("test store lock")
                .get(&token_hash)
                .cloned())
        }

        async fn revoke_session(&self, token_hash: SessionTokenHash) -> Result<(), StoreError> {
            self.0.lock().expect("test store lock").remove(&token_hash);
            Ok(())
        }
    }

    #[tokio::test]
    async fn absent_demo_config_leaves_health_and_session_routes_mounted() {
        let sessions = Arc::new(MemorySessionStore::default());
        let session_config = production_session_config();
        let router = Router::new()
            .route("/health", get(|| async { StatusCode::OK }))
            .merge(session_router(Arc::clone(&sessions), session_config))
            .merge(live_demo_router(sessions, None, session_config));

        let health = router
            .clone()
            .oneshot(
                Request::get("/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(health.status(), StatusCode::OK);

        let current_session = router
            .clone()
            .oneshot(
                Request::get("/api/auth/session")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(current_session.status(), StatusCode::UNAUTHORIZED);

        let logout = router
            .clone()
            .oneshot(
                Request::post("/api/auth/logout")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(logout.status(), StatusCode::OK);

        let demo = router
            .oneshot(
                Request::get("/api/auth/live-demo/accounts")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(demo.status(), StatusCode::NOT_FOUND);
    }
}
