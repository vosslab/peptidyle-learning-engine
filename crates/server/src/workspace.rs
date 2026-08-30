//! Authenticated private workspace draft routes (MOD-UI-EDITOR).
//!
//! A workspace is deliberately unversioned authoring state. The route only
//! accepts the browser-safe draft definition and derives the actor from the
//! resolved server session; publication and source preparation remain owned by
//! the catalog and adapter boundaries.

use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};
use learning_data_access::{CatalogStore, SessionStore, Store};

use crate::catalog::BackendRegistry;

mod crud;
mod publication;
mod state;
mod support;

use crud::{delete_workspace, get_workspace, list_workspaces, save_workspace};
use publication::{publication_diff, validate_publication};
use state::WorkspaceRouteState;
use support::{MAX_WORKSPACE_BODY_BYTES, no_store_response};

/// Builds the author-only private workspace route group.
pub fn router<S, B>(store: Arc<S>, backends: Arc<B>) -> Router
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    Router::new()
        .route("/api/workspaces", get(list_workspaces::<S, B>))
        .route(
            "/api/workspaces/{workspace}",
            get(get_workspace::<S, B>)
                .put(save_workspace::<S, B>)
                .delete(delete_workspace::<S, B>),
        )
        .route(
            "/api/workspaces/{workspace}/publication-validation",
            post(validate_publication::<S, B>),
        )
        .route(
            "/api/workspaces/{workspace}/publication-diff",
            get(publication_diff::<S, B>),
        )
        .layer(DefaultBodyLimit::max(MAX_WORKSPACE_BODY_BYTES))
        // This also covers extractor rejections (invalid JSON, oversized
        // bodies, and malformed path values), which never reach a handler.
        .layer(middleware::map_response(no_store_response))
        .with_state(WorkspaceRouteState { store, backends })
}

#[cfg(test)]
#[path = "workspace/tests/mod.rs"]
mod tests;
