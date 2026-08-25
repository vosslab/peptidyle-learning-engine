//! Projection of resolved policy decisions into instructor preview views.

use domain::effective_assignment_policy::{LateVerdict, PolicySource, ResolvedField, StartVerdict};
use learning_data_access::{CourseGroupManagementStore, TeachingAuthorityReferenceStore};
use question_model::{
    AssignmentTeachingSettingsField, CourseId, TeachingLateVerdict,
    TeachingPreviewDeadlineBehaviorField, TeachingPreviewFieldSource, TeachingPreviewGroupSource,
    TeachingPreviewGroupSources, TeachingPreviewLateSubmissionField, TeachingPreviewLimitField,
    TeachingPreviewTimeField, TeachingPreviewView, TeachingStartVerdict,
};

use super::super::super::projection::{error_response, store_error_response};
use super::super::super::routing::CourseRouteState;
use super::support::{hypothetical_source_response, label};
use crate::http_refusal::{HttpRefusal, HttpResult};

pub(super) async fn preview_view<S>(
    state: &CourseRouteState<S>,
    auth: &crate::auth::AuthenticatedSession,
    course: CourseId,
    student: question_model::StudentId,
    term: &question_model::CourseTerm,
    policy: domain::effective_assignment_policy::EffectiveAssignmentPolicy,
    start: StartVerdict,
) -> HttpResult<TeachingPreviewView>
where
    S: CourseGroupManagementStore + TeachingAuthorityReferenceStore + 'static,
{
    Ok(TeachingPreviewView::Allowed {
        time_zone: term.time_zone().clone(),
        start: start_view(start),
        available_at: preview_time_field(
            state,
            auth,
            course,
            student,
            term,
            policy.available_at,
            AssignmentTeachingSettingsField::AvailableAt,
        )
        .await?,
        due_at: preview_time_field(
            state,
            auth,
            course,
            student,
            term,
            policy.due_at,
            AssignmentTeachingSettingsField::DueAt,
        )
        .await?,
        closes_at: preview_time_field(
            state,
            auth,
            course,
            student,
            term,
            policy.closes_at,
            AssignmentTeachingSettingsField::ClosesAt,
        )
        .await?,
        time_limit_seconds: TeachingPreviewLimitField {
            value: policy.time_limit_seconds.value,
            source: source(
                state,
                auth,
                course,
                student,
                policy.time_limit_seconds.source,
            )
            .await?,
        },
        attempt_limit: TeachingPreviewLimitField {
            value: policy.attempt_limit.value,
            source: source(state, auth, course, student, policy.attempt_limit.source).await?,
        },
        late_submission: TeachingPreviewLateSubmissionField {
            value: policy.late_submission.value,
            source: source(state, auth, course, student, policy.late_submission.source).await?,
        },
        deadline_behavior: TeachingPreviewDeadlineBehaviorField {
            value: policy.deadline_behavior.value,
            source: source(
                state,
                auth,
                course,
                student,
                policy.deadline_behavior.source,
            )
            .await?,
        },
    })
}

async fn preview_time_field<S>(
    state: &CourseRouteState<S>,
    auth: &crate::auth::AuthenticatedSession,
    course: CourseId,
    student: question_model::StudentId,
    term: &question_model::CourseTerm,
    field: ResolvedField<Option<question_model::ActivityTimestamp>>,
    settings_field: AssignmentTeachingSettingsField,
) -> HttpResult<TeachingPreviewTimeField>
where
    S: CourseGroupManagementStore + TeachingAuthorityReferenceStore + 'static,
{
    let source = source(state, auth, course, student, field.source).await?;
    question_model::project_teaching_preview_time_field(field.value, source, term, settings_field)
        .map_err(|_| {
            HttpRefusal::from(error_response(
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "preview time is invalid",
            ))
        })
}

fn start_view(value: StartVerdict) -> TeachingStartVerdict {
    match value {
        StartVerdict::MayStart { late } => TeachingStartVerdict::MayStart {
            late: match late {
                LateVerdict::OnTime => TeachingLateVerdict::OnTime,
                LateVerdict::AcceptedLate => TeachingLateVerdict::AcceptedLate,
                LateVerdict::MarkedLate => TeachingLateVerdict::MarkedLate,
                LateVerdict::RejectedLate => TeachingLateVerdict::MarkedLate,
            },
        },
        StartVerdict::NotYetAvailable => TeachingStartVerdict::NotYetAvailable,
        StartVerdict::Closed => TeachingStartVerdict::Closed,
        StartVerdict::AttemptLimitReached => TeachingStartVerdict::AttemptLimitReached,
        StartVerdict::DueDateRejectsNewRun => TeachingStartVerdict::DueDateRejectsNewRun,
    }
}

async fn source<S>(
    state: &CourseRouteState<S>,
    auth: &crate::auth::AuthenticatedSession,
    course: CourseId,
    student: question_model::StudentId,
    value: PolicySource,
) -> HttpResult<TeachingPreviewFieldSource>
where
    S: CourseGroupManagementStore + TeachingAuthorityReferenceStore + 'static,
{
    match value {
        PolicySource::Base => Ok(TeachingPreviewFieldSource::Base {
            label: label("Assignment policy")?,
        }),
        PolicySource::GroupScheduleOffsets(groups) => {
            Ok(TeachingPreviewFieldSource::GroupScheduleOffsets {
                groups: groups_view(state, auth, course, groups).await?,
            })
        }
        PolicySource::GroupAccommodations(groups) => {
            Ok(TeachingPreviewFieldSource::GroupAccommodations {
                groups: groups_view(state, auth, course, groups).await?,
            })
        }
        PolicySource::IndividualException(_) => {
            let view = state
                .store
                .active_student_membership_reference_view(
                    auth.tenant_context,
                    auth.record.subject.user(),
                    course,
                    student,
                )
                .await
                .map_err(store_error_response)?;
            let Some(view) = view else {
                return Err(HttpRefusal::from(error_response(
                    axum::http::StatusCode::NOT_FOUND,
                    "student not found",
                )));
            };
            Ok(TeachingPreviewFieldSource::Membership {
                membership: view.reference,
                label: label(&view.display_name)?,
            })
        }
        PolicySource::HypotheticalIndividualException => {
            Err(HttpRefusal::from(hypothetical_source_response()))
        }
    }
}

async fn groups_view<S>(
    state: &CourseRouteState<S>,
    auth: &crate::auth::AuthenticatedSession,
    course: CourseId,
    groups: Vec<question_model::CourseGroupId>,
) -> HttpResult<TeachingPreviewGroupSources>
where
    S: CourseGroupManagementStore + 'static,
{
    let mut views = Vec::with_capacity(groups.len());
    for group in groups {
        let view = state
            .store
            .get_course_group_by_id_for_instructor(
                auth.tenant_context,
                auth.record.subject.user(),
                course,
                group,
            )
            .await
            .map_err(store_error_response)?;
        let Some(view) = view else {
            return Err(HttpRefusal::from(error_response(
                axum::http::StatusCode::NOT_FOUND,
                "group not found",
            )));
        };
        views.push(TeachingPreviewGroupSource {
            group: view.reference,
            label: label(&view.group.record.title)?,
        });
    }
    TeachingPreviewGroupSources::try_from(views).map_err(|_| {
        HttpRefusal::from(error_response(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "preview provenance is invalid",
        ))
    })
}
