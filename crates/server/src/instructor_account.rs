//! Sysadmin-only Instructor Account management routes.

use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use learning_data_access::{
    CreateInstructorAccountInput, DeactivateInstructorAccountInput, InstructorAccountStore,
    SessionTokenHash, StoreError,
    postgres::{PostgresInstructorAccountStore, PostgresSessionStore},
};
use question_model::{AccountReference, ProductRole};

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct InstructorAccountRouteState {
    sessions: Arc<PostgresSessionStore>,
    accounts: PostgresInstructorAccountStore,
}

/// Registers the Sysadmin Instructor Accounts task.
pub fn instructor_account_router(
    sessions: Arc<PostgresSessionStore>,
    accounts: PostgresInstructorAccountStore,
) -> Router {
    Router::new()
        .route(
            "/api/instructor-accounts",
            get(list_instructor_accounts).post(create_instructor_account),
        )
        .route(
            "/api/instructor-accounts/{reference}/deactivate",
            post(deactivate_instructor_account),
        )
        .route(
            "/api/instructor-accounts/{reference}/reactivate",
            post(reactivate_instructor_account),
        )
        .with_state(InstructorAccountRouteState { sessions, accounts })
}

async fn list_instructor_accounts(
    State(state): State<InstructorAccountRouteState>,
    headers: HeaderMap,
) -> Response {
    let token = match sysadmin_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    match state.accounts.list_instructor_accounts(token).await {
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
        Ok(account) => crate::auth::no_store((StatusCode::CREATED, Json(account)).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn deactivate_instructor_account(
    State(state): State<InstructorAccountRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    Json(input): Json<DeactivateInstructorAccountInput>,
) -> Response {
    let reference = match AccountReference::from_str(&reference) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match sysadmin_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    match state
        .accounts
        .deactivate_instructor_account(token, reference, input)
        .await
    {
        Ok(account) => crate::auth::no_store(Json(account).into_response()),
        Err(error) => store_error_response(error),
    }
}

async fn reactivate_instructor_account(
    State(state): State<InstructorAccountRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let reference = match AccountReference::from_str(&reference) {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let token = match sysadmin_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    match state
        .accounts
        .reactivate_instructor_account(token, reference)
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
        Ok(session) if session.record.product_role == ProductRole::Sysadmin => {
            Ok(session.session_hash)
        }
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
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Instructor Account is invalid",
        ),
        StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Instructor Account conflict")
        }
        StoreError::AssignmentActivity(_) | StoreError::TimedOut | StoreError::Unavailable(_) => {
            route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Instructor Accounts unavailable",
            )
        }
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Instructor Account not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}
