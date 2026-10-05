//! Sysadmin-only Instructor Account management routes.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use learning_data_access::{
    CreateInstructorAccountInput, DeactivateInstructorAccountInput, InstructorAccountBrowse,
    InstructorAccountState, InstructorAccountStore, SessionTokenHash, StoreError,
    postgres::{PostgresInstructorAccountStore, PostgresSessionStore},
};
use question_model::{AccountId, UserRole};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AuthError, resolve_session},
    instructor_setup_email_delivery::InstructorSetupEmailDelivery,
};

#[derive(Clone)]
struct InstructorAccountRouteState {
    sessions: Arc<PostgresSessionStore>,
    accounts: PostgresInstructorAccountStore,
    setup_email: Arc<dyn InstructorSetupEmailDelivery>,
}

/// Registers the Sysadmin Instructor Accounts task.
pub fn instructor_account_router(
    sessions: Arc<PostgresSessionStore>,
    accounts: PostgresInstructorAccountStore,
    setup_email: Arc<dyn InstructorSetupEmailDelivery>,
) -> Router {
    Router::new()
        .route(
            "/api/instructor-accounts",
            get(list_instructor_accounts).post(create_instructor_account),
        )
        .route(
            "/api/instructor-accounts/find",
            post(find_instructor_accounts),
        )
        .route(
            "/api/instructor-accounts/{account_id}/send-setup-email",
            post(send_setup_email),
        )
        .route(
            "/api/instructor-accounts/{account_id}/deactivate",
            post(deactivate_instructor_account),
        )
        .route(
            "/api/instructor-accounts/{account_id}/reactivate",
            post(reactivate_instructor_account),
        )
        .with_state(InstructorAccountRouteState {
            sessions,
            accounts,
            setup_email,
        })
}

async fn list_instructor_accounts(
    State(state): State<InstructorAccountRouteState>,
    headers: HeaderMap,
) -> Response {
    let token = match sysadmin_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    match state
        .accounts
        .list_instructor_accounts(
            token,
            InstructorAccountBrowse {
                query: None,
                state: None,
                after_account_id: None,
                page_size: None,
            },
        )
        .await
    {
        // ASVS 8.2.3: rows expose only closed Account state plus a nullable
        // static provided-avatar ID; the display zone is viewer-owned context.
        Ok(accounts) => crate::auth::no_store(Json(accounts).into_response()),
        Err(error) => store_error_response(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct FindInstructorAccountsRequest {
    query: String,
    state: Option<InstructorAccountState>,
    page_size: i32,
    after_account_id: Option<AccountId>,
}

async fn find_instructor_accounts(
    State(state): State<InstructorAccountRouteState>,
    headers: HeaderMap,
    payload: Result<Json<FindInstructorAccountsRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(value) => value,
        Err(_) => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Instructor Account search is invalid",
            );
        }
    };
    let token = match sysadmin_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    // ASVS 2.2.1, 7.1.1, and 8.2.3: state, page size, and cursor stay in the
    // body with the query. The response keeps the closed Account summary.
    // Neither the authentication email nor the Profile display name is written back.
    match state
        .accounts
        .list_instructor_accounts(
            token,
            InstructorAccountBrowse {
                query: Some(request.query),
                state: request.state,
                after_account_id: request.after_account_id,
                page_size: Some(request.page_size),
            },
        )
        .await
    {
        Ok(accounts) => crate::auth::no_store(Json(accounts).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn create_instructor_account(
    State(state): State<InstructorAccountRouteState>,
    headers: HeaderMap,
    Json(input): Json<CreateInstructorAccountInput>,
) -> Response {
    let token = match sysadmin_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    match state.accounts.create_instructor_account(token, input).await {
        Ok(created) => {
            let setup_email_sent = state
                .setup_email
                .send_setup_email(&created.setup_email_destination)
                .await
                .is_ok();
            crate::auth::no_store(
                (
                    StatusCode::CREATED,
                    Json(InstructorAccountCreationResponse {
                        account: created.account,
                        setup_email_sent,
                    }),
                )
                    .into_response(),
            )
        }
        Err(error) => store_error_response(error),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstructorAccountCreationResponse {
    account: learning_data_access::InstructorAccountSummary,
    setup_email_sent: bool,
}

async fn send_setup_email(
    State(state): State<InstructorAccountRouteState>,
    headers: HeaderMap,
    Path(account_id): Path<String>,
) -> Response {
    let account_id = match AccountId::from_str(&account_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match sysadmin_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    match state
        .accounts
        .instructor_setup_email_destination(token, account_id)
        .await
    {
        Ok(destination) => crate::auth::no_store(
            Json(InstructorSetupEmailResponse {
                setup_email_sent: state
                    .setup_email
                    .send_setup_email(&destination)
                    .await
                    .is_ok(),
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstructorSetupEmailResponse {
    setup_email_sent: bool,
}

async fn deactivate_instructor_account(
    State(state): State<InstructorAccountRouteState>,
    headers: HeaderMap,
    Path(account_id): Path<String>,
    Json(input): Json<DeactivateInstructorAccountInput>,
) -> Response {
    let account_id = match AccountId::from_str(&account_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match sysadmin_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    match state
        .accounts
        .deactivate_instructor_account(token, account_id, input)
        .await
    {
        Ok(account) => crate::auth::no_store(Json(account).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn reactivate_instructor_account(
    State(state): State<InstructorAccountRouteState>,
    headers: HeaderMap,
    Path(account_id): Path<String>,
) -> Response {
    let account_id = match AccountId::from_str(&account_id) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match sysadmin_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    match state
        .accounts
        .reactivate_instructor_account(token, account_id)
        .await
    {
        Ok(account) => crate::auth::no_store(Json(account).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn sysadmin_session_hash(
    state: &InstructorAccountRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        // ASVS 8.2.1: route filtering is only an early boundary; the Store's
        // database procedures repeat the active Sysadmin authorization.
        Ok(session) if session.record.user_role == UserRole::Sysadmin => Ok(session.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Instructor Accounts authentication unavailable",
        ))),
    }
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => route_error(
            StatusCode::PRECONDITION_FAILED,
            "Instructor Account changed",
        ),
        StoreError::LifecycleConflict => route_error(
            StatusCode::CONFLICT,
            "Instructor Account lifecycle conflict",
        ),
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Instructor Account is invalid",
        ),
        StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Instructor Account conflict")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Instructor Accounts unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Instructor Account not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

#[cfg(test)]
mod tests {
    use axum::Json;
    use axum::extract::{Path, State};
    use axum::http::{HeaderMap, StatusCode};

    use super::*;

    fn unreachable_pool() -> learning_data_access::postgres::Pool {
        learning_data_access::postgres::lazy_pool("postgres://ple:ple@127.0.0.1:1/ple")
            .expect("lazy pool")
    }

    #[tokio::test]
    async fn deactivate_instructor_account_rejects_a_bad_checksum_before_database_lookup() {
        let pool = unreachable_pool();
        let state = InstructorAccountRouteState {
            sessions: Arc::new(PostgresSessionStore::new(pool.clone())),
            accounts: PostgresInstructorAccountStore::new(pool),
            setup_email: Arc::new(
                crate::instructor_setup_email_delivery::NotConfiguredInstructorSetupEmailDelivery,
            ),
        };
        let response = deactivate_instructor_account(
            State(state),
            HeaderMap::new(),
            Path("UABCDEFG0".to_string()),
            Json(DeactivateInstructorAccountInput {
                reason: "Checksum gate".to_string(),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
