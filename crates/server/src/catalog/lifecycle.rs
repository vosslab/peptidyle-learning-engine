use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{CatalogStore, CatalogTransition, SessionStore, Store};
use question_model::{ProblemId, ProblemVersionRef, UserRole, VersionId};
use serde::Deserialize;

use crate::auth::{auth_error_response, no_store, resolve_request_session};

use super::routes::CatalogRouteState;
use super::{BackendRegistry, PublicReviewGate};
use super::{error_response, store_error_response};

#[derive(Debug, Deserialize)]
pub(super) struct DeprecateProblemRequest {
    reason: String,
}

pub(super) async fn deprecate_problem<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Path((problem, version)): Path<(ProblemId, VersionId)>,
    Json(request): Json<DeprecateProblemRequest>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    transition_problem(
        state,
        headers,
        ProblemVersionRef { problem, version },
        CatalogTransition::Deprecate {
            reason: request.reason,
        },
    )
    .await
}

pub(super) async fn archive_problem<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Path((problem, version)): Path<(ProblemId, VersionId)>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    transition_problem(
        state,
        headers,
        ProblemVersionRef { problem, version },
        CatalogTransition::Archive,
    )
    .await
}

async fn transition_problem<S, B, R>(
    state: CatalogRouteState<S, B, R>,
    headers: HeaderMap,
    reference: ProblemVersionRef,
    transition: CatalogTransition,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_manage_catalog(authenticated.record.subject.roles()) {
        return error_response(StatusCode::FORBIDDEN, "catalog change is not authorized");
    }
    let actor = authenticated.record.subject.user();
    match state
        .store
        .transition_catalog_problem(authenticated.tenant_context, actor, reference, transition)
        .await
    {
        Ok(record) => no_store(Json(record.summary()).into_response()),
        Err(error) => store_error_response(error),
    }
}

fn may_manage_catalog(roles: &[UserRole]) -> bool {
    roles
        .iter()
        .any(|role| matches!(role, UserRole::Instructor | UserRole::Sysadmin))
}
