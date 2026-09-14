//! PostgreSQL adapter for the generic worker's narrow Attempt-expiry sweep.

use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use super::{Pool, assignment_delivery::finalization_source_from_row, connection::map_sqlx_error};
use crate::{
    AssignmentAttemptExpirySweepStore, ExpiredAssignmentAttemptFinalizationPreparation, StoreError,
    StudentAssignmentAttemptFinalizationEvaluation, StudentAssignmentAttemptFinalizationKind,
    StudentAssignmentAttemptFinalizationPreparation,
};

/// PostgreSQL Store with only the deadline-sweep procedure capability.
#[derive(Clone)]
pub struct PostgresAssignmentAttemptExpirySweepStore {
    pool: Pool,
}

impl PostgresAssignmentAttemptExpirySweepStore {
    /// Binds an attested generic-worker pool to the expiry procedure.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AssignmentAttemptExpirySweepStore for PostgresAssignmentAttemptExpirySweepStore {
    async fn prepare_expired_assignment_attempt_finalizations(
        &self,
        limit: u32,
    ) -> Result<Vec<ExpiredAssignmentAttemptFinalizationPreparation>, StoreError> {
        let limit = i32::try_from(limit)
            .ok()
            .filter(|value| (1..=1000).contains(value))
            .ok_or_else(|| {
                StoreError::InvalidRecord("Expiry sweep limit is invalid".to_string())
            })?;
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_assignment_attempt_expiry_worker")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let rows = sqlx::query(
            "SELECT assignment_attempt_id, question_attempt_id, saved_at_millis, \
                    question_id, revision_number, source_object_id::text AS source_object_id, \
                    source_object_checksum, question_seed::text AS question_seed, student_response, \
                    backend, webwork_pg_path \
             FROM ple_api.prepare_expired_student_assignment_attempt_finalizations($1)",
        )
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        let mut preparations = std::collections::BTreeMap::new();
        for row in rows {
            let assignment_attempt_id = row
                .try_get::<Uuid, _>("assignment_attempt_id")
                .map_err(map_sqlx_error)?;
            let source = finalization_source_from_row(&row)?;
            let preparation = preparations
                .entry(assignment_attempt_id)
                .or_insert_with(|| ExpiredAssignmentAttemptFinalizationPreparation {
                    assignment_attempt_id,
                    preparation: StudentAssignmentAttemptFinalizationPreparation {
                        kind: StudentAssignmentAttemptFinalizationKind::Deadline,
                        saved_responses: Vec::new(),
                    },
                });
            if let Some(source) = source {
                preparation.preparation.saved_responses.push(source);
            }
        }
        Ok(preparations.into_values().collect())
    }

    async fn commit_expired_assignment_attempt_finalization(
        &self,
        assignment_attempt_id: Uuid,
        preparation: StudentAssignmentAttemptFinalizationPreparation,
        evaluations: Vec<StudentAssignmentAttemptFinalizationEvaluation>,
    ) -> Result<(), StoreError> {
        if preparation.kind != StudentAssignmentAttemptFinalizationKind::Deadline {
            return Err(StoreError::InvalidRecord(
                "Expired Assignment Attempt finalization kind is invalid".to_string(),
            ));
        }
        validate_evaluations(&preparation, &evaluations)?;
        let evaluation_payload = serde_json::Value::Array(
            evaluations
                .iter()
                .map(|evaluation| {
                    serde_json::json!({
                        "question_attempt_id": evaluation.question_attempt_id,
                        "saved_at_millis": evaluation.saved_at.as_unix_millis(),
                        "student_response": evaluation.student_response,
                        "normalized_credit": evaluation.normalized_credit,
                    })
                })
                .collect(),
        );
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_assignment_attempt_expiry_worker")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query(
            "SELECT ple_api.commit_expired_student_assignment_attempt_finalization($1, $2)",
        )
        .bind(assignment_attempt_id)
        .bind(evaluation_payload)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(())
    }
}

fn validate_evaluations(
    preparation: &StudentAssignmentAttemptFinalizationPreparation,
    evaluations: &[StudentAssignmentAttemptFinalizationEvaluation],
) -> Result<(), StoreError> {
    if evaluations.len() != preparation.saved_responses.len()
        || evaluations.iter().any(|evaluation| {
            !evaluation.normalized_credit.is_finite()
                || !(0.0..=1.0).contains(&evaluation.normalized_credit)
        })
    {
        return Err(StoreError::InvalidRecord(
            "Expired Assignment Attempt evaluation is invalid".to_string(),
        ));
    }
    let expected = preparation
        .saved_responses
        .iter()
        .map(|source| source.question_attempt_id)
        .collect::<std::collections::BTreeSet<_>>();
    let actual = evaluations
        .iter()
        .map(|evaluation| evaluation.question_attempt_id)
        .collect::<std::collections::BTreeSet<_>>();
    if expected != actual {
        return Err(StoreError::InvalidRecord(
            "Expired Assignment Attempt evaluation does not match its snapshot".to_string(),
        ));
    }
    Ok(())
}
