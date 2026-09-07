//! PostgreSQL persistence for the current published Blueprint Course lifecycle.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use question_model::{
    BlueprintCourseReadAccess, BlueprintCourseReference, BlueprintRevision,
    CreateBlueprintCourseContentInput, QuestionId, QuestionRevisionNumber,
    QuestionRevisionReference, ReplaceBlueprintCourseContentInput,
};
use sqlx::{Postgres, Row, Transaction, types::Json};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    BlueprintCourseStore, SessionTokenHash, StoreError, StoredBlueprintCourse,
    StoredBlueprintCourseContent, StoredBlueprintCourseSummary,
};

/// PostgreSQL Store for published Blueprint Courses visible to active Instructors.
#[derive(Clone)]
pub struct PostgresBlueprintCourseStore {
    pool: Pool,
}

impl PostgresBlueprintCourseStore {
    /// Binds the attested API pool to Blueprint Course lifecycle procedures.
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
impl BlueprintCourseStore for PostgresBlueprintCourseStore {
    async fn list_blueprint_courses(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<StoredBlueprintCourseSummary>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let rows = sqlx::query("SELECT * FROM ple_api.list_live_demo_blueprint_courses()")
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let records = rows
            .iter()
            .map(decode_summary)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(records)
    }

    async fn load_blueprint_course(
        &self,
        session_token_hash: SessionTokenHash,
        reference: BlueprintCourseReference,
    ) -> Result<StoredBlueprintCourse, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row = sqlx::query("SELECT * FROM ple_api.load_live_demo_blueprint_course($1)")
            .bind(i64::from(reference.number()))
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let record = row
            .as_ref()
            .map(decode_record)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn create_blueprint_course(
        &self,
        session_token_hash: SessionTokenHash,
        input: CreateBlueprintCourseContentInput,
    ) -> Result<StoredBlueprintCourse, StoreError> {
        input.validate().map_err(invalid_input)?;
        let requested = StoredBlueprintCourseContent::requested_question_ids_from_create(&input);
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let pins = resolve_current_question_pins(&mut transaction, requested).await?;
        let content = StoredBlueprintCourseContent::from_create(input, &pins)?;
        let checksum = content.checksum()?.as_bytes().to_vec();
        let encoded =
            serde_json::to_value(&content).map_err(|_| invalid("Blueprint Revision Content"))?;
        let reference_number: i64 = sqlx::query_scalar(
            "SELECT ple_api.create_live_demo_blueprint_course($1, $2, $3, $4, $5)",
        )
        .bind(random_uuid()?)
        .bind(random_uuid()?)
        .bind(random_uuid()?)
        .bind(encoded)
        .bind(checksum)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        self.load_blueprint_course(session_token_hash, reference(reference_number)?)
            .await
    }

    async fn replace_blueprint_course(
        &self,
        session_token_hash: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_revision: BlueprintRevision,
        input: ReplaceBlueprintCourseContentInput,
    ) -> Result<StoredBlueprintCourse, StoreError> {
        input.validate().map_err(invalid_input)?;
        let prior = self
            .load_blueprint_course(session_token_hash, reference)
            .await?;
        if prior.read_access != BlueprintCourseReadAccess::BlueprintCourseOwner {
            return Err(StoreError::Forbidden);
        }
        let requested = StoredBlueprintCourseContent::requested_question_ids_from_replace(&input);
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let pins = resolve_current_question_pins(&mut transaction, requested).await?;
        let content = StoredBlueprintCourseContent::from_replace(input, &prior.content, &pins)?;
        let checksum = content.checksum()?.as_bytes().to_vec();
        let encoded =
            serde_json::to_value(&content).map_err(|_| invalid("Blueprint Revision Content"))?;
        sqlx::query("SELECT ple_api.replace_live_demo_blueprint_course($1, $2, $3, $4, $5, $6)")
            .bind(i64::from(reference.number()))
            .bind(
                i64::try_from(expected_revision.value())
                    .map_err(|_| invalid("Blueprint Revision"))?,
            )
            .bind(random_uuid()?)
            .bind(random_uuid()?)
            .bind(encoded)
            .bind(checksum)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        self.load_blueprint_course(session_token_hash, reference)
            .await
    }
}

async fn resolve_current_question_pins(
    transaction: &mut Transaction<'_, Postgres>,
    requested: Vec<QuestionId>,
) -> Result<BTreeMap<QuestionId, QuestionRevisionReference>, StoreError> {
    let requested = requested.into_iter().collect::<BTreeSet<_>>();
    if requested.is_empty() {
        return Err(invalid("Blueprint Course without Published Questions"));
    }
    let identifiers = requested
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let rows = sqlx::query(
        "SELECT question_id, revision_number FROM ple_api.list_question_library_entries() \
         WHERE question_id = ANY($1) AND availability = 'available'",
    )
    .bind(identifiers)
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let mut pins = BTreeMap::new();
    for row in rows {
        let question_id = row
            .try_get::<String, _>("question_id")
            .map_err(map_sqlx_error)?
            .parse::<QuestionId>()
            .map_err(|_| invalid("Published Question ID"))?;
        let revision_number = row
            .try_get::<i32, _>("revision_number")
            .map_err(map_sqlx_error)?;
        let revision_number = u32::try_from(revision_number)
            .ok()
            .and_then(|value| QuestionRevisionNumber::new(value).ok())
            .ok_or_else(|| invalid("Published Question Revision"))?;
        pins.insert(
            question_id.clone(),
            QuestionRevisionReference {
                question_id,
                revision_number,
            },
        );
    }
    if pins.len() != requested.len() {
        return Err(StoreError::InvalidRecord(
            "Blueprint Course requires currently available Published Questions".to_string(),
        ));
    }
    Ok(pins)
}

fn decode_summary(row: &sqlx::postgres::PgRow) -> Result<StoredBlueprintCourseSummary, StoreError> {
    Ok(StoredBlueprintCourseSummary {
        reference: reference(row.try_get("reference_number").map_err(map_sqlx_error)?)?,
        title: row.try_get("title").map_err(map_sqlx_error)?,
        revision: revision(row.try_get("revision_number").map_err(map_sqlx_error)?)?,
        read_access: read_access(row.try_get("is_owner").map_err(map_sqlx_error)?),
    })
}

fn decode_record(row: &sqlx::postgres::PgRow) -> Result<StoredBlueprintCourse, StoreError> {
    let Json(content): Json<StoredBlueprintCourseContent> = row
        .try_get("blueprint_course_content")
        .map_err(map_sqlx_error)?;
    let title: String = row.try_get("title").map_err(map_sqlx_error)?;
    if content.title != title {
        return Err(invalid("Blueprint Revision title"));
    }
    let expected_checksum: Vec<u8> = row
        .try_get("blueprint_content_checksum")
        .map_err(map_sqlx_error)?;
    if expected_checksum.as_slice() != content.checksum()?.as_bytes() {
        return Err(invalid("Blueprint Content Checksum"));
    }
    Ok(StoredBlueprintCourse {
        reference: reference(row.try_get("reference_number").map_err(map_sqlx_error)?)?,
        revision: revision(row.try_get("revision_number").map_err(map_sqlx_error)?)?,
        read_access: read_access(row.try_get("is_owner").map_err(map_sqlx_error)?),
        content,
    })
}

fn reference(value: i64) -> Result<BlueprintCourseReference, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(BlueprintCourseReference::new)
        .ok_or_else(|| invalid("Blueprint Course Reference"))
}

fn revision(value: i64) -> Result<BlueprintRevision, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(BlueprintRevision::new)
        .ok_or_else(|| invalid("Blueprint Revision"))
}

fn read_access(is_owner: bool) -> BlueprintCourseReadAccess {
    if is_owner {
        BlueprintCourseReadAccess::BlueprintCourseOwner
    } else {
        BlueprintCourseReadAccess::ActiveInstructor
    }
}

fn invalid_input(error: question_model::BlueprintCourseValidationError) -> StoreError {
    StoreError::InvalidRecord(format!("Blueprint Course request is invalid: {error}"))
}

fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}

fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Blueprint Course UUID randomness unavailable".to_string())
    })
}
