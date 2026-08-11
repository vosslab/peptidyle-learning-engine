use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, put};
use learning_data_access::{
    CatalogStore, CourseRecordsAccessStore, CourseRosterStore, ManualGradeExportStore,
    SessionStore, Store,
};
use serde::{Deserialize, Serialize};

use super::assignments::{create_assignment, get_assignment, update_assignment};
use super::queries::{create_course, get_course, list_assignments, list_courses, list_gradebook};
use super::roster::{
    CourseInvitationDelivery, CourseInvitationIssuer, LocalDevelopmentRosterDirectory,
    UnavailableCourseInvitationDelivery, roster_router,
};

pub(super) const DEFAULT_PAGE_SIZE: u16 = 50;
const MAX_COURSE_BODY_BYTES: usize = 64 * 1_024;

/// Builds the authenticated course and assignment route group.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: Store
        + CatalogStore
        + CourseRecordsAccessStore
        + CourseRosterStore
        + ManualGradeExportStore
        + SessionStore
        + 'static,
{
    router_with_invitations_and_local_development(
        store,
        CourseInvitationIssuer::unavailable(),
        Arc::new(UnavailableCourseInvitationDelivery),
        None,
    )
}

/// Builds course routes with a configured server-only invitation issuer and
/// delivery service. The ordinary [`router`] keeps invitation creation
/// fail-closed for tests or deployments that have not configured mail.
pub fn router_with_invitations<S>(
    store: Arc<S>,
    issuer: CourseInvitationIssuer,
    delivery: Arc<dyn CourseInvitationDelivery>,
) -> Router
where
    S: Store
        + CatalogStore
        + CourseRecordsAccessStore
        + CourseRosterStore
        + ManualGradeExportStore
        + SessionStore
        + 'static,
{
    router_with_invitations_and_local_development(store, issuer, delivery, None)
}

/// Builds the local-development-only course router. The caller can only
/// construct its alias directory from the paired file-backed identity mode.
pub(crate) fn router_with_invitations_and_local_development<S>(
    store: Arc<S>,
    issuer: CourseInvitationIssuer,
    delivery: Arc<dyn CourseInvitationDelivery>,
    local_development_roster: Option<Arc<LocalDevelopmentRosterDirectory>>,
) -> Router
where
    S: Store
        + CatalogStore
        + CourseRecordsAccessStore
        + CourseRosterStore
        + ManualGradeExportStore
        + SessionStore
        + 'static,
{
    let course_routes = Router::new()
        .route(
            "/api/courses",
            get(list_courses::<S>).post(create_course::<S>),
        )
        .route(
            "/api/courses/{course}/assignments",
            get(list_assignments::<S>).post(create_assignment::<S>),
        )
        .route("/api/courses/{course}/gradebook", get(list_gradebook::<S>))
        .route("/api/courses/{course}", get(get_course::<S>))
        .route("/api/assignments/{assignment}", get(get_assignment::<S>))
        .route(
            "/api/courses/{course}/assignments/{assignment}",
            put(update_assignment::<S>),
        )
        .layer(DefaultBodyLimit::max(MAX_COURSE_BODY_BYTES))
        .with_state(CourseRouteState {
            store: Arc::clone(&store),
        });
    course_routes.merge(roster_router(
        store,
        issuer,
        delivery,
        local_development_roster,
    ))
}

pub(super) struct CourseRouteState<S> {
    pub(super) store: Arc<S>,
}

impl<S> Clone for CourseRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub(super) struct CourseQuery {
    pub(super) cursor: Option<String>,
    pub(super) page_size: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub(super) struct CreateCourseRequest {
    pub(super) title: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub(super) struct CreateAssignmentRequest {
    pub(super) title: String,
    pub(super) problems: Vec<question_model::ProblemVersionRef>,
    pub(super) policies: question_model::RunPolicies,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UpdateAssignmentRequest {
    pub(super) title: String,
    pub(super) problems: Vec<question_model::ProblemVersionRef>,
    pub(super) policies: question_model::RunPolicies,
}

/// Rejects unknown fields at every level by comparing the request to the
/// canonical wire form of the typed model, mirroring the workspace boundary.
pub(super) fn strict_assignment_request<T>(value: serde_json::Value) -> Result<T, ()>
where
    T: serde::de::DeserializeOwned + Serialize,
{
    let request = serde_json::from_value(value.clone()).map_err(|_| ())?;
    if serde_json::to_value(&request).map_err(|_| ())? == value {
        Ok(request)
    } else {
        Err(())
    }
}
