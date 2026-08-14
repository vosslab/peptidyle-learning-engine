use async_trait::async_trait;

use super::{MemoryStore, State};
use crate::{AssignmentId, CourseId, RunId, StoreError, TenantContext, UserId, WorkspaceId};
use question_model::TenantId;

fn next_public_number(counter: &mut u32) -> Result<u64, StoreError> {
    *counter = counter
        .checked_add(1)
        .filter(|value| *value <= question_model::MAX_PUBLIC_ROUTE_NUMBER)
        .ok_or_else(|| StoreError::Unavailable("public route number limit reached".to_string()))?;
    Ok(u64::from(*counter))
}

pub(super) fn ensure_course_public_id(
    state: &mut State,
    tenant: TenantId,
    course: CourseId,
) -> Result<question_model::CoursePublicId, StoreError> {
    if let Some(public_id) = state.course_public_ids.get(&(tenant, course)).copied() {
        return Ok(public_id);
    }
    let public_id =
        question_model::CoursePublicId::new(next_public_number(&mut state.next_course_public_id)?)
            .ok_or_else(|| StoreError::Unavailable("invalid course route number".to_string()))?;
    state.course_public_ids.insert((tenant, course), public_id);
    state
        .courses_by_public_id
        .insert((tenant, public_id), course);
    Ok(public_id)
}

pub(super) fn ensure_assignment_public_id(
    state: &mut State,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<question_model::AssignmentPublicId, StoreError> {
    if let Some(public_id) = state
        .assignment_public_ids
        .get(&(tenant, assignment))
        .copied()
    {
        return Ok(public_id);
    }
    let public_id = question_model::AssignmentPublicId::new(next_public_number(
        &mut state.next_assignment_public_id,
    )?)
    .ok_or_else(|| StoreError::Unavailable("invalid assignment route number".to_string()))?;
    state
        .assignment_public_ids
        .insert((tenant, assignment), public_id);
    state
        .assignments_by_public_id
        .insert((tenant, public_id), assignment);
    Ok(public_id)
}

pub(super) fn ensure_run_public_id(
    state: &mut State,
    tenant: TenantId,
    run: RunId,
) -> Result<question_model::RunPublicId, StoreError> {
    if let Some(public_id) = state.run_public_ids.get(&(tenant, run)).copied() {
        return Ok(public_id);
    }
    let public_id =
        question_model::RunPublicId::new(next_public_number(&mut state.next_run_public_id)?)
            .ok_or_else(|| StoreError::Unavailable("invalid run route number".to_string()))?;
    state.run_public_ids.insert((tenant, run), public_id);
    state.runs_by_public_id.insert((tenant, public_id), run);
    Ok(public_id)
}

pub(super) fn ensure_workspace_public_id(
    state: &mut State,
    tenant: TenantId,
    workspace: WorkspaceId,
) -> Result<question_model::WorkspacePublicId, StoreError> {
    if let Some(public_id) = state
        .workspace_public_ids
        .get(&(tenant, workspace))
        .copied()
    {
        return Ok(public_id);
    }
    let public_id = question_model::WorkspacePublicId::new(next_public_number(
        &mut state.next_workspace_public_id,
    )?)
    .ok_or_else(|| StoreError::Unavailable("invalid workspace route number".to_string()))?;
    state
        .workspace_public_ids
        .insert((tenant, workspace), public_id);
    state
        .workspaces_by_public_id
        .insert((tenant, public_id), workspace);
    Ok(public_id)
}

#[async_trait]
impl crate::NavigationReferenceStore for MemoryStore {
    async fn course_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
    ) -> Result<Option<question_model::CoursePublicId>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state.courses.get(&(context.tenant_id(), course)) else {
            return Ok(None);
        };
        if record.role_for(actor).is_none() {
            return Ok(None);
        }
        Ok(state
            .course_public_ids
            .get(&(context.tenant_id(), course))
            .copied())
    }

    async fn resolve_course_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        public_id: question_model::CoursePublicId,
    ) -> Result<Option<CourseId>, StoreError> {
        let state = self.read_state()?;
        let Some(course) = state
            .courses_by_public_id
            .get(&(context.tenant_id(), public_id))
            .copied()
        else {
            return Ok(None);
        };
        Ok(state
            .courses
            .get(&(context.tenant_id(), course))
            .and_then(|record| record.role_for(actor).map(|_| course)))
    }

    async fn assignment_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
    ) -> Result<Option<question_model::AssignmentPublicId>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state.assignments.get(&(context.tenant_id(), assignment)) else {
            return Ok(None);
        };
        let allowed = state
            .courses
            .get(&(context.tenant_id(), record.course_id))
            .is_some_and(|course| course.role_for(actor).is_some());
        Ok(allowed
            .then(|| {
                state
                    .assignment_public_ids
                    .get(&(context.tenant_id(), assignment))
                    .copied()
            })
            .flatten())
    }

    async fn resolve_assignment_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        public_id: question_model::AssignmentPublicId,
    ) -> Result<Option<crate::AssignmentRouteIdentity>, StoreError> {
        let state = self.read_state()?;
        let Some(assignment) = state
            .assignments_by_public_id
            .get(&(context.tenant_id(), public_id))
            .copied()
        else {
            return Ok(None);
        };
        let Some(record) = state.assignments.get(&(context.tenant_id(), assignment)) else {
            return Ok(None);
        };
        let allowed = state
            .courses
            .get(&(context.tenant_id(), record.course_id))
            .is_some_and(|course| course.role_for(actor).is_some());
        Ok(allowed.then_some(crate::AssignmentRouteIdentity {
            course: record.course_id,
            assignment,
        }))
    }

    async fn run_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        run: RunId,
    ) -> Result<Option<question_model::RunPublicId>, StoreError> {
        let state = self.read_state()?;
        let allowed = state
            .runs
            .get(&(context.tenant_id(), run))
            .and_then(|record| {
                state
                    .enrollments
                    .get(&(context.tenant_id(), record.enrollment))
            })
            .is_some_and(|enrollment| enrollment.user == actor);
        Ok(allowed
            .then(|| {
                state
                    .run_public_ids
                    .get(&(context.tenant_id(), run))
                    .copied()
            })
            .flatten())
    }

    async fn resolve_run_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        public_id: question_model::RunPublicId,
    ) -> Result<Option<RunId>, StoreError> {
        let state = self.read_state()?;
        let Some(run) = state
            .runs_by_public_id
            .get(&(context.tenant_id(), public_id))
            .copied()
        else {
            return Ok(None);
        };
        let allowed = state
            .runs
            .get(&(context.tenant_id(), run))
            .and_then(|record| {
                state
                    .enrollments
                    .get(&(context.tenant_id(), record.enrollment))
            })
            .is_some_and(|enrollment| enrollment.user == actor);
        Ok(allowed.then_some(run))
    }

    async fn workspace_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<question_model::WorkspacePublicId>, StoreError> {
        let state = self.read_state()?;
        let allowed = state
            .draft_access
            .contains_key(&(context.tenant_id(), workspace, actor));
        Ok(allowed
            .then(|| {
                state
                    .workspace_public_ids
                    .get(&(context.tenant_id(), workspace))
                    .copied()
            })
            .flatten())
    }

    async fn resolve_workspace_public_id(
        &self,
        context: TenantContext,
        actor: UserId,
        public_id: question_model::WorkspacePublicId,
    ) -> Result<Option<WorkspaceId>, StoreError> {
        let state = self.read_state()?;
        let Some(workspace) = state
            .workspaces_by_public_id
            .get(&(context.tenant_id(), public_id))
            .copied()
        else {
            return Ok(None);
        };
        Ok(state
            .draft_access
            .contains_key(&(context.tenant_id(), workspace, actor))
            .then_some(workspace))
    }
}
