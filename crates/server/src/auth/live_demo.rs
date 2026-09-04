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
use question_model::{AccountId, ProductRole};
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

    fn required_product_role(self) -> ProductRole {
        match self {
            Self::ElenaInstructor => ProductRole::Instructor,
            Self::MaryStudent | Self::JackStudent | Self::AveryStudent => ProductRole::Student,
            Self::MorganSysadmin => ProductRole::Sysadmin,
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
    accounts: Vec<SeededDemoAccount>,
}

impl SeededDemoConfig {
    /// Validates a unique nonempty subset of the fixed persona configuration.
    ///
    /// ASVS 2.2.1: configuration is positively validated against the closed
    /// persona set and unique Account identities before it reaches a route.
    pub fn new(accounts: Vec<SeededDemoAccount>) -> Result<Self, String> {
        if accounts.is_empty() {
            return Err("seeded demo configuration must contain at least one persona".to_string());
        }
        let personas = accounts
            .iter()
            .map(|account| account.persona)
            .collect::<BTreeSet<_>>();
        if personas.len() != accounts.len()
            || !personas
                .iter()
                .all(|persona| SeededDemoPersona::ALL.contains(persona))
        {
            return Err("seeded demo configuration must contain unique known personas".to_string());
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

    pub(crate) fn unavailable_account_count(&self) -> usize {
        SeededDemoPersona::ALL.len() - self.accounts.len()
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
#[serde(rename_all = "camelCase")]
struct SeededDemoAccountsResponse {
    accounts: Vec<SeededDemoAccountResponse>,
    unavailable_account_count: usize,
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
            unavailable_account_count: state.config.unavailable_account_count(),
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
    if issued.record.product_role != selected.persona.required_product_role() {
        // ASVS 2.3.3 and 8.3.1: a store-derived role mismatch invalidates the
        // whole operation.  The just-issued credential was never exposed, and
        // is revoked before this bounded failure response.
        let _ = state
            .sessions
            .revoke_session(issued.record.token_hash)
            .await;
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
    use async_trait::async_trait;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use learning_data_access::{
        SessionId, SessionLifetime, SessionRecord, SessionStore, SessionTokenHash, StoreError,
    };
    use question_model::Timestamp;
    use std::{collections::BTreeMap, sync::Mutex};
    use tower::ServiceExt;
    use uuid::Uuid;

    fn account(value: u128) -> AccountId {
        AccountId::from_uuid(Uuid::from_u128(value))
    }

    fn entry(persona: SeededDemoPersona, value: u128) -> SeededDemoAccount {
        SeededDemoAccount::new(persona, account(value), format!("{persona:?}"))
            .expect("bounded fixture entry")
    }

    #[test]
    fn configuration_accepts_a_unique_nonempty_subset() {
        let entries = vec![
            entry(SeededDemoPersona::ElenaInstructor, 1),
            entry(SeededDemoPersona::MaryStudent, 2),
        ];
        assert!(SeededDemoConfig::new(entries).is_ok());
    }

    #[test]
    fn configuration_rejects_empty_or_duplicate_identity_subsets() {
        assert!(SeededDemoConfig::new(vec![]).is_err());
        assert!(
            SeededDemoConfig::new(vec![
                entry(SeededDemoPersona::ElenaInstructor, 1),
                entry(SeededDemoPersona::MaryStudent, 1),
            ])
            .is_err()
        );
    }

    #[derive(Clone)]
    struct RoleMismatchStore(Arc<Mutex<BTreeMap<SessionTokenHash, SessionRecord>>>);

    #[async_trait]
    impl SessionStore for RoleMismatchStore {
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
    async fn list_reports_only_retained_personas_and_unavailable_count() {
        let store = Arc::new(RoleMismatchStore(Arc::new(Mutex::new(BTreeMap::new()))));
        let router = live_demo_router(
            store,
            Some(
                SeededDemoConfig::new(vec![entry(SeededDemoPersona::MaryStudent, 2)])
                    .expect("valid subset"),
            ),
            SessionConfig::new(
                SessionLifetime::from_seconds(60).expect("positive lifetime"),
                super::super::CookieTransport::FirstPartyHttps,
            ),
        );
        let response = router
            .oneshot(
                Request::get("/api/auth/live-demo/accounts")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        assert_eq!(
            body,
            r#"{"accounts":[{"persona":"maryStudent","displayName":"MaryStudent"}],"unavailableAccountCount":4}"#
        );
    }

    #[tokio::test]
    async fn wrong_role_selection_revokes_the_unexposed_session() {
        let records = Arc::new(Mutex::new(BTreeMap::new()));
        let store = Arc::new(RoleMismatchStore(Arc::clone(&records)));
        let router = live_demo_router(
            store,
            Some(
                SeededDemoConfig::new(vec![entry(SeededDemoPersona::ElenaInstructor, 2)])
                    .expect("valid subset"),
            ),
            SessionConfig::new(
                SessionLifetime::from_seconds(60).expect("positive lifetime"),
                super::super::CookieTransport::FirstPartyHttps,
            ),
        );
        let response = router
            .oneshot(
                Request::post("/api/auth/live-demo/accounts")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"persona":"elenaInstructor"}"#))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(records.lock().expect("test store lock").is_empty());
    }

    #[tokio::test]
    async fn surviving_persona_issues_an_ordinary_session_with_its_account_role() {
        let records = Arc::new(Mutex::new(BTreeMap::new()));
        let store = Arc::new(RoleMismatchStore(Arc::clone(&records)));
        let session_config = SessionConfig::new(
            SessionLifetime::from_seconds(60).expect("positive lifetime"),
            super::super::CookieTransport::FirstPartyHttps,
        );
        let router = super::super::session_router(Arc::clone(&store), session_config).merge(
            live_demo_router(
                store,
                Some(
                    SeededDemoConfig::new(vec![entry(SeededDemoPersona::MaryStudent, 2)])
                        .expect("valid subset"),
                ),
                session_config,
            ),
        );

        let unavailable = router
            .clone()
            .oneshot(
                Request::post("/api/auth/live-demo/accounts")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"persona":"elenaInstructor"}"#))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(unavailable.status(), StatusCode::NOT_FOUND);

        let selection = router
            .clone()
            .oneshot(
                Request::post("/api/auth/live-demo/accounts")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"persona":"maryStudent"}"#))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(selection.status(), StatusCode::OK);
        let cookie = selection
            .headers()
            .get(SET_COOKIE)
            .expect("successful demo selection issues the ordinary session cookie")
            .to_str()
            .expect("cookie is ASCII")
            .split(';')
            .next()
            .expect("cookie has a name and value")
            .to_owned();

        let current_session = router
            .oneshot(
                Request::get("/api/auth/session")
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(current_session.status(), StatusCode::OK);
        let body = to_bytes(current_session.into_body(), usize::MAX)
            .await
            .expect("body");
        assert_eq!(
            body,
            r#"{"authenticated":true,"account":{"id":"00000000-0000-0000-0000-000000000002","productRole":"student"}}"#
        );
    }
}
