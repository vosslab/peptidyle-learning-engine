//! Disposable live-demo entry that mints the ordinary PLE session.
//!
//! This module deliberately owns no alternate identity or session record. Its
//! closed deployment configuration replaces only the identity-verification
//! ceremony for the five documented demo personas.

use std::{collections::BTreeSet, sync::Arc};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderValue, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use question_model::{AccountId, AccountRole};
use serde::{Deserialize, Serialize};

use super::{AuthError, SessionConfig, SessionStore, issue_session, no_store};

const MAX_DISPLAY_NAME_CHARACTERS: usize = 200;

/// The five fixed, display-safe personas in a disposable live demo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SeededDemoPersona {
    ElenaInstructor,
    MaryStudent,
    JackStudent,
    AveryStudent,
    MorganSysadmin,
}

impl SeededDemoPersona {
    const ALL: [Self; 5] = [
        Self::ElenaInstructor,
        Self::MaryStudent,
        Self::JackStudent,
        Self::AveryStudent,
        Self::MorganSysadmin,
    ];

    fn required_role(self) -> AccountRole {
        match self {
            Self::ElenaInstructor => AccountRole::Instructor,
            Self::MaryStudent | Self::JackStudent | Self::AveryStudent => AccountRole::Student,
            Self::MorganSysadmin => AccountRole::Sysadmin,
        }
    }
}

/// One fixed deployment mapping from a browser-selectable persona to an Account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeededDemoAccount {
    persona: SeededDemoPersona,
    account: AccountId,
    display_name: Arc<str>,
}

impl SeededDemoAccount {
    /// Creates a display-safe, role-fixed demo mapping from trusted deployment input.
    pub fn new(
        persona: SeededDemoPersona,
        account: AccountId,
        display_name: impl Into<Arc<str>>,
    ) -> Result<Self, String> {
        let display_name = display_name.into();
        if display_name.trim().is_empty()
            || display_name.chars().count() > MAX_DISPLAY_NAME_CHARACTERS
        {
            return Err("seeded demo display name must be nonblank and bounded".to_string());
        }
        Ok(Self {
            persona,
            account,
            display_name,
        })
    }
}

/// Closed deployment configuration for the disposable direct-entry surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeededDemoConfig {
    accounts: [SeededDemoAccount; 5],
}

impl SeededDemoConfig {
    /// Validates the complete five-persona configuration before route assembly.
    ///
    /// ASVS 2.2.1: configuration is positively validated against the closed
    /// persona set and unique Account identities before it reaches a route.
    pub fn new(accounts: [SeededDemoAccount; 5]) -> Result<Self, String> {
        let personas = accounts
            .iter()
            .map(|account| account.persona)
            .collect::<BTreeSet<_>>();
        if personas.len() != SeededDemoPersona::ALL.len()
            || !SeededDemoPersona::ALL
                .iter()
                .all(|persona| personas.contains(persona))
        {
            return Err(
                "seeded demo configuration must contain each persona exactly once".to_string(),
            );
        }
        let identities = accounts
            .iter()
            .map(|account| account.account)
            .collect::<BTreeSet<_>>();
        if identities.len() != accounts.len() {
            return Err("seeded demo accounts must have distinct AccountIds".to_string());
        }
        Ok(Self { accounts })
    }

    fn account(&self, persona: SeededDemoPersona) -> Option<&SeededDemoAccount> {
        self.accounts
            .iter()
            .find(|account| account.persona == persona)
    }

    fn public_accounts(&self) -> Vec<SeededDemoAccountResponse> {
        self.accounts
            .iter()
            .map(|account| SeededDemoAccountResponse {
                persona: account.persona,
                display_name: account.display_name.to_string(),
            })
            .collect()
    }
}

struct LiveDemoState<S> {
    sessions: Arc<S>,
    config: SeededDemoConfig,
    session_config: SessionConfig,
}

impl<S> Clone for LiveDemoState<S> {
    fn clone(&self) -> Self {
        Self {
            sessions: Arc::clone(&self.sessions),
            config: self.config.clone(),
            session_config: self.session_config,
        }
    }
}

/// Adds the deployment-gated direct-entry routes to a normal session router.
pub fn live_demo_router<S>(
    sessions: Arc<S>,
    config: Option<SeededDemoConfig>,
    session_config: SessionConfig,
) -> Router
where
    S: SessionStore + 'static,
{
    let Some(config) = config else {
        return Router::new();
    };
    Router::new()
        .route(
            "/api/auth/live-demo/accounts",
            get(list_accounts::<S>).post(select_account::<S>),
        )
        .with_state(LiveDemoState {
            sessions,
            config,
            session_config,
        })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SeededDemoAccountResponse {
    persona: SeededDemoPersona,
    display_name: String,
}

#[derive(Debug, Serialize)]
struct SeededDemoAccountsResponse {
    accounts: Vec<SeededDemoAccountResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SelectSeededDemoAccountRequest {
    persona: SeededDemoPersona,
}

#[derive(Debug, Serialize)]
struct SelectedSeededDemoAccountResponse {
    authenticated: bool,
}

async fn list_accounts<S>(State(state): State<LiveDemoState<S>>) -> Response
where
    S: SessionStore + 'static,
{
    no_store(
        Json(SeededDemoAccountsResponse {
            accounts: state.config.public_accounts(),
        })
        .into_response(),
    )
}

async fn select_account<S>(
    State(state): State<LiveDemoState<S>>,
    Json(request): Json<SelectSeededDemoAccountRequest>,
) -> Response
where
    S: SessionStore + 'static,
{
    let Some(selected) = state.config.account(request.persona) else {
        return no_store((StatusCode::NOT_FOUND, "demo persona unavailable").into_response());
    };

    // ASVS 2.3.1 and 3.5.1: the route accepts only a closed persona and the
    // production browser boundary verifies the request's first-party origin.
    // ASVS 3.3.1, 3.3.3, 3.3.4: `issue_session` returns only a host-only,
    // Secure, HttpOnly session cookie carrying an opaque random credential.
    let issued = match issue_session(
        state.sessions.as_ref(),
        selected.account,
        state.session_config,
    )
    .await
    {
        Ok(issued) => issued,
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => {
            return no_store(
                (StatusCode::SERVICE_UNAVAILABLE, "demo entry unavailable").into_response(),
            );
        }
        Err(AuthError::Unauthenticated) => {
            return no_store((StatusCode::UNAUTHORIZED, "demo entry unavailable").into_response());
        }
    };
    if issued.record.role != selected.persona.required_role() {
        return no_store(
            (StatusCode::SERVICE_UNAVAILABLE, "demo entry unavailable").into_response(),
        );
    }
    let mut response = Json(SelectedSeededDemoAccountResponse {
        authenticated: true,
    })
    .into_response();
    let Ok(cookie) = HeaderValue::from_str(&issued.set_cookie) else {
        return no_store(
            (StatusCode::SERVICE_UNAVAILABLE, "demo entry unavailable").into_response(),
        );
    };
    response.headers_mut().append(SET_COOKIE, cookie);
    no_store(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn account(value: u128) -> AccountId {
        AccountId::from_uuid(Uuid::from_u128(value))
    }

    fn entry(persona: SeededDemoPersona, value: u128) -> SeededDemoAccount {
        SeededDemoAccount::new(persona, account(value), format!("{persona:?}"))
            .expect("bounded fixture entry")
    }

    #[test]
    fn configuration_requires_unique_personas_and_accounts() {
        let entries = [
            entry(SeededDemoPersona::ElenaInstructor, 1),
            entry(SeededDemoPersona::MaryStudent, 2),
            entry(SeededDemoPersona::JackStudent, 3),
            entry(SeededDemoPersona::AveryStudent, 4),
            entry(SeededDemoPersona::MorganSysadmin, 5),
        ];
        assert!(SeededDemoConfig::new(entries).is_ok());
    }
}
