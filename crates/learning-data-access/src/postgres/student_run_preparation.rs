//! Transaction-local hydration for Student-owned attempt issuance.

use domain::entitlement::{EntitlementDecision, EntitlementFacts, evaluate_assignment_entitlement};
use question_model::{AssignmentEnrollment, RunId, TenantId, UserId};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::entitlement::{hydrate_assignment_from_witness, hydrate_entitlement_witness_sources};
use super::learner_work_preparation::{
    StudentRunPreparationWitness, prepare_student_run_work as prepare_student_run_work_witness,
};
use super::{decode_payload_row, load_postgres_enrollment, map_sqlx_error};
use crate::{LearnerWorkRoutingBinding, StoreError};

/// Broker-authorized, fully hydrated source aggregate for attempt issuance.
///
/// This value cannot escape the PostgreSQL transaction module. It combines a
/// strictly decoded structural witness with ordinary reads performed while
/// the broker's course/assignment/enrollment/run locks remain held.
pub(super) struct PreparedStudentRunWork {
    witness: StudentRunPreparationWitness,
    assignment: crate::AssignmentRecord,
    enrollment: AssignmentEnrollment,
    run: question_model::AssignmentRun,
    grant: domain::entitlement::EntitlementGrant,
}

impl PreparedStudentRunWork {
    pub(super) fn assignment(&self) -> &crate::AssignmentRecord {
        &self.assignment
    }

    pub(super) fn enrollment(&self) -> &AssignmentEnrollment {
        &self.enrollment
    }

    pub(super) fn run(&self) -> &question_model::AssignmentRun {
        &self.run
    }

    pub(super) fn grant(&self) -> &domain::entitlement::EntitlementGrant {
        &self.grant
    }

    pub(super) fn assignment_revision(&self) -> u64 {
        self.witness.source.assignment_revision
    }
}

/// Executes the run-specific learner-work preparation and hydrates every
/// source fact needed by issuance under the broker-retained locks.
pub(super) async fn prepare_student_run_work(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    binding: LearnerWorkRoutingBinding,
    actor: UserId,
    run: RunId,
) -> Result<PreparedStudentRunWork, StoreError> {
    let witness =
        prepare_student_run_work_witness(transaction, tenant, binding, actor, run).await?;
    let (membership, audience, groups) =
        hydrate_entitlement_witness_sources(transaction, &witness.source).await?;
    let decision = evaluate_assignment_entitlement(EntitlementFacts {
        tenant,
        course: binding.course,
        assignment: binding.assignment,
        learner: actor,
        membership,
        audience,
        current_groups: groups,
    });
    let EntitlementDecision::Granted(grant) = decision else {
        return Err(StoreError::NotFound);
    };
    if grant.membership() != witness.source.student_membership
        || grant.learner() != actor
        || grant.course() != binding.course
        || grant.assignment() != binding.assignment
    {
        return Err(StoreError::InvalidRecord(
            "prepared entitlement grant disagrees with learner-work witness".to_string(),
        ));
    }
    let assignment = hydrate_assignment_from_witness(transaction, &witness.source).await?;
    let enrollment_id = witness
        .source
        .existing_enrollment
        .ok_or_else(|| StoreError::InvalidRecord("prepared run lacks enrollment".to_string()))?;
    let enrollment_row = sqlx::query(
        "SELECT course_id, course_membership_id FROM enrollment \
         WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment_id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| StoreError::InvalidRecord("prepared enrollment disappeared".to_string()))?;
    let enrollment_course: Uuid = enrollment_row
        .try_get("course_id")
        .map_err(map_sqlx_error)?;
    let enrollment_membership: Uuid = enrollment_row
        .try_get("course_membership_id")
        .map_err(map_sqlx_error)?;
    let enrollment = load_postgres_enrollment(transaction, tenant, enrollment_id).await?;
    if enrollment.tenant != tenant
        || enrollment.assignment != binding.assignment
        || enrollment.user != actor
        || enrollment.student != grant.student()
        || enrollment_course != binding.course.as_uuid()
        || enrollment_membership != witness.source.student_membership.as_uuid()
    {
        return Err(StoreError::InvalidRecord(
            "prepared enrollment disagrees with learner-work witness".to_string(),
        ));
    }
    let run_row = sqlx::query(
        "SELECT payload, payload_sha256 FROM assignment_run \
         WHERE tenant_id = $1 AND run_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(run.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| StoreError::InvalidRecord("prepared run disappeared".to_string()))?;
    let hydrated_run: question_model::AssignmentRun = decode_payload_row(&run_row)?;
    if hydrated_run.id != witness.run
        || hydrated_run.tenant != tenant
        || hydrated_run.enrollment != enrollment.id
        || witness.locked_summary_enrollments.as_slice() != [enrollment.id]
    {
        return Err(StoreError::InvalidRecord(
            "prepared run disagrees with learner-work witness".to_string(),
        ));
    }
    Ok(PreparedStudentRunWork {
        witness,
        assignment,
        enrollment,
        run: hydrated_run,
        grant,
    })
}
