//! Authorized public-navigation reference resolution.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use learning_data_access::{NavigationReferenceStore, SessionStore};
use question_model::{
    AssignmentReference, CourseReference, NavigationResolution, RunReference, WorkspaceReference,
};

use crate::auth::{auth_error_response, no_store, resolve_request_session};

pub fn router<S>(store: Arc<S>) -> Router
where
    S: NavigationReferenceStore + SessionStore + 'static,
{
    Router::new()
        .route("/api/navigation/{reference}", get(resolve_reference::<S>))
        .with_state(store)
}

async fn resolve_reference<S>(
    State(store): State<Arc<S>>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response
where
    S: NavigationReferenceStore + SessionStore + 'static,
{
    let authenticated = match resolve_request_session(store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    let actor = authenticated.record.subject.user();
    let context = authenticated.tenant_context;
    let resolved = if let Ok(reference) = reference.parse::<CourseReference>() {
        store
            .resolve_course_reference(context, actor, reference)
            .await
            .map(|value| value.map(|course_id| NavigationResolution::Course { course_id }))
    } else if let Ok(reference) = reference.parse::<AssignmentReference>() {
        store
            .resolve_assignment_reference(context, actor, reference)
            .await
            .map(|value| {
                value.map(|identity| NavigationResolution::Assignment {
                    course_id: identity.course,
                    assignment_id: identity.assignment,
                })
            })
    } else if let Ok(reference) = reference.parse::<RunReference>() {
        store
            .resolve_run_reference(context, actor, reference)
            .await
            .map(|value| {
                value.map(|identity| NavigationResolution::Run {
                    course_id: identity.course,
                    assignment_id: identity.assignment,
                    enrollment_id: identity.enrollment,
                    run_id: identity.run,
                })
            })
    } else if let Ok(reference) = reference.parse::<WorkspaceReference>() {
        store
            .resolve_workspace_reference(context, actor, reference)
            .await
            .map(|value| value.map(|workspace_id| NavigationResolution::Workspace { workspace_id }))
    } else {
        return no_store(
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "navigation reference is invalid"})),
            )
                .into_response(),
        );
    };
    match resolved {
        Ok(Some(value)) => no_store(Json(value).into_response()),
        Ok(None) => no_store(
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "navigation target not found"})),
            )
                .into_response(),
        ),
        Err(_) => no_store(
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "navigation is temporarily unavailable"})),
            )
                .into_response(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use question_model::CourseId;
    use uuid::Uuid;

    use question_model::NavigationResolution;

    #[test]
    fn navigation_resolution_uses_the_camel_case_browser_contract() {
        let course_id = CourseId::from_uuid(Uuid::from_u128(1));
        let resolved = NavigationResolution::Course { course_id };

        let value = serde_json::to_value(resolved).expect("navigation resolution serializes");

        assert_eq!(value["kind"], "course");
        assert_eq!(value["courseId"], course_id.to_string());
        assert!(value.get("course_id").is_none());
    }
}
