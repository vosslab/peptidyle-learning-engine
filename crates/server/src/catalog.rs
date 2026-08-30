//! Authenticated catalog, publication, and lifecycle routes (MOD-API-CAT).
//!
//! This facade preserves the stable catalog API while capability owners keep
//! route assembly, publication, browsing, lifecycle, and response behavior
//! independently readable.

mod capabilities;
mod lifecycle;
mod publication;
mod query;
mod response;
mod routes;

pub use capabilities::{
    BackendRegistry, BackendRegistryError, PublicReviewGate, ReviewGateError, ReviewNotRequired,
};
pub use routes::router;

pub(crate) use publication::{
    dispatch_publication, may_publish, mint_publication_reference, prepare_published_source,
};
pub(crate) use response::{error_response, store_error_response};

#[cfg(test)]
pub(crate) use publication::PUBLICATION_MINT_COUNT;
