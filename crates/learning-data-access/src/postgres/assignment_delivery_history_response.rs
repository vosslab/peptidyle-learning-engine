//! Completed Student response reproduction evidence reader.

use super::{assignment_delivery::source_from_row, connection::map_sqlx_error};
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
        "SELECT * FROM ple_api.read_student_assignment_attempt_history_response_sources($1)",
    )
    .bind(i64::from(assignment_attempt.number()))
    .fetch_all(&mut *tx)
    .await
    .map_err(map_sqlx_error)?;
    let sources = rows
        .iter()
        .map(|row| {
            let position = u32::try_from(
                row.try_get::<i32, _>("position").map_err(map_sqlx_error)?,
            )
            .map_err(|_| StoreError::InvalidRecord("History position is invalid".to_string()))?;
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
                presentation_source: source_from_row(row)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(sources)
}
