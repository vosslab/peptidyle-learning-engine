//! In-memory Gradebook student and submitted-run selection.

use super::{
    MemoryStore, State, active_student_memberships, course_grade_scheme,
    require_course_records_accessible,
};
use crate::gradebook_cursor::{GradebookSelectionCursor, SubmittedRunChoicesCursor};
use crate::in_memory::course_roster::require_course_instructor;
use crate::{
    AssignmentId, AssignmentInspectionChoice, CourseGradebookStore, CourseId, GradebookFilter,
    GradebookOperationSelection, GradebookSelectionRequest, GradebookSelectionResult,
    SessionTokenHash, StoreError, SubmittedRunChoicesPage, SubmittedRunChoicesRequest,
    TenantContext,
};
use crate::{GradebookFilterRequest, StudentSelectionRow, SubmittedRunChoice};
use question_model::{ActivityTimestamp, CourseMembershipRole, GradePolicy, RunCompletionStatus};

pub(super) async fn gradebook_selection(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    course: CourseId,
    request: GradebookSelectionRequest,
) -> Result<GradebookSelectionResult, StoreError> {
    let operation_selection = match request.filter {
        GradebookFilterRequest::Operation(operation) => Some((
            operation,
            store
                .resolve_gradebook_operation(context, session, course, operation)
                .await?,
        )),
        GradebookFilterRequest::All
        | GradebookFilterRequest::Assignment(_)
        | GradebookFilterRequest::Student(_) => None,
    };
    let state = store.read_state()?;
    let tenant = context.tenant_id();
    require_course_records_accessible(&state, tenant, course)?;
    require_course_instructor(&state, context, session, course)?;
    let (assignment_reference, operation) = match (request.filter, operation_selection) {
        (GradebookFilterRequest::Assignment(assignment), None) => (assignment, None),
        (GradebookFilterRequest::Operation(_), Some((operation, selection))) => match selection {
            GradebookOperationSelection::Assignment { assignment } => (assignment, Some(operation)),
            GradebookOperationSelection::SingleStudent {
                membership,
                assignment,
            } => {
                let choice =
                    selection_inspection_choice(&state, tenant, course, membership, assignment)?;
                return Ok(GradebookSelectionResult::SingleStudent {
                    membership,
                    assignment,
                    inspection_choice: choice,
                });
            }
        },
        _ => return Err(StoreError::NotFound),
    };
    let assignment = assignment_id_for_course(&state, tenant, course, assignment_reference)?;
    let scheme = course_grade_scheme(&state, tenant, course);
    let roster_revision =
        crate::in_memory::course_roster::roster_policy(&state, tenant, course).revision;
    let after = request
        .page
        .after
        .as_ref()
        .map(GradebookSelectionCursor::decode)
        .transpose()?;
    if let Some(cursor) = after
        && (cursor.scheme_revision != scheme.revision
            || cursor.roster_revision != roster_revision
            || cursor.assignment != assignment_reference
            || cursor.operation != operation)
    {
        return Err(StoreError::NotFound);
    }
    let after_membership = after.map(|cursor| cursor.last_membership);
    let mut memberships = active_student_memberships(&state, tenant, course, GradebookFilter::All)?;
    memberships.sort_by_key(|membership| {
        state
            .course_membership_references
            .get(&(tenant, membership.id))
            .map(|value| value.number())
            .unwrap_or_default()
    });
    let mut memberships = memberships
        .into_iter()
        .filter(|membership| {
            after_membership.is_none_or(|after| {
                state
                    .course_membership_references
                    .get(&(tenant, membership.id))
                    .is_some_and(|reference| *reference > after)
            })
        })
        .take(usize::from(request.page.size.get()) + 1)
        .collect::<Vec<_>>();
    let has_more = memberships.len() > usize::from(request.page.size.get());
    if has_more {
        memberships.pop();
    }
    let rows = memberships
        .iter()
        .map(|membership| {
            let reference = *state
                .course_membership_references
                .get(&(tenant, membership.id))
                .ok_or(StoreError::NotFound)?;
            let profile = state
                .roster_profiles
                .get(&(tenant, course, membership.id))
                .ok_or(StoreError::NotFound)?;
            let choice = selection_inspection_choice(
                &state,
                tenant,
                course,
                reference,
                assignment_reference,
            )?;
            Ok(StudentSelectionRow {
                membership: reference,
                display_label: profile.display_name.clone(),
                assignment: assignment_reference,
                inspection_choice: choice,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let next_cursor = if has_more {
        let last_membership = rows
            .last()
            .expect("following selection page has final row")
            .membership;
        Some(
            GradebookSelectionCursor {
                scheme_revision: scheme.revision,
                roster_revision,
                assignment: assignment_reference,
                operation,
                last_membership,
            }
            .encode(),
        )
    } else {
        None
    };
    let _ = assignment;
    Ok(GradebookSelectionResult::StudentSelection { rows, next_cursor })
}

pub(super) async fn submitted_run_choices(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    course: CourseId,
    request: SubmittedRunChoicesRequest,
) -> Result<SubmittedRunChoicesPage, StoreError> {
    if let Some(operation) = request.operation {
        match store
            .resolve_gradebook_operation(context, session, course, operation)
            .await?
        {
            GradebookOperationSelection::Assignment { assignment }
                if assignment == request.assignment => {}
            GradebookOperationSelection::SingleStudent {
                membership,
                assignment,
            } if membership == request.membership && assignment == request.assignment => {}
            _ => return Err(StoreError::NotFound),
        }
    }
    let state = store.read_state()?;
    let tenant = context.tenant_id();
    require_course_records_accessible(&state, tenant, course)?;
    require_course_instructor(&state, context, session, course)?;
    let roster_revision =
        crate::in_memory::course_roster::roster_policy(&state, tenant, course).revision;
    let after = request
        .page
        .after
        .as_ref()
        .map(SubmittedRunChoicesCursor::decode)
        .transpose()?;
    if let Some(cursor) = after
        && (cursor.roster_revision != roster_revision
            || cursor.membership != request.membership
            || cursor.assignment != request.assignment
            || cursor.operation != request.operation)
    {
        return Err(StoreError::NotFound);
    }
    let membership = active_student_membership(&state, tenant, course, request.membership)?;
    let assignment = assignment_id_for_course(&state, tenant, course, request.assignment)?;
    let student = membership.student.ok_or(StoreError::NotFound)?;
    let enrollment = state
        .enrollments
        .values()
        .find(|value| {
            value.tenant == tenant && value.assignment == assignment && value.student == student
        })
        .ok_or(StoreError::NotFound)?;
    let mut runs = state
        .runs
        .values()
        .filter(|run| run.tenant == tenant && run.enrollment == enrollment.id)
        .filter(|run| run.completion_status() == RunCompletionStatus::Completed)
        .collect::<Vec<_>>();
    runs.sort_by_key(|run| {
        std::cmp::Reverse((run.completed_at.expect("completed run"), run.reference))
    });
    let after_key = after.map(|cursor| {
        (
            ActivityTimestamp::from_unix_millis(cursor.submitted_at_millis),
            cursor.last_run,
        )
    });
    let mut runs = runs
        .into_iter()
        .filter(|run| {
            after_key.is_none_or(|after| {
                (run.completed_at.expect("completed run"), run.reference) < after
            })
        })
        .take(usize::from(request.page.size.get()) + 1)
        .collect::<Vec<_>>();
    let has_more = runs.len() > usize::from(request.page.size.get());
    if has_more {
        runs.pop();
    }
    let selected = enrollment.current_grade_run;
    let rows = runs
        .iter()
        .map(|run| SubmittedRunChoice {
            run: run.reference,
            submitted_at: run.completed_at.expect("completed run"),
            score_selected: selected == Some(run.id),
        })
        .collect::<Vec<_>>();
    let next_cursor = rows.last().and_then(|last| {
        has_more.then(|| {
            SubmittedRunChoicesCursor {
                roster_revision,
                membership: request.membership,
                assignment: request.assignment,
                operation: request.operation,
                submitted_at_millis: last.submitted_at.as_unix_millis(),
                last_run: last.run,
            }
            .encode()
        })
    });
    Ok(SubmittedRunChoicesPage {
        roster_revision,
        next_cursor,
        rows,
    })
}

fn active_student_membership(
    state: &State,
    tenant: question_model::TenantId,
    course: CourseId,
    reference: question_model::CourseMembershipReference,
) -> Result<&crate::CourseMembershipRecord, StoreError> {
    let id = state
        .course_memberships_by_reference
        .get(&(tenant, reference))
        .ok_or(StoreError::NotFound)?;
    state
        .course_memberships
        .get(&(tenant, *id))
        .filter(|membership| {
            membership.course == course
                && membership.status == crate::CourseMemberStatus::Active
                && membership.role == CourseMembershipRole::Student
        })
        .ok_or(StoreError::NotFound)
}

fn assignment_id_for_course(
    state: &State,
    tenant: question_model::TenantId,
    course: CourseId,
    reference: question_model::AssignmentReference,
) -> Result<AssignmentId, StoreError> {
    let assignment = *state
        .assignments_by_reference
        .get(&(tenant, reference))
        .ok_or(StoreError::NotFound)?;
    state
        .assignments
        .get(&(tenant, assignment))
        .filter(|record| record.course_id == course)
        .map(|_| assignment)
        .ok_or(StoreError::NotFound)
}

fn selection_inspection_choice(
    state: &State,
    tenant: question_model::TenantId,
    course: CourseId,
    membership: question_model::CourseMembershipReference,
    assignment: question_model::AssignmentReference,
) -> Result<AssignmentInspectionChoice, StoreError> {
    let membership = active_student_membership(state, tenant, course, membership)?;
    let assignment_id = assignment_id_for_course(state, tenant, course, assignment)?;
    let assignment_record = state
        .assignments
        .get(&(tenant, assignment_id))
        .ok_or(StoreError::NotFound)?;
    let student = membership.student.ok_or(StoreError::NotFound)?;
    let enrollment = state.enrollments.values().find(|value| {
        value.tenant == tenant && value.assignment == assignment_id && value.student == student
    });
    inspection_choice(state, tenant, enrollment, assignment_record.policies.grade)
}

pub(super) fn inspection_choice(
    state: &State,
    tenant: question_model::TenantId,
    enrollment: Option<&question_model::AssignmentEnrollment>,
    policy: GradePolicy,
) -> Result<AssignmentInspectionChoice, StoreError> {
    let Some(enrollment) = enrollment else {
        return Ok(AssignmentInspectionChoice::NoSubmittedRun);
    };
    if let Some(selected) = enrollment.current_grade_run {
        let run = state
            .runs
            .get(&(tenant, selected))
            .ok_or(StoreError::NotFound)?;
        let submitted_at = run.completed_at.ok_or_else(|| {
            StoreError::Unavailable("selected grade run is not completed".to_string())
        })?;
        return Ok(AssignmentInspectionChoice::SelectedRun {
            basis: policy.into(),
            run: run.reference,
            submitted_at,
        });
    }
    let completed_run_count = state
        .runs
        .values()
        .filter(|run| run.tenant == tenant && run.enrollment == enrollment.id)
        .filter(|run| run.completion_status() == RunCompletionStatus::Completed)
        .count();
    let completed_run_count = u32::try_from(completed_run_count).map_err(|_| {
        StoreError::Unavailable("completed run count exceeds supported range".to_string())
    })?;
    Ok(if completed_run_count == 0 {
        AssignmentInspectionChoice::NoSubmittedRun
    } else {
        AssignmentInspectionChoice::ChooseRun {
            completed_run_count,
        }
    })
}
