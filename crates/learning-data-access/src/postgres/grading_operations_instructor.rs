//! Instructor-facing recovery-operation broker adapter.
//!
//! The SQL capabilities own authority, state transitions, and immutable
//! action receipts. This module only binds the caller's session to the tenant
//! transaction and validates the deliberately small public result shapes.

use async_trait::async_trait;
use question_model::{
    ActivityTimestamp, AssignmentRevision, CourseMembershipReference, GradingOperationReference,
    GradingOperationState, QuestionId, ScoringGeneration, TeachingDisplayLabel,
};
use sqlx::{Row, postgres::PgRow};

use super::{PostgresStore, map_sqlx_error};
use crate::{
    GradingExecutionGeneration, GradingOperationActionReceipt, GradingOperationCursor,
    GradingOperationGroup, GradingOperationGroupBy, GradingOperationRevision,
    GradingOperationStore, GradingOperationTrustGeneration, InstructorGradingOperationProjection,
    InstructorGradingOperationRow, ListInstructorGradingOperationsCommand, Page,
    RecalculateAssignmentCommand, RetryGradingOperationCommand, StoreError, TenantContext,
};

const LIST_SQL: &str = "SELECT * FROM public.ple_list_instructor_grading_operations_v1(\
    $1,$2,$3,$4,$5,$6,$7,$8)";
const RETRY_SQL: &str = "SELECT * FROM public.ple_retry_instructor_grading_operation_v1(\
    $1,$2,$3,$4,$5,$6,$7)";
const RECALCULATE_SQL: &str = "SELECT * FROM public.ple_recalculate_instructor_assignment_v1(\
    $1,$2,$3,$4,$5,$6)";

#[async_trait]
impl GradingOperationStore for PostgresStore {
    async fn list_instructor_grading_operations(
        &self,
        context: TenantContext,
        command: ListInstructorGradingOperationsCommand,
    ) -> Result<Page<InstructorGradingOperationRow>, StoreError> {
        let seek = command
            .page
            .after
            .as_ref()
            .map(|cursor| {
                GradingOperationCursor::decode(
                    cursor,
                    command.course,
                    command.assignment,
                    command.group_by,
                )
            })
            .transpose()?;
        // The public page-size contract is 1..=100. PostgreSQL performs the
        // bounded one-row overfetch internally so the adapter never widens
        // that contract when calling the broker function.
        let page_size = i32::from(command.page.size.get());
        let mut transaction = self.begin_tenant_session(context, command.session).await?;
        let rows = sqlx::query(LIST_SQL)
            .bind(context.tenant_id().as_uuid())
            .bind(command.session.to_string())
            .bind(command.course.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(group_by_name(command.group_by))
            .bind(seek.as_ref().map(|value| value.group_key.as_str()))
            .bind(
                seek.as_ref()
                    .map(|value| i32::try_from(value.operation.number()))
                    .transpose()
                    .map_err(|_| invalid("grading operation cursor reference is too large"))?,
            )
            .bind(page_size)
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_operation_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        operation_page(self, rows, &command)
    }

    async fn retry_instructor_grading_operation(
        &self,
        context: TenantContext,
        command: RetryGradingOperationCommand,
    ) -> Result<GradingOperationActionReceipt, StoreError> {
        let mut transaction = self.begin_tenant_session(context, command.session).await?;
        let row = sqlx::query(RETRY_SQL)
            .bind(context.tenant_id().as_uuid())
            .bind(command.session.to_string())
            .bind(command.course.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(
                i32::try_from(command.operation.number())
                    .map_err(|_| invalid("grading operation reference is too large"))?,
            )
            .bind(
                i64::try_from(command.expected_revision.as_u64())
                    .map_err(|_| invalid("grading operation revision is too large"))?,
            )
            .bind(command.action.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_operation_error)?
            .ok_or(StoreError::NotFound)?;
        let receipt = decode_retry_receipt(&row, &command)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }

    async fn recalculate_instructor_assignment(
        &self,
        context: TenantContext,
        command: RecalculateAssignmentCommand,
    ) -> Result<GradingOperationActionReceipt, StoreError> {
        let mut transaction = self.begin_tenant_session(context, command.session).await?;
        let row = sqlx::query(RECALCULATE_SQL)
            .bind(context.tenant_id().as_uuid())
            .bind(command.session.to_string())
            .bind(command.course.as_uuid())
            .bind(command.assignment.as_uuid())
            .bind(
                i64::try_from(command.expected_assignment_revision.value())
                    .map_err(|_| invalid("assignment revision is too large"))?,
            )
            .bind(command.action.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_operation_error)?
            .ok_or(StoreError::NotFound)?;
        let receipt = decode_recalculation_receipt(&row, &command)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }
}

fn operation_page(
    store: &PostgresStore,
    mut rows: Vec<PgRow>,
    command: &ListInstructorGradingOperationsCommand,
) -> Result<Page<InstructorGradingOperationRow>, StoreError> {
    let limit = usize::from(command.page.size.get());
    let has_more = rows.len() > limit;
    rows.truncate(limit);
    let items = rows
        .iter()
        .map(|row| decode_operation_row(store, row, command))
        .collect::<Result<Vec<_>, _>>()?;
    let next_cursor = has_more
        .then(|| items.last().map(|row| row.stable_cursor.clone()))
        .flatten();
    Ok(Page { items, next_cursor })
}

fn decode_operation_row(
    store: &PostgresStore,
    row: &PgRow,
    command: &ListInstructorGradingOperationsCommand,
) -> Result<InstructorGradingOperationRow, StoreError> {
    let reference = positive_reference(row, "operation_reference")?;
    let target_kind: String = row.try_get("target_kind").map_err(map_sqlx_error)?;
    let group = decode_group(store, row, command.group_by)?;
    validate_target_group(&target_kind, &group)?;
    let group_key: String = row.try_get("group_key").map_err(map_sqlx_error)?;
    if group_key != crate::operation_group_key(&group) {
        return Err(unavailable(
            "grading-operation broker returned a mismatched group key",
        ));
    }
    let reason = decode_symbol(row, "reason", "grading operation reason")?;
    let state = decode_symbol(row, "operation_state", "grading operation state")?;
    let next_action = decode_optional_symbol(row, "next_action", "grading operation action")?;
    let projection = InstructorGradingOperationProjection {
        reference,
        reason,
        state,
        revision: positive_operation_revision(row, "operation_revision")?,
        next_action,
    };
    validate_projection(&projection, &target_kind)?;
    let affected_learner_count = bounded_count(row, "affected_learner_count")?;
    let trust_generation = decode_generation(row, &target_kind, projection.state)?;
    let stable_cursor = GradingOperationCursor::encode(
        command.course,
        command.assignment,
        command.group_by,
        &group,
        reference,
    );
    Ok(InstructorGradingOperationRow {
        operation: projection,
        group,
        affected_learner_count,
        trust_generation,
        stable_cursor,
    })
}

fn decode_group(
    store: &PostgresStore,
    row: &PgRow,
    group_by: GradingOperationGroupBy,
) -> Result<GradingOperationGroup, StoreError> {
    let kind: String = row.try_get("group_kind").map_err(map_sqlx_error)?;
    match kind.as_str() {
        "assignment" => Ok(GradingOperationGroup::Assignment),
        "question" if group_by == GradingOperationGroupBy::Question => {
            let raw: String = row.try_get("question_id").map_err(map_sqlx_error)?;
            let question_id: QuestionId = raw.parse().map_err(|_| {
                unavailable("grading-operation broker returned an invalid Question ID")
            })?;
            if !store.question_ids.validates(&question_id) {
                return Err(unavailable(
                    "grading-operation broker returned an unauthenticated Question ID",
                ));
            }
            let title: String = row.try_get("question_title").map_err(map_sqlx_error)?;
            if title.trim().is_empty() || title.trim() != title || title.chars().count() > 400 {
                return Err(unavailable(
                    "grading-operation broker returned an invalid question title",
                ));
            }
            Ok(GradingOperationGroup::Question { question_id, title })
        }
        "learner" if group_by == GradingOperationGroupBy::Learner => {
            let membership = positive_membership_reference(row, "course_membership_reference")?;
            let label: String = row
                .try_get("student_display_name")
                .map_err(map_sqlx_error)?;
            let display_name = TeachingDisplayLabel::try_from(label).map_err(|_| {
                unavailable("grading-operation broker returned an invalid learner label")
            })?;
            Ok(GradingOperationGroup::Learner {
                membership,
                display_name,
            })
        }
        _ => Err(unavailable(
            "grading-operation broker returned an invalid group",
        )),
    }
}

fn decode_generation(
    row: &PgRow,
    target_kind: &str,
    state: GradingOperationState,
) -> Result<GradingOperationTrustGeneration, StoreError> {
    match target_kind {
        "submission" => {
            let generation = positive_u64(row, "execution_generation")?;
            Ok(GradingOperationTrustGeneration::Execution(
                GradingExecutionGeneration::from_u64(generation)
                    .ok_or_else(|| unavailable("invalid grading execution generation"))?,
            ))
        }
        "assignment_scoring_generation" => {
            let generation = positive_u64(row, "assignment_scoring_generation")?;
            let status: String = row
                .try_get("assignment_scoring_status")
                .map_err(map_sqlx_error)?;
            if !matches!(status.as_str(), "current" | "recalculating" | "failed") {
                return Err(unavailable("invalid assignment scoring status"));
            }
            let state_matches_status = match state {
                GradingOperationState::ActionInProgress => status == "recalculating",
                GradingOperationState::Actionable | GradingOperationState::Failed => {
                    status == "failed"
                }
                GradingOperationState::Completed => status == "current",
                // Terminal repair/supersession retains history while scoring
                // may advance independently after the operation finishes.
                GradingOperationState::RepairRequired | GradingOperationState::Superseded => true,
            };
            if !state_matches_status {
                return Err(unavailable(
                    "grading-operation state disagrees with scoring status",
                ));
            }
            Ok(GradingOperationTrustGeneration::AssignmentScoring(
                ScoringGeneration::new(generation)
                    .ok_or_else(|| unavailable("invalid assignment scoring generation"))?,
            ))
        }
        _ => Err(unavailable(
            "grading-operation broker returned an invalid target",
        )),
    }
}

fn decode_retry_receipt(
    row: &PgRow,
    command: &RetryGradingOperationCommand,
) -> Result<GradingOperationActionReceipt, StoreError> {
    accepted_or_replayed(row)?;
    let operation = positive_reference(row, "operation_reference")?;
    if operation != command.operation {
        return Err(unavailable("retry broker returned another operation"));
    }
    let resulting_operation_revision =
        positive_operation_revision(row, "resulting_operation_revision")?;
    if resulting_operation_revision != next_operation_revision(command.expected_revision)? {
        return Err(unavailable(
            "retry broker returned an incoherent operation revision",
        ));
    }
    let generation = positive_u64(row, "resulting_execution_generation")?;
    if GradingExecutionGeneration::from_u64(generation).is_none() {
        return Err(unavailable(
            "retry broker returned an invalid execution generation",
        ));
    }
    let state: String = row.try_get("resulting_state").map_err(map_sqlx_error)?;
    if state != "ready" {
        return Err(unavailable(
            "retry broker returned an invalid execution state",
        ));
    }
    Ok(GradingOperationActionReceipt::Retry {
        action: command.action,
        operation,
        resulting_operation_revision,
        safe_category: crate::GradingOperationReceiptSafeCategory::InstructorRetry,
        occurred_at: timestamp(row, "action_occurred_at_millis")?,
    })
}

fn decode_recalculation_receipt(
    row: &PgRow,
    command: &RecalculateAssignmentCommand,
) -> Result<GradingOperationActionReceipt, StoreError> {
    accepted_or_replayed(row)?;
    let assignment_revision = positive_assignment_revision(row, "assignment_revision")?;
    if assignment_revision != command.expected_assignment_revision {
        return Err(unavailable(
            "recalculation broker returned another assignment revision",
        ));
    }
    let operation = positive_reference(row, "operation_reference")?;
    let resulting_operation_revision =
        positive_operation_revision(row, "created_operation_revision")?;
    let scoring_generation = ScoringGeneration::new(positive_u64(row, "scoring_generation")?)
        .ok_or_else(|| unavailable("invalid recalculation scoring generation"))?;
    let state: String = row.try_get("scoring_status").map_err(map_sqlx_error)?;
    if state != "recalculating" {
        return Err(unavailable(
            "recalculation broker returned an invalid scoring state",
        ));
    }
    Ok(GradingOperationActionReceipt::Recalculation {
        action: command.action,
        operation,
        resulting_operation_revision,
        assignment_revision,
        scoring_generation,
        safe_category: crate::GradingOperationReceiptSafeCategory::InstructorRecalculation,
        occurred_at: timestamp(row, "action_occurred_at_millis")?,
    })
}

fn validate_target_group(
    target_kind: &str,
    group: &GradingOperationGroup,
) -> Result<(), StoreError> {
    match (target_kind, group) {
        (
            "submission",
            GradingOperationGroup::Question { .. } | GradingOperationGroup::Learner { .. },
        )
        | ("assignment_scoring_generation", GradingOperationGroup::Assignment) => Ok(()),
        _ => Err(unavailable(
            "grading-operation broker returned a target/group mismatch",
        )),
    }
}

fn validate_projection(
    projection: &InstructorGradingOperationProjection,
    target_kind: &str,
) -> Result<(), StoreError> {
    let coherent = match (target_kind, projection.state) {
        ("submission", GradingOperationState::Actionable) => {
            projection.next_action == Some(question_model::GradingOperationAction::Retry)
        }
        ("assignment_scoring_generation", GradingOperationState::Actionable) => {
            projection.next_action == Some(question_model::GradingOperationAction::Recalculate)
        }
        (
            "submission" | "assignment_scoring_generation",
            GradingOperationState::ActionInProgress,
        )
        | ("submission" | "assignment_scoring_generation", GradingOperationState::Completed)
        | ("submission" | "assignment_scoring_generation", GradingOperationState::RepairRequired)
        | ("submission" | "assignment_scoring_generation", GradingOperationState::Failed)
        | ("submission" | "assignment_scoring_generation", GradingOperationState::Superseded) => {
            projection.next_action.is_none()
        }
        (
            _,
            GradingOperationState::Actionable
            | GradingOperationState::ActionInProgress
            | GradingOperationState::Completed
            | GradingOperationState::RepairRequired
            | GradingOperationState::Failed
            | GradingOperationState::Superseded,
        ) => false,
    };
    coherent
        .then_some(())
        .ok_or_else(|| unavailable("invalid grading-operation action state"))
}

fn next_operation_revision(
    revision: GradingOperationRevision,
) -> Result<GradingOperationRevision, StoreError> {
    revision
        .as_u64()
        .checked_add(1)
        .and_then(GradingOperationRevision::from_u64)
        .ok_or_else(|| invalid("grading operation revision overflow"))
}

fn accepted_or_replayed(row: &PgRow) -> Result<(), StoreError> {
    let disposition: String = row.try_get("disposition").map_err(map_sqlx_error)?;
    matches!(disposition.as_str(), "accepted" | "replayed")
        .then_some(())
        .ok_or_else(|| {
            unavailable("grading-operation broker returned an invalid action disposition")
        })
}

fn group_by_name(value: GradingOperationGroupBy) -> &'static str {
    match value {
        GradingOperationGroupBy::Question => "question",
        GradingOperationGroupBy::Learner => "learner",
    }
}

fn positive_reference(row: &PgRow, column: &str) -> Result<GradingOperationReference, StoreError> {
    let value: i32 = row.try_get(column).map_err(map_sqlx_error)?;
    u32::try_from(value)
        .ok()
        .and_then(|value| GradingOperationReference::new(u64::from(value)))
        .ok_or_else(|| unavailable("invalid grading operation reference"))
}

fn positive_membership_reference(
    row: &PgRow,
    column: &str,
) -> Result<CourseMembershipReference, StoreError> {
    let value: i32 = row.try_get(column).map_err(map_sqlx_error)?;
    u32::try_from(value)
        .ok()
        .and_then(|value| CourseMembershipReference::new(u64::from(value)))
        .ok_or_else(|| unavailable("invalid course membership reference"))
}

fn positive_operation_revision(
    row: &PgRow,
    column: &str,
) -> Result<GradingOperationRevision, StoreError> {
    GradingOperationRevision::from_u64(positive_u64(row, column)?)
        .ok_or_else(|| unavailable("invalid grading operation revision"))
}

fn positive_assignment_revision(
    row: &PgRow,
    column: &str,
) -> Result<AssignmentRevision, StoreError> {
    AssignmentRevision::new(positive_u64(row, column)?)
        .ok_or_else(|| unavailable("invalid assignment revision"))
}

fn positive_u64(row: &PgRow, column: &str) -> Result<u64, StoreError> {
    let value: i64 = row.try_get(column).map_err(map_sqlx_error)?;
    u64::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| unavailable("invalid positive grading-operation value"))
}

fn bounded_count(row: &PgRow, column: &str) -> Result<u32, StoreError> {
    let value: i64 = row.try_get(column).map_err(map_sqlx_error)?;
    bounded_count_value(value)
}

fn bounded_count_value(value: i64) -> Result<u32, StoreError> {
    u32::try_from(value)
        .map_err(|_| unavailable("grading-operation affected count is outside u32 bounds"))
}

fn timestamp(row: &PgRow, column: &str) -> Result<ActivityTimestamp, StoreError> {
    let value: i64 = row.try_get(column).map_err(map_sqlx_error)?;
    (value > 0)
        .then(|| ActivityTimestamp::from_unix_millis(value))
        .ok_or_else(|| unavailable("invalid grading-operation timestamp"))
}

fn decode_symbol<T>(row: &PgRow, column: &str, description: &str) -> Result<T, StoreError>
where
    T: serde::de::DeserializeOwned,
{
    let value: String = row.try_get(column).map_err(map_sqlx_error)?;
    serde_json::from_value(serde_json::Value::String(value))
        .map_err(|_| unavailable(&format!("invalid {description}")))
}

fn decode_optional_symbol<T>(
    row: &PgRow,
    column: &str,
    description: &str,
) -> Result<Option<T>, StoreError>
where
    T: serde::de::DeserializeOwned,
{
    row.try_get::<Option<String>, _>(column)
        .map_err(map_sqlx_error)?
        .map(|value| {
            serde_json::from_value(serde_json::Value::String(value))
                .map_err(|_| unavailable(&format!("invalid {description}")))
        })
        .transpose()
}

fn invalid(message: &str) -> StoreError {
    StoreError::InvalidRecord(message.to_string())
}

fn unavailable(message: &str) -> StoreError {
    StoreError::Unavailable(message.to_string())
}

fn map_operation_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database) = &error {
        match database.code().as_deref() {
            Some("55000") => return StoreError::Conflict,
            Some("22023") => return invalid("grading-operation broker rejected invalid input"),
            Some("42501") => return StoreError::NotFound,
            _ => {}
        }
    }
    map_sqlx_error(error)
}

#[cfg(test)]
mod tests {
    use super::{bounded_count_value, validate_projection};
    use crate::{GradingOperationRevision, InstructorGradingOperationProjection};
    use question_model::{
        GradingOperationReason, GradingOperationReference, GradingOperationState,
    };

    #[test]
    fn operation_impact_count_accepts_an_empty_assignment_and_bounds_storage_values() {
        assert_eq!(
            bounded_count_value(0).expect("empty enrollment is valid"),
            0
        );
        assert_eq!(
            bounded_count_value(12).expect("ordinary count is valid"),
            12
        );
        assert!(bounded_count_value(-1).is_err());
        assert!(bounded_count_value(i64::from(u32::MAX) + 1).is_err());
    }

    #[test]
    fn terminal_operation_states_remain_listable_without_a_follow_up_action() {
        let projection = |state| InstructorGradingOperationProjection {
            reference: GradingOperationReference::new(1).expect("positive reference"),
            reason: GradingOperationReason::GraderExecutionFailure,
            state,
            revision: GradingOperationRevision::INITIAL,
            next_action: None,
        };
        for state in [
            GradingOperationState::Completed,
            GradingOperationState::RepairRequired,
            GradingOperationState::Failed,
            GradingOperationState::Superseded,
        ] {
            validate_projection(&projection(state), "submission")
                .expect("terminal submission operation is safe to list");
            validate_projection(&projection(state), "assignment_scoring_generation")
                .expect("terminal assignment operation is safe to list");
        }
    }
}
