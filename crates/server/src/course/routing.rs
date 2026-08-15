use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, post, put};
use learning_data_access::{
    CatalogStore, CourseInvitationDeliveryStore, CourseRecordsAccessStore, CourseRosterStore,
    ManualGradeExportStore, SessionStore, Store,
};
use serde::{Deserialize, Serialize};

use super::assignments::{
    add_assignment_item, create_assignment, get_assignment, remove_assignment_item,
    replace_assignment_item_question, update_assignment,
};
use super::invitation_capability::CourseInvitationIssuer;
use super::queries::{create_course, get_course, list_assignments, list_courses, list_gradebook};
use super::roster::{LocalTeachingRosterDirectory, roster_router};

pub(super) const DEFAULT_PAGE_SIZE: u16 = 50;
const MAX_COURSE_BODY_BYTES: usize = 64 * 1_024;

/// Builds the authenticated course and assignment route group.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: Store
        + CatalogStore
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + ManualGradeExportStore
        + SessionStore
        + 'static,
{
    router_with_invitations_and_local_teaching(store, CourseInvitationIssuer::unavailable(), None)
}

/// Builds course routes with a configured server-only invitation issuer.
pub fn router_with_invitations<S>(store: Arc<S>, issuer: CourseInvitationIssuer) -> Router
where
    S: Store
        + CatalogStore
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + ManualGradeExportStore
        + SessionStore
        + 'static,
{
    router_with_invitations_and_local_teaching(store, issuer, None)
}

/// Builds the local-teaching course router with a server-owned learner directory.
/// Production composition always passes `None`, so this route cannot be mounted
/// outside the paired local authentication mode.
pub(crate) fn router_with_invitations_and_local_teaching<S>(
    store: Arc<S>,
    issuer: CourseInvitationIssuer,
    local_teaching_roster: Option<Arc<LocalTeachingRosterDirectory>>,
) -> Router
where
    S: Store
        + CatalogStore
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
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
        .route(
            "/api/courses/{course}/assignments/{assignment}/items",
            post(add_assignment_item::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/items/{item}",
            delete(remove_assignment_item::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/items/{item}/question",
            put(replace_assignment_item_question::<S>),
        )
        .layer(DefaultBodyLimit::max(MAX_COURSE_BODY_BYTES))
        .with_state(CourseRouteState {
            store: Arc::clone(&store),
        });
    course_routes.merge(roster_router(store, issuer, local_teaching_roster))
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
    pub(super) question_ids: Vec<question_model::QuestionId>,
    pub(super) policies: question_model::RunPolicies,
    /// Whole-run timing is always an explicit instructor decision. `null`
    /// within the object deliberately means Untimed.
    pub(super) assignment_timing: question_model::AssignmentRunTiming,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UpdateAssignmentRequest {
    pub(super) title: String,
    pub(super) items: Vec<AssignmentItemUpdateRequest>,
    pub(super) policies: question_model::RunPolicies,
    pub(super) assignment_timing: question_model::AssignmentRunTiming,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AssignmentItemUpdateRequest {
    pub(super) id: question_model::AssignmentItemId,
    pub(super) question_id: question_model::QuestionId,
    pub(super) position: u32,
    pub(super) points_possible: question_model::PointValue,
    pub(super) delivery_state: question_model::AssignmentDeliveryState,
    pub(super) scoring_mode: question_model::AssignmentScoringMode,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AddAssignmentItemRequest {
    pub(super) question_id: question_model::QuestionId,
    pub(super) position: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ReplaceAssignmentItemQuestionRequest {
    pub(super) question_id: question_model::QuestionId,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn explicit_request() -> serde_json::Value {
        serde_json::to_value(CreateAssignmentRequest {
            title: "Mastery".to_string(),
            question_ids: Vec::new(),
            policies: question_model::RunPolicies {
                completion: question_model::CompletionRequirement::AllCorrect,
                grade: question_model::GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
            assignment_timing: question_model::AssignmentRunTiming::default(),
        })
        .expect("request fixture serializes")
    }

    #[test]
    fn assignment_timing_requires_an_object_with_nullable_member() {
        let explicit = strict_assignment_request::<CreateAssignmentRequest>(explicit_request())
            .expect("explicit null member is an untimed editor choice");
        assert_eq!(
            explicit.assignment_timing,
            question_model::AssignmentRunTiming::default()
        );

        let mut omitted = explicit_request();
        omitted
            .as_object_mut()
            .expect("request object")
            .remove("assignmentTiming");
        assert!(strict_assignment_request::<CreateAssignmentRequest>(omitted).is_err());

        let mut invalid = explicit_request();
        invalid["assignmentTiming"] = serde_json::Value::Null;
        assert!(strict_assignment_request::<CreateAssignmentRequest>(invalid).is_err());
    }
}
