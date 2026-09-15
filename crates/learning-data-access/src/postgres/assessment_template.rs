//! PostgreSQL persistence for private Instructor-owned Assessment Templates.

use async_trait::async_trait;
use question_model::{
    AssessmentTemplate, AssessmentTemplateEditNumber, AssessmentTemplateId, AssessmentTemplateName,
    AssessmentTemplateSettings, AssessmentType,
};
use sqlx::{Postgres, Row, Transaction, types::Json};

use super::{Pool, connection::map_sqlx_error};
use crate::{AssessmentTemplateStore, SaveAssessmentTemplateInput, SessionTokenHash, StoreError};

/// PostgreSQL Store for current private Assessment Template aggregates.
#[derive(Clone)]
pub struct PostgresAssessmentTemplateStore {
    pool: Pool,
}

impl PostgresAssessmentTemplateStore {
    /// Binds the attested API pool to the Assessment Template procedures.
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
impl AssessmentTemplateStore for PostgresAssessmentTemplateStore {
    async fn list_assessment_templates(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<AssessmentTemplate>, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        // ASVS 1.2.4 and 8.2.2: the fixed procedure derives ownership from
        // the installed session; no browser identity can select another owner.
        let rows = sqlx::query(
            "SELECT assessment_template_id, template_name, assessment_type, settings, \
             assessment_template_edit_number FROM ple_api.list_assessment_templates()",
        )
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let templates = rows
            .into_iter()
            .map(decode_template)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(templates)
    }

    async fn read_assessment_template(
        &self,
        session_token_hash: SessionTokenHash,
        id: AssessmentTemplateId,
    ) -> Result<AssessmentTemplate, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        // ASVS 1.2.4 and 8.2.2: the private UUID is a bound locator only;
        // session-derived ownership remains the database authorization rule.
        let row = sqlx::query(
            "SELECT assessment_template_id, template_name, assessment_type, settings, \
             assessment_template_edit_number FROM ple_api.read_assessment_template($1)",
        )
        .bind(id.as_uuid())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let template = decode_template(row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(template)
    }

    async fn create_assessment_template(
        &self,
        session_token_hash: SessionTokenHash,
        template: AssessmentTemplate,
    ) -> Result<AssessmentTemplate, StoreError> {
        if template.edit_number != AssessmentTemplateEditNumber::INITIAL {
            return Err(StoreError::InvalidRecord(
                "new Assessment Template must have initial Edit Number".to_owned(),
            ));
        }
        validate_settings(template.assessment_type, &template.settings)?;
        let mut transaction = self.begin(session_token_hash).await?;
        // ASVS 1.2.4, 2.2.2, and 8.3.1: all values are native bound
        // parameters and the procedure derives the active Instructor owner.
        let row = sqlx::query(
            "SELECT assessment_template_id, template_name, assessment_type, settings, \
             assessment_template_edit_number \
             FROM ple_api.create_assessment_template($1, $2, $3, $4)",
        )
        .bind(template.id.as_uuid())
        .bind(template.name.as_str())
        .bind(template.assessment_type.as_str())
        .bind(Json(&template.settings))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let created = decode_template(row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(created)
    }

    async fn save_assessment_template(
        &self,
        session_token_hash: SessionTokenHash,
        input: SaveAssessmentTemplateInput,
    ) -> Result<AssessmentTemplate, StoreError> {
        validate_settings(input.assessment_type, &input.settings)?;
        let expected = i64::try_from(input.expected_edit_number.value()).map_err(|_| {
            StoreError::InvalidRecord(
                "Assessment Template Edit Number exceeds PostgreSQL bigint".to_owned(),
            )
        })?;
        let mut transaction = self.begin(session_token_hash).await?;
        // ASVS 1.2.4, 2.3.3, and 8.2.2: the one fully bound procedure locks,
        // authorizes, compares, and replaces the complete aggregate atomically.
        let row = sqlx::query(
            "SELECT assessment_template_id, template_name, assessment_type, settings, \
             assessment_template_edit_number \
             FROM ple_api.save_assessment_template($1, $2, $3, $4, $5)",
        )
        .bind(input.id.as_uuid())
        .bind(expected)
        .bind(input.name.as_str())
        .bind(input.assessment_type.as_str())
        .bind(Json(&input.settings))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_save_error)?;
        let saved = decode_template(row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(saved)
    }
}

fn decode_template(row: sqlx::postgres::PgRow) -> Result<AssessmentTemplate, StoreError> {
    let id = row
        .try_get::<uuid::Uuid, _>("assessment_template_id")
        .map(AssessmentTemplateId::from_uuid)
        .map_err(map_sqlx_error)?;
    let name = row
        .try_get::<String, _>("template_name")
        .map_err(map_sqlx_error)
        .and_then(|value| {
            AssessmentTemplateName::try_new(value).map_err(|_| {
                StoreError::InvalidRecord(
                    "database returned an invalid Assessment Template Name".to_owned(),
                )
            })
        })?;
    let assessment_type = row
        .try_get::<String, _>("assessment_type")
        .map_err(map_sqlx_error)
        .and_then(|value| {
            AssessmentType::parse(&value).ok_or_else(|| {
                StoreError::InvalidRecord("database returned an invalid Assessment Type".to_owned())
            })
        })?;
    // ASVS 1.5.2 and 2.2.1: deserialize only the closed accepted settings
    // model; malformed, unknown, or inconsistent stored JSON fails closed.
    let Json(settings) = row
        .try_get::<Json<AssessmentTemplateSettings>, _>("settings")
        .map_err(map_sqlx_error)?;
    validate_settings(assessment_type, &settings)?;
    let edit_number = row
        .try_get::<i64, _>("assessment_template_edit_number")
        .map_err(map_sqlx_error)
        .and_then(|value| {
            u64::try_from(value)
                .ok()
                .and_then(AssessmentTemplateEditNumber::new)
                .ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "database returned an invalid Assessment Template Edit Number".to_owned(),
                    )
                })
        })?;
    Ok(AssessmentTemplate {
        id,
        name,
        assessment_type,
        settings,
        edit_number,
    })
}

fn validate_settings(
    assessment_type: AssessmentType,
    settings: &AssessmentTemplateSettings,
) -> Result<(), StoreError> {
    // ASVS 2.2.2-2.2.3: enforce the combined Type/settings invariant at the
    // trusted persistence boundary for both caller and stored JSON values.
    settings
        .validate_for_assessment_type(assessment_type)
        .map_err(|_| {
            StoreError::InvalidRecord("Assessment Template settings are invalid".to_owned())
        })
}

fn map_save_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("40001")
    {
        // This guarded procedure reserves 40001 for its exact Edit Number
        // precondition; a caller must refresh rather than retry the stale save.
        return StoreError::Conflict;
    }
    map_sqlx_error(error)
}
