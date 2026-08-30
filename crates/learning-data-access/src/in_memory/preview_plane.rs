//! Memory implementation of the identity-free T3 preview plane.

use crate::ActorContext;
use async_trait::async_trait;
use domain::effective_assignment_policy::{
    AuthorizationGate, HypotheticalIndividualPolicyException, PolicyModificationMode, PolicyPatch,
    PolicyPatchSet, ResolveEffectivePolicyInput, ResolveSyntheticPreviewPolicyInput,
    assignment_lifecycle_gate, resolve_effective_policy, resolve_synthetic_preview_policy,
};
use domain::entitlement::{
    SyntheticPreviewEntitlementFacts, evaluate_synthetic_preview_entitlement,
};
use objects::Sha256Digest;
use question_model::{
    AssignmentTeachingSettingsField, CourseMembershipRole, DerivedPreviewSubjectRequest,
    InstructorPreviewSchedulePage, InstructorPreviewScheduleRow, PreviewAccommodationComparison,
    PreviewDisclosureMoment, PreviewEntitlementGrantReason, PreviewEvaluation, PreviewGroupFact,
    PreviewPriorRunCount, PreviewSubject, PreviewSubjectKind, SyntheticPreviewSubjectRequest,
    TeachingAttemptLimitFieldPatch, TeachingLimitFieldPatch, TeachingOperationRevision,
    TeachingTimeFieldPatch,
};

use super::*;

#[async_trait]
impl crate::PoolPreviewStore for MemoryStore {
    async fn preview_pool_draw(
        &self,
        context: ActorContext,
        command: crate::PoolPreviewCommand,
    ) -> Result<question_model::PoolDrawPreview, StoreError> {
        let state = self.read_state()?;
        (context.user_id() == command.actor)
            .then_some(())
            .ok_or(StoreError::NotFound)?;
        super::teaching_authority::require_direct_instructor(
            &state,
            command.course,
            command.actor,
        )?;
        let (assignment_id, assignment) =
            preview_assignment(&state, command.course, command.assignment, command.revision)?;
        let group = assignment
            .selection_groups
            .iter()
            .find(|group| group.position == command.group_position)
            .ok_or(StoreError::NotFound)?;
        let (_, sampled) = crate::select_assignment_group_candidates(
            group,
            question_model::PoolDrawBasis::preview(assignment_id, group.id, command.nonce),
        )?;
        let question = |candidate: &question_model::AssignmentSelectionCandidate| {
            let published = state
                .published
                .get(&(candidate.reference.problem, candidate.reference.version))
                .ok_or(StoreError::NotFound)?;
            Ok::<question_model::PoolDrawPreviewQuestion, StoreError>(
                question_model::PoolDrawPreviewQuestion {
                    question_id: published.question_id.clone(),
                    title: published.question.metadata.title.clone(),
                },
            )
        };
        Ok(question_model::PoolDrawPreview {
            assignment: command.assignment,
            revision: command.revision,
            group_position: command.group_position,
            group_label: pool_preview_group_label(command.group_position),
            draw_count: group.draw_count,
            ordering: group.ordering,
            algorithm: group.algorithm,
            candidates: group
                .candidates
                .iter()
                .filter(|candidate| {
                    candidate.delivery_state == question_model::AssignmentDeliveryState::Active
                })
                .map(question)
                .collect::<Result<Vec<_>, _>>()?,
            sampled: sampled
                .into_iter()
                .map(question)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

#[async_trait]
impl crate::PreviewPlaneStore for MemoryStore {
    async fn list_instructor_preview_schedule(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        assignment_ref: question_model::AssignmentReference,
        revision: TeachingOperationRevision,
        page: crate::PageRequest,
    ) -> Result<InstructorPreviewSchedulePage, StoreError> {
        let state = self.read_state()?;
        (context.user_id() == actor)
            .then_some(())
            .ok_or(StoreError::NotFound)?;
        super::teaching_authority::require_direct_instructor(&state, course, actor)?;
        let (assignment, record) = preview_assignment(&state, course, assignment_ref, revision)?;
        let mut rows = Vec::new();
        for membership in state.course_memberships.values().filter(|value| {
            value.course == course
                && value.status == crate::CourseMemberStatus::Active
                && value.role == CourseMembershipRole::Student
        }) {
            let student = membership.student.ok_or_else(|| {
                StoreError::InvalidRecord(
                    "active Student course membership is missing its Student identity".into(),
                )
            })?;
            let reference = state
                .course_membership_references
                .get(&membership.id)
                .copied()
                .ok_or(StoreError::NotFound)?;
            let display = state
                .roster_profiles
                .get(&(course, membership.id))
                .and_then(|profile| {
                    question_model::TeachingDisplayLabel::try_from(profile.display_name.clone())
                        .ok()
                })
                .ok_or(StoreError::NotFound)?;
            let entitlement =
                super::entitlement::evaluate_locked(&state, membership.user, course, assignment)?;
            match entitlement {
                domain::entitlement::EntitlementDecision::Granted(grant) => {
                    let prior = completed_run_count(&state, assignment, student)?;
                    let policy = super::course_policy::resolve_granted_memory_effective_policy(
                        &state,
                        record,
                        grant.clone(),
                        prior,
                    )?;
                    if let domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
                        policy,
                        ..
                    } = policy
                    {
                        rows.push((
                            format!("{:010}", reference.number()),
                            InstructorPreviewScheduleRow::Granted {
                                membership: reference,
                                display,
                                entitlement: grant_reason(grant.basis()),
                                schedule: domain::preview_plane::project_preview_schedule(
                                    &policy,
                                    &state.courses.get(&course).ok_or(StoreError::NotFound)?.term,
                                )
                                .map_err(local_error)?,
                            },
                        ));
                    } else {
                        rows.push((
                            format!("{:010}", reference.number()),
                            InstructorPreviewScheduleRow::Denied {
                                membership: reference,
                                display,
                                reason: question_model::PreviewEntitlementDenialReason::NotEntitled,
                            },
                        ));
                    }
                }
                domain::entitlement::EntitlementDecision::Denied(_) => rows.push((
                    format!("{:010}", reference.number()),
                    InstructorPreviewScheduleRow::Denied {
                        membership: reference,
                        display,
                        reason: question_model::PreviewEntitlementDenialReason::NotEntitled,
                    },
                )),
            }
        }
        let page = super::catalog::page_records(rows, &page);
        Ok(InstructorPreviewSchedulePage {
            revision,
            rows: page.items,
            next_cursor: page.next_cursor.map(|cursor| cursor.as_str().to_owned()),
        })
    }

    async fn construct_synthetic_preview(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        request: SyntheticPreviewSubjectRequest,
    ) -> Result<crate::PreviewPlaneResult, StoreError> {
        let state = self.read_state()?;
        (context.user_id() == actor)
            .then_some(())
            .ok_or(StoreError::NotFound)?;
        super::teaching_authority::require_direct_instructor(&state, course, actor)?;
        resolve_synthetic_preview_locked(&state, course, request)
    }

    async fn construct_derived_preview(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        request: DerivedPreviewSubjectRequest,
    ) -> Result<crate::PreviewPlaneResult, StoreError> {
        let mut state = self.write_state()?;
        (context.user_id() == actor)
            .then_some(())
            .ok_or(StoreError::NotFound)?;
        super::teaching_authority::require_direct_instructor(&state, course, actor)?;
        let result = resolve_derived_preview_locked(
            &state,
            course,
            request.assignment,
            request.revision,
            request.membership,
            request.selected_moment,
        )?;
        if matches!(result.evaluation, PreviewEvaluation::Allowed { .. }) {
            let assignment_id = state
                .assignments_by_reference
                .get(&request.assignment)
                .copied()
                .ok_or(StoreError::NotFound)?;
            let payload = format!(
                "previewSubjectDerived:v1:{}:{}:{}",
                actor, course, assignment_id
            );
            let membership_id = state
                .course_memberships_by_reference
                .get(&request.membership)
                .copied()
                .ok_or(StoreError::NotFound)?;
            state
                .preview_subject_audits
                .push(crate::PreviewSubjectAudit {
                    actor,
                    course,
                    assignment: request.assignment,
                    target_membership: membership_id,
                    action: "preview.subject.derived",
                    schema_version: 1,
                    payload_sha256: Sha256Digest::compute(payload.as_bytes()),
                });
        }
        Ok(result)
    }
}

fn pool_preview_group_label(position: u32) -> String {
    format!("Pool {}", position.saturating_add(1))
}

pub(super) fn resolve_synthetic_preview_locked(
    state: &State,
    course: CourseId,
    request: SyntheticPreviewSubjectRequest,
) -> Result<crate::PreviewPlaneResult, StoreError> {
    let (assignment, record) =
        preview_assignment(state, course, request.assignment, request.revision)?;
    let term = &state.courses.get(&course).ok_or(StoreError::NotFound)?.term;
    let now = selected_moment(&request.selected_moment, term)?;
    let groups = request
        .groups
        .as_slice()
        .iter()
        .map(|reference| {
            let id = state
                .course_groups_by_reference
                .get(reference)
                .copied()
                .ok_or(StoreError::NotFound)?;
            let group = state
                .course_groups
                .get(&id)
                .filter(|group| group.course == course)
                .ok_or(StoreError::NotFound)?;
            Ok((id, group.purpose))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let entitlement =
        evaluate_synthetic_preview_entitlement(SyntheticPreviewEntitlementFacts::new(
            course,
            assignment,
            record.audience.clone(),
            groups.clone(),
        ));
    let inputs = synthetic_inputs(state, assignment, &groups)?;
    let entitlement_reason = match &entitlement {
        domain::entitlement::SyntheticPreviewEntitlementDecision::Granted(grant) => {
            grant_reason(grant.basis())
        }
        domain::entitlement::SyntheticPreviewEntitlementDecision::Denied(_) => {
            PreviewEntitlementGrantReason::CourseWide
        }
    };
    let before = resolve_synthetic_preview_policy(ResolveSyntheticPreviewPolicyInput {
        lifecycle: assignment_lifecycle_gate(record.lifecycle),
        entitlement: entitlement.clone(),
        authorization: AuthorizationGate::Authorized,
        now,
        prior_run_count: 0,
        base: inputs.base,
        group_schedule_offsets: inputs.schedule_offsets.clone(),
        group_accommodations: inputs.accommodations.clone(),
        hypothetical_individual_exception: None,
    })
    .map_err(policy_error)?;
    let after = resolve_synthetic_preview_policy(ResolveSyntheticPreviewPolicyInput {
        lifecycle: assignment_lifecycle_gate(record.lifecycle),
        entitlement,
        authorization: AuthorizationGate::Authorized,
        now,
        prior_run_count: 0,
        base: inputs.base,
        group_schedule_offsets: inputs.schedule_offsets,
        group_accommodations: inputs.accommodations,
        hypothetical_individual_exception: Some(hypothetical(request.modifiers, term)?),
    })
    .map_err(policy_error)?;
    preview_result(PreviewResultInput {
        kind: PreviewSubjectKind::Synthetic,
        assignment: request.assignment,
        revision: request.revision,
        selected_moment: request.selected_moment,
        groups,
        prior: 0,
        record,
        term,
        now,
        entitlement: entitlement_reason,
        before,
        after,
    })
}

pub(super) fn resolve_derived_preview_locked(
    state: &State,
    course: CourseId,
    assignment_ref: question_model::AssignmentReference,
    revision: TeachingOperationRevision,
    membership_ref: question_model::CourseMembershipReference,
    selected_moment_value: question_model::PreviewSelectedMoment,
) -> Result<crate::PreviewPlaneResult, StoreError> {
    let (assignment, record) = preview_assignment(state, course, assignment_ref, revision)?;
    let term = state
        .courses
        .get(&course)
        .ok_or(StoreError::NotFound)?
        .term
        .clone();
    let now = selected_moment(&selected_moment_value, &term)?;
    let membership_id = state
        .course_memberships_by_reference
        .get(&membership_ref)
        .copied()
        .ok_or(StoreError::NotFound)?;
    let membership = super::entitlement::active_membership_by_id(state, membership_id)
        .filter(|value| value.course == course && value.role == CourseMembershipRole::Student)
        .ok_or(StoreError::NotFound)?;
    let student_user = membership.user;
    let student = membership.student.ok_or(StoreError::NotFound)?;
    let groups = current_groups(state, course, membership_id);
    let entitlement = super::entitlement::evaluate_locked(state, student_user, course, assignment)?;
    let domain::entitlement::EntitlementDecision::Granted(grant) = entitlement else {
        return Ok(denied(question_model::PreviewDenialReason::NotEntitled));
    };
    let prior = completed_run_count(state, assignment, student)?;
    let inputs =
        super::course_policy::memory_effective_policy_inputs_for_grant(state, assignment, &grant)?;
    let before = resolve_effective_policy(ResolveEffectivePolicyInput {
        lifecycle: assignment_lifecycle_gate(record.lifecycle),
        entitlement: domain::entitlement::EntitlementDecision::Granted(grant.clone()),
        authorization: AuthorizationGate::Authorized,
        now,
        prior_run_count: prior,
        base: inputs.base,
        group_schedule_offsets: inputs.schedule_offsets.clone(),
        group_accommodations: inputs.accommodations.clone(),
        individual_exception: None,
    })
    .map_err(policy_error)?;
    let after = resolve_effective_policy(ResolveEffectivePolicyInput {
        lifecycle: assignment_lifecycle_gate(record.lifecycle),
        entitlement: domain::entitlement::EntitlementDecision::Granted(grant.clone()),
        authorization: AuthorizationGate::Authorized,
        now,
        prior_run_count: prior,
        base: inputs.base,
        group_schedule_offsets: inputs.schedule_offsets,
        group_accommodations: inputs.accommodations,
        individual_exception: state
            .assignment_individual_policy_exceptions
            .get(&(assignment, student))
            .map(|value| value.exception),
    })
    .map_err(policy_error)?;
    let result = preview_result(PreviewResultInput {
        kind: PreviewSubjectKind::Derived,
        assignment: assignment_ref,
        revision,
        selected_moment: selected_moment_value,
        groups,
        prior,
        record,
        term: &term,
        now,
        entitlement: grant_reason(grant.basis()),
        before,
        after,
    })?;
    Ok(result)
}

fn preview_assignment(
    state: &State,
    course: CourseId,
    reference: question_model::AssignmentReference,
    revision: TeachingOperationRevision,
) -> Result<(AssignmentId, &AssignmentRecord), StoreError> {
    let assignment = state
        .assignments_by_reference
        .get(&reference)
        .copied()
        .ok_or(StoreError::NotFound)?;
    let record = state
        .assignments
        .get(&assignment)
        .filter(|value| value.course_id == course)
        .ok_or(StoreError::NotFound)?;
    let current = state
        .assignment_revisions
        .get(&assignment)
        .copied()
        .ok_or(StoreError::NotFound)?;
    (current.value() == revision.value())
        .then_some((assignment, record))
        .ok_or(StoreError::Conflict)
}
fn selected_moment(
    value: &question_model::PreviewSelectedMoment,
    term: &question_model::CourseTerm,
) -> Result<ActivityTimestamp, StoreError> {
    if value.time_zone != *term.time_zone() {
        return Err(StoreError::InvalidRecord(
            "preview moment must use the course time zone".into(),
        ));
    }
    value
        .value
        .resolve_for_course(term, AssignmentTeachingSettingsField::AvailableAt)
        .map_err(local_error)
}
fn local_error(error: question_model::AssignmentTeachingSettingsLocalError) -> StoreError {
    StoreError::InvalidRecord(format!("invalid preview local time: {error:?}"))
}
fn policy_error(error: domain::effective_assignment_policy::EffectivePolicyError) -> StoreError {
    StoreError::InvalidRecord(format!("invalid preview policy: {error:?}"))
}
fn completed_run_count(
    state: &State,
    assignment: AssignmentId,
    student: StudentId,
) -> Result<u32, StoreError> {
    u32::try_from(
        state
            .runs
            .values()
            .filter(|run| {
                run.completed_at.is_some()
                    && state
                        .enrollments
                        .get(&run.enrollment)
                        .is_some_and(|enrollment| {
                            enrollment.assignment == assignment && enrollment.student == student
                        })
            })
            .count(),
    )
    .map_err(|_| StoreError::Unavailable("run count exceeds policy range".into()))
}
fn current_groups(
    state: &State,
    course: CourseId,
    membership: CourseMembershipId,
) -> Vec<(CourseGroupId, question_model::CourseGroupPurpose)> {
    state
        .course_groups
        .iter()
        .filter_map(|(id, group)| {
            (group.course == course && group.members.contains(&membership))
                .then_some((*id, group.purpose))
        })
        .collect()
}
fn synthetic_inputs(
    state: &State,
    assignment: AssignmentId,
    groups: &[(CourseGroupId, question_model::CourseGroupPurpose)],
) -> Result<crate::EffectivePolicyInputs, StoreError> {
    let base = state
        .assignment_base_policy
        .get(&assignment)
        .ok_or(StoreError::NotFound)?
        .policy;
    let ids = groups
        .iter()
        .map(|(id, _)| *id)
        .collect::<std::collections::BTreeSet<_>>();
    Ok(crate::EffectivePolicyInputs {
        base,
        schedule_offsets: state
            .assignment_group_schedule_offsets
            .iter()
            .filter_map(|((entry_assignment, group), value)| {
                (*entry_assignment == assignment && ids.contains(group)).then_some(*value)
            })
            .collect(),
        accommodations: state
            .assignment_group_accommodations
            .iter()
            .filter_map(|((entry_assignment, group), value)| {
                (*entry_assignment == assignment && ids.contains(group)).then_some(*value)
            })
            .collect(),
        individual: None,
    })
}
fn hypothetical(
    modifiers: question_model::SyntheticPreviewModifiers,
    term: &question_model::CourseTerm,
) -> Result<HypotheticalIndividualPolicyException, StoreError> {
    Ok(HypotheticalIndividualPolicyException {
        mode: match modifiers.mode {
            question_model::PolicyModificationModeView::ExtendOnly => {
                PolicyModificationMode::ExtendOnly
            }
            question_model::PolicyModificationModeView::Override => {
                PolicyModificationMode::Override
            }
        },
        patch: PolicyPatchSet {
            available_at: time_patch(
                modifiers.patch.available_at,
                term,
                AssignmentTeachingSettingsField::AvailableAt,
            )?,
            due_at: time_patch(
                modifiers.patch.due_at,
                term,
                AssignmentTeachingSettingsField::DueAt,
            )?,
            closes_at: time_patch(
                modifiers.patch.closes_at,
                term,
                AssignmentTeachingSettingsField::ClosesAt,
            )?,
            time_limit_seconds: limit_patch(modifiers.patch.time_limit_seconds),
            attempt_limit: attempt_patch(modifiers.patch.attempt_limit),
        },
    })
}
fn time_patch(
    value: TeachingTimeFieldPatch,
    term: &question_model::CourseTerm,
    field: AssignmentTeachingSettingsField,
) -> Result<PolicyPatch<ActivityTimestamp>, StoreError> {
    Ok(match value {
        TeachingTimeFieldPatch::Inherit => PolicyPatch::Inherit,
        TeachingTimeFieldPatch::Unrestricted => PolicyPatch::Unrestricted,
        TeachingTimeFieldPatch::Set { value } => {
            PolicyPatch::Set(value.resolve_for_course(term, field).map_err(local_error)?)
        }
    })
}
fn limit_patch(value: TeachingLimitFieldPatch) -> PolicyPatch<std::num::NonZeroU32> {
    match value {
        TeachingLimitFieldPatch::Inherit => PolicyPatch::Inherit,
        TeachingLimitFieldPatch::Unrestricted => PolicyPatch::Unrestricted,
        TeachingLimitFieldPatch::Set { value } => {
            PolicyPatch::Set(std::num::NonZeroU32::new(u32::from(value)).expect("validated"))
        }
    }
}
fn attempt_patch(value: TeachingAttemptLimitFieldPatch) -> PolicyPatch<std::num::NonZeroU32> {
    match value {
        TeachingAttemptLimitFieldPatch::Inherit => PolicyPatch::Inherit,
        TeachingAttemptLimitFieldPatch::Unrestricted => PolicyPatch::Unrestricted,
        TeachingAttemptLimitFieldPatch::Set { value } => {
            PolicyPatch::Set(std::num::NonZeroU32::new(u32::from(value)).expect("validated"))
        }
    }
}
fn grant_reason(basis: question_model::MaterializationBasis) -> PreviewEntitlementGrantReason {
    match basis {
        question_model::MaterializationBasis::CourseWide => {
            PreviewEntitlementGrantReason::CourseWide
        }
        question_model::MaterializationBasis::GroupAudience { .. } => {
            PreviewEntitlementGrantReason::GroupAudience
        }
    }
}
fn denied(reason: question_model::PreviewDenialReason) -> crate::PreviewPlaneResult {
    crate::PreviewPlaneResult {
        evaluation: PreviewEvaluation::Denied { reason },
        accommodation: None,
    }
}
struct PreviewResultInput<'a> {
    kind: PreviewSubjectKind,
    assignment: question_model::AssignmentReference,
    revision: TeachingOperationRevision,
    selected_moment: question_model::PreviewSelectedMoment,
    groups: Vec<(CourseGroupId, question_model::CourseGroupPurpose)>,
    prior: u32,
    record: &'a AssignmentRecord,
    term: &'a question_model::CourseTerm,
    now: ActivityTimestamp,
    entitlement: PreviewEntitlementGrantReason,
    before: domain::effective_assignment_policy::EffectivePolicyDecision,
    after: domain::effective_assignment_policy::EffectivePolicyDecision,
}

fn preview_result(input: PreviewResultInput<'_>) -> Result<crate::PreviewPlaneResult, StoreError> {
    let PreviewResultInput {
        kind,
        assignment,
        revision,
        selected_moment,
        groups,
        prior,
        record,
        term,
        now,
        entitlement,
        before,
        after,
    } = input;
    let domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
        policy: before_policy,
        ..
    } = before
    else {
        return Ok(denied(question_model::PreviewDenialReason::NotEntitled));
    };
    let domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
        policy: after_policy,
        start: after_start,
    } = after
    else {
        return Ok(denied(question_model::PreviewDenialReason::NotEntitled));
    };
    let policy = domain::preview_plane::project_preview_policy(&after_policy, term)
        .map_err(|_| StoreError::InvalidRecord("invalid preview policy".into()))?;
    let subject = PreviewSubject::new(
        kind,
        assignment,
        revision,
        selected_moment,
        groups
            .iter()
            .map(|(_, purpose)| *purpose)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(PreviewGroupFact::from_purpose)
            .collect(),
        policy,
        PreviewPriorRunCount::try_from(prior).map_err(|e| StoreError::InvalidRecord(e.into()))?,
    )
    .map_err(|e| StoreError::InvalidRecord(e.into()))?;
    let schedule = domain::preview_plane::project_preview_schedule(&after_policy, term)
        .map_err(local_error)?;
    let disclosure = [
        PreviewDisclosureMoment::Now,
        PreviewDisclosureMoment::Due,
        PreviewDisclosureMoment::Close,
    ]
    .into_iter()
    .map(|moment| {
        domain::preview_plane::project_preview_disclosure(
            &domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
                policy: after_policy.clone(),
                start: after_start,
            },
            record.disclosure_policy,
            moment,
            now,
            None,
        )
    })
    .collect();
    Ok(crate::PreviewPlaneResult {
        evaluation: PreviewEvaluation::Allowed {
            subject,
            entitlement,
            schedule,
            disclosure,
        },
        accommodation: Some(PreviewAccommodationComparison {
            before: domain::preview_plane::project_preview_schedule(&before_policy, term)
                .map_err(local_error)?,
            after: domain::preview_plane::project_preview_schedule(&after_policy, term)
                .map_err(local_error)?,
        }),
    })
}
