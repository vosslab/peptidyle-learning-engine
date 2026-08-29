//! PostgreSQL implementation of the one audited Student-work detail read.
//!
//! The SQL broker is the only private-response reader. It resolves the public
//! composite, validates its SQL-owned immutable witness, appends both audit
//! facts, and returns bounded canonical evidence inside the caller's open
//! transaction. Rust verifies the typed receipt/presentation/response before
//! committing, so any later decoding failure rolls the paired audit facts back.

use async_trait::async_trait;
use domain::disclosure_policy::project_inspected_student_score_feedback;
use objects::Sha256Digest;
use question_model::presentation::PresentationDigestV1;
use question_model::{
    ActivityTimestamp, AttemptStatus, QuestionAttempt, QuestionAttemptId, ScoringGeneration,
    ScoringStatus, StudentResponse, TeachingDisplayLabel, presentation,
};
use serde_json::Value;
use sqlx::types::{Json, Uuid};
use sqlx::{Row, postgres::PgRow};

use super::{PostgresStore, SessionTokenHash, StoreError, TenantContext, map_sqlx_error};
use crate::{
    InspectStudentWorkRequest, InspectedStudentSubmissionV1, InspectedStudentWorkDetailV1,
    InspectedSubmissionEvidenceV1, ReceiptPresentationSnapshot, StudentWorkInspectionStore,
    canonical_student_response_json,
};

#[async_trait]
impl StudentWorkInspectionStore for PostgresStore {
    async fn inspect_student_work(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: InspectStudentWorkRequest,
    ) -> Result<InspectedStudentWorkDetailV1, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant_writable_snapshot(context).await?;
        let rows = sqlx::query(
            "SELECT attempt_id, assignment_position, submitted_at_millis, \
                    response_canonical_json, response_sha256, canonical_json_version, \
                    receipt_attempt_canonical_json, receipt_attempt_payload, \
                    receipt_attempt_payload_sha256, presentation_canonical_json, \
                    presentation_payload, presentation_payload_sha256, presentation_required, \
                    issued_presentation_digest, presentation_capability, scoring_generation, \
                    scoring_status, score_visible, correctness_visible, \
                    student_display_label, assignment_title \
             FROM public.ple_inspect_student_work_v1($1,$2,$3,$4,$5,$6)",
        )
        .bind(tenant.as_uuid())
        .bind(session.to_string())
        .bind(i32::try_from(request.course.number()).map_err(|_| StoreError::NotFound)?)
        .bind(i32::try_from(request.membership.number()).map_err(|_| StoreError::NotFound)?)
        .bind(i32::try_from(request.assignment.number()).map_err(|_| StoreError::NotFound)?)
        .bind(i32::try_from(request.run.number()).map_err(|_| StoreError::NotFound)?)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if rows.is_empty() {
            return Err(StoreError::NotFound);
        }

        let (student_display_label, assignment_title) = decode_inspection_safe_labels(&rows)?;
        let submissions = rows
            .iter()
            .map(decode_inspected_submission)
            .collect::<Result<Vec<_>, StoreError>>()?;
        validate_inspection_return_context(&request)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(InspectedStudentWorkDetailV1 {
            course: request.course,
            membership: request.membership,
            assignment: request.assignment,
            run: request.run,
            student_display_label,
            assignment_title,
            submissions,
            return_context: request.return_context,
        })
    }
}

fn decode_inspection_safe_labels(
    rows: &[PgRow],
) -> Result<(TeachingDisplayLabel, String), StoreError> {
    let first = rows.first().ok_or_else(inspection_evidence_unavailable)?;
    let student_display_label = decode_student_display_label(first)?;
    let assignment_title = decode_assignment_title(first)?;
    for row in &rows[1..] {
        if decode_student_display_label(row)? != student_display_label
            || decode_assignment_title(row)? != assignment_title
        {
            return Err(inspection_evidence_unavailable());
        }
    }
    Ok((student_display_label, assignment_title))
}

fn decode_student_display_label(row: &PgRow) -> Result<TeachingDisplayLabel, StoreError> {
    TeachingDisplayLabel::try_from(
        row.try_get::<String, _>("student_display_label")
            .map_err(|_| inspection_evidence_unavailable())?,
    )
    .map_err(|_| inspection_evidence_unavailable())
}

fn decode_assignment_title(row: &PgRow) -> Result<String, StoreError> {
    let title: String = row
        .try_get("assignment_title")
        .map_err(|_| inspection_evidence_unavailable())?;
    if title.trim().is_empty() || title.trim() != title || title.chars().count() > 200 {
        return Err(inspection_evidence_unavailable());
    }
    Ok(title)
}

fn decode_inspected_submission(row: &PgRow) -> Result<InspectedStudentSubmissionV1, StoreError> {
    let attempt = QuestionAttemptId::from_uuid(
        row.try_get::<Uuid, _>("attempt_id")
            .map_err(map_sqlx_error)?,
    );
    let assignment_position = u32::try_from(
        row.try_get::<i32, _>("assignment_position")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| inspection_evidence_unavailable())?;
    let submitted_at = ActivityTimestamp::from_unix_millis(
        row.try_get("submitted_at_millis").map_err(map_sqlx_error)?,
    );
    let response_source: String = row
        .try_get("response_canonical_json")
        .map_err(map_sqlx_error)?;
    let response_sha256: String = row.try_get("response_sha256").map_err(map_sqlx_error)?;
    let response: StudentResponse =
        serde_json::from_str(&response_source).map_err(|_| inspection_evidence_unavailable())?;
    if canonical_student_response_json(&response)? != response_source
        || Sha256Digest::compute(response_source.as_bytes()).to_string() != response_sha256
    {
        return Err(inspection_evidence_unavailable());
    }

    let receipt_attempt: QuestionAttempt = decode_canonical_json(
        row,
        "receipt_attempt_canonical_json",
        "receipt_attempt_payload",
        "receipt_attempt_payload_sha256",
    )?;
    if receipt_attempt.id != attempt
        || receipt_attempt.response.is_some()
        || receipt_attempt.assignment_position != assignment_position
        || receipt_attempt.timer.submitted_at != Some(submitted_at)
        || receipt_attempt.status != AttemptStatus::Submitted
    {
        return Err(inspection_evidence_unavailable());
    }

    let presentation_required: bool = row
        .try_get("presentation_required")
        .map_err(map_sqlx_error)?;
    let capability: String = row
        .try_get("presentation_capability")
        .map_err(map_sqlx_error)?;
    let evidence_and_response = if presentation_required {
        if capability != "envelope_v1" {
            return Err(inspection_evidence_unavailable());
        }
        let snapshot: ReceiptPresentationSnapshot = decode_canonical_json(
            row,
            "presentation_canonical_json",
            "presentation_payload",
            "presentation_payload_sha256",
        )?;
        let digest_bytes: Vec<u8> = row
            .try_get("issued_presentation_digest")
            .map_err(map_sqlx_error)?;
        let digest: [u8; 32] = digest_bytes
            .try_into()
            .map_err(|_| inspection_evidence_unavailable())?;
        let digest = PresentationDigestV1::from_bytes(digest);
        let presentation = presentation::rebuild_public_presentation_v1(
            &snapshot.envelope,
            &snapshot.asset_bindings,
        )
        .map_err(|_| inspection_evidence_unavailable())?;
        if presentation.digest != digest {
            return Err(inspection_evidence_unavailable());
        }
        (
            InspectedSubmissionEvidenceV1::IssuedPresentation {
                presentation: Box::new(snapshot),
                issued_presentation_digest: digest,
            },
            presentation::project_rendered_response_for_inspection_v1(&response, &presentation)
                .map_err(|_| inspection_evidence_unavailable())?,
        )
    } else {
        if capability != "not_applicable" || !matches!(response, StudentResponse::ExternalTool {}) {
            return Err(inspection_evidence_unavailable());
        }
        (
            InspectedSubmissionEvidenceV1::PresentationNotApplicable,
            question_model::presentation::InspectedStudentResponseV1::ExternalTool {
                completion:
                    question_model::presentation::InspectedExternalToolStateV1::SubmissionRecorded,
            },
        )
    };

    let generation = ScoringGeneration::new(
        u64::try_from(
            row.try_get::<i64, _>("scoring_generation")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| inspection_evidence_unavailable())?,
    )
    .ok_or_else(inspection_evidence_unavailable)?;
    let scoring_status = decode_scoring_status(
        &row.try_get::<String, _>("scoring_status")
            .map_err(map_sqlx_error)?,
    )?;
    let score_visible: bool = row.try_get("score_visible").map_err(map_sqlx_error)?;
    let correctness_visible: bool = row.try_get("correctness_visible").map_err(map_sqlx_error)?;
    let feedback = project_inspected_student_score_feedback(
        domain::disclosure_policy::StudentDisclosureDecision {
            score: score_visible,
            per_item_correctness: correctness_visible,
            feedback_text: false,
            solution: false,
            class_statistics: false,
        },
        scoring_status,
        receipt_attempt.result,
    );
    Ok(InspectedStudentSubmissionV1 {
        attempt,
        assignment_position,
        submitted_at,
        evidence: evidence_and_response.0,
        scoring_generation: generation,
        feedback,
        response: evidence_and_response.1,
        scoring_status,
    })
}

fn decode_canonical_json<T: serde::de::DeserializeOwned>(
    row: &PgRow,
    source_name: &str,
    payload_name: &str,
    checksum_name: &str,
) -> Result<T, StoreError> {
    let version: i16 = row
        .try_get("canonical_json_version")
        .map_err(map_sqlx_error)?;
    let source: String = row.try_get(source_name).map_err(map_sqlx_error)?;
    let Json(payload): Json<Value> = row.try_get(payload_name).map_err(map_sqlx_error)?;
    let checksum: String = row.try_get(checksum_name).map_err(map_sqlx_error)?;
    if version != 1
        || source.is_empty()
        || Sha256Digest::compute(source.as_bytes()).to_string() != checksum
        || serde_json::from_str::<Value>(&source).map_err(|_| inspection_evidence_unavailable())?
            != payload
    {
        return Err(inspection_evidence_unavailable());
    }
    serde_json::from_str(&source).map_err(|_| inspection_evidence_unavailable())
}

fn decode_scoring_status(value: &str) -> Result<ScoringStatus, StoreError> {
    match value {
        "current" => Ok(ScoringStatus::Current),
        "recalculating" => Ok(ScoringStatus::Recalculating),
        "failed" => Ok(ScoringStatus::Failed),
        _ => Err(inspection_evidence_unavailable()),
    }
}

fn validate_inspection_return_context(
    request: &InspectStudentWorkRequest,
) -> Result<(), StoreError> {
    use crate::{StudentWorkInspectionFocusTarget, StudentWorkInspectionReturnContext};
    let valid = match request.return_context {
        StudentWorkInspectionReturnContext::Gradebook {
            course,
            membership,
            assignment,
            focus,
        } => {
            course == request.course
                && membership == request.membership
                && assignment == request.assignment
                && matches!(focus, StudentWorkInspectionFocusTarget::GradebookCell { membership, assignment }
                    if membership == request.membership && assignment == request.assignment)
        }
        StudentWorkInspectionReturnContext::GradingOperation {
            course,
            membership,
            assignment,
            operation,
            focus,
        } => {
            course == request.course
                && membership == request.membership
                && assignment == request.assignment
                && matches!(focus, StudentWorkInspectionFocusTarget::GradingOperationControl { membership, assignment, operation: focused }
                    if membership == request.membership && assignment == request.assignment && focused == operation)
        }
    };
    valid
        .then_some(())
        .ok_or_else(inspection_evidence_unavailable)
}

fn inspection_evidence_unavailable() -> StoreError {
    StoreError::NotFound
}
