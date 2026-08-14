use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use learning_data_access::{CatalogStore, OwnerCorrectionStore, SessionStore, Store};

use super::{BackendRegistry, PublicReviewGate};
use super::{lifecycle, publication, query};

const MAX_CATALOG_BODY_BYTES: usize = 64 * 1_024;

/// Builds the authenticated `/api/problems` and `/api/taxonomy` route group.
pub fn router<S, B, R>(store: Arc<S>, backends: Arc<B>, review_gate: Arc<R>) -> Router
where
    S: Store + CatalogStore + OwnerCorrectionStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let state = CatalogRouteState {
        store,
        backends,
        review_gate,
    };
    Router::new()
        .route("/api/problems", get(query::list_problems::<S, B, R>))
        .route(
            "/api/problems/search",
            get(query::search_problems::<S, B, R>),
        )
        .route(
            "/api/problems/by-id/{reference}",
            get(query::resolve_problem_reference::<S, B, R>),
        )
        .route(
            "/api/problems/{problem}/versions/{version}",
            get(query::get_problem::<S, B, R>),
        )
        .route(
            "/api/problems/{problem}/versions/{version}/detail",
            get(query::get_problem_detail::<S, B, R>),
        )
        .route(
            "/api/problems/{workspace}/publish",
            post(publication::publish_problem::<S, B, R>),
        )
        .route(
            "/api/problems/{problem}/versions/{version}/deprecate",
            post(lifecycle::deprecate_problem::<S, B, R>),
        )
        .route(
            "/api/problems/{problem}/versions/{version}/archive",
            post(lifecycle::archive_problem::<S, B, R>),
        )
        .route("/api/taxonomy", get(query::list_taxonomy::<S, B, R>))
        .layer(DefaultBodyLimit::max(MAX_CATALOG_BODY_BYTES))
        .with_state(state)
}

pub(super) struct CatalogRouteState<S, B, R> {
    pub(super) store: Arc<S>,
    pub(super) backends: Arc<B>,
    pub(super) review_gate: Arc<R>,
}

impl<S, B, R> Clone for CatalogRouteState<S, B, R> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            backends: Arc::clone(&self.backends),
            review_gate: Arc::clone(&self.review_gate),
        }
    }
}
