use axum::Json;
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    AssignmentRecord, AssignmentRevision, AuthoritativeTimeStore, CatalogStore,
    CourseGroupManagementStore, SessionStore, Store, StoredAssignment,
};
use question_model::AssignmentTeachingSettings;
use serde::Serialize;

use super::projection::{error_response, store_error_response};
use super::routing::CourseRouteState;
use crate::auth::no_store;
use crate::http_refusal::{HttpRefusal, HttpResult};

mod definition_request;
mod student;
mod workspace;

pub(super) use workspace::{
    create_assignment_draft, get_assignment_workspace, get_instructor_student_view,
    replace_assignment_content, replace_assignment_fixed_item, replace_assignment_policies,
};

pub(super) use student::{
    assignment_landing_presentation, get_assignment_summary, get_student_assignment,
    instructor_student_view_delivery,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AssignmentRevisionHeaderError {
    Missing,
    Malformed,
}

pub(super) fn required_assignment_revision(
    headers: &HeaderMap,
) -> Result<AssignmentRevision, AssignmentRevisionHeaderError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(AssignmentRevisionHeaderError::Missing);
    };
    if values.next().is_some() {
        return Err(AssignmentRevisionHeaderError::Malformed);
    }
    let value = value
        .to_str()
        .map_err(|_| AssignmentRevisionHeaderError::Malformed)?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(AssignmentRevisionHeaderError::Malformed);
    };
    value
        .parse()
        .map_err(|_| AssignmentRevisionHeaderError::Malformed)
}

pub(super) async fn assignment_response<S>(
    state: &CourseRouteState<S>,
    authenticated: &crate::auth::AuthenticatedSession,
    status: StatusCode,
    assignment: StoredAssignment,
) -> Response
where
    S: Store
        + AuthoritativeTimeStore
        + CatalogStore
        + CourseGroupManagementStore
        + SessionStore
        + 'static,
{
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct AssignmentEditorResponse {
        #[serde(flatten)]
        assignment: question_model::AssignmentSummary,
        teaching_settings: question_model::assignment::InstructorAssignmentTeachingSettingsLocal,
        current_state: question_model::InstructorAssignmentCurrentState,
        publication_readiness: question_model::AssignmentPublicationReadiness,
        audience: question_model::AssignmentAudienceRequest,
    }
    let public_id = match state
        .store
        .assignment_reference(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            assignment.record.id,
        )
        .await
    {
        Ok(Some(public_id)) => public_id,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "assignment not found"),
        Err(error) => return store_error_response(error),
    };
    let course = match state
        .store
        .get_course(authenticated.tenant_context, assignment.record.course_id)
        .await
    {
        Ok(Some(course)) => course,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "course not found"),
        Err(error) => return store_error_response(error),
    };
    let (items, selection_groups) =
        match assignment_summary_items(state, authenticated.tenant_context, &assignment.record)
            .await
        {
            Ok(value) => value,
            Err(response) => return response.into_response(),
        };
    let settings = AssignmentTeachingSettings {
        lifecycle: assignment.record.lifecycle,
        instructions: assignment.record.instructions.clone(),
        base_policy: assignment.base_policy,
    };
    let now = match state
        .store
        .authoritative_time(authenticated.tenant_context)
        .await
    {
        Ok(now) => now,
        Err(error) => return store_error_response(error),
    };
    let audience =
        match assignment_audience_response(state, authenticated, &assignment.record).await {
            Ok(audience) => audience,
            Err(response) => return response.into_response(),
        };
    let mut response = (
        status,
        Json(AssignmentEditorResponse {
            assignment: assignment
                .record
                .summary(public_id, items, selection_groups),
        teaching_settings: match question_model::assignment::InstructorAssignmentTeachingSettingsLocal::from_absolute(&course.term, &settings) {
            Ok(settings) => settings,
            Err(_) => return error_response(StatusCode::SERVICE_UNAVAILABLE, "assignment teaching settings are invalid"),
        },
        current_state: match question_model::derive_instructor_assignment_current_state(&course.term, &settings, now) {
            Ok(value) => value,
            Err(_) => return error_response(StatusCode::SERVICE_UNAVAILABLE, "assignment teaching settings are invalid"),
        },
        publication_readiness: assignment.record.publication_readiness(),
        audience,
        }),
    )
        .into_response();
    let etag = format!("\"{}\"", assignment.revision.value());
    let header = HeaderValue::from_str(&etag).expect("positive revision produces a valid ETag");
    response.headers_mut().insert(ETAG, header);
    no_store(response)
}

async fn assignment_audience_response<S>(
    state: &CourseRouteState<S>,
    authenticated: &crate::auth::AuthenticatedSession,
    assignment: &AssignmentRecord,
) -> HttpResult<question_model::AssignmentAudienceRequest>
where
    S: CourseGroupManagementStore + 'static,
{
    match &assignment.audience {
        question_model::AssignmentAudience::CourseWide => {
            Ok(question_model::AssignmentAudienceRequest::CourseWide)
        }
        question_model::AssignmentAudience::AnyOfGroups(groups) => {
            let mut references = Vec::with_capacity(groups.iter().len());
            for group in groups.iter() {
                match state
                    .store
                    .get_course_group_by_id_for_instructor(
                        authenticated.tenant_context,
                        authenticated.record.subject.user(),
                        assignment.course_id,
                        group,
                    )
                    .await
                {
                    Ok(Some(group)) => references.push(group.reference),
                    Ok(None) => {
                        return Err(HttpRefusal::from(error_response(
                            StatusCode::SERVICE_UNAVAILABLE,
                            "assignment audience group is unavailable",
                        )));
                    }
                    Err(error) => return Err(HttpRefusal::from(store_error_response(error))),
                }
            }
            // Stored audience identities are canonicalized by opaque internal
            // IDs.  The browser contract instead uses public references, so
            // give it a stable public order independent of those IDs.
            references.sort_unstable();
            Ok(question_model::AssignmentAudienceRequest::AnyOfGroups { groups: references })
        }
    }
}

pub(super) async fn assignment_summary_items<S>(
    state: &CourseRouteState<S>,
    context: learning_data_access::TenantContext,
    assignment: &AssignmentRecord,
) -> HttpResult<(
    Vec<question_model::AssignmentItemSummary>,
    Vec<question_model::AssignmentSelectionGroupSummary>,
)>
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    let mut summaries = std::collections::BTreeMap::new();
    for reference in assignment.references() {
        let Some(record) = state
            .store
            .get_catalog_problem(context, reference)
            .await
            .map_err(|error| crate::http_refusal::HttpRefusal::from(store_error_response(error)))?
        else {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment contains an unavailable published question",
            )
            .into());
        };
        summaries.insert(reference, record);
    }
    let fixed = assignment
        .items
        .iter()
        .map(|item| {
            let record = summaries
                .get(&item.reference)
                .expect("every fixed item was resolved above");
            question_model::AssignmentItemSummary {
                id: item.id,
                question_id: record.question_id.clone(),
                title: record.question.metadata.title.clone(),
                backend: question_model::QuestionBackend::from(&record.question.source),
                capabilities: record.capabilities.clone(),
                position: item.position,
                points_possible: item.points_possible,
                delivery_state: item.delivery_state,
                scoring_mode: item.scoring_mode,
            }
        })
        .collect();
    let groups = assignment
        .selection_groups
        .iter()
        .map(|group| question_model::AssignmentSelectionGroupSummary {
            id: group.id,
            position: group.position,
            draw_count: group.draw_count,
            points_per_item: group.points_per_item,
            ordering: group.ordering,
            algorithm_version: group.algorithm.storage_version(),
            candidates: group
                .candidates
                .iter()
                .map(|candidate| {
                    let record = summaries
                        .get(&candidate.reference)
                        .expect("every candidate was resolved above");
                    question_model::AssignmentSelectionCandidateSummary {
                        id: candidate.id,
                        question_id: record.question_id.clone(),
                        title: record.question.metadata.title.clone(),
                        backend: question_model::QuestionBackend::from(&record.question.source),
                        capabilities: record.capabilities.clone(),
                        position: candidate.position,
                        delivery_state: candidate.delivery_state,
                    }
                })
                .collect(),
        })
        .collect();
    Ok((fixed, groups))
}
