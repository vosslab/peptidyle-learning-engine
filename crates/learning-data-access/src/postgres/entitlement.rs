//! PostgreSQL ownership of current entitlement and receipt materialization.
//!
//! This is deliberately the only PostgreSQL path that evaluates the current
//! membership/audience/group facts.  A retained enrollment is evidence only.

use async_trait::async_trait;
use domain::entitlement::{
    ActiveStudentMembership, EntitlementDecision, EntitlementFacts, evaluate_assignment_entitlement,
};
use question_model::{
    ActivityTimestamp, AssignmentAudience, AssignmentEnrollment, CourseGroupId, CourseGroupPurpose,
    CourseMembershipId, EnrollmentId, EntitlementMaterialization, EvaluatorVersion,
    MaterializationAuthority, MaterializationBasis, MaterializationDisposition,
    StudentAssignmentSummary, TenantId, UserId,
};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::learner_work_preparation::{
    EntitlementPreparationWitness, WitnessAssignmentLifecycle, WitnessAudienceKind,
    prepare_entitlement_materialization,
};
use super::{
    PostgresStore, database_timestamp, decode_summary_row, load_assignment,
    load_postgres_enrollment, map_sqlx_error, retry_transaction,
};
use crate::{
    AssignmentEntitlementMaterialization, EntitlementStore,
    MaterializeAssignmentEntitlementCommand, MaterializedAssignmentEntitlement, Page, PageRequest,
    StoreError, TenantContext,
};

mod prepared_student_attempt;
pub(super) use prepared_student_attempt::{
    PreparedStudentAttemptWork, hydrate_prepared_student_attempt_work,
};

#[async_trait]
impl EntitlementStore for PostgresStore {
    async fn list_learner_entitled_assignments_impl(
        &self,
        context: TenantContext,
        learner: UserId,
        course: question_model::CourseId,
        page: PageRequest,
    ) -> Result<Page<crate::AssignmentRecord>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let mut scan_after = page
            .after
            .as_ref()
            .map(|cursor| cursor.as_str().to_string());
        // SQL owns only candidate ordering and tenant/course confinement. The
        // pure domain evaluator owns *all* membership, audience, and group
        // decisions so list visibility cannot drift from learner actions.
        let visible_limit = usize::from(page.size.get()) + 1;
        let batch_limit = i64::from(page.size.get()) + 1;
        let mut records = Vec::with_capacity(visible_limit);
        while records.len() < visible_limit {
            let candidates = sqlx::query_scalar::<_, Uuid>(
                "SELECT assignment_id FROM assignment \
                 WHERE tenant_id = $1 AND course_id = $2 \
                   AND ($3::text IS NULL OR assignment_id::text > $3) \
                 ORDER BY assignment_id::text LIMIT $4",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(course.as_uuid())
            .bind(scan_after.as_deref())
            .bind(batch_limit)
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            if candidates.is_empty() {
                break;
            }
            let exhausted = candidates.len() < usize::try_from(batch_limit).expect("page bound");
            for id in candidates {
                scan_after = Some(id.to_string());
                let assignment = question_model::AssignmentId::from_uuid(id);
                let decision = evaluate_current(
                    &mut transaction,
                    context.tenant_id(),
                    learner,
                    course,
                    assignment,
                )
                .await?;
                let EntitlementDecision::Granted(grant) = decision else {
                    continue;
                };
                // S5 alone decides audience and membership.  S3 decides
                // whether that currently entitled learner may see/start this
                // assignment; only an allowed candidate consumes page space.
                let prior_runs: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM assignment_run run JOIN enrollment enrollment ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id WHERE run.tenant_id=$1 AND enrollment.assignment_id=$2 AND enrollment.student_id=$3 AND run.completed_at IS NOT NULL",
                ).bind(context.tenant_id().as_uuid()).bind(assignment.as_uuid()).bind(grant.student().as_uuid()).fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
                let (policy, _) = super::course_policy::resolve_granted_effective_policy(
                    &mut transaction,
                    grant,
                    domain::effective_assignment_policy::AuthorizationGate::Authorized,
                    u32::try_from(prior_runs).map_err(|_| {
                        StoreError::Unavailable("run count exceeds policy range".to_string())
                    })?,
                )
                .await?;
                if matches!(
                    policy,
                    domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
                        start: domain::effective_assignment_policy::StartVerdict::MayStart { .. },
                        ..
                    }
                ) {
                    records.push(
                        load_assignment(&mut transaction, context.tenant_id(), assignment).await?,
                    );
                    if records.len() == visible_limit {
                        break;
                    }
                }
            }
            if exhausted {
                break;
            }
        }
        let has_more = records.len() == visible_limit;
        if has_more {
            records.pop();
        }
        let next_cursor = has_more.then(|| {
            crate::Cursor::from_stable_key(
                records
                    .last()
                    .expect("a visible continuation follows a nonempty page")
                    .id
                    .to_string(),
            )
        });
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Page {
            items: records,
            next_cursor,
        })
    }

    async fn evaluate_assignment_entitlement_impl(
        &self,
        context: TenantContext,
        learner: UserId,
        course: question_model::CourseId,
        assignment: question_model::AssignmentId,
    ) -> Result<EntitlementDecision, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let result = evaluate_current(
            &mut transaction,
            context.tenant_id(),
            learner,
            course,
            assignment,
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn issue_assignment_entitlement_impl(
        &self,
        context: TenantContext,
        command: MaterializeAssignmentEntitlementCommand,
    ) -> Result<AssignmentEntitlementMaterialization, StoreError> {
        retry_transaction(|| async move {
            let mut transaction = self.begin_tenant(context).await?;
            let result = materialize(&mut transaction, context.tenant_id(), command).await?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(result)
        })
        .await
    }
}

pub(super) async fn materialize(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    command: MaterializeAssignmentEntitlementCommand,
) -> Result<AssignmentEntitlementMaterialization, StoreError> {
    match prepare_materialization(transaction, tenant, command).await? {
        PreparedEntitlementMaterialization::Denied(reason) => {
            Ok(AssignmentEntitlementMaterialization::Denied(reason))
        }
        PreparedEntitlementMaterialization::Granted(prepared) => {
            materialize_prepared_entitlement(transaction, *prepared).await
        }
    }
}

/// Broker-authorized entitlement evaluation retained for this transaction.
pub(super) struct PreparedGrantedEntitlement {
    command: MaterializeAssignmentEntitlementCommand,
    witness: EntitlementPreparationWitness,
    grant: domain::entitlement::EntitlementGrant,
}

impl PreparedGrantedEntitlement {
    pub(super) fn grant(&self) -> &domain::entitlement::EntitlementGrant {
        &self.grant
    }

    pub(super) fn existing_enrollment(&self) -> Option<EnrollmentId> {
        self.witness.existing_enrollment
    }

    fn command(&self) -> MaterializeAssignmentEntitlementCommand {
        self.command
    }
}

pub(super) enum PreparedEntitlementMaterialization {
    Granted(Box<PreparedGrantedEntitlement>),
    Denied(domain::entitlement::EntitlementDenial),
}

/// Runs the broker, exact hydration, and pure evaluator once.
pub(super) async fn prepare_materialization(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    command: MaterializeAssignmentEntitlementCommand,
) -> Result<PreparedEntitlementMaterialization, StoreError> {
    let witness = prepare_entitlement_materialization(transaction, tenant, command).await?;
    let (membership, audience, groups) =
        hydrate_entitlement_witness_sources(transaction, &witness).await?;
    let decision = evaluate_assignment_entitlement(EntitlementFacts {
        tenant,
        course: command.course(),
        assignment: command.assignment(),
        learner: command.learner(),
        membership,
        audience,
        current_groups: groups,
    });
    let EntitlementDecision::Granted(grant) = decision else {
        let EntitlementDecision::Denied(reason) = decision else {
            unreachable!();
        };
        return Ok(PreparedEntitlementMaterialization::Denied(reason));
    };

    Ok(PreparedEntitlementMaterialization::Granted(Box::new(
        PreparedGrantedEntitlement {
            command,
            witness,
            grant,
        },
    )))
}

/// Materializes only a prepared entitlement.
pub(super) async fn materialize_prepared_entitlement(
    transaction: &mut Transaction<'_, Postgres>,
    prepared: PreparedGrantedEntitlement,
) -> Result<AssignmentEntitlementMaterialization, StoreError> {
    let tenant = prepared.witness.tenant;
    let now = database_timestamp(transaction).await?;
    let existing = load_existing_receipt(
        transaction,
        tenant,
        prepared.command.assignment(),
        prepared.grant.student(),
    )
    .await?;
    if existing.as_ref().map(|value| value.0.id) != prepared.witness.existing_enrollment {
        return Err(StoreError::InvalidRecord(
            "entitlement receipt disagrees with learner-work preparation witness".to_string(),
        ));
    }
    let (enrollment, summary, provenance, disposition) = match existing {
        Some(value) => value,
        None => insert_receipt(transaction, &prepared.grant, prepared.command(), now).await?,
    };
    Ok(AssignmentEntitlementMaterialization::Granted(
        MaterializedAssignmentEntitlement {
            enrollment,
            summary,
            provenance,
            disposition,
            applicable_policy_scopes: prepared.grant.applicable_policy_scopes().clone(),
        },
    ))
}

/// Hydrates full assignment facts after the broker witness is validated.
pub(super) async fn hydrate_prepared_assignment(
    transaction: &mut Transaction<'_, Postgres>,
    prepared: &PreparedGrantedEntitlement,
) -> Result<crate::AssignmentRecord, StoreError> {
    hydrate_assignment_from_witness(transaction, &prepared.witness).await
}

pub(super) async fn hydrate_assignment_from_witness(
    transaction: &mut Transaction<'_, Postgres>,
    witness: &EntitlementPreparationWitness,
) -> Result<crate::AssignmentRecord, StoreError> {
    let assignment = load_assignment(transaction, witness.tenant, witness.assignment).await?;
    if assignment.course_id != witness.course
        || !witness_lifecycle_matches(
            match assignment.lifecycle {
                question_model::AssignmentLifecycle::Draft => "draft",
                question_model::AssignmentLifecycle::Published => "published",
                question_model::AssignmentLifecycle::Closed => "closed",
                question_model::AssignmentLifecycle::Archived => "archived",
            },
            witness.lifecycle,
        )
        || !witness_audience_matches(&assignment.audience, witness)
    {
        return Err(StoreError::InvalidRecord(
            "prepared assignment record disagrees with learner-work witness".to_string(),
        ));
    }
    let revision: i64 = sqlx::query_scalar(
        "SELECT revision FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(witness.tenant.as_uuid())
    .bind(witness.assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| StoreError::InvalidRecord("prepared assignment disappeared".to_string()))?;
    if u64::try_from(revision).ok() != Some(witness.assignment_revision) {
        return Err(StoreError::InvalidRecord(
            "prepared assignment revision disagrees with learner-work witness".to_string(),
        ));
    }
    Ok(assignment)
}

/// Hydrates only identifiers and current facts named by a validated broker
/// witness.  The broker owns the source locks; these deliberately remain
/// ordinary reads under the same transaction.
pub(super) async fn hydrate_entitlement_witness_sources(
    transaction: &mut Transaction<'_, Postgres>,
    witness: &EntitlementPreparationWitness,
) -> Result<
    (
        Option<ActiveStudentMembership>,
        AssignmentAudience,
        Vec<(CourseGroupId, CourseGroupPurpose)>,
    ),
    StoreError,
> {
    let assignment = sqlx::query(
        "SELECT course_id, audience_kind, revision, lifecycle FROM assignment \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(witness.tenant.as_uuid())
    .bind(witness.assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| StoreError::InvalidRecord("prepared assignment disappeared".to_string()))?;
    let assignment_course: Uuid = assignment.try_get("course_id").map_err(map_sqlx_error)?;
    let assignment_revision: i64 = assignment.try_get("revision").map_err(map_sqlx_error)?;
    let lifecycle: String = assignment.try_get("lifecycle").map_err(map_sqlx_error)?;
    if assignment_course != witness.course.as_uuid()
        || u64::try_from(assignment_revision).ok() != Some(witness.assignment_revision)
        || !witness_lifecycle_matches(&lifecycle, witness.lifecycle)
    {
        return Err(StoreError::InvalidRecord(
            "prepared assignment facts disagree with learner-work witness".to_string(),
        ));
    }
    let member = sqlx::query(
        "SELECT course_membership_id, student_id FROM course_member \
         WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 \
           AND course_membership_id = $4 \
           AND role = 'student' AND status = 'active'",
    )
    .bind(witness.tenant.as_uuid())
    .bind(witness.course.as_uuid())
    .bind(witness.learner.as_uuid())
    .bind(witness.student_membership.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let membership = member
        .map(|row| {
            Ok::<_, StoreError>(ActiveStudentMembership {
                id: CourseMembershipId::from_uuid(
                    row.try_get("course_membership_id")
                        .map_err(map_sqlx_error)?,
                ),
                student: question_model::StudentId::from_uuid(
                    row.try_get("student_id").map_err(map_sqlx_error)?,
                ),
            })
        })
        .transpose()?;
    if membership.as_ref().map(|value| value.id) != Some(witness.student_membership) {
        return Err(StoreError::InvalidRecord(
            "prepared student membership disagrees with learner-work witness".to_string(),
        ));
    }
    match witness.authority {
        super::learner_work_preparation::EntitlementPreparationAuthority::StudentSelfService
        | super::learner_work_preparation::EntitlementPreparationAuthority::StudentSelf
            if witness.actor == witness.learner
                && witness.authority_membership == witness.student_membership => {}
        super::learner_work_preparation::EntitlementPreparationAuthority::DirectInstructor => {
            let actual: Option<Uuid> = sqlx::query_scalar(
                "SELECT course_membership_id FROM course_member \
                 WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 \
                   AND course_membership_id = $4 \
                   AND role = 'instructor' AND status = 'active'",
            )
            .bind(witness.tenant.as_uuid())
            .bind(witness.course.as_uuid())
            .bind(witness.actor.as_uuid())
            .bind(witness.authority_membership.as_uuid())
            .fetch_optional(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
            if actual != Some(witness.authority_membership.as_uuid()) {
                return Err(StoreError::InvalidRecord(
                    "prepared instructor membership disagrees with learner-work witness"
                        .to_string(),
                ));
            }
        }
        _ => {
            return Err(StoreError::InvalidRecord(
                "prepared authority membership disagrees with learner-work witness".to_string(),
            ));
        }
    }
    let audience = load_audience(
        transaction,
        witness.tenant,
        witness.assignment,
        &assignment,
        false,
    )
    .await?;
    if !witness_audience_matches(&audience, witness) {
        return Err(StoreError::InvalidRecord(
            "prepared audience disagrees with learner-work witness".to_string(),
        ));
    }
    let mut groups = load_current_groups(
        transaction,
        witness.tenant,
        witness.course,
        Some(witness.student_membership),
        false,
    )
    .await?;
    groups.sort_by_key(|(group, _)| group.as_uuid());
    if groups.iter().map(|(group, _)| *group).collect::<Vec<_>>() != witness.current_groups {
        return Err(StoreError::InvalidRecord(
            "prepared current groups disagree with learner-work witness".to_string(),
        ));
    }
    Ok((membership, audience, groups))
}

fn witness_lifecycle_matches(value: &str, expected: WitnessAssignmentLifecycle) -> bool {
    matches!(
        (value, expected),
        ("draft", WitnessAssignmentLifecycle::Draft)
            | ("published", WitnessAssignmentLifecycle::Published)
            | ("closed", WitnessAssignmentLifecycle::Closed)
            | ("archived", WitnessAssignmentLifecycle::Archived)
    )
}

fn witness_audience_matches(
    audience: &AssignmentAudience,
    witness: &EntitlementPreparationWitness,
) -> bool {
    match (audience, witness.audience_kind) {
        (AssignmentAudience::CourseWide, WitnessAudienceKind::CourseWide) => {
            witness.audience_groups.is_empty()
        }
        (AssignmentAudience::AnyOfGroups(groups), WitnessAudienceKind::AnyOfGroups) => {
            groups.iter().eq(witness.audience_groups.iter().copied())
        }
        _ => false,
    }
}

pub(super) async fn evaluate_current(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    learner: UserId,
    course: question_model::CourseId,
    assignment: question_model::AssignmentId,
) -> Result<EntitlementDecision, StoreError> {
    evaluate_current_with_locks(transaction, tenant, learner, course, assignment, true, true).await
}

pub(super) async fn evaluate_current_read_only(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    learner: UserId,
    course: question_model::CourseId,
    assignment: question_model::AssignmentId,
) -> Result<EntitlementDecision, StoreError> {
    evaluate_current_with_locks(
        transaction,
        tenant,
        learner,
        course,
        assignment,
        false,
        false,
    )
    .await
}

/// Evaluates S5 current facts under the broker-held course/assignment lock.
/// The 1812 prepare serializes roster and group changes, so this entire read
/// set is intentionally plain; learner runtime locks belong to 1817.
pub(super) async fn evaluate_current_broker_prelocked_current_facts(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    learner: UserId,
    course: question_model::CourseId,
    assignment: question_model::AssignmentId,
) -> Result<EntitlementDecision, StoreError> {
    evaluate_current_with_locks(
        transaction,
        tenant,
        learner,
        course,
        assignment,
        false,
        false,
    )
    .await
}

async fn evaluate_current_with_locks(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    learner: UserId,
    course: question_model::CourseId,
    assignment: question_model::AssignmentId,
    lock_assignment_audience: bool,
    lock_membership_groups: bool,
) -> Result<EntitlementDecision, StoreError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM course WHERE tenant_id = $1 AND course_id = $2)",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if !exists {
        return Ok(EntitlementDecision::Denied(
            domain::entitlement::EntitlementDenial::CourseNotFound,
        ));
    }
    let row = sqlx::query("SELECT course_id, audience_kind FROM assignment WHERE tenant_id = $1 AND assignment_id = $2")
        .bind(tenant.as_uuid()).bind(assignment.as_uuid()).fetch_optional(&mut **transaction).await.map_err(map_sqlx_error)?;
    let Some(row) = row else {
        return Ok(EntitlementDecision::Denied(
            domain::entitlement::EntitlementDenial::AssignmentNotFound,
        ));
    };
    let stored_course: Uuid = row.try_get("course_id").map_err(map_sqlx_error)?;
    if stored_course != course.as_uuid() {
        return Ok(EntitlementDecision::Denied(
            domain::entitlement::EntitlementDenial::AssignmentOutsideCourse,
        ));
    }
    let member = sqlx::query("SELECT course_membership_id, student_id FROM course_member WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 AND role = 'student' AND status = 'active'")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(learner.as_uuid()).fetch_optional(&mut **transaction).await.map_err(map_sqlx_error)?;
    let membership = member
        .map(|member| {
            Ok::<_, StoreError>(ActiveStudentMembership {
                id: CourseMembershipId::from_uuid(
                    member
                        .try_get("course_membership_id")
                        .map_err(map_sqlx_error)?,
                ),
                student: question_model::StudentId::from_uuid(
                    member.try_get("student_id").map_err(map_sqlx_error)?,
                ),
            })
        })
        .transpose()?;
    let audience = load_audience(
        transaction,
        tenant,
        assignment,
        &row,
        lock_assignment_audience,
    )
    .await?;
    let groups = load_current_groups(
        transaction,
        tenant,
        course,
        membership.map(|member| member.id),
        lock_membership_groups,
    )
    .await?;
    Ok(evaluate_assignment_entitlement(EntitlementFacts {
        tenant,
        course,
        assignment,
        learner,
        membership,
        audience,
        current_groups: groups,
    }))
}

/// Resolves a current learner identity and evaluates plain S5 facts under the
/// broker-held course/assignment lock established by the 1812 prepare.
pub(super) async fn evaluate_current_student_broker_prelocked_current_facts(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: question_model::CourseId,
    assignment: question_model::AssignmentId,
    student: question_model::StudentId,
) -> Result<Option<EntitlementDecision>, StoreError> {
    let actor = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM course_member WHERE tenant_id=$1 AND course_id=$2 AND student_id=$3 AND role='student' AND status='active'",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(student.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(actor) = actor else {
        return Ok(None);
    };
    evaluate_current_broker_prelocked_current_facts(
        transaction,
        tenant,
        UserId::from_uuid(actor),
        course,
        assignment,
    )
    .await
    .map(Some)
}

async fn load_audience(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: question_model::AssignmentId,
    row: &sqlx::postgres::PgRow,
    lock: bool,
) -> Result<AssignmentAudience, StoreError> {
    let kind: String = row.try_get("audience_kind").map_err(map_sqlx_error)?;
    match kind.as_str() {
        "course_wide" => Ok(AssignmentAudience::CourseWide),
        "any_of_groups" => {
            let query = if lock {
                "SELECT course_group_id FROM assignment_audience_group \
                 WHERE tenant_id = $1 AND assignment_id = $2 ORDER BY course_group_id FOR UPDATE"
            } else {
                "SELECT course_group_id FROM assignment_audience_group \
                 WHERE tenant_id = $1 AND assignment_id = $2 ORDER BY course_group_id"
            };
            let groups = sqlx::query_scalar::<_, Uuid>(query)
                .bind(tenant.as_uuid())
                .bind(assignment.as_uuid())
                .fetch_all(&mut **transaction)
                .await
                .map_err(map_sqlx_error)?
                .into_iter()
                .map(CourseGroupId::from_uuid)
                .collect();
            AssignmentAudience::any_of_groups(groups).map_err(|error| {
                StoreError::Unavailable(format!("stored assignment audience is invalid: {error:?}"))
            })
        }
        _ => Err(StoreError::Unavailable(
            "stored assignment audience kind is invalid".to_string(),
        )),
    }
}

async fn load_current_groups(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: question_model::CourseId,
    membership: Option<CourseMembershipId>,
    lock: bool,
) -> Result<Vec<(CourseGroupId, CourseGroupPurpose)>, StoreError> {
    let Some(membership) = membership else {
        return Ok(Vec::new());
    };
    let query = if lock {
        "SELECT groups.course_group_id, groups.purpose FROM course_group_member AS member \
         JOIN course_group AS groups \
           ON groups.tenant_id = member.tenant_id \
          AND groups.course_id = member.course_id \
          AND groups.course_group_id = member.course_group_id \
         WHERE member.tenant_id = $1 AND member.course_id = $2 \
         AND member.course_membership_id = $3 FOR UPDATE OF member, groups"
    } else {
        "SELECT groups.course_group_id, groups.purpose FROM course_group_member AS member \
         JOIN course_group AS groups \
           ON groups.tenant_id = member.tenant_id \
          AND groups.course_id = member.course_id \
          AND groups.course_group_id = member.course_group_id \
         WHERE member.tenant_id = $1 AND member.course_id = $2 \
         AND member.course_membership_id = $3"
    };
    let rows = sqlx::query(query)
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(membership.as_uuid())
        .fetch_all(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    rows.iter()
        .map(|row| {
            Ok((
                CourseGroupId::from_uuid(row.try_get("course_group_id").map_err(map_sqlx_error)?),
                decode_group_purpose(row.try_get("purpose").map_err(map_sqlx_error)?)?,
            ))
        })
        .collect()
}

fn decode_group_purpose(value: String) -> Result<CourseGroupPurpose, StoreError> {
    match value.as_str() {
        "section" => Ok(CourseGroupPurpose::Section),
        "lab" => Ok(CourseGroupPurpose::Lab),
        "cohort" => Ok(CourseGroupPurpose::Cohort),
        "accommodation" => Ok(CourseGroupPurpose::Accommodation),
        "work" => Ok(CourseGroupPurpose::Work),
        _ => Err(StoreError::Unavailable(
            "stored group purpose is invalid".to_string(),
        )),
    }
}

fn decode_materialization_authority(
    row: &sqlx::postgres::PgRow,
) -> Result<MaterializationAuthority, StoreError> {
    let actor: Option<Uuid> = row
        .try_get("materialized_by_user_id")
        .map_err(map_sqlx_error)?;
    let rule: Option<String> = row
        .try_get("materialization_rule")
        .map_err(map_sqlx_error)?;
    match (actor, rule.as_deref()) {
        (Some(actor), None) => Ok(MaterializationAuthority::Actor(UserId::from_uuid(actor))),
        (None, Some("imported_grade")) => Ok(MaterializationAuthority::Rule(
            question_model::MaterializationRule::ImportedGrade,
        )),
        (None, Some("automated_grader")) => Ok(MaterializationAuthority::Rule(
            question_model::MaterializationRule::AutomatedGrader,
        )),
        _ => Err(StoreError::Unavailable(
            "stored materialization authority is invalid".to_string(),
        )),
    }
}

pub(super) async fn load_existing_receipt(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: question_model::AssignmentId,
    student: question_model::StudentId,
) -> Result<
    Option<(
        AssignmentEnrollment,
        StudentAssignmentSummary,
        EntitlementMaterialization,
        MaterializationDisposition,
    )>,
    StoreError,
> {
    let row = sqlx::query("SELECT enrollment_id, course_membership_id, floor(extract(epoch FROM materialized_at)*1000)::bigint AS occurred, materialization_purpose, materialized_by_user_id, materialization_rule, evaluator_version FROM enrollment WHERE tenant_id=$1 AND assignment_id=$2 AND student_id=$3 FOR UPDATE")
        .bind(tenant.as_uuid()).bind(assignment.as_uuid()).bind(student.as_uuid()).fetch_optional(&mut **transaction).await.map_err(map_sqlx_error)?;
    let Some(row) = row else { return Ok(None) };
    let id = EnrollmentId::from_uuid(row.try_get("enrollment_id").map_err(map_sqlx_error)?);
    let enrollment = load_postgres_enrollment(transaction, tenant, id).await?;
    let enrolled_user = enrollment.user;
    let summary_row = sqlx::query("SELECT tenant_id, enrollment_id, current_score, best_score, latest_score, completed_run_count, total_question_attempts, floor(extract(epoch FROM last_activity_at) * 1000)::bigint AS last_activity_at_millis FROM student_assignment_summary WHERE tenant_id=$1 AND enrollment_id=$2").bind(tenant.as_uuid()).bind(id.as_uuid()).fetch_optional(&mut **transaction).await.map_err(map_sqlx_error)?.ok_or_else(|| StoreError::Unavailable("entitlement receipt has no summary".to_string()))?;
    let summary = decode_summary_row(&summary_row)?;
    let basis_row = sqlx::query("SELECT scope_kind, course_group_id, course_group_purpose FROM enrollment_entitlement_basis_receipt WHERE tenant_id=$1 AND enrollment_id=$2").bind(tenant.as_uuid()).bind(id.as_uuid()).fetch_optional(&mut **transaction).await.map_err(map_sqlx_error)?.ok_or_else(|| StoreError::Unavailable("entitlement receipt has no basis".to_string()))?;
    let basis = match basis_row
        .try_get::<String, _>("scope_kind")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "course_wide" => MaterializationBasis::CourseWide,
        "group_audience" => MaterializationBasis::GroupAudience {
            group: CourseGroupId::from_uuid(
                basis_row
                    .try_get("course_group_id")
                    .map_err(map_sqlx_error)?,
            ),
            purpose: decode_group_purpose(
                basis_row
                    .try_get("course_group_purpose")
                    .map_err(map_sqlx_error)?,
            )?,
        },
        _ => {
            return Err(StoreError::Unavailable(
                "stored receipt basis is invalid".to_string(),
            ));
        }
    };
    let purpose = match row
        .try_get::<String, _>("materialization_purpose")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "start_run" => question_model::EntitlementPurpose::StartRun,
        "grade_bearing_action" => question_model::EntitlementPurpose::GradeBearingAction,
        "instructor_issue" => question_model::EntitlementPurpose::InstructorIssue,
        _ => {
            return Err(StoreError::Unavailable(
                "stored materialization purpose is invalid".to_string(),
            ));
        }
    };
    Ok(Some((
        enrollment,
        summary,
        EntitlementMaterialization {
            enrollment: id,
            membership: CourseMembershipId::from_uuid(
                row.try_get("course_membership_id")
                    .map_err(map_sqlx_error)?,
            ),
            user: enrolled_user,
            occurred_at: ActivityTimestamp::from_unix_millis(
                row.try_get("occurred").map_err(map_sqlx_error)?,
            ),
            purpose,
            authority: decode_materialization_authority(&row)?,
            basis,
            evaluator_version: EvaluatorVersion(
                u16::try_from(
                    row.try_get::<i32, _>("evaluator_version")
                        .map_err(map_sqlx_error)?,
                )
                .map_err(|_| {
                    StoreError::Unavailable("stored evaluator version is invalid".to_string())
                })?,
            ),
        },
        MaterializationDisposition::Existing,
    )))
}

async fn insert_receipt(
    transaction: &mut Transaction<'_, Postgres>,
    grant: &domain::entitlement::EntitlementGrant,
    command: MaterializeAssignmentEntitlementCommand,
    now: question_model::ActivityTimestamp,
) -> Result<
    (
        AssignmentEnrollment,
        StudentAssignmentSummary,
        EntitlementMaterialization,
        MaterializationDisposition,
    ),
    StoreError,
> {
    let id = EnrollmentId::from_uuid(random_uuid()?);
    let purpose = match command.purpose() {
        question_model::EntitlementPurpose::StartRun => "start_run",
        question_model::EntitlementPurpose::GradeBearingAction => "grade_bearing_action",
        question_model::EntitlementPurpose::InstructorIssue => "instructor_issue",
    };
    let (actor, rule) = match command.authority() {
        MaterializationAuthority::Actor(actor) => (Some(actor.as_uuid()), None),
        MaterializationAuthority::Rule(question_model::MaterializationRule::ImportedGrade) => {
            (None, Some("imported_grade"))
        }
        MaterializationAuthority::Rule(question_model::MaterializationRule::AutomatedGrader) => {
            (None, Some("automated_grader"))
        }
    };
    let (kind, group, group_purpose) = match grant.basis() {
        MaterializationBasis::CourseWide => ("course_wide", None, None),
        MaterializationBasis::GroupAudience { group, purpose } => (
            "group_audience",
            Some(group.as_uuid()),
            Some(match purpose {
                CourseGroupPurpose::Section => "section",
                CourseGroupPurpose::Lab => "lab",
                CourseGroupPurpose::Cohort => "cohort",
                _ => return Err(StoreError::Conflict),
            }),
        ),
    };
    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO enrollment \
         (tenant_id, enrollment_id, assignment_id, course_id, course_membership_id, \
          user_id, student_id, materialized_at, materialization_purpose, \
          materialized_by_user_id, materialization_rule, evaluator_version) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, \
                 to_timestamp($8::double precision / 1000), $9, $10, $11, $12) \
         ON CONFLICT (tenant_id, assignment_id, student_id) DO NOTHING \
         RETURNING enrollment_id",
    )
    .bind(grant.tenant().as_uuid())
    .bind(id.as_uuid())
    .bind(grant.assignment().as_uuid())
    .bind(grant.course().as_uuid())
    .bind(grant.membership().as_uuid())
    .bind(grant.learner().as_uuid())
    .bind(grant.student().as_uuid())
    .bind(now.as_unix_millis())
    .bind(purpose)
    .bind(actor)
    .bind(rule)
    .bind(i32::from(EvaluatorVersion::INITIAL.0))
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(materialized_id) = inserted.map(EnrollmentId::from_uuid) else {
        return load_existing_receipt(
            transaction,
            grant.tenant(),
            grant.assignment(),
            grant.student(),
        )
        .await?
        .ok_or(StoreError::Conflict);
    };
    sqlx::query(
        "INSERT INTO enrollment_entitlement_basis_receipt \
         (tenant_id, enrollment_id, scope_receipt_id, scope_kind, course_id, \
          course_group_id, course_group_purpose) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(grant.tenant().as_uuid())
    .bind(materialized_id.as_uuid())
    .bind(random_uuid()?)
    .bind(kind)
    .bind(grant.course().as_uuid())
    .bind(group)
    .bind(group_purpose)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    for (scope_group, scope_purpose) in grant.applicable_policy_scopes().iter() {
        let scope_purpose = match scope_purpose {
            CourseGroupPurpose::Section => "section",
            CourseGroupPurpose::Lab => "lab",
            CourseGroupPurpose::Cohort => "cohort",
            CourseGroupPurpose::Accommodation => "accommodation",
            CourseGroupPurpose::Work => continue,
        };
        sqlx::query(
            "INSERT INTO enrollment_applicable_policy_scope_receipt \
             (tenant_id, enrollment_id, course_id, course_group_id, course_group_purpose) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(grant.tenant().as_uuid())
        .bind(materialized_id.as_uuid())
        .bind(grant.course().as_uuid())
        .bind(scope_group.as_uuid())
        .bind(scope_purpose)
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    sqlx::query(
        "UPDATE enrollment SET entitlement_receipts_sealed_at = transaction_timestamp() \
         WHERE tenant_id = $1 AND enrollment_id = $2 \
           AND entitlement_receipts_sealed_at IS NULL",
    )
    .bind(grant.tenant().as_uuid())
    .bind(materialized_id.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let summary = StudentAssignmentSummary::empty(grant.tenant(), materialized_id);
    sqlx::query("INSERT INTO student_assignment_summary (tenant_id, enrollment_id, current_score, best_score, latest_score, completed_run_count, total_question_attempts, last_activity_at) VALUES ($1, $2, NULL, NULL, NULL, 0, 0, NULL)").bind(grant.tenant().as_uuid()).bind(materialized_id.as_uuid()).execute(&mut **transaction).await.map_err(map_sqlx_error)?;
    let enrollment = load_postgres_enrollment(transaction, grant.tenant(), materialized_id).await?;
    Ok((
        enrollment,
        summary,
        EntitlementMaterialization {
            enrollment: materialized_id,
            membership: grant.membership(),
            user: grant.learner(),
            occurred_at: now,
            purpose: command.purpose(),
            authority: command.authority(),
            basis: grant.basis(),
            evaluator_version: EvaluatorVersion::INITIAL,
        },
        MaterializationDisposition::Created,
    ))
}

fn random_uuid() -> Result<Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("entitlement ID randomness unavailable: {error}"))
    })
}
