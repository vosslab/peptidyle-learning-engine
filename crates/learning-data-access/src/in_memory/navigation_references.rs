use async_trait::async_trait;

use super::{MemoryStore, State};
use crate::{
    AssignmentId, AssignmentRouteIdentity, CourseId, RunId, RunRouteIdentity, StoreError,
    TenantContext, UserId, WorkspaceId,
};
use question_model::{CourseMembershipRole, TenantId};

fn next_reference(counter: &mut u32) -> Result<u64, StoreError> {
    *counter = counter
        .checked_add(1)
        .filter(|value| *value <= question_model::MAX_PUBLIC_ROUTE_NUMBER)
        .ok_or_else(|| StoreError::Unavailable("reference number limit reached".into()))?;
    Ok(u64::from(*counter))
}

pub(super) fn ensure_course_reference(
    state: &mut State,
    tenant: TenantId,
    course: CourseId,
) -> Result<question_model::CourseReference, StoreError> {
    if let Some(reference) = state.course_references.get(&(tenant, course)).copied() {
        return Ok(reference);
    }
    let reference =
        question_model::CourseReference::new(next_reference(&mut state.next_course_reference)?)
            .ok_or_else(|| StoreError::Unavailable("invalid course reference".into()))?;
    state.course_references.insert((tenant, course), reference);
    state
        .courses_by_reference
        .insert((tenant, reference), course);
    Ok(reference)
}
pub(super) fn ensure_assignment_reference(
    state: &mut State,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<question_model::AssignmentReference, StoreError> {
    if let Some(reference) = state
        .assignment_references
        .get(&(tenant, assignment))
        .copied()
    {
        return Ok(reference);
    }
    let reference = question_model::AssignmentReference::new(next_reference(
        &mut state.next_assignment_reference,
    )?)
    .ok_or_else(|| StoreError::Unavailable("invalid assignment reference".into()))?;
    state
        .assignment_references
        .insert((tenant, assignment), reference);
    state
        .assignments_by_reference
        .insert((tenant, reference), assignment);
    Ok(reference)
}
pub(super) fn ensure_run_reference(
    state: &mut State,
    tenant: TenantId,
    run: RunId,
) -> Result<question_model::RunReference, StoreError> {
    if let Some(reference) = state.run_references.get(&(tenant, run)).copied() {
        return Ok(reference);
    }
    let reference =
        question_model::RunReference::new(next_reference(&mut state.next_run_reference)?)
            .ok_or_else(|| StoreError::Unavailable("invalid run reference".into()))?;
    state.run_references.insert((tenant, run), reference);
    state.runs_by_reference.insert((tenant, reference), run);
    Ok(reference)
}
pub(super) fn ensure_workspace_reference(
    state: &mut State,
    tenant: TenantId,
    workspace: WorkspaceId,
) -> Result<question_model::WorkspaceReference, StoreError> {
    if let Some(reference) = state
        .workspace_references
        .get(&(tenant, workspace))
        .copied()
    {
        return Ok(reference);
    }
    let reference = question_model::WorkspaceReference::new(next_reference(
        &mut state.next_workspace_reference,
    )?)
    .ok_or_else(|| StoreError::Unavailable("invalid workspace reference".into()))?;
    state
        .workspace_references
        .insert((tenant, workspace), reference);
    state
        .workspaces_by_reference
        .insert((tenant, reference), workspace);
    Ok(reference)
}

fn run_identity(
    state: &State,
    tenant: TenantId,
    actor: UserId,
    run: RunId,
) -> Option<RunRouteIdentity> {
    let run_record = state.runs.get(&(tenant, run))?;
    let enrollment = state.enrollments.get(&(tenant, run_record.enrollment))?;
    let assignment = state.assignments.get(&(tenant, enrollment.assignment))?;
    state.courses.get(&(tenant, assignment.course_id))?;
    let role = super::entitlement::current_course_role(state, tenant, assignment.course_id, actor)?;
    let allowed = super::entitlement::require_current_enrollment_entitlement(
        state,
        tenant,
        actor,
        assignment.course_id,
        assignment.id,
        enrollment,
    )
    .is_ok()
        || role == CourseMembershipRole::Instructor;
    allowed.then_some(RunRouteIdentity {
        course: assignment.course_id,
        assignment: enrollment.assignment,
        enrollment: run_record.enrollment,
        run,
    })
}

#[async_trait]
impl crate::NavigationReferenceStore for MemoryStore {
    async fn course_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
    ) -> Result<Option<question_model::CourseReference>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .courses
            .get(&(context.tenant_id(), course))
            .and_then(|_| {
                super::entitlement::current_course_role(&state, context.tenant_id(), course, actor)
            })
            .and_then(|_| {
                state
                    .course_references
                    .get(&(context.tenant_id(), course))
                    .copied()
            }))
    }
    async fn resolve_course_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: question_model::CourseReference,
    ) -> Result<Option<CourseId>, StoreError> {
        let state = self.read_state()?;
        let course = state
            .courses_by_reference
            .get(&(context.tenant_id(), reference))
            .copied();
        Ok(course.filter(|course| {
            state.courses.contains_key(&(context.tenant_id(), *course))
                && super::entitlement::current_course_role(
                    &state,
                    context.tenant_id(),
                    *course,
                    actor,
                )
                .is_some()
        }))
    }
    async fn assignment_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
    ) -> Result<Option<question_model::AssignmentReference>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .assignments
            .get(&(context.tenant_id(), assignment))
            .filter(|record| {
                state
                    .courses
                    .contains_key(&(context.tenant_id(), record.course_id))
                    && super::entitlement::current_course_role(
                        &state,
                        context.tenant_id(),
                        record.course_id,
                        actor,
                    )
                    .is_some()
            })
            .and_then(|_| {
                state
                    .assignment_references
                    .get(&(context.tenant_id(), assignment))
                    .copied()
            }))
    }
    async fn resolve_assignment_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: question_model::AssignmentReference,
    ) -> Result<Option<AssignmentRouteIdentity>, StoreError> {
        let state = self.read_state()?;
        let assignment = state
            .assignments_by_reference
            .get(&(context.tenant_id(), reference))
            .copied();
        Ok(assignment.and_then(|assignment| {
            let record = state.assignments.get(&(context.tenant_id(), assignment))?;
            state
                .courses
                .get(&(context.tenant_id(), record.course_id))?;
            super::entitlement::current_course_role(
                &state,
                context.tenant_id(),
                record.course_id,
                actor,
            )?;
            Some(AssignmentRouteIdentity {
                course: record.course_id,
                assignment,
            })
        }))
    }
    async fn run_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<question_model::RunReference>, StoreError> {
        let state = self.read_state()?;
        Ok(
            run_identity(&state, context.tenant_id(), actor, run).and_then(|_| {
                state
                    .run_references
                    .get(&(context.tenant_id(), run))
                    .copied()
            }),
        )
    }
    async fn resolve_run_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: question_model::RunReference,
    ) -> Result<Option<RunRouteIdentity>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .runs_by_reference
            .get(&(context.tenant_id(), reference))
            .copied()
            .and_then(|run| run_identity(&state, context.tenant_id(), actor, run)))
    }
    async fn workspace_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<question_model::WorkspaceReference>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .draft_access
            .contains_key(&(context.tenant_id(), workspace, actor))
            .then(|| {
                state
                    .workspace_references
                    .get(&(context.tenant_id(), workspace))
                    .copied()
            })
            .flatten())
    }
    async fn resolve_workspace_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: question_model::WorkspaceReference,
    ) -> Result<Option<WorkspaceId>, StoreError> {
        let state = self.read_state()?;
        Ok(state
            .workspaces_by_reference
            .get(&(context.tenant_id(), reference))
            .copied()
            .filter(|workspace| {
                state
                    .draft_access
                    .contains_key(&(context.tenant_id(), *workspace, actor))
            }))
    }
}
