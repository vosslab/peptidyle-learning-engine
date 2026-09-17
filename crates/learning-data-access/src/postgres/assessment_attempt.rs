//! PostgreSQL persistence for authenticated Assessment Attempt starts.

use async_trait::async_trait;
use question_model::{
    AssessmentAttemptId, IssuedQuestionId, QuestionBackend, QuestionPoolSelectionId,
    QuestionRevisionReference,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    AssessmentAttemptStart, AssessmentAttemptStartResult, AssessmentAttemptStore,
    PreparedIssuedQuestion, SessionTokenHash, StoreError,
};

/// PostgreSQL implementation of the authenticated Assessment Attempt Store.
#[derive(Clone)]
pub struct PostgresAssessmentAttemptStore {
    pool: Pool,
}

impl PostgresAssessmentAttemptStore {
    /// Binds the already-attested API pool to Assessment Attempt persistence.
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

    pub(super) async fn start_assessment_attempt_in_transaction(
        transaction: &mut Transaction<'_, Postgres>,
        start: AssessmentAttemptStart,
    ) -> Result<AssessmentAttemptStartResult, StoreError> {
        start.validate()?;
        let assessment_attempt =
            AssessmentAttemptId::from_uuid(crate::random_uuid::random_uuid_v4(|error| {
                StoreError::Unavailable(format!(
                    "Assessment Attempt ID randomness unavailable: {error}"
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
            storage_issued_questions(&start, assessment_attempt, &selection_ids)?;
        let row = sqlx::query(
            "SELECT assessment_attempt_id, assessment_attempt_number, resumed \
             FROM ple_api.start_assessment_attempt($1, $2, $3, $4, $5)",
        )
        .bind(assessment_attempt.as_uuid())
        .bind(start.student_record.as_uuid())
        .bind(start.assessment.as_uuid())
        .bind(selections)
        .bind(issued_questions)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        let attempt_number: i32 = row
            .try_get("assessment_attempt_number")
            .map_err(map_sqlx_error)?;
        Ok(AssessmentAttemptStartResult {
            assessment_attempt: AssessmentAttemptId::from_uuid(
                row.try_get("assessment_attempt_id")
                    .map_err(map_sqlx_error)?,
            ),
            attempt_number: u32::try_from(attempt_number).map_err(|_| {
                StoreError::Unavailable("stored Assessment Attempt number is invalid".to_string())
            })?,
            resumed: row.try_get("resumed").map_err(map_sqlx_error)?,
        })
    }
}

#[async_trait]
impl AssessmentAttemptStore for PostgresAssessmentAttemptStore {
    async fn start_assessment_attempt(
        &self,
        session_token_hash: SessionTokenHash,
        start: AssessmentAttemptStart,
    ) -> Result<AssessmentAttemptStartResult, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let result = Self::start_assessment_attempt_in_transaction(&mut transaction, start).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

fn storage_selections(
    start: &AssessmentAttemptStart,
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
                    "assessment_entry_id": selection.question_pool_assessment_entry.as_uuid(),
                    "question_pool_id": selection.question_pool_revision.question_pool_id.as_str(),
                    "question_pool_revision_number": selection.question_pool_revision.revision_number.get(),
                    "selected_items": selection.selected_items.iter().map(|item| {
                        // PostgreSQL stores Pool Revision member positions one-based;
                        // the shared contract deliberately exposes them zero-based.
                        i32::try_from(item.pool_revision_member.member_position + 1)
                            .expect("Pool Revision member position fits PostgreSQL INTEGER")
                    }).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

fn storage_issued_questions(
    start: &AssessmentAttemptStart,
    assessment_attempt: AssessmentAttemptId,
    selection_ids: &[QuestionPoolSelectionId],
) -> Result<Value, StoreError> {
    start
        .issued_questions
        .iter()
        .enumerate()
        .map(|(position, question)| {
            let (
                assessment_entry,
                question_pool_selection,
                pool_revision_member,
                reference,
                backend,
            ): (
                _,
                Option<QuestionPoolSelectionId>,
                _,
                &QuestionRevisionReference,
                &QuestionBackend,
            ) = match question {
                PreparedIssuedQuestion::FixedQuestion {
                    assessment_entry,
                    reference,
                    backend,
                } => (*assessment_entry, None, None, reference, backend),
                PreparedIssuedQuestion::QuestionPoolItem {
                    assessment_entry,
                    question_pool_selection_index,
                    pool_revision_member,
                    reference,
                    backend,
                } => (
                    *assessment_entry,
                    Some(*selection_ids.get(*question_pool_selection_index).ok_or_else(|| {
                        StoreError::InvalidRecord(
                            "a pooled Issued Question must name a prepared Question Pool Selection"
                                .to_string(),
                        )
                    })?),
                    Some(pool_revision_member),
                    reference,
                    backend,
                ),
            };
            let question_seed = match backend {
                QuestionBackend::Ple => Value::Null,
                QuestionBackend::Webwork => json!(
                    crate::random_uuid::random_question_seed(|error| {
                        StoreError::Unavailable(format!(
                            "Question seed randomness unavailable: {error}"
                        ))
                    })?
                ),
                QuestionBackend::Imathas => {
                    return Err(StoreError::InvalidRecord(
                        "Question Backend is unavailable for new Assessment work".to_string(),
                    ));
                }
            };
            let issued_question = IssuedQuestionId::for_frozen_content(
                assessment_attempt,
                assessment_entry,
                pool_revision_member,
            );
            Ok(json!({
                "issued_question_id": issued_question.as_uuid(),
                "assessment_entry_id": assessment_entry.as_uuid(),
                "issued_position": position,
                "question_id": reference.question_id.as_str(),
                "revision_number": reference.revision_number.get(),
                "question_pool_selection_id": question_pool_selection.map(|selection| selection.as_uuid()),
                "question_pool_member_position": pool_revision_member.map(|member| {
                    i32::try_from(member.member_position + 1)
                        .expect("Pool Revision member position fits PostgreSQL INTEGER")
                }),
                "question_seed": question_seed,
            }))
        })
        .collect::<Result<Vec<_>, StoreError>>()
        .map(Value::Array)
}

#[cfg(test)]
mod tests {
    use question_model::{
        AssessmentEntryId, AssessmentId, QuestionId, QuestionRevisionNumber, StudentRecordId,
    };
    use uuid::Uuid;

    use super::*;

    #[test]
    fn static_issued_question_payload_has_no_seed() {
        let assessment_entry = AssessmentEntryId::from_uuid(Uuid::from_u128(3));
        let start = AssessmentAttemptStart {
            student_record: StudentRecordId::from_uuid(Uuid::from_u128(1)),
            assessment: AssessmentId::from_uuid(Uuid::from_u128(2)),
            question_pool_selections: Vec::new(),
            issued_questions: vec![PreparedIssuedQuestion::FixedQuestion {
                assessment_entry,
                reference: QuestionRevisionReference {
                    question_id: "1234-H567".parse::<QuestionId>().expect("Question ID"),
                    revision_number: QuestionRevisionNumber::new(1).expect("revision number"),
                },
                backend: QuestionBackend::Ple,
            }],
        };

        let payload = storage_issued_questions(
            &start,
            AssessmentAttemptId::from_uuid(Uuid::from_u128(4)),
            &[],
        )
        .expect("serialized Issued Question");
        let issued = payload
            .as_array()
            .and_then(|items| items.first())
            .and_then(Value::as_object)
            .expect("one JSON object");

        assert_eq!(issued.get("question_seed"), Some(&Value::Null));
        assert_eq!(
            issued.get("question_id").and_then(Value::as_str),
            Some("1234-H567")
        );
        assert!(!issued.contains_key("questionSeed"));
    }

    #[test]
    fn renderer_backed_issued_question_payload_has_a_seed() {
        let assessment_entry = AssessmentEntryId::from_uuid(Uuid::from_u128(3));
        let start = AssessmentAttemptStart {
            student_record: StudentRecordId::from_uuid(Uuid::from_u128(1)),
            assessment: AssessmentId::from_uuid(Uuid::from_u128(2)),
            question_pool_selections: Vec::new(),
            issued_questions: vec![PreparedIssuedQuestion::FixedQuestion {
                assessment_entry,
                reference: QuestionRevisionReference {
                    question_id: "1234-H567".parse::<QuestionId>().expect("Question ID"),
                    revision_number: QuestionRevisionNumber::new(1).expect("revision number"),
                },
                backend: QuestionBackend::Webwork,
            }],
        };

        let payload = storage_issued_questions(
            &start,
            AssessmentAttemptId::from_uuid(Uuid::from_u128(4)),
            &[],
        )
        .expect("serialized Issued Question");
        let issued = payload
            .as_array()
            .and_then(|items| items.first())
            .and_then(Value::as_object)
            .expect("one JSON object");

        assert!(
            issued
                .get("question_seed")
                .and_then(Value::as_u64)
                .is_some()
        );
    }
}
