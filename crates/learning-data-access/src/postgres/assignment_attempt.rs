//! PostgreSQL persistence for authenticated Assignment Attempt starts.

use async_trait::async_trait;
use question_model::{
    AssignmentAttemptId, IssuedQuestionId, QuestionPoolSelectionId, QuestionRevisionReference,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    AssignmentAttemptStart, AssignmentAttemptStartResult, AssignmentAttemptStore,
    PreparedIssuedQuestion, SessionTokenHash, StoreError,
};

/// PostgreSQL implementation of the authenticated Assignment Attempt Store.
#[derive(Clone)]
pub struct PostgresAssignmentAttemptStore {
    pool: Pool,
}

impl PostgresAssignmentAttemptStore {
    /// Binds the already-attested API pool to Assignment Attempt persistence.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin_authenticated_application_transaction(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token_hash.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[async_trait]
impl AssignmentAttemptStore for PostgresAssignmentAttemptStore {
    async fn start_assignment_attempt(
        &self,
        session_token_hash: SessionTokenHash,
        start: AssignmentAttemptStart,
    ) -> Result<AssignmentAttemptStartResult, StoreError> {
        start.validate()?;
        let assignment_attempt =
            AssignmentAttemptId::from_uuid(crate::random_uuid::random_uuid_v4(|error| {
                StoreError::Unavailable(format!(
                    "Assignment Attempt ID randomness unavailable: {error}"
                ))
            })?);
        let selection_ids = start
            .question_pool_selections
            .iter()
            .map(|_| {
                crate::random_uuid::random_uuid_v4(|error| {
                    StoreError::Unavailable(format!(
                        "Question Pool Selection ID randomness unavailable: {error}"
                    ))
                })
                .map(QuestionPoolSelectionId::from_uuid)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let selections = storage_selections(&start, &selection_ids);
        let issued_questions =
            storage_issued_questions(&start, assignment_attempt, &selection_ids)?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row = sqlx::query(
            "SELECT assignment_attempt_id, attempt_number, resumed \
             FROM ple_api.start_assignment_attempt($1, $2, $3, $4, $5)",
        )
        .bind(assignment_attempt.as_uuid())
        .bind(start.student_record.as_uuid())
        .bind(start.assignment.as_uuid())
        .bind(selections)
        .bind(issued_questions)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let attempt_number: i32 = row.try_get("attempt_number").map_err(map_sqlx_error)?;
        let result = AssignmentAttemptStartResult {
            assignment_attempt: AssignmentAttemptId::from_uuid(
                row.try_get("assignment_attempt_id")
                    .map_err(map_sqlx_error)?,
            ),
            attempt_number: u32::try_from(attempt_number).map_err(|_| {
                StoreError::Unavailable("stored Assignment Attempt number is invalid".to_string())
            })?,
            resumed: row.try_get("resumed").map_err(map_sqlx_error)?,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

fn storage_selections(
    start: &AssignmentAttemptStart,
    selection_ids: &[QuestionPoolSelectionId],
) -> Value {
    Value::Array(
        start
            .question_pool_selections
            .iter()
            .zip(selection_ids)
            .map(|(selection, id)| {
                json!({
                    "question_pool_selection_id": id.as_uuid(),
                    "assignment_entry_id": selection.question_pool_entry.as_uuid(),
                    "reused_from_question_pool_selection_id": selection
                        .reused_from_question_pool_selection
                        .map(|source| source.as_uuid()),
                    "selected_candidates": selection.selected_candidates.iter().enumerate().map(
                        |(position, candidate)| json!({
                            "question_pool_candidate_id": candidate.candidate.as_uuid(),
                            "selection_position": position,
                            "question_id": candidate.reference.question_id.to_string(),
                            "revision_number": candidate.reference.revision_number.get(),
                        })
                    ).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

fn storage_issued_questions(
    start: &AssignmentAttemptStart,
    assignment_attempt: AssignmentAttemptId,
    selection_ids: &[QuestionPoolSelectionId],
) -> Result<Value, StoreError> {
    start
        .issued_questions
        .iter()
        .enumerate()
        .map(|(position, question)| {
            let (
                assignment_entry,
                question_pool_selection,
                question_pool_candidate,
                reference,
            ): (_, Option<QuestionPoolSelectionId>, _, &QuestionRevisionReference) = match question {
                PreparedIssuedQuestion::FixedQuestion {
                    assignment_entry,
                    reference,
                } => (*assignment_entry, None, None, reference),
                PreparedIssuedQuestion::QuestionPoolCandidate {
                    assignment_entry,
                    question_pool_selection_index,
                    question_pool_candidate,
                    reference,
                } => (
                    *assignment_entry,
                    Some(*selection_ids.get(*question_pool_selection_index).ok_or_else(|| {
                        StoreError::InvalidRecord(
                            "a pooled Issued Question must name a prepared Question Pool Selection"
                                .to_string(),
                        )
                    })?),
                    Some(*question_pool_candidate),
                    reference,
                ),
            };
            let issued_question = IssuedQuestionId::for_frozen_content(
                assignment_attempt,
                assignment_entry,
                question_pool_candidate,
            );
            Ok(json!({
                "issued_question_id": issued_question.as_uuid(),
                "assignment_entry_id": assignment_entry.as_uuid(),
                "issued_position": position,
                "question_id": reference.question_id.to_string(),
                "revision_number": reference.revision_number.get(),
                "question_pool_selection_id": question_pool_selection.map(|selection| selection.as_uuid()),
                "question_pool_candidate_id": question_pool_candidate.map(|candidate| candidate.as_uuid()),
            }))
        })
        .collect::<Result<Vec<_>, StoreError>>()
        .map(Value::Array)
}
