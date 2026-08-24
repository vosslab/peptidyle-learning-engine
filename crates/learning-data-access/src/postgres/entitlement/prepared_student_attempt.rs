//! Exact source hydration under broker-retained Student attempt locks.

use domain::entitlement::{EntitlementDecision, EntitlementFacts, evaluate_assignment_entitlement};
use question_model::{AssignmentRun, StudentAssignmentSummary};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::{hydrate_assignment_from_witness, hydrate_entitlement_witness_sources};
use crate::postgres::learner_work_preparation::StudentAttemptPreparationWitness;
use crate::postgres::{
    decode_current_attempt_row, decode_presentation_binding_row, decode_summary_row, map_sqlx_error,
};
use crate::{
    AssignmentEnrollment, AssignmentRecord, IssuedFlatGradingContract, IssuedQtiGradingContractV1,
    IssuedWebworkGradingContract, PresentationCapability, ReceiptPresentationSnapshot, StoreError,
    WebworkGradeReplayStateV1,
};

/// One exact aggregate hydrated while the broker's locks remain retained.
pub(in crate::postgres) struct PreparedStudentAttemptWork {
    pub(in crate::postgres) assignment: AssignmentRecord,
    pub(in crate::postgres) enrollment: AssignmentEnrollment,
    pub(in crate::postgres) run: AssignmentRun,
    pub(in crate::postgres) attempt: question_model::QuestionAttempt,
    pub(in crate::postgres) issued_question_snapshot: crate::IssuedQuestionSnapshotV1,
    pub(in crate::postgres) summary: StudentAssignmentSummary,
    pub(in crate::postgres) presentation_binding: Option<question_model::PresentationBindingV1>,
    pub(in crate::postgres) presentation_capability: PresentationCapability,
    pub(in crate::postgres) presentation: Option<ReceiptPresentationSnapshot>,
    pub(in crate::postgres) grading_envelope: Option<question_model::QuestionEnvelope>,
    pub(in crate::postgres) flat_grading: Option<IssuedFlatGradingContract>,
    pub(in crate::postgres) webwork_grading: Option<IssuedWebworkGradingContract>,
    pub(in crate::postgres) issued_qti_grading: Option<IssuedQtiGradingContractV1>,
    pub(in crate::postgres) webwork_replay: Option<WebworkGradeReplayStateV1>,
}

/// Hydrates only rows named by the strictly decoded witness. All queries are
/// parameterized and plain reads under the already-retained broker locks
/// (ASVS 1.2.4, 2.3.3, 2.3.4, 8.2.2).
pub(in crate::postgres) async fn hydrate_prepared_student_attempt_work(
    transaction: &mut Transaction<'_, Postgres>,
    witness: &StudentAttemptPreparationWitness,
) -> Result<PreparedStudentAttemptWork, StoreError> {
    let source = &witness.source;
    let assignment = hydrate_assignment_from_witness(transaction, source).await?;
    let (membership, audience, groups) =
        hydrate_entitlement_witness_sources(transaction, source).await?;
    let EntitlementDecision::Granted(grant) = evaluate_assignment_entitlement(EntitlementFacts {
        tenant: source.tenant,
        course: source.course,
        assignment: source.assignment,
        learner: source.learner,
        membership,
        audience,
        current_groups: groups,
    }) else {
        return Err(StoreError::NotFound);
    };
    if grant.membership() != source.student_membership {
        return Err(invalid("entitlement grant"));
    }

    let enrollment_id = source
        .existing_enrollment
        .ok_or_else(|| invalid("enrollment"))?;
    let enrollment_row = sqlx::query(
        "SELECT enrollment_id, tenant_id, assignment_id, course_id, course_membership_id, \
                user_id, student_id, \
                floor(extract(epoch FROM first_completed_at) * 1000)::bigint \
                    AS first_completed_at_millis, \
                current_grade_run_id, best_grade_run_id \
           FROM enrollment WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(source.tenant.as_uuid())
    .bind(enrollment_id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| invalid("enrollment"))?;
    let enrollment = crate::postgres::decode_postgres_enrollment_row(&enrollment_row)?;
    if enrollment.tenant != source.tenant
        || enrollment.assignment != source.assignment
        || enrollment.user != source.learner
        || enrollment.student != grant.student()
        || enrollment_row
            .try_get::<Uuid, _>("course_id")
            .map_err(map_sqlx_error)?
            != source.course.as_uuid()
        || enrollment_row
            .try_get::<Uuid, _>("course_membership_id")
            .map_err(map_sqlx_error)?
            != source.student_membership.as_uuid()
    {
        return Err(invalid("enrollment"));
    }

    let run_row = sqlx::query(
        "SELECT payload, payload_sha256 FROM assignment_run \
         WHERE tenant_id = $1 AND run_id = $2",
    )
    .bind(source.tenant.as_uuid())
    .bind(witness.run.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| invalid("run"))?;
    let run: AssignmentRun = crate::postgres::decode_payload_row(&run_row)?;
    if run.tenant != source.tenant || run.id != witness.run || run.enrollment != enrollment.id {
        return Err(invalid("run"));
    }

    let attempt_row = sqlx::query(
        "SELECT payload, payload_sha256, attempt_status AS current_attempt_status, \
                floor(extract(epoch FROM submitted_at) * 1000)::bigint AS current_submitted_at, \
                floor(extract(epoch FROM timing.effective_deadline) * 1000)::bigint \
                    AS current_deadline_at, \
                presentation_descriptor_version, presentation_nonce, presentation_digest, \
                presentation_capability, presentation_payload, presentation_payload_sha256, \
                grading_envelope_payload, grading_envelope_payload_sha256, \
                flat_grading_required, flat_grading_payload, flat_grading_payload_sha256, \
                webwork_grading_required, webwork_grading_payload, \
                webwork_grading_payload_sha256, \
                qti_grading_required, issued_qti_grading_payload, \
                issued_qti_grading_payload_sha256, \
                issued_question_snapshot_payload, issued_question_snapshot_payload_sha256 \
           FROM question_attempt AS attempt \
           LEFT JOIN attempt_effective_policy_current AS current_effect \
             ON current_effect.tenant_id=attempt.tenant_id \
            AND current_effect.attempt_id=attempt.attempt_id \
           LEFT JOIN attempt_effective_policy_receipt AS timing \
             ON timing.tenant_id=current_effect.tenant_id \
            AND timing.attempt_id=current_effect.attempt_id \
            AND timing.receipt_generation=current_effect.receipt_generation \
          WHERE attempt.tenant_id=$1 AND attempt.attempt_id=$2 \
          ORDER BY attempt.occurred_at LIMIT 1",
    )
    .bind(source.tenant.as_uuid())
    .bind(witness.attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| invalid("attempt"))?;
    let attempt = decode_current_attempt_row(&attempt_row)?;
    if attempt.tenant != source.tenant
        || attempt.id != witness.attempt
        || attempt.run != witness.run
        || attempt.status != witness.attempt_status
    {
        return Err(invalid("attempt"));
    }
    let run_item = crate::postgres::run_lifecycle::load_assignment_run_items(
        transaction,
        source.tenant,
        run.id,
    )
    .await?
    .into_iter()
    .find(|item| item.issued_position == attempt.assignment_position)
    .ok_or_else(|| invalid("run item"))?;
    if run_item.reference.problem != attempt.problem
        || run_item.reference.version != attempt.question_version
    {
        return Err(invalid("run item"));
    }
    let presentation_binding = decode_presentation_binding_row(&attempt_row)?;
    let capability =
        crate::postgres::runs::attempt_issuance::presentation_capability_from_row(&attempt_row)?;
    let stored_presentation =
        crate::postgres::runs::attempt_issuance::decode_attempt_presentation_snapshot(
            &attempt_row,
            capability,
        )?;
    let grading_envelope =
        crate::postgres::runs::attempt_issuance::decode_attempt_grading_envelope(
            &attempt_row,
            capability,
        )?;
    let presentation = crate::validate_issued_presentation(
        capability,
        &attempt,
        presentation_binding,
        stored_presentation.as_ref(),
        grading_envelope.as_ref(),
    )?;
    let flat_grading = crate::postgres::runs::attempt_issuance::validate_attempt_flat_grading(
        &attempt_row,
        &attempt,
    )?;
    let webwork_grading =
        crate::postgres::runs::attempt_issuance::validate_attempt_webwork_grading(
            &attempt_row,
            &attempt,
        )?;
    let issued_question_snapshot =
        crate::postgres::runs::attempt_issuance::decode_issued_question_snapshot(&attempt_row)?;
    issued_question_snapshot.validate_for_attempt(attempt.problem, attempt.question_version)?;
    issued_question_snapshot.validate_for_issuance_context(
        crate::postgres::runs::attempt_issuance::flat_grading_capability_from_row(&attempt_row)?,
        crate::postgres::runs::attempt_issuance::webwork_grading_capability_from_row(&attempt_row)?,
        crate::postgres::runs::attempt_issuance::qti_grading_capability_from_row(&attempt_row)?,
        presentation.as_ref(),
    )?;
    let issued_qti_grading = crate::postgres::runs::attempt_issuance::validate_attempt_qti_grading(
        &attempt_row,
        &attempt,
        &issued_question_snapshot,
    )?;
    if flat_grading
        .as_ref()
        .is_some_and(|contract| contract.question() != issued_question_snapshot.question())
        || webwork_grading
            .as_ref()
            .is_some_and(|contract| contract.question() != issued_question_snapshot.question())
    {
        return Err(StoreError::Unavailable(
            "specialized grading authority disagrees with issued question snapshot".to_string(),
        ));
    }
    let webwork_replay = load_webwork_replay(transaction, source.tenant, &attempt).await?;
    validate_private_shape(
        capability,
        &attempt,
        presentation_binding,
        &webwork_grading,
        &webwork_replay,
    )?;

    let summary_row = sqlx::query(
        "SELECT tenant_id, enrollment_id, current_score, best_score, latest_score, \
                completed_run_count, total_question_attempts, \
                floor(extract(epoch FROM last_activity_at) * 1000)::bigint \
                    AS last_activity_at_millis \
           FROM student_assignment_summary WHERE tenant_id=$1 AND enrollment_id=$2",
    )
    .bind(source.tenant.as_uuid())
    .bind(enrollment.id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| invalid("summary"))?;
    let summary = decode_summary_row(&summary_row)?;
    if summary.tenant != source.tenant
        || summary.enrollment != enrollment.id
        || witness.locked_summary_enrollments.as_slice() != [enrollment.id]
    {
        return Err(invalid("summary"));
    }

    Ok(PreparedStudentAttemptWork {
        assignment,
        enrollment,
        run,
        attempt,
        issued_question_snapshot,
        summary,
        presentation_binding,
        presentation_capability: capability,
        presentation,
        grading_envelope,
        flat_grading,
        webwork_grading,
        issued_qti_grading,
        webwork_replay,
    })
}

async fn load_webwork_replay(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: question_model::TenantId,
    attempt: &question_model::QuestionAttempt,
) -> Result<Option<WebworkGradeReplayStateV1>, StoreError> {
    let row = sqlx::query(
        "SELECT replay.problem_id, replay.version_id, replay.source_object_id, \
                replay.source_sha256, replay.seed::text AS seed, replay.renderer_id, \
                replay.renderer_version, replay.presentation_digest AS replay_presentation_digest, \
                replay.mapping, replay.mapping_sha256 \
           FROM webwork_grade_replay_state AS replay \
          WHERE replay.tenant_id=$1 AND replay.attempt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    row.as_ref()
        .map(crate::postgres::runs::decode_webwork_replay_state)
        .transpose()
}

fn validate_private_shape(
    capability: PresentationCapability,
    attempt: &question_model::QuestionAttempt,
    binding: Option<question_model::PresentationBindingV1>,
    webwork_grading: &Option<IssuedWebworkGradingContract>,
    replay: &Option<WebworkGradeReplayStateV1>,
) -> Result<(), StoreError> {
    let webwork_required = matches!(
        attempt.issued_capability,
        question_model::IssuedAttemptCapabilityV1::WebworkPresentation
    );
    if webwork_required != (webwork_grading.is_some() && replay.is_some()) {
        return Err(StoreError::Unavailable(
            "stored WebWork submission authority is incomplete".to_string(),
        ));
    }
    if let Some(replay) = replay {
        crate::validate_persisted_webwork_replay_state(attempt, binding, replay)?;
    }
    if capability.requires_snapshot() != binding.is_some() {
        return Err(StoreError::Unavailable(
            "stored presentation binding capability disagrees".to_string(),
        ));
    }
    Ok(())
}

fn invalid(name: &'static str) -> StoreError {
    StoreError::InvalidRecord(format!(
        "prepared Student attempt {name} disagrees with its learner-work witness"
    ))
}
