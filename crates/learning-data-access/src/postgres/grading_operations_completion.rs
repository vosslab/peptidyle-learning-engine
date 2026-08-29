//! Completion-plan decoding for the sealed automated-grading worker.

use super::*;
use question_model::{
    AssignmentAudience, AssignmentDeliveryState, AssignmentItem, AssignmentSelectionCandidate,
    AssignmentSelectionGroup, AssignmentSelectionGroupId, CourseGroupId, PoolDrawAlgorithm,
    ProblemVersionRef, RunPolicies, SelectionOrdering, StudentDisclosurePolicy,
};
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LockedAssignmentHeader {
    assignment_id: Uuid,
    course_id: Uuid,
    title: String,
    lifecycle: String,
    instructions: String,
    completion_policy: String,
    completion_threshold: Option<String>,
    attempt_selection_policy: String,
    continued_practice_policy: String,
    practice_max_additional_runs: Option<i32>,
    variation_policy: String,
    audience_kind: String,
    score_disclosure: String,
    per_item_correctness_disclosure: String,
    feedback_text_disclosure: String,
    solution_disclosure: String,
    class_statistics_disclosure: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LockedAssignmentItem {
    assignment_item_id: Uuid,
    position: i32,
    problem_id: Uuid,
    version_id: Uuid,
    points_possible: String,
    delivery_state: String,
    scoring_mode: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LockedSelectionGroup {
    selection_group_id: Uuid,
    position: i32,
    draw_count: i32,
    points_per_item: String,
    ordering_policy: String,
    algorithm_version: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LockedSelectionCandidate {
    selection_group_id: Uuid,
    candidate_id: Uuid,
    position: i32,
    problem_id: Uuid,
    version_id: Uuid,
    delivery_state: String,
}

fn decode_locked_assignment(
    row: &sqlx::postgres::PgRow,
    tenant: TenantId,
    expected_assignment: AssignmentId,
) -> Result<crate::AssignmentRecord, StoreError> {
    let header: LockedAssignmentHeader = decode_locked_json(row, "assignment_header")?;
    if header.assignment_id != expected_assignment.as_uuid() {
        return Err(StoreError::Unavailable(
            "completion lock assignment identity disagrees with its claim".to_string(),
        ));
    }
    let audience_groups: Vec<Uuid> = decode_locked_json(row, "assignment_audience_groups")?;
    let audience = match header.audience_kind.as_str() {
        "course_wide" if audience_groups.is_empty() => AssignmentAudience::CourseWide,
        "any_of_groups" => AssignmentAudience::any_of_groups(
            audience_groups
                .into_iter()
                .map(CourseGroupId::from_uuid)
                .collect(),
        )
        .map_err(|_| {
            StoreError::Unavailable("locked assignment audience is invalid".to_string())
        })?,
        _ => {
            return Err(StoreError::Unavailable(
                "locked assignment audience is invalid".to_string(),
            ));
        }
    };
    let items: Vec<LockedAssignmentItem> = decode_locked_json(row, "assignment_items")?;
    let items = items
        .into_iter()
        .map(|item| {
            Ok(AssignmentItem {
                id: question_model::AssignmentItemId::from_uuid(item.assignment_item_id),
                reference: ProblemVersionRef {
                    problem: question_model::ProblemId::from_uuid(item.problem_id),
                    version: question_model::VersionId::from_uuid(item.version_id),
                },
                position: locked_u32(item.position, "assignment item position")?,
                points_possible: item.points_possible.parse().map_err(|_| {
                    StoreError::Unavailable("locked assignment item points are invalid".to_string())
                })?,
                delivery_state: locked_delivery_state(&item.delivery_state)?,
                scoring_mode: locked_scoring_mode(&item.scoring_mode)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let candidates: Vec<LockedSelectionCandidate> =
        decode_locked_json(row, "assignment_selection_candidates")?;
    let mut candidates_by_group =
        BTreeMap::<AssignmentSelectionGroupId, Vec<AssignmentSelectionCandidate>>::new();
    for candidate in candidates {
        let group = AssignmentSelectionGroupId::from_uuid(candidate.selection_group_id);
        candidates_by_group
            .entry(group)
            .or_default()
            .push(AssignmentSelectionCandidate {
                id: question_model::AssignmentItemId::from_uuid(candidate.candidate_id),
                position: locked_u32(candidate.position, "selection candidate position")?,
                reference: ProblemVersionRef {
                    problem: question_model::ProblemId::from_uuid(candidate.problem_id),
                    version: question_model::VersionId::from_uuid(candidate.version_id),
                },
                delivery_state: locked_delivery_state(&candidate.delivery_state)?,
            });
    }
    let groups: Vec<LockedSelectionGroup> = decode_locked_json(row, "assignment_selection_groups")?;
    let selection_groups = groups
        .into_iter()
        .map(|group| {
            let id = AssignmentSelectionGroupId::from_uuid(group.selection_group_id);
            Ok(AssignmentSelectionGroup {
                id,
                position: locked_u32(group.position, "selection group position")?,
                draw_count: locked_u32(group.draw_count, "selection group draw count")?,
                points_per_item: group.points_per_item.parse().map_err(|_| {
                    StoreError::Unavailable("locked selection group points are invalid".to_string())
                })?,
                ordering: locked_selection_ordering(&group.ordering_policy)?,
                algorithm: PoolDrawAlgorithm::from_storage_version(locked_u16(
                    group.algorithm_version,
                    "selection algorithm version",
                )?)
                .ok_or_else(|| {
                    StoreError::Unavailable(
                        "locked selection algorithm version is unsupported".to_string(),
                    )
                })?,
                candidates: candidates_by_group.remove(&id).unwrap_or_default(),
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    if !candidates_by_group.is_empty() {
        return Err(StoreError::Unavailable(
            "locked selection candidate has no selection group".to_string(),
        ));
    }
    let assignment = crate::AssignmentRecord {
        id: expected_assignment,
        tenant,
        course_id: CourseId::from_uuid(header.course_id),
        title: header.title,
        lifecycle: super::course_policy::parse_assignment_lifecycle(&header.lifecycle)?,
        instructions: question_model::AssignmentInstructions::try_new(header.instructions)
            .map_err(|_| {
                StoreError::Unavailable("locked assignment instructions are invalid".to_string())
            })?,
        audience,
        items,
        selection_groups,
        policies: RunPolicies {
            completion: super::parse_completion_policy(
                &header.completion_policy,
                header.completion_threshold,
            )?,
            grade: super::parse_grade_policy(&header.attempt_selection_policy)?,
            continued_practice: super::parse_continued_practice(
                &header.continued_practice_policy,
                header.practice_max_additional_runs,
            )?,
            variation: super::parse_variation_policy(&header.variation_policy)?,
        },
        disclosure_policy: StudentDisclosurePolicy {
            score: super::parse_student_disclosure_timing(&header.score_disclosure)?,
            per_item_correctness: super::parse_student_disclosure_timing(
                &header.per_item_correctness_disclosure,
            )?,
            feedback_text: super::parse_student_disclosure_timing(
                &header.feedback_text_disclosure,
            )?,
            solution: super::parse_student_disclosure_timing(&header.solution_disclosure)?,
            class_statistics: super::parse_student_disclosure_timing(
                &header.class_statistics_disclosure,
            )?,
        },
    };
    crate::validate_assignment(&assignment).map_err(|error| {
        StoreError::Unavailable(format!("locked assignment definition is invalid: {error}"))
    })?;
    Ok(assignment)
}

fn decode_locked_json<T: serde::de::DeserializeOwned>(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<T, StoreError> {
    let value: serde_json::Value = row.try_get(column).map_err(map_sqlx_error)?;
    serde_json::from_value(value).map_err(|error| {
        StoreError::Unavailable(format!("locked completion {column} is invalid: {error}"))
    })
}

fn locked_u32(value: i32, field: &str) -> Result<u32, StoreError> {
    u32::try_from(value).map_err(|_| StoreError::Unavailable(format!("locked {field} is invalid")))
}

fn locked_u16(value: i32, field: &str) -> Result<u16, StoreError> {
    u16::try_from(value).map_err(|_| StoreError::Unavailable(format!("locked {field} is invalid")))
}

fn locked_delivery_state(value: &str) -> Result<AssignmentDeliveryState, StoreError> {
    super::parse_assignment_delivery_state(value)
}

fn locked_scoring_mode(value: &str) -> Result<question_model::AssignmentScoringMode, StoreError> {
    super::parse_assignment_scoring_mode(value)
}

fn locked_selection_ordering(value: &str) -> Result<SelectionOrdering, StoreError> {
    super::parse_selection_ordering(value)
}

/// An established mutable current projection and the exact bytes that its
/// existing digest attests.
pub(super) struct CurrentProjection {
    pub(super) source: String,
    pub(super) projection: serde_json::Value,
    pub(super) sha256: String,
}

/// Encodes an established mutable current projection.
///
/// Immutable receipt evidence uses `ple-canonical-json-v1` through the
/// submission helpers.  These legacy hashes remain the current-projection
/// contract for the three mutable aggregate rows.
pub(super) fn encode_current_projection<T: Serialize>(
    value: &T,
) -> Result<CurrentProjection, StoreError> {
    let (Json(projection), sha256) = encode_payload(value)?;
    let source = String::from_utf8(
        serde_json::to_vec(&projection)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
    )
    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    Ok(CurrentProjection {
        source,
        projection,
        sha256,
    })
}

pub(super) struct LockedCompletionSource {
    pub(super) input: crate::AcceptedSubmissionCompletionInput,
    pub(super) scoring_generation: i64,
}

/// The scalar columns returned by the completion lock, kept named after the
/// SQL aliases so the worker boundary cannot silently drop either identity.
#[derive(Debug, PartialEq)]
pub(super) struct LockedCompletionSummaryRow {
    pub(super) summary_tenant_id: Uuid,
    pub(super) summary_enrollment_id: Uuid,
    pub(super) summary_current_score: Option<f64>,
    pub(super) summary_best_score: Option<f64>,
    pub(super) summary_latest_score: Option<f64>,
    pub(super) summary_completed_run_count: i64,
    pub(super) summary_total_question_attempts: i64,
    pub(super) summary_last_activity_at_millis: Option<i64>,
}

fn decode_locked_completion_source_summary(
    row: LockedCompletionSummaryRow,
) -> Result<question_model::StudentAssignmentSummary, StoreError> {
    super::summary::decode_summary_values(super::summary::SummaryRowValues {
        tenant_id: row.summary_tenant_id,
        enrollment_id: row.summary_enrollment_id,
        current_score: row.summary_current_score,
        best_score: row.summary_best_score,
        latest_score: row.summary_latest_score,
        completed_run_count: row.summary_completed_run_count,
        total_question_attempts: row.summary_total_question_attempts,
        last_activity_at_millis: row.summary_last_activity_at_millis,
    })
}

fn decode_locked_completion_source_summary_row(
    row: &sqlx::postgres::PgRow,
) -> Result<question_model::StudentAssignmentSummary, StoreError> {
    decode_locked_completion_source_summary(LockedCompletionSummaryRow {
        summary_tenant_id: row.try_get("summary_tenant_id").map_err(map_sqlx_error)?,
        summary_enrollment_id: row
            .try_get("summary_enrollment_id")
            .map_err(map_sqlx_error)?,
        summary_current_score: row
            .try_get("summary_current_score")
            .map_err(map_sqlx_error)?,
        summary_best_score: row.try_get("summary_best_score").map_err(map_sqlx_error)?,
        summary_latest_score: row
            .try_get("summary_latest_score")
            .map_err(map_sqlx_error)?,
        summary_completed_run_count: row
            .try_get("summary_completed_run_count")
            .map_err(map_sqlx_error)?,
        summary_total_question_attempts: row
            .try_get("summary_total_question_attempts")
            .map_err(map_sqlx_error)?,
        summary_last_activity_at_millis: row
            .try_get("summary_last_activity_at_millis")
            .map_err(map_sqlx_error)?,
    })
}

pub(super) fn decode_locked_completion_source(
    row: &sqlx::postgres::PgRow,
    claim: AcceptedSubmissionExecutionClaim,
    grade: crate::AcceptedSubmissionGrade,
) -> Result<LockedCompletionSource, StoreError> {
    let attempt_id =
        QuestionAttemptId::from_uuid(row.try_get("attempt_id").map_err(map_sqlx_error)?);
    let assignment_id =
        AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?);
    let assignment = decode_locked_assignment(row, claim.tenant, assignment_id)?;
    let mut base_attempt: question_model::QuestionAttempt =
        decode_payload_row_named(row, "attempt_payload", "attempt_payload_sha256")?;
    if base_attempt.id != attempt_id
        || base_attempt.tenant != claim.tenant
        || base_attempt.response.is_some()
        || base_attempt.result.is_some()
    {
        return Err(StoreError::Unavailable(
            "completion lock attempt evidence is incoherent".to_string(),
        ));
    }
    let run: question_model::AssignmentRun =
        decode_payload_row_named(row, "run_payload", "run_payload_sha256")?;
    let run_id = question_model::RunId::from_uuid(row.try_get("run_id").map_err(map_sqlx_error)?);
    if run.id != run_id || base_attempt.run != run_id {
        return Err(StoreError::Unavailable(
            "completion lock run evidence is incoherent".to_string(),
        ));
    }
    let enrollment_id = question_model::EnrollmentId::from_uuid(
        row.try_get("enrollment_id").map_err(map_sqlx_error)?,
    );
    if run.enrollment != enrollment_id {
        return Err(StoreError::Unavailable(
            "completion lock enrollment identity is incoherent".to_string(),
        ));
    }
    let enrollment = question_model::AssignmentEnrollment {
        id: enrollment_id,
        tenant: claim.tenant,
        assignment: assignment_id,
        user: UserId::from_uuid(row.try_get("enrollment_user_id").map_err(map_sqlx_error)?),
        student: question_model::StudentId::from_uuid(
            row.try_get("enrollment_student_id")
                .map_err(map_sqlx_error)?,
        ),
        first_completed_at: row
            .try_get::<Option<i64>, _>("enrollment_first_completed_at_millis")
            .map_err(map_sqlx_error)?
            .map(ActivityTimestamp::from_unix_millis),
        current_grade_run: row
            .try_get::<Option<Uuid>, _>("enrollment_current_grade_run_id")
            .map_err(map_sqlx_error)?
            .map(question_model::RunId::from_uuid),
        best_grade_run: row
            .try_get::<Option<Uuid>, _>("enrollment_best_grade_run_id")
            .map_err(map_sqlx_error)?
            .map(question_model::RunId::from_uuid),
    };
    let summary = decode_locked_completion_source_summary_row(row)?;
    if summary.tenant != claim.tenant || summary.enrollment != enrollment_id {
        return Err(StoreError::Unavailable(
            "completion lock summary evidence is incoherent".to_string(),
        ));
    }
    let presentation = decode_locked_optional_payload(
        row,
        "presentation_payload",
        "presentation_payload_sha256",
        "presentation_required",
    )?;
    let run_items: Vec<question_model::AssignmentRunItem> = decode_locked_json(row, "run_items")?;
    if run_items.iter().any(|item| item.run != run_id) {
        return Err(StoreError::Unavailable(
            "completion lock run item evidence is incoherent".to_string(),
        ));
    }
    let attempts = decode_locked_run_attempts(row, claim.tenant, run_id)?;
    if attempts.iter().all(|attempt| attempt.id != base_attempt.id) {
        return Err(StoreError::Unavailable(
            "completion lock omits the accepted attempt".to_string(),
        ));
    }
    let accepted_at = ActivityTimestamp::from_unix_millis(
        row.try_get("accepted_at_millis").map_err(map_sqlx_error)?,
    );
    let scoring_generation: i64 = row
        .try_get("assignment_scoring_generation")
        .map_err(map_sqlx_error)?;
    if scoring_generation <= 0 || assignment.id != claim_submission_assignment(row, assignment_id)?
    {
        return Err(StoreError::Unavailable(
            "completion lock assignment generation is invalid".to_string(),
        ));
    }
    base_attempt.response = None;
    Ok(LockedCompletionSource {
        input: crate::AcceptedSubmissionCompletionInput {
            base_attempt,
            grade,
            assignment,
            run,
            enrollment,
            previous_summary: summary,
            run_items,
            attempts,
            accepted_at,
            presentation,
        },
        scoring_generation,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LockedRunAttempt {
    attempt_id: Uuid,
    payload: serde_json::Value,
    payload_sha256: String,
    status: String,
    submitted_at_millis: Option<i64>,
    evaluation: Option<serde_json::Value>,
    evaluation_sha256: Option<String>,
    evaluation_status: Option<String>,
}

fn decode_locked_run_attempts(
    row: &sqlx::postgres::PgRow,
    tenant: TenantId,
    run: question_model::RunId,
) -> Result<Vec<question_model::QuestionAttempt>, StoreError> {
    let entries: Vec<LockedRunAttempt> = decode_locked_json(row, "same_run_attempts")?;
    entries
        .into_iter()
        .map(|entry| {
            let mut attempt: question_model::QuestionAttempt =
                decode_payload_parts(entry.payload, entry.payload_sha256)?;
            if attempt.id.as_uuid() != entry.attempt_id
                || attempt.tenant != tenant
                || attempt.run != run
            {
                return Err(StoreError::Unavailable(
                    "completion lock same-run attempt identity is incoherent".to_string(),
                ));
            }
            attempt.status = decode_attempt_status(&entry.status)?;
            attempt.timer.submitted_at = entry
                .submitted_at_millis
                .map(ActivityTimestamp::from_unix_millis);
            match entry.evaluation_status.as_deref() {
                None => attempt.result = None,
                Some("automated_pending" | "automated_exception") => {
                    attempt.result = None;
                }
                Some("graded" | "exempt") => {
                    let evaluation = entry.evaluation.ok_or_else(|| {
                        StoreError::Unavailable(
                            "completion lock graded attempt lacks evaluation".to_string(),
                        )
                    })?;
                    let checksum = entry.evaluation_sha256.ok_or_else(|| {
                        StoreError::Unavailable(
                            "completion lock graded attempt lacks evaluation checksum".to_string(),
                        )
                    })?;
                    let result: question_model::AttemptResult =
                        decode_payload_parts(evaluation, checksum)?;
                    crate::validate_attempt_result(result)?;
                    attempt.result = Some(result);
                }
                Some(_) => {
                    return Err(StoreError::Unavailable(
                        "completion lock has invalid evaluation state".to_string(),
                    ));
                }
            }
            Ok(attempt)
        })
        .collect()
}

fn decode_locked_optional_payload<T: serde::de::DeserializeOwned>(
    row: &sqlx::postgres::PgRow,
    payload_name: &str,
    checksum_name: &str,
    required_name: &str,
) -> Result<Option<T>, StoreError> {
    let required: bool = row.try_get(required_name).map_err(map_sqlx_error)?;
    let payload: Option<serde_json::Value> = row.try_get(payload_name).map_err(map_sqlx_error)?;
    let checksum: Option<String> = row.try_get(checksum_name).map_err(map_sqlx_error)?;
    match (required, payload, checksum) {
        (false, None, None) => Ok(None),
        (true, Some(payload), Some(checksum)) => decode_payload_parts(payload, checksum).map(Some),
        _ => Err(StoreError::Unavailable(
            "completion lock presentation evidence is incomplete".to_string(),
        )),
    }
}

pub(super) fn verify_locked_claim(
    row: &sqlx::postgres::PgRow,
    claim: AcceptedSubmissionExecutionClaim,
) -> Result<(), StoreError> {
    let returned = decode_worker_claim(row, claim.worker, claim.lease_token)?;
    if returned.tenant != claim.tenant
        || returned.job != claim.job
        || returned.submission != claim.submission
        || returned.execution_generation != claim.execution_generation
    {
        return Err(StoreError::Unavailable(
            "completion lock disagrees with the exact worker claim".to_string(),
        ));
    }
    Ok(())
}

fn claim_submission_assignment(
    row: &sqlx::postgres::PgRow,
    assignment: AssignmentId,
) -> Result<AssignmentId, StoreError> {
    let stored = AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?);
    (stored == assignment).then_some(stored).ok_or_else(|| {
        StoreError::Unavailable("completion lock assignment identity is invalid".to_string())
    })
}

pub(super) fn execution_generation_i64(
    claim: AcceptedSubmissionExecutionClaim,
) -> Result<i64, StoreError> {
    i64::try_from(claim.execution_generation.as_u64()).map_err(|_| {
        StoreError::InvalidRecord("grading execution generation is too large".to_string())
    })
}

pub(super) fn encode_statistics(
    statistics: &Option<Vec<crate::StatisticsContribution>>,
) -> Result<serde_json::Value, StoreError> {
    let entries = statistics
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|entry| {
            serde_json::json!({
                "attemptId": entry.first_scored_attempt.as_uuid(),
                "problemId": entry.reference.problem.as_uuid(),
                "versionId": entry.reference.version.as_uuid(),
                "normalizedScore": entry.observation.normalized_score(),
                "attempts": entry.observation.attempts(),
                "durationSeconds": entry.observation.duration_seconds(),
                "restScore": entry.observation.rest_score(),
                "observationSha256": entry.checksum.to_string(),
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::Value::Array(entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_prepares_distinct_receipt_and_current_projection_evidence() {
        let value = serde_json::json!({"attempt": "a-1", "status": "submitted"});
        let receipt = crate::canonical_json::canonical_json_bytes_v1("receipt", &value)
            .expect("canonical receipt evidence");
        let current = encode_current_projection(&value).expect("current projection");

        assert_eq!(
            receipt.version,
            crate::canonical_json::CANONICAL_JSON_V1_VERSION
        );
        assert_eq!(receipt.projection, current.projection);
        assert_eq!(
            current.sha256,
            objects::Sha256Digest::compute(
                serde_json::to_vec(&current.projection)
                    .expect("current projection serialization")
                    .as_slice(),
            )
            .to_string()
        );
        assert_eq!(
            current.source,
            String::from_utf8(
                serde_json::to_vec(&current.projection).expect("current projection serialization"),
            )
            .expect("JSON is UTF-8")
        );
    }

    #[test]
    fn decode_locked_completion_source_preserves_scalar_summary_row() {
        let tenant = Uuid::from_u128(0x1111);
        let enrollment = Uuid::from_u128(0x2222);
        let lock_row = LockedCompletionSummaryRow {
            summary_tenant_id: tenant,
            summary_enrollment_id: enrollment,
            summary_current_score: Some(0.875),
            summary_best_score: Some(0.9375),
            summary_latest_score: Some(0.875),
            summary_completed_run_count: 4,
            summary_total_question_attempts: 11,
            summary_last_activity_at_millis: Some(1_725_000_123_456),
        };

        let summary = decode_locked_completion_source_summary(lock_row)
            .expect("completion lock scalar summary");

        assert_eq!(
            summary,
            question_model::StudentAssignmentSummary {
                tenant: TenantId::from_uuid(tenant),
                enrollment: question_model::EnrollmentId::from_uuid(enrollment),
                current_score: Some(0.875),
                best_score: Some(0.9375),
                latest_score: Some(0.875),
                completed_run_count: 4,
                total_question_attempts: 11,
                last_activity_at: Some(ActivityTimestamp::from_unix_millis(1_725_000_123_456)),
            }
        );
    }
}
