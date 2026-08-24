//! PostgreSQL parity for the identity-free T3 preview plane.
//!
//! The derived path deliberately uses a writable repeatable-read snapshot: it
//! reads one consistent set of S5 -> S3 -> S4 facts, retains the internal IDs
//! bound by those reads, then appends its one private record-read audit as the
//! final statement before commit. Source mutation brokers own serialization;
//! this application-role projection never acquires mutation row locks.

use async_trait::async_trait;
use domain::effective_assignment_policy::{
    AuthorizationGate, HypotheticalIndividualPolicyException, PolicyModificationMode, PolicyPatch,
    PolicyPatchSet, ResolveEffectivePolicyInput, ResolveSyntheticPreviewPolicyInput,
    assignment_lifecycle_gate, resolve_effective_policy, resolve_synthetic_preview_policy,
};
use domain::entitlement::{
    SyntheticPreviewEntitlementFacts, evaluate_synthetic_preview_entitlement,
};
use question_model::{
    ActivityTimestamp, AssignmentId, AssignmentReference, AssignmentTeachingSettingsField,
    CourseGroupId, CourseGroupPurpose, CourseId, CourseMembershipId, DerivedPreviewSubjectRequest,
    InstructorPreviewSchedulePage, InstructorPreviewScheduleRow, PreviewAccommodationComparison,
    PreviewDenialReason, PreviewDisclosureMoment, PreviewEntitlementGrantReason, PreviewEvaluation,
    PreviewGroupFact, PreviewPriorRunCount, PreviewSubject, PreviewSubjectKind,
    SyntheticPreviewSubjectRequest, TeachingAttemptLimitFieldPatch, TeachingLimitFieldPatch,
    TeachingOperationRevision, TeachingTimeFieldPatch, TenantId, UserId,
};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::*;

#[async_trait]
impl crate::PreviewPlaneStore for PostgresStore {
    async fn list_instructor_preview_schedule(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        reference: AssignmentReference,
        revision: TeachingOperationRevision,
        page: PageRequest,
    ) -> Result<InstructorPreviewSchedulePage, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant_snapshot(context).await?;
        require_direct_instructor_read_only(&mut tx, tenant, course, actor).await?;
        let (assignment, _record) =
            preview_assignment_read_only(&mut tx, tenant, course, reference, revision).await?;
        let term = course_policy::load_course_term_for_preview(&mut tx, tenant, course).await?;
        let rows = sqlx::query(
            "SELECT member.course_membership_id, member.user_id, member.student_id, member.public_id, profile.display_name \
             FROM course_member member JOIN course_roster_profile profile \
               ON profile.tenant_id=member.tenant_id AND profile.course_id=member.course_id \
              AND profile.course_membership_id=member.course_membership_id \
             WHERE member.tenant_id=$1 AND member.course_id=$2 AND member.role='student' \
               AND member.status='active' ORDER BY member.public_id"
        ).bind(tenant.as_uuid()).bind(course.as_uuid()).fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        let mut keyed = Vec::with_capacity(rows.len());
        for row in rows {
            let membership = CourseMembershipId::from_uuid(
                row.try_get("course_membership_id")
                    .map_err(map_sqlx_error)?,
            );
            let learner = UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_error)?);
            let student = question_model::StudentId::from_uuid(
                row.try_get("student_id").map_err(map_sqlx_error)?,
            );
            let public_id: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
            let public_id = u64::try_from(public_id).map_err(|_| {
                StoreError::Unavailable("stored membership reference is invalid".into())
            })?;
            let membership_ref = question_model::CourseMembershipReference::new(public_id)
                .ok_or_else(|| {
                    StoreError::Unavailable("stored membership reference is invalid".into())
                })?;
            let display_name: String = row.try_get("display_name").map_err(map_sqlx_error)?;
            let display = question_model::TeachingDisplayLabel::try_from(display_name)
                .map_err(|_| StoreError::Unavailable("stored roster display is invalid".into()))?;
            let item = match entitlement::evaluate_current_read_only(
                &mut tx, tenant, learner, course, assignment,
            )
            .await?
            {
                domain::entitlement::EntitlementDecision::Granted(grant) => {
                    let prior = completed_run_count(&mut tx, tenant, assignment, student).await?;
                    match course_policy::resolve_granted_effective_policy_read_only(
                        &mut tx,
                        grant.clone(),
                        AuthorizationGate::Authorized,
                        prior,
                    )
                    .await?
                    .0
                    {
                        domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
                            policy,
                            ..
                        } => InstructorPreviewScheduleRow::Granted {
                            membership: membership_ref,
                            display,
                            entitlement: grant_reason(grant.basis()),
                            schedule: domain::preview_plane::project_preview_schedule(
                                &policy, &term,
                            )
                            .map_err(local_error)?,
                        },
                        _ => InstructorPreviewScheduleRow::Denied {
                            membership: membership_ref,
                            display,
                            reason: question_model::PreviewEntitlementDenialReason::NotEntitled,
                        },
                    }
                }
                domain::entitlement::EntitlementDecision::Denied(_) => {
                    InstructorPreviewScheduleRow::Denied {
                        membership: membership_ref,
                        display,
                        reason: question_model::PreviewEntitlementDenialReason::NotEntitled,
                    }
                }
            };
            keyed.push((format!("{public_id:010}"), item));
            let _ = membership; // Membership is intentionally never projected.
        }
        let after = page.after.as_ref().map(Cursor::as_str);
        let mut visible = keyed
            .into_iter()
            .filter(|(key, _)| after.is_none_or(|after| key.as_str() > after))
            .collect::<Vec<_>>();
        let limit = usize::from(page.size.get());
        let has_more = visible.len() > limit;
        if has_more {
            visible.truncate(limit);
        }
        let next_cursor = has_more.then(|| {
            Cursor::from_stable_key(
                visible
                    .last()
                    .expect("a continuation has a final row")
                    .0
                    .clone(),
            )
        });
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(InstructorPreviewSchedulePage {
            revision,
            rows: visible.into_iter().map(|(_, row)| row).collect(),
            next_cursor: next_cursor.map(|cursor| cursor.as_str().to_owned()),
        })
    }

    async fn construct_synthetic_preview(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        request: SyntheticPreviewSubjectRequest,
    ) -> Result<crate::PreviewPlaneResult, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant_snapshot(context).await?;
        require_direct_instructor_read_only(&mut tx, tenant, course, actor).await?;
        let result = resolve_synthetic_preview_read_only(&mut tx, tenant, course, request).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn construct_derived_preview(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        request: DerivedPreviewSubjectRequest,
    ) -> Result<crate::PreviewPlaneResult, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant_writable_snapshot(context).await?;
        require_direct_instructor_read_only(&mut tx, tenant, course, actor).await?;
        let resolved = resolve_derived_preview_by_membership_read_only_bound(
            &mut tx,
            tenant,
            course,
            request.assignment,
            request.revision,
            request.membership,
            request.selected_moment.clone(),
        )
        .await?;
        if matches!(
            resolved.result.evaluation,
            PreviewEvaluation::Allowed { .. }
        ) {
            append_audit(
                &mut tx,
                tenant,
                actor,
                course,
                resolved.assignment,
                resolved.membership,
            )
            .await?;
        }
        let result = resolved.result;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

/// Preserves the browser route's identity-free, read-only snapshot contract.
pub(super) async fn resolve_synthetic_preview_read_only(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    request: SyntheticPreviewSubjectRequest,
) -> Result<crate::PreviewPlaneResult, StoreError> {
    let (assignment, record) =
        preview_assignment_read_only(tx, tenant, course, request.assignment, request.revision)
            .await?;
    let term = course_policy::load_course_term_for_preview(tx, tenant, course).await?;
    let now = selected_moment(&request.selected_moment, &term)?;
    let mut groups = Vec::new();
    for reference in request.groups.as_slice() {
        let row = sqlx::query(
            "SELECT course_group_id, purpose FROM course_group \
             WHERE tenant_id=$1 AND course_id=$2 AND public_id=$3",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(i64::from(reference.number()))
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        groups.push((
            CourseGroupId::from_uuid(row.try_get("course_group_id").map_err(map_sqlx_error)?),
            decode_group_purpose(row.try_get("purpose").map_err(map_sqlx_error)?)?,
        ));
    }
    let entitlement =
        evaluate_synthetic_preview_entitlement(SyntheticPreviewEntitlementFacts::new(
            tenant,
            course,
            assignment,
            record.audience.clone(),
            groups.clone(),
        ));
    let mut inputs = course_policy::load_inputs(tx, tenant, assignment, None, None).await?;
    let selected_groups = groups
        .iter()
        .map(|(group, _)| *group)
        .collect::<std::collections::BTreeSet<_>>();
    inputs
        .schedule_offsets
        .retain(|value| selected_groups.contains(&value.group));
    inputs
        .accommodations
        .retain(|value| selected_groups.contains(&value.group));
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
        entitlement: entitlement.clone(),
        authorization: AuthorizationGate::Authorized,
        now,
        prior_run_count: 0,
        base: inputs.base,
        group_schedule_offsets: inputs.schedule_offsets,
        group_accommodations: inputs.accommodations,
        hypothetical_individual_exception: Some(hypothetical(request.modifiers, &term)?),
    })
    .map_err(policy_error)?;
    preview_result(PreviewResultInput {
        kind: PreviewSubjectKind::Synthetic,
        assignment: request.assignment,
        revision: request.revision,
        selected_moment: request.selected_moment,
        groups,
        prior: 0,
        record: &record,
        term: &term,
        now,
        entitlement: synthetic_reason(&entitlement),
        before,
        after,
    })
}

/// Resolves a derived preview from the route-owned membership reference.
///
/// Resolves a derived preview with the internal membership identity that the
/// public reference selected. Rehearsal uses this after its broker preparation
/// witness has locked that exact membership. The preview plane instead relies
/// on its repeatable-read snapshot. In both cases, the returned internal IDs
/// prevent changed public references from being mistaken for audited sources.
/// // ASVS 2.2.1, 2.3.1, 8.2.2, 8.3.1
pub(super) async fn resolve_derived_preview_by_membership_read_only_bound(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    membership: question_model::CourseMembershipReference,
    selected_moment_value: question_model::PreviewSelectedMoment,
) -> Result<DerivedPreviewResolution, StoreError> {
    let (assignment_id, record) =
        preview_assignment_read_only(tx, tenant, course, assignment, revision).await?;
    let term = course_policy::load_course_term_for_preview(tx, tenant, course).await?;
    let now = selected_moment(&selected_moment_value, &term)?;
    let row = sqlx::query("SELECT course_membership_id, user_id, student_id FROM course_member WHERE tenant_id=$1 AND course_id=$2 AND public_id=$3 AND role='student' AND status='active' AND student_id IS NOT NULL")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(i64::from(membership.number()))
        .fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.ok_or(StoreError::NotFound)?;
    let membership = CourseMembershipId::from_uuid(
        row.try_get("course_membership_id")
            .map_err(map_sqlx_error)?,
    );
    let learner = UserId::from_uuid(row.try_get("user_id").map_err(map_sqlx_error)?);
    let student =
        question_model::StudentId::from_uuid(row.try_get("student_id").map_err(map_sqlx_error)?);
    let entitlement =
        entitlement::evaluate_current_read_only(tx, tenant, learner, course, assignment_id).await?;
    let domain::entitlement::EntitlementDecision::Granted(grant) = entitlement else {
        return Ok(DerivedPreviewResolution {
            result: denied(PreviewDenialReason::NotEntitled),
            assignment: assignment_id,
            membership,
        });
    };
    let groups = current_groups(tx, tenant, course, membership).await?;
    let prior = completed_run_count(tx, tenant, assignment_id, student).await?;
    let inputs = course_policy::load_inputs(
        tx,
        tenant,
        assignment_id,
        Some(student),
        Some(grant.applicable_policy_scopes()),
    )
    .await?;
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
        individual_exception: inputs.individual,
    })
    .map_err(policy_error)?;
    Ok(DerivedPreviewResolution {
        result: preview_result(PreviewResultInput {
            kind: PreviewSubjectKind::Derived,
            assignment,
            revision,
            selected_moment: selected_moment_value,
            groups,
            prior,
            record: &record,
            term: &term,
            now,
            entitlement: grant_reason(grant.basis()),
            before,
            after,
        })?,
        assignment: assignment_id,
        membership,
    })
}

/// Current-fact resolution result used only while the audit-free helper
/// constructs the standard preview projection.
pub(super) struct DerivedPreviewResolution {
    pub(super) result: crate::PreviewPlaneResult,
    pub(super) assignment: AssignmentId,
    pub(super) membership: CourseMembershipId,
}

async fn require_direct_instructor_read_only(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> Result<(), StoreError> {
    let found: Option<Uuid> = sqlx::query_scalar(
        "SELECT course_membership_id FROM course_member \
         WHERE tenant_id=$1 AND course_id=$2 AND user_id=$3 \
         AND role='instructor' AND status='active'",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(actor.as_uuid())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    found.map(|_| ()).ok_or(StoreError::NotFound)
}
async fn preview_assignment_read_only(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    reference: AssignmentReference,
    revision: TeachingOperationRevision,
) -> Result<(AssignmentId, crate::AssignmentRecord), StoreError> {
    let id: Option<Uuid> = sqlx::query_scalar(
        "SELECT assignment_id FROM assignment \
         WHERE tenant_id=$1 AND course_id=$2 AND public_id=$3",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(i64::from(reference.number()))
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let assignment = id
        .map(AssignmentId::from_uuid)
        .ok_or(StoreError::NotFound)?;
    let record = load_assignment(tx, tenant, assignment).await?;
    let stored: i64 = sqlx::query_scalar(
        "SELECT revision FROM assignment WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    (u64::try_from(stored).ok() == Some(revision.value()))
        .then_some((assignment, record))
        .ok_or(StoreError::Conflict)
}
async fn completed_run_count(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
    student: question_model::StudentId,
) -> Result<u32, StoreError> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM assignment_run run JOIN enrollment e ON e.tenant_id=run.tenant_id AND e.enrollment_id=run.enrollment_id WHERE run.tenant_id=$1 AND e.assignment_id=$2 AND e.student_id=$3 AND run.completed_at IS NOT NULL")
        .bind(tenant.as_uuid()).bind(assignment.as_uuid()).bind(student.as_uuid()).fetch_one(&mut **tx).await.map_err(map_sqlx_error)?;
    u32::try_from(count)
        .map_err(|_| StoreError::Unavailable("run count exceeds policy range".into()))
}
async fn current_groups(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    membership: CourseMembershipId,
) -> Result<Vec<(CourseGroupId, CourseGroupPurpose)>, StoreError> {
    let rows = sqlx::query("SELECT g.course_group_id, g.purpose FROM course_group_member m JOIN course_group g ON g.tenant_id=m.tenant_id AND g.course_id=m.course_id AND g.course_group_id=m.course_group_id WHERE m.tenant_id=$1 AND m.course_id=$2 AND m.course_membership_id=$3 ORDER BY g.course_group_id")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(membership.as_uuid()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
    rows.into_iter()
        .map(|r| {
            Ok((
                CourseGroupId::from_uuid(r.try_get("course_group_id").map_err(map_sqlx_error)?),
                decode_group_purpose(r.try_get("purpose").map_err(map_sqlx_error)?)?,
            ))
        })
        .collect()
}
async fn append_audit(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: UserId,
    course: CourseId,
    assignment: AssignmentId,
    target: CourseMembershipId,
) -> Result<(), StoreError> {
    let (payload, checksum) = encode_payload(&serde_json::json!({
        "assignmentId": assignment.as_uuid(),
        "kind": "previewSubjectDerived",
        "schemaVersion": 1,
    }))?;
    sqlx::query("INSERT INTO audit_event (tenant_id,audit_event_id,occurred_at,actor_id,course_id,action,target_kind,target_id,payload,payload_sha256) VALUES ($1,$2,transaction_timestamp(),$3,$4,'preview.subject.derived','course_membership',$5,$6,$7)")
        .bind(tenant.as_uuid()).bind(sqlx::query_scalar::<_, Uuid>("SELECT gen_random_uuid()").fetch_one(&mut **tx).await.map_err(map_sqlx_error)?).bind(actor.as_uuid()).bind(course.as_uuid()).bind(target.as_uuid()).bind(payload).bind(checksum).execute(&mut **tx).await.map_err(map_sqlx_error)?;
    Ok(())
}
fn decode_group_purpose(value: String) -> Result<CourseGroupPurpose, StoreError> {
    match value.as_str() {
        "section" => Ok(CourseGroupPurpose::Section),
        "lab" => Ok(CourseGroupPurpose::Lab),
        "cohort" => Ok(CourseGroupPurpose::Cohort),
        "accommodation" => Ok(CourseGroupPurpose::Accommodation),
        "work" => Ok(CourseGroupPurpose::Work),
        _ => Err(StoreError::Unavailable(
            "stored group purpose is invalid".into(),
        )),
    }
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
fn synthetic_reason(
    value: &domain::entitlement::SyntheticPreviewEntitlementDecision,
) -> PreviewEntitlementGrantReason {
    match value {
        domain::entitlement::SyntheticPreviewEntitlementDecision::Granted(v) => {
            grant_reason(v.basis())
        }
        domain::entitlement::SyntheticPreviewEntitlementDecision::Denied(_) => {
            PreviewEntitlementGrantReason::CourseWide
        }
    }
}
fn denied(reason: PreviewDenialReason) -> crate::PreviewPlaneResult {
    crate::PreviewPlaneResult {
        evaluation: PreviewEvaluation::Denied { reason },
        accommodation: None,
    }
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

struct PreviewResultInput<'a> {
    kind: PreviewSubjectKind,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    selected_moment: question_model::PreviewSelectedMoment,
    groups: Vec<(CourseGroupId, CourseGroupPurpose)>,
    prior: u32,
    record: &'a crate::AssignmentRecord,
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
        return Ok(denied(PreviewDenialReason::NotEntitled));
    };
    let domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
        policy: after_policy,
        start,
    } = after
    else {
        return Ok(denied(PreviewDenialReason::NotEntitled));
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
            .map(|(_, p)| *p)
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
                start,
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
