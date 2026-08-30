use async_trait::async_trait;

use super::{MemoryStore, State};
use crate::{
    ActorContext, AssignmentId, AssignmentRouteIdentity, CourseId, RunId, RunRouteIdentity,
    StoreError, UserId, WorkspaceId,
};
use question_model::CourseMembershipRole;

fn next_reference(counter: &mut u32) -> Result<u64, StoreError> {
    *counter = counter
        .checked_add(1)
        .filter(|value| *value <= question_model::MAX_PUBLIC_ROUTE_NUMBER)
        .ok_or_else(|| StoreError::Unavailable("reference number limit reached".into()))?;
    Ok(u64::from(*counter))
}

pub(super) fn ensure_account_reference(
    state: &mut State,
    user: UserId,
) -> Result<question_model::AccountReference, StoreError> {
    if let Some(reference) = state.account_references.get(&user).copied() {
        return Ok(reference);
    }
    let reference =
        question_model::AccountReference::new(next_reference(&mut state.next_account_reference)?)
            .ok_or_else(|| StoreError::Unavailable("invalid account reference".into()))?;
    state.account_references.insert(user, reference);
    state.accounts_by_reference.insert(reference, user);
    Ok(reference)
}

pub(super) fn ensure_course_membership_reference(
    state: &mut State,
    membership: question_model::CourseMembershipId,
) -> Result<question_model::CourseMembershipReference, StoreError> {
    if let Some(reference) = state.course_membership_references.get(&membership).copied() {
        return Ok(reference);
    }
    let reference = question_model::CourseMembershipReference::new(next_reference(
        &mut state.next_course_membership_reference,
    )?)
    .ok_or_else(|| StoreError::Unavailable("invalid course membership reference".into()))?;
    state
        .course_membership_references
        .insert(membership, reference);
    state
        .course_memberships_by_reference
        .insert(reference, membership);
    Ok(reference)
}

pub(super) fn ensure_co_instructor_invitation_reference(
    state: &mut State,
    invitation: question_model::CoInstructorInvitationId,
) -> Result<question_model::CoInstructorInvitationReference, StoreError> {
    if let Some(reference) = state
        .co_instructor_invitation_references
        .get(&invitation)
        .copied()
    {
        return Ok(reference);
    }
    let reference = question_model::CoInstructorInvitationReference::new(next_reference(
        &mut state.next_co_instructor_invitation_reference,
    )?)
    .ok_or_else(|| StoreError::Unavailable("invalid co-instructor invitation reference".into()))?;
    state
        .co_instructor_invitation_references
        .insert(invitation, reference);
    state
        .co_instructor_invitations_by_reference
        .insert(reference, invitation);
    Ok(reference)
}

pub(super) fn ensure_course_reference(
    state: &mut State,
    course: CourseId,
) -> Result<question_model::CourseReference, StoreError> {
    if let Some(reference) = state.course_references.get(&course).copied() {
        return Ok(reference);
    }
    let reference =
        question_model::CourseReference::new(next_reference(&mut state.next_course_reference)?)
            .ok_or_else(|| StoreError::Unavailable("invalid course reference".into()))?;
    state.course_references.insert(course, reference);
    state.courses_by_reference.insert(reference, course);
    Ok(reference)
}
pub(super) fn ensure_assignment_reference(
    state: &mut State,
    assignment: AssignmentId,
) -> Result<question_model::AssignmentReference, StoreError> {
    if let Some(reference) = state.assignment_references.get(&assignment).copied() {
        return Ok(reference);
    }
    let reference = question_model::AssignmentReference::new(next_reference(
        &mut state.next_assignment_reference,
    )?)
    .ok_or_else(|| StoreError::Unavailable("invalid assignment reference".into()))?;
    state.assignment_references.insert(assignment, reference);
    state.assignments_by_reference.insert(reference, assignment);
    Ok(reference)
}

pub(super) fn ensure_course_group_reference(
    state: &mut State,
    group: crate::CourseGroupId,
) -> Result<question_model::CourseGroupReference, StoreError> {
    if let Some(reference) = state.course_group_references.get(&group).copied() {
        return Ok(reference);
    }
    let reference = question_model::CourseGroupReference::new(next_reference(
        &mut state.next_course_group_reference,
    )?)
    .ok_or_else(|| StoreError::Unavailable("invalid course group reference".into()))?;
    state.course_group_references.insert(group, reference);
    state.course_groups_by_reference.insert(reference, group);
    Ok(reference)
}
pub(super) fn ensure_run_reference(
    state: &mut State,
    run: RunId,
) -> Result<question_model::RunReference, StoreError> {
    if let Some(reference) = state.run_references.get(&run).copied() {
        return Ok(reference);
    }
    let reference =
        question_model::RunReference::new(next_reference(&mut state.next_run_reference)?)
            .ok_or_else(|| StoreError::Unavailable("invalid run reference".into()))?;
    state.run_references.insert(run, reference);
    state.runs_by_reference.insert(reference, run);
    Ok(reference)
}
pub(super) fn ensure_workspace_reference(
    state: &mut State,
    workspace: WorkspaceId,
) -> Result<question_model::WorkspaceReference, StoreError> {
    if let Some(reference) = state.workspace_references.get(&workspace).copied() {
        return Ok(reference);
    }
    let reference = question_model::WorkspaceReference::new(next_reference(
        &mut state.next_workspace_reference,
    )?)
    .ok_or_else(|| StoreError::Unavailable("invalid workspace reference".into()))?;
    state.workspace_references.insert(workspace, reference);
    state.workspaces_by_reference.insert(reference, workspace);
    Ok(reference)
}

fn run_identity(state: &State, actor: UserId, run: RunId) -> Option<RunRouteIdentity> {
    let run_record = state.runs.get(&run)?;
    let enrollment = state.enrollments.get(&run_record.enrollment)?;
    let assignment = state.assignments.get(&enrollment.assignment)?;
    state.courses.get(&assignment.course_id)?;
    let role = super::entitlement::current_course_role(state, assignment.course_id, actor)?;
    let allowed = super::entitlement::require_current_enrollment_entitlement(
        state,
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
        context: ActorContext,
        _actor: UserId,
        course: CourseId,
    ) -> Result<Option<question_model::CourseReference>, StoreError> {
        let state = self.read_state()?;
        let actor = context.user_id();
        Ok(state
            .courses
            .get(&course)
            .and_then(|_| super::entitlement::current_course_role(&state, course, actor))
            .and_then(|_| state.course_references.get(&course).copied()))
    }
    async fn resolve_course_reference(
        &self,
        context: ActorContext,
        _actor: UserId,
        reference: question_model::CourseReference,
    ) -> Result<Option<CourseId>, StoreError> {
        let state = self.read_state()?;
        let actor = context.user_id();
        let course = state.courses_by_reference.get(&reference).copied();
        Ok(course.filter(|course| {
            state.courses.contains_key(course)
                && super::entitlement::current_course_role(&state, *course, actor).is_some()
        }))
    }
    async fn assignment_reference(
        &self,
        context: ActorContext,
        _actor: UserId,
        assignment: AssignmentId,
    ) -> Result<Option<question_model::AssignmentReference>, StoreError> {
        let state = self.read_state()?;
        let actor = context.user_id();
        Ok(state
            .assignments
            .get(&assignment)
            .filter(|record| {
                state.courses.contains_key(&record.course_id)
                    && super::entitlement::current_course_role(&state, record.course_id, actor)
                        .is_some()
            })
            .and_then(|_| state.assignment_references.get(&assignment).copied()))
    }
    async fn resolve_assignment_reference(
        &self,
        context: ActorContext,
        _actor: UserId,
        reference: question_model::AssignmentReference,
    ) -> Result<Option<AssignmentRouteIdentity>, StoreError> {
        let state = self.read_state()?;
        let actor = context.user_id();
        let assignment = state.assignments_by_reference.get(&reference).copied();
        Ok(assignment.and_then(|assignment| {
            let record = state.assignments.get(&assignment)?;
            state.courses.get(&record.course_id)?;
            super::entitlement::current_course_role(&state, record.course_id, actor)?;
            Some(AssignmentRouteIdentity {
                course: record.course_id,
                assignment,
            })
        }))
    }
    async fn run_reference(
        &self,
        context: ActorContext,
        _actor: UserId,
        run: RunId,
    ) -> Result<Option<question_model::RunReference>, StoreError> {
        let state = self.read_state()?;
        let actor = context.user_id();
        Ok(run_identity(&state, actor, run).and_then(|_| state.run_references.get(&run).copied()))
    }
    async fn resolve_run_reference(
        &self,
        context: ActorContext,
        _actor: UserId,
        reference: question_model::RunReference,
    ) -> Result<Option<RunRouteIdentity>, StoreError> {
        let state = self.read_state()?;
        let actor = context.user_id();
        Ok(state
            .runs_by_reference
            .get(&reference)
            .copied()
            .and_then(|run| run_identity(&state, actor, run)))
    }
    async fn workspace_reference(
        &self,
        context: ActorContext,
        _actor: UserId,
        workspace: WorkspaceId,
    ) -> Result<Option<question_model::WorkspaceReference>, StoreError> {
        let state = self.read_state()?;
        let actor = context.user_id();
        Ok(state
            .draft_access
            .contains_key(&(workspace, actor))
            .then(|| state.workspace_references.get(&workspace).copied())
            .flatten())
    }
    async fn resolve_workspace_reference(
        &self,
        context: ActorContext,
        _actor: UserId,
        reference: question_model::WorkspaceReference,
    ) -> Result<Option<WorkspaceId>, StoreError> {
        let state = self.read_state()?;
        let actor = context.user_id();
        Ok(state
            .workspaces_by_reference
            .get(&reference)
            .copied()
            .filter(|workspace| state.draft_access.contains_key(&(*workspace, actor))))
    }
}
