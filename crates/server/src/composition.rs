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

const LIVE_DEMO_ACCOUNT_ID_ENV: [&str; 5] = [
    "PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID",
    "PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID",
    "PLE_LIVE_DEMO_JACK_STUDENT_ACCOUNT_ID",
    "PLE_LIVE_DEMO_AVERY_STUDENT_ACCOUNT_ID",
    "PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID",
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
    let values = LIVE_DEMO_ACCOUNT_ID_ENV.map(std::env::var);
    if values.iter().all(Result::is_err) {
        return Ok(None);
    }
    let accounts: [AccountId; 5] = values
        .into_iter()
        .map(|value| {
            let value = value.context("the seeded Live Demo requires all five Account IDs")?;
            let id = uuid::Uuid::parse_str(&value)
                .context("a seeded Live Demo Account ID must be a UUID")?;
            Ok(AccountId::from_uuid(id))
        })
        .collect::<Result<Vec<_>>>()?
        .try_into()
        .map_err(|_: Vec<AccountId>| {
            anyhow::anyhow!("the seeded Live Demo requires five Accounts")
        })?;
    let [elena, mary, jack, avery, morgan] = accounts;
    SeededDemoConfig::new([
        SeededDemoAccount::new(
            SeededDemoPersona::ElenaInstructor,
            elena,
            "Elena Instructor",
        )
        .map_err(anyhow::Error::msg)?,
        SeededDemoAccount::new(SeededDemoPersona::MaryStudent, mary, "Mary Student")
            .map_err(anyhow::Error::msg)?,
        SeededDemoAccount::new(SeededDemoPersona::JackStudent, jack, "Jack Student")
            .map_err(anyhow::Error::msg)?,
        SeededDemoAccount::new(SeededDemoPersona::AveryStudent, avery, "Avery Student")
            .map_err(anyhow::Error::msg)?,
        SeededDemoAccount::new(SeededDemoPersona::MorganSysadmin, morgan, "Morgan Sysadmin")
            .map_err(anyhow::Error::msg)?,
    ])
    .map(Some)
    .map_err(anyhow::Error::msg)
}

fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} must be set"))?;
    if value.is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value)
}
