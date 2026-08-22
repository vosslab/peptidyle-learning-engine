use async_trait::async_trait;
use domain::entitlement::{
    ActiveStudentMembership, EntitlementDecision, EntitlementFacts, evaluate_assignment_entitlement,
};
use question_model::{
    AssignmentEnrollment, CourseMembershipId, EntitlementMaterialization, EvaluatorVersion,
    MaterializationDisposition, StudentAssignmentSummary,
};

use super::*;

#[async_trait]
impl crate::EntitlementStore for MemoryStore {
    async fn list_learner_entitled_assignments_impl(
        &self,
        context: TenantContext,
        learner: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let mut records = Vec::new();
        for record in state
            .assignments
            .values()
            .filter(|record| record.tenant == tenant && record.course_id == course)
        {
            let EntitlementDecision::Granted(grant) =
                evaluate_locked(&state, tenant, learner, course, record.id)?
            else {
                continue;
            };
            let prior_run_count = state
                .runs
                .values()
                .filter(|run| {
                    run.tenant == tenant
                        && run.completed_at.is_some()
                        && state
                            .enrollments
                            .get(&(tenant, run.enrollment))
                            .is_some_and(|enrollment| {
                                enrollment.assignment == record.id
                                    && enrollment.student == grant.student()
                            })
                })
                .count();
            let prior_run_count = u32::try_from(prior_run_count).map_err(|_| {
                StoreError::Unavailable("run count exceeds policy range".to_string())
            })?;
            if matches!(
                super::course_policy::resolve_granted_memory_effective_policy(
                    &state,
                    tenant,
                    record,
                    grant,
                    prior_run_count,
                )?,
                domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
                    start: domain::effective_assignment_policy::StartVerdict::MayStart { .. },
                    ..
                }
            ) {
                records.push((record.id.to_string(), record.clone()));
            }
        }
        Ok(super::page_records(records, &page))
    }

    async fn evaluate_assignment_entitlement_impl(
        &self,
        context: TenantContext,
        learner: UserId,
        course: CourseId,
        assignment: AssignmentId,
    ) -> Result<EntitlementDecision, StoreError> {
        let state = self.read_state()?;
        evaluate_locked(&state, context.tenant_id(), learner, course, assignment)
    }

    async fn issue_assignment_entitlement_impl(
        &self,
        context: TenantContext,
        command: crate::MaterializeAssignmentEntitlementCommand,
    ) -> Result<crate::AssignmentEntitlementMaterialization, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        materialize_locked(&mut state, tenant, command)
    }
}

pub(super) fn materialize_locked(
    state: &mut State,
    tenant: TenantId,
    command: crate::MaterializeAssignmentEntitlementCommand,
) -> Result<crate::AssignmentEntitlementMaterialization, StoreError> {
    match (command.purpose(), command.authority()) {
        (
            question_model::EntitlementPurpose::StartRun,
            question_model::MaterializationAuthority::Actor(actor),
        ) if actor == command.learner() => {}
        (
            question_model::EntitlementPurpose::GradeBearingAction,
            question_model::MaterializationAuthority::Actor(actor),
        ) if actor == command.learner()
            || current_course_instructor(state, tenant, command.course(), actor) => {}
        (
            question_model::EntitlementPurpose::InstructorIssue,
            question_model::MaterializationAuthority::Actor(actor),
        ) if current_course_instructor(state, tenant, command.course(), actor) => {}
        (
            question_model::EntitlementPurpose::GradeBearingAction,
            question_model::MaterializationAuthority::Rule(_),
        ) => {}
        _ => return Err(StoreError::Forbidden),
    }
    let decision = evaluate_locked(
        state,
        tenant,
        command.learner(),
        command.course(),
        command.assignment(),
    )?;
    let EntitlementDecision::Granted(grant) = decision else {
        let EntitlementDecision::Denied(reason) = decision else {
            unreachable!("entitlement decision is closed");
        };
        return Ok(crate::AssignmentEntitlementMaterialization::Denied(reason));
    };
    if let Some(enrollment) = state
        .enrollments
        .values()
        .find(|enrollment| {
            enrollment.tenant == tenant
                && enrollment.assignment == command.assignment()
                // The educational receipt belongs to the durable student
                // identity.  A reinvited user can receive a new membership
                // episode without duplicating completed work or mutating its
                // immutable first-grant provenance.
                && enrollment.student == grant.student()
        })
        .cloned()
    {
        let summary = state
            .summaries
            .get(&(tenant, enrollment.id))
            .cloned()
            .ok_or(StoreError::Unavailable(
                "entitlement receipt is missing its summary".to_string(),
            ))?;
        let provenance = state
            .entitlement_materializations
            .get(&(tenant, enrollment.id))
            .cloned()
            .ok_or(StoreError::Unavailable(
                "entitlement receipt is missing immutable provenance".to_string(),
            ))?;
        return Ok(crate::AssignmentEntitlementMaterialization::Granted(
            crate::MaterializedAssignmentEntitlement {
                enrollment,
                summary,
                provenance,
                disposition: MaterializationDisposition::Existing,
                applicable_policy_scopes: grant.applicable_policy_scopes().clone(),
            },
        ));
    }
    let enrollment = AssignmentEnrollment {
        id: random_enrollment_id()?,
        tenant,
        assignment: command.assignment(),
        user: command.learner(),
        student: grant.student(),
        first_completed_at: None,
        current_grade_run: None,
        best_grade_run: None,
    };
    let summary = StudentAssignmentSummary::empty(tenant, enrollment.id);
    let provenance = EntitlementMaterialization {
        enrollment: enrollment.id,
        membership: grant.membership(),
        user: command.learner(),
        occurred_at: state.authoritative_time,
        purpose: command.purpose(),
        authority: command.authority(),
        basis: grant.basis(),
        evaluator_version: EvaluatorVersion::INITIAL,
    };
    state
        .summaries
        .insert((tenant, enrollment.id), summary.clone());
    state
        .entitlement_materializations
        .insert((tenant, enrollment.id), provenance.clone());
    state
        .enrollments
        .insert((tenant, enrollment.id), enrollment.clone());
    Ok(crate::AssignmentEntitlementMaterialization::Granted(
        crate::MaterializedAssignmentEntitlement {
            enrollment,
            summary,
            provenance,
            disposition: MaterializationDisposition::Created,
            applicable_policy_scopes: grant.applicable_policy_scopes().clone(),
        },
    ))
}

fn current_course_instructor(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> bool {
    active_membership_for(state, tenant, course, actor).is_some_and(|membership| {
        membership.role == question_model::CourseMembershipRole::Instructor
    })
}

pub(super) fn evaluate_locked(
    state: &State,
    tenant: TenantId,
    learner: UserId,
    course: CourseId,
    assignment: AssignmentId,
) -> Result<EntitlementDecision, StoreError> {
    let Some(record) = state.assignments.get(&(tenant, assignment)) else {
        return Ok(EntitlementDecision::Denied(
            domain::entitlement::EntitlementDenial::AssignmentNotFound,
        ));
    };
    if record.course_id != course {
        return Ok(EntitlementDecision::Denied(
            domain::entitlement::EntitlementDenial::AssignmentOutsideCourse,
        ));
    }
    if !state.courses.contains_key(&(tenant, course)) {
        return Ok(EntitlementDecision::Denied(
            domain::entitlement::EntitlementDenial::CourseNotFound,
        ));
    }
    let membership = state
        .active_course_membership_by_user
        .get(&(tenant, course, learner))
        .and_then(|id| state.course_memberships.get(&(tenant, *id)))
        .filter(|member| {
            member.status == crate::CourseMemberStatus::Active
                && member.role == question_model::CourseMembershipRole::Student
        })
        .and_then(|member| {
            member.student.map(|student| ActiveStudentMembership {
                id: member.id,
                student,
            })
        });
    let current_groups = state
        .course_groups
        .values()
        .filter(|group| group.tenant == tenant && group.course == course)
        .filter(|group| membership.is_some_and(|member| group.members.contains(&member.id)))
        .map(|group| (group.id, group.purpose))
        .collect();
    Ok(evaluate_assignment_entitlement(EntitlementFacts {
        tenant,
        course,
        assignment,
        learner,
        membership,
        audience: record.audience.clone(),
        current_groups,
    }))
}

/// Re-evaluates current authority for a read, replay, or transition that
/// already has a receipt.  The receipt proves history only and is never used
/// as the authority predicate.
pub(super) fn require_current_assignment_entitlement(
    state: &State,
    tenant: TenantId,
    learner: UserId,
    course: CourseId,
    assignment: AssignmentId,
) -> Result<domain::entitlement::EntitlementGrant, StoreError> {
    match evaluate_locked(state, tenant, learner, course, assignment)? {
        EntitlementDecision::Granted(grant) => Ok(grant),
        EntitlementDecision::Denied(_) => Err(StoreError::NotFound),
    }
}

/// Authorizes the current actor against the durable pedagogical identity on a
/// historical receipt.  `AssignmentEnrollment::user` records who first
/// materialized the receipt; it is not a continuing identity binding after a
/// student is reinvited through a new membership episode.
pub(super) fn require_current_enrollment_entitlement(
    state: &State,
    tenant: TenantId,
    actor: UserId,
    course: CourseId,
    assignment: AssignmentId,
    enrollment: &AssignmentEnrollment,
) -> Result<domain::entitlement::EntitlementGrant, StoreError> {
    let grant = require_current_assignment_entitlement(state, tenant, actor, course, assignment)?;
    (grant.student() == enrollment.student)
        .then_some(grant)
        .ok_or(StoreError::NotFound)
}

fn random_enrollment_id() -> Result<EnrollmentId, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("enrollment ID randomness unavailable: {error}"))
    })
    .map(EnrollmentId::from_uuid)
}

pub(super) fn ensure_course_membership_id(
    state: &mut State,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
    student: StudentId,
) -> Result<CourseMembershipId, StoreError> {
    if let Some(id) = state
        .active_course_membership_by_user
        .get(&(tenant, course, user))
        .copied()
    {
        let existing = state.course_memberships.get(&(tenant, id)).ok_or_else(|| {
            StoreError::Unavailable("active course-membership index is inconsistent".to_string())
        })?;
        return (existing.role == question_model::CourseMembershipRole::Student
            && existing.student == Some(student)
            && existing.status == crate::CourseMemberStatus::Active)
            .then_some(id)
            .ok_or(StoreError::Conflict);
    }
    let id = crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!(
            "course membership ID randomness unavailable: {error}"
        ))
    })
    .map(CourseMembershipId::from_uuid)?;
    state.course_memberships.insert(
        (tenant, id),
        CourseMembershipRecord {
            id,
            tenant,
            course,
            user,
            student: Some(student),
            role: question_model::CourseMembershipRole::Student,
            roster_id: None,
            status: crate::CourseMemberStatus::Active,
            joined_at: state.authoritative_time,
            revoked_at: None,
        },
    );
    state
        .active_course_membership_by_user
        .insert((tenant, course, user), id);
    if let Err(error) =
        super::navigation_references::ensure_course_membership_reference(state, tenant, id)
    {
        state.course_memberships.remove(&(tenant, id));
        state
            .active_course_membership_by_user
            .remove(&(tenant, course, user));
        return Err(error);
    }
    Ok(id)
}

pub(super) fn create_initial_instructor_membership(
    state: &mut State,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
) -> Result<CourseMembershipId, StoreError> {
    if state
        .active_course_membership_by_user
        .contains_key(&(tenant, course, user))
    {
        return Err(StoreError::Conflict);
    }
    let id = crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!(
            "course membership ID randomness unavailable: {error}"
        ))
    })
    .map(CourseMembershipId::from_uuid)?;
    state.course_memberships.insert(
        (tenant, id),
        CourseMembershipRecord {
            id,
            tenant,
            course,
            user,
            student: None,
            role: question_model::CourseMembershipRole::Instructor,
            roster_id: None,
            status: crate::CourseMemberStatus::Active,
            joined_at: state.authoritative_time,
            revoked_at: None,
        },
    );
    state
        .active_course_membership_by_user
        .insert((tenant, course, user), id);
    if let Err(error) =
        super::navigation_references::ensure_course_membership_reference(state, tenant, id)
    {
        state.course_memberships.remove(&(tenant, id));
        state
            .active_course_membership_by_user
            .remove(&(tenant, course, user));
        return Err(error);
    }
    Ok(id)
}

pub(super) fn active_membership_for(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
) -> Option<&CourseMembershipRecord> {
    let id = state
        .active_course_membership_by_user
        .get(&(tenant, course, user))?;
    active_membership_by_id(state, tenant, *id)
}

/// Resolves current authority from the canonical membership episode index.
/// Course aggregates deliberately carry no mirrored member or role data.
pub(super) fn current_course_role(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    user: UserId,
) -> Option<question_model::CourseMembershipRole> {
    active_membership_for(state, tenant, course, user).map(|membership| membership.role)
}

pub(super) fn active_membership_by_id(
    state: &State,
    tenant: TenantId,
    id: CourseMembershipId,
) -> Option<&CourseMembershipRecord> {
    state
        .course_memberships
        .get(&(tenant, id))
        .filter(|membership| membership.status == crate::CourseMemberStatus::Active)
}
