//! PostgreSQL adapter for the generic worker's narrow Attempt-expiry sweep.

use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use super::{Pool, assessment_delivery::finalization_source_from_row, connection::map_sqlx_error};
use crate::{
    AssessmentAttemptExpirySweepStore, ExpiredAssessmentAttemptFinalizationPreparation, StoreError,
    StudentAssessmentAttemptFinalizationEvaluation, StudentAssessmentAttemptFinalizationKind,
    StudentAssessmentAttemptFinalizationPreparation,
};

/// PostgreSQL Store with only the deadline-sweep procedure capability.
#[derive(Clone)]
pub struct PostgresAssessmentAttemptExpirySweepStore {
    pool: Pool,
}

impl PostgresAssessmentAttemptExpirySweepStore {
    /// Binds an attested generic-worker pool to the expiry procedure.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AssessmentAttemptExpirySweepStore for PostgresAssessmentAttemptExpirySweepStore {
    async fn prepare_expired_assessment_attempt_finalizations(
        &self,
        limit: u32,
    ) -> Result<Vec<ExpiredAssessmentAttemptFinalizationPreparation>, StoreError> {
        let limit = i32::try_from(limit)
            .ok()
            .filter(|value| (1..=1000).contains(value))
            .ok_or_else(|| {
                StoreError::InvalidRecord("Expiry sweep limit is invalid".to_string())
            })?;
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_assessment_attempt_expiry_worker")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let rows = sqlx::query(
            "SELECT assessment_attempt_id, question_attempt_id, saved_at_millis, \
                    question_id, revision_number, source_object_id::text AS source_object_id, \
                    source_object_checksum, question_seed::text AS question_seed, generated_parameter_sha256, student_response, \
                    backend, webwork_pg_path \
             FROM ple_api.prepare_expired_student_assessment_attempt_finalizations($1)",
        )
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        let mut preparations = std::collections::BTreeMap::new();
        for row in rows {
            let assessment_attempt_id = row
                .try_get::<Uuid, _>("assessment_attempt_id")
                .map_err(map_sqlx_error)?;
            let source = finalization_source_from_row(&row)?;
            let preparation = preparations
                .entry(assessment_attempt_id)
                .or_insert_with(|| ExpiredAssessmentAttemptFinalizationPreparation {
                    assessment_attempt_id,
                    preparation: StudentAssessmentAttemptFinalizationPreparation {
                        kind: StudentAssessmentAttemptFinalizationKind::Deadline,
                        saved_responses: Vec::new(),
                    },
                });
            if let Some(source) = source {
                preparation.preparation.saved_responses.push(source);
            }
        }
        Ok(preparations.into_values().collect())
    }

    async fn commit_expired_assessment_attempt_finalization(
        &self,
        assessment_attempt_id: Uuid,
        preparation: StudentAssessmentAttemptFinalizationPreparation,
        evaluations: Vec<StudentAssessmentAttemptFinalizationEvaluation>,
    ) -> Result<(), StoreError> {
        if preparation.kind != StudentAssessmentAttemptFinalizationKind::Deadline {
            return Err(StoreError::InvalidRecord(
                "Expired Assessment Attempt finalization kind is invalid".to_string(),
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
        sqlx::query("SET LOCAL ROLE ple_assessment_attempt_expiry_worker")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query(
            "SELECT ple_api.commit_expired_student_assessment_attempt_finalization($1, $2)",
        )
        .bind(assessment_attempt_id)
        .bind(evaluation_payload)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(())
    }
}

fn validate_evaluations(
    preparation: &StudentAssessmentAttemptFinalizationPreparation,
    evaluations: &[StudentAssessmentAttemptFinalizationEvaluation],
) -> Result<(), StoreError> {
    if evaluations.len() != preparation.saved_responses.len()
        || evaluations.iter().any(|evaluation| {
            !evaluation.normalized_credit.is_finite()
                || !(0.0..=1.0).contains(&evaluation.normalized_credit)
        })
    {
        return Err(StoreError::InvalidRecord(
            "Expired Assessment Attempt evaluation is invalid".to_string(),
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
            "Expired Assessment Attempt evaluation does not match its snapshot".to_string(),
        ));
    }
    Ok(())
}
