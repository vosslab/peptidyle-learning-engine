//! PostgreSQL adapter for atomic Assessment-owned Question Pool fork import.

use async_trait::async_trait;
use question_model::{
    AssessmentEditNumber, AssessmentEntryScoringRule, QuestionPoolRevisionNumber,
    QuestionPoolRevisionReference, QuestionPoolSelectedQuestionOrder,
};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    AppendAssessmentPoolForkRevisionInput, AppendedAssessmentPoolForkRevision,
    AssessmentPoolForkStore, ImportAssessmentPoolForkInput, ImportedAssessmentPoolFork,
    SessionTokenHash, StoreError,
};

#[derive(Clone)]
pub struct PostgresAssessmentPoolForkStore {
    pool: Pool,
}

impl PostgresAssessmentPoolForkStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(
        &self,
        token: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
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
impl AssessmentPoolForkStore for PostgresAssessmentPoolForkStore {
    async fn import_assessment_question_pool_fork(
        &self,
        session_token_hash: SessionTokenHash,
        input: ImportAssessmentPoolForkInput,
    ) -> Result<ImportedAssessmentPoolFork, StoreError> {
        let expected_next_edit = input
            .expected_assessment_edit_number
            .checked_next()
            .ok_or_else(|| invalid("Assessment Edit Number successor"))?;
        let expected_edit = i64::try_from(input.expected_assessment_edit_number.value())
            .map_err(|_| invalid("Assessment Edit Number"))?;
        let authored_position = i32::try_from(input.authored_position)
            .map_err(|_| invalid("Assessment Entry position"))?;
        let selection_count = i32::try_from(input.selection_count.get())
            .map_err(|_| invalid("Question Pool selection count"))?;
        let points_per_item = input
            .points_per_item
            .to_string()
            .parse::<bigdecimal::BigDecimal>()
            .map_err(|_| invalid("Question Pool points per item"))?;
        let mut transaction = self.begin(session_token_hash).await?;
        // This resolver remains inside the session-authorized Store boundary:
        // callers provide only a public source Pool identity, never a UUID or
        // source Revision pin.
        let source = sqlx::query(
            "SELECT question_pool_id, current_revision_number \
             FROM ple_api.resolve_current_published_question_pool($1)",
        )
        .bind(input.source_public_question_pool_id.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let source_question_pool_id: uuid::Uuid =
            source.try_get("question_pool_id").map_err(map_sqlx_error)?;
        let source_revision = source
            .try_get::<i64, _>("current_revision_number")
            .map_err(map_sqlx_error)?;
        let row = sqlx::query(
            "SELECT * FROM ple_api.import_assessment_question_pool_fork_for_reference(\
             $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(input.course.as_string())
        .bind(input.assessment.as_string())
        .bind(input.assessment_entry.as_uuid())
        .bind(expected_edit)
        .bind(input.fork_question_pool_id)
        .bind(input.fork_public_question_pool_id.as_compact_str())
        .bind(source_question_pool_id)
        .bind(source_revision)
        .bind(authored_position)
        .bind(selection_count)
        .bind(points_per_item)
        .bind(selected_question_order(input.selected_question_order))
        .bind(scoring_rule(input.scoring_rule))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let assessment_entry = question_model::AssessmentEntryId::from_uuid(
            row.try_get("assessment_entry_id").map_err(map_sqlx_error)?,
        );
        if assessment_entry != input.assessment_entry {
            return Err(invalid("Assessment Pool fork Entry"));
        }
        let question_pool_id: uuid::Uuid =
            row.try_get("question_pool_id").map_err(map_sqlx_error)?;
        if question_pool_id != input.fork_question_pool_id {
            return Err(invalid("Assessment Pool fork ID"));
        }
        let revision_number = QuestionPoolRevisionNumber::new(
            u64::try_from(
                row.try_get::<i64, _>("question_pool_revision_number")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| invalid("Assessment Pool fork Revision"))?,
        )
        .map_err(|_| invalid("Assessment Pool fork Revision"))?;
        if revision_number.get() != 1 {
            return Err(invalid("Assessment Pool fork Revision"));
        }
        let assessment_edit_number = AssessmentEditNumber::new(
            u64::try_from(
                row.try_get::<i64, _>("assessment_edit_number")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| invalid("Assessment Edit Number"))?,
        )
        .ok_or_else(|| invalid("Assessment Edit Number"))?;
        if assessment_edit_number != expected_next_edit {
            return Err(invalid("Assessment Pool fork import receipt"));
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(ImportedAssessmentPoolFork {
            assessment_entry,
            question_pool_revision: QuestionPoolRevisionReference {
                question_pool_id: input.fork_public_question_pool_id,
                revision_number,
            },
            assessment_edit_number,
        })
    }

    async fn append_assessment_question_pool_fork_revision(
        &self,
        session_token_hash: SessionTokenHash,
        input: AppendAssessmentPoolForkRevisionInput,
    ) -> Result<AppendedAssessmentPoolForkRevision, StoreError> {
        if input.members.is_empty() || !input.interchangeability_attested {
            return Err(invalid("Assessment Pool fork Revision"));
        }
        let question_ids = input
            .members
            .iter()
            .map(|member| member.question_id.as_compact_str())
            .collect::<Vec<_>>();
        let revision_numbers = input
            .members
            .iter()
            .map(|member| {
                i32::try_from(member.revision_number.get())
                    .map_err(|_| invalid("Question Revision"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let expected_edit = i64::try_from(input.expected_assessment_edit_number.value())
            .map_err(|_| invalid("Assessment Edit Number"))?;
        let mut transaction = self.begin(session_token_hash).await?;
        let row = sqlx::query("SELECT * FROM ple_api.append_assessment_question_pool_fork_revision_for_reference($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(input.course.as_string()).bind(input.assessment.as_string()).bind(input.assessment_entry.as_uuid()).bind(expected_edit)
            .bind(input.expected_pool_metadata_etag).bind(question_ids).bind(revision_numbers).bind(true)
            .fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
        let assessment_entry = question_model::AssessmentEntryId::from_uuid(
            row.try_get("assessment_entry_id").map_err(map_sqlx_error)?,
        );
        if assessment_entry != input.assessment_entry {
            return Err(invalid("Assessment Pool fork Entry"));
        }
        let revision_number = question_model::QuestionPoolRevisionNumber::new(
            u64::try_from(
                row.try_get::<i64, _>("question_pool_revision_number")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| invalid("Assessment Pool fork Revision"))?,
        )
        .map_err(|_| invalid("Assessment Pool fork Revision"))?;
        let metadata_etag = row
            .try_get("question_pool_metadata_etag")
            .map_err(map_sqlx_error)?;
        let assessment_edit_number = question_model::AssessmentEditNumber::new(
            u64::try_from(
                row.try_get::<i64, _>("assessment_edit_number")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| invalid("Assessment Edit Number"))?,
        )
        .ok_or_else(|| invalid("Assessment Edit Number"))?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(AppendedAssessmentPoolForkRevision {
            assessment_entry,
            question_pool_revision_number: revision_number,
            metadata_etag,
            assessment_edit_number,
        })
    }
}

fn invalid(field: &str) -> StoreError {
    StoreError::InvalidRecord(format!("invalid {field}"))
}

fn selected_question_order(value: QuestionPoolSelectedQuestionOrder) -> &'static str {
    match value {
        QuestionPoolSelectedQuestionOrder::QuestionPoolOrder => "question_pool_order",
        QuestionPoolSelectedQuestionOrder::RandomOrder => "random_order",
    }
}

fn scoring_rule(value: AssessmentEntryScoringRule) -> &'static str {
    match value {
        AssessmentEntryScoringRule::Normal => "normal",
        AssessmentEntryScoringRule::FullCredit => "full_credit",
        AssessmentEntryScoringRule::ExtraCredit => "extra_credit",
        AssessmentEntryScoringRule::Excluded => "excluded",
    }
}
