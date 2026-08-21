//! Course-local composition boundary for future teaching-operations routes.

mod authority;
mod groups;
mod modifiers_preview;
mod preview_plane;
mod student_targets;

use std::sync::Arc;

use axum::Router;
use learning_data_access::{
    AuthoritativeTimeStore, CourseGroupManagementStore, CourseRecordsAccessStore,
    NavigationReferenceStore, PreviewPlaneStore, SessionStore, Store,
    TeachingAuthorityReferenceStore, TeachingAuthorityStore,
};

/// Merges the course-local teaching-operations route groups.
pub(super) fn router<S>(store: Arc<S>) -> Router
where
    S: Store
        + CourseRecordsAccessStore
        + SessionStore
        + AuthoritativeTimeStore
        + CourseGroupManagementStore
        + TeachingAuthorityStore
        + TeachingAuthorityReferenceStore
        + NavigationReferenceStore
        + PreviewPlaneStore
        + 'static,
{
    Router::new()
        .merge(groups::router(Arc::clone(&store)))
        .merge(modifiers_preview::router(Arc::clone(&store)))
        .merge(preview_plane::router(Arc::clone(&store)))
        .merge(student_targets::router(Arc::clone(&store)))
        .merge(authority::router(store))
}
