//! Completed Student response reproduction evidence reader.

use super::{
    assignment_delivery::{positive_i32, source_from_row},
    assignment_delivery_source::presentation_evidence_from_row,
    connection::map_sqlx_error,
};
use crate::{SessionTokenHash, StoreError, StudentAssignmentAttemptHistoryResponseSource};
use question_model::{AssignmentAttemptReference, StudentResponse};
use sqlx::Row;

pub(super) async fn read(
    store: &super::assignment_delivery::PostgresLiveAssignmentDeliveryStore,
    token: SessionTokenHash,
    assignment_attempt: AssignmentAttemptReference,
) -> Result<Vec<StudentAssignmentAttemptHistoryResponseSource>, StoreError> {
    let mut tx = store.begin(token).await?;
    // ASVS 8.2.2 and 8.3.1: this completed-history reader repeats exact
    // Student ownership and active membership before returning private inputs.
    let rows = sqlx::query(
        "SELECT *, position AS issued_position \
         FROM ple_api.read_student_assignment_attempt_history_response_sources($1)",
    )
    .bind(i64::from(assignment_attempt.number()))
    .fetch_all(&mut *tx)
    .await
    .map_err(map_sqlx_error)?;
    let sources = rows
        .iter()
        .map(|row| {
            let position = positive_i32(row, "position", "History position")?;
            let response = row
                .try_get::<Option<serde_json::Value>, _>("student_response")
                .map_err(map_sqlx_error)?
                .map(serde_json::from_value::<StudentResponse>)
                .transpose()
                .map_err(|_| {
                    StoreError::InvalidRecord("History response is invalid".to_string())
                })?;
            Ok(StudentAssignmentAttemptHistoryResponseSource {
                position,
                response,
                presentation_evidence: presentation_evidence_from_row(row)?,
                presentation_source: match row
                    .try_get::<Option<String>, _>("backend")
                    .map_err(map_sqlx_error)?
                    .as_deref()
                {
                    Some("ple") => Some(source_from_row(row)?),
                    _ => None,
                },
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(sources)
}
