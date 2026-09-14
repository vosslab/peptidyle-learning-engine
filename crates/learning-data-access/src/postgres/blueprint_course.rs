//! PostgreSQL persistence for Blueprint Revisions and lineage metadata.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use question_model::{
    AccountId, BlueprintAvailability, BlueprintCourseReadAccess, BlueprintCourseReference,
    BlueprintMetadataEtag, BlueprintMetadataState, BlueprintRevision, BlueprintRevisionReference,
    CreateBlueprintCourseInput, CreateBlueprintCourseReceipt, QuestionId, QuestionRevisionNumber,
    QuestionRevisionReference, RenameBlueprintCourseInput, ReplaceBlueprintCourseContentInput,
    RequestChecksum, SaveBlueprintCourseReceipt, Timestamp,
};
use serde_json::Value;
use sqlx::{Postgres, Row, Transaction, types::Json};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::blueprint_course::StoredBlueprintRevision;
use crate::{
    BlueprintCourseStore, SessionTokenHash, StoreError, StoredBlueprintCourse,
    StoredBlueprintCourseContent, StoredBlueprintCourseSummary,
};

/// PostgreSQL Store for Blueprint lineages visible to active Instructors.
#[derive(Clone)]
pub struct PostgresBlueprintCourseStore {
    pool: Pool,
}

impl PostgresBlueprintCourseStore {
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
        session: SessionTokenHash,
    ) -> Result<Vec<StoredBlueprintCourseSummary>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let rows = sqlx::query("SELECT * FROM ple_api.list_blueprint_courses()")
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
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
    ) -> Result<StoredBlueprintCourse, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let row = sqlx::query("SELECT * FROM ple_api.load_blueprint_course($1)")
            .bind(reference_number(reference)?)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let record = row
            .as_ref()
            .map(decode_course)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn load_blueprint_revision(
        &self,
        session: SessionTokenHash,
        reference: BlueprintRevisionReference,
    ) -> Result<StoredBlueprintRevision, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let row = sqlx::query("SELECT * FROM ple_api.load_blueprint_revision($1, $2)")
            .bind(reference_number(reference.reference)?)
            .bind(revision_number(reference.revision)?)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let row = row.ok_or(StoreError::NotFound)?;
        let Json(content): Json<Value> = row.try_get("content").map_err(map_sqlx_error)?;
        let content = decode_stored_content(content)?;
        verify_content_checksum(
            &content,
            &row.try_get::<Vec<u8>, _>("content_checksum")
                .map_err(map_sqlx_error)?,
        )?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(StoredBlueprintRevision { reference, content })
    }

    async fn create_blueprint_course(
        &self,
        session: SessionTokenHash,
        request_checksum: RequestChecksum,
        input: CreateBlueprintCourseInput,
    ) -> Result<CreateBlueprintCourseReceipt, StoreError> {
        input.validate().map_err(invalid_input)?;
        let short_name = input.short_name.clone();
        let long_name = input.long_name.clone();
        let requested = StoredBlueprintCourseContent::requested_question_ids_from_create(&input);
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let actor = current_actor(&mut transaction).await?;
        let pins = resolve_current_question_pins(&mut transaction, requested).await?;
        let content = StoredBlueprintCourseContent::from_create(input, &pins)?;
        let encoded = encode_content(&content)?;
        let row = sqlx::query(
            "SELECT reference_number, blueprint_revision_number, metadata_etag, \
             (EXTRACT(EPOCH FROM accepted_at) * 1000)::bigint AS accepted_at_millis \
             FROM ple_api.create_blueprint_course($1, $2, $3, $4, $5, $6)",
        )
        .bind(random_uuid()?)
        .bind(request_checksum.into_bytes().to_vec())
        .bind(short_name)
        .bind(long_name)
        .bind(encoded)
        .bind(content.checksum()?.as_bytes().to_vec())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let blueprint = reference(row.try_get("reference_number").map_err(map_sqlx_error)?)?;
        let receipt = CreateBlueprintCourseReceipt {
            blueprint_revision: BlueprintRevisionReference {
                reference: blueprint,
                revision: revision(
                    row.try_get("blueprint_revision_number")
                        .map_err(map_sqlx_error)?,
                )?,
            },
            metadata_etag: metadata_etag(row.try_get("metadata_etag").map_err(map_sqlx_error)?),
            actor,
            request_checksum,
            accepted_at: timestamp(row.try_get("accepted_at_millis").map_err(map_sqlx_error)?)?,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }

    async fn save_blueprint_course(
        &self,
        session: SessionTokenHash,
        reference_value: BlueprintCourseReference,
        expected_revision: BlueprintRevision,
        request_checksum: RequestChecksum,
        input: ReplaceBlueprintCourseContentInput,
    ) -> Result<SaveBlueprintCourseReceipt, StoreError> {
        input.validate().map_err(invalid_input)?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let actor = current_actor(&mut transaction).await?;
        let prior =
            load_revision_content(&mut transaction, reference_value, expected_revision).await?;
        let requested = StoredBlueprintCourseContent::requested_question_ids_from_replace(&input);
        let pins = resolve_revision_question_pins(&mut transaction, requested, &prior).await?;
        let content = StoredBlueprintCourseContent::from_replace(input, &prior, &pins)?;
        let encoded = encode_content(&content)?;
        let row = sqlx::query(
            "SELECT resulting_blueprint_revision_number, changed, \
             (EXTRACT(EPOCH FROM accepted_at) * 1000)::bigint AS accepted_at_millis \
             FROM ple_api.save_blueprint_course($1, $2, $3, $4, $5)",
        )
        .bind(reference_number(reference_value)?)
        .bind(revision_number(expected_revision)?)
        .bind(request_checksum.into_bytes().to_vec())
        .bind(encoded)
        .bind(content.checksum()?.as_bytes().to_vec())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let receipt = SaveBlueprintCourseReceipt {
            blueprint_revision: BlueprintRevisionReference {
                reference: reference_value,
                revision: revision(
                    row.try_get("resulting_blueprint_revision_number")
                        .map_err(map_sqlx_error)?,
                )?,
            },
            changed: row.try_get("changed").map_err(map_sqlx_error)?,
            actor,
            request_checksum,
            accepted_at: timestamp(row.try_get("accepted_at_millis").map_err(map_sqlx_error)?)?,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }

    async fn rename_blueprint_course(
        &self,
        session: SessionTokenHash,
        reference_value: BlueprintCourseReference,
        expected_metadata_etag: BlueprintMetadataEtag,
        input: RenameBlueprintCourseInput,
    ) -> Result<BlueprintMetadataState, StoreError> {
        question_model::validate_blueprint_course_title(&input.short_name)
            .map_err(|_| invalid("Blueprint short name"))?;
        question_model::validate_blueprint_course_title(&input.long_name)
            .map_err(|_| invalid("Blueprint long name"))?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let row = sqlx::query(
            "SELECT short_name, long_name, availability, metadata_etag \
             FROM ple_api.rename_blueprint_course($1, $2, $3, $4)",
        )
        .bind(reference_number(reference_value)?)
        .bind(expected_metadata_etag.into_uuid())
        .bind(input.short_name)
        .bind(input.long_name)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let state = decode_metadata_state(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(state)
    }

    async fn archive_blueprint(
        &self,
        session: SessionTokenHash,
        reference_value: BlueprintCourseReference,
        expected_metadata_etag: BlueprintMetadataEtag,
        confirmation_title: &str,
    ) -> Result<BlueprintMetadataState, StoreError> {
        self.set_availability(
            session,
            reference_value,
            expected_metadata_etag,
            "archived",
            Some(confirmation_title),
        )
        .await
    }

    async fn restore_blueprint(
        &self,
        session: SessionTokenHash,
        reference_value: BlueprintCourseReference,
        expected_metadata_etag: BlueprintMetadataEtag,
    ) -> Result<BlueprintMetadataState, StoreError> {
        self.set_availability(
            session,
            reference_value,
            expected_metadata_etag,
            "available",
            None,
        )
        .await
    }
}

impl PostgresBlueprintCourseStore {
    async fn set_availability(
        &self,
        session: SessionTokenHash,
        reference_value: BlueprintCourseReference,
        expected_metadata_etag: BlueprintMetadataEtag,
        availability: &'static str,
        archive_confirmation_title: Option<&str>,
    ) -> Result<BlueprintMetadataState, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let row = sqlx::query(
            "SELECT short_name, long_name, availability, metadata_etag \
             FROM ple_api.set_blueprint_availability($1, $2, $3, $4)",
        )
        .bind(reference_number(reference_value)?)
        .bind(expected_metadata_etag.into_uuid())
        .bind(availability)
        .bind(archive_confirmation_title)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = decode_metadata_state(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

async fn load_revision_content(
    transaction: &mut Transaction<'_, Postgres>,
    reference_value: BlueprintCourseReference,
    revision_value: BlueprintRevision,
) -> Result<StoredBlueprintCourseContent, StoreError> {
    let row = sqlx::query("SELECT * FROM ple_api.load_blueprint_revision($1, $2)")
        .bind(reference_number(reference_value)?)
        .bind(revision_number(revision_value)?)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
    let Json(content): Json<Value> = row.try_get("content").map_err(map_sqlx_error)?;
    let content = decode_stored_content(content)?;
    verify_content_checksum(
        &content,
        &row.try_get::<Vec<u8>, _>("content_checksum")
            .map_err(map_sqlx_error)?,
    )?;
    Ok(content)
}

async fn current_actor(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<AccountId, StoreError> {
    let account_id = sqlx::query_scalar("SELECT ple_api.current_session_account_id()")
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    Ok(AccountId::from_uuid(account_id))
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
        .map(|question_id| question_id.as_compact_str().to_owned())
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
        let number = row
            .try_get::<i32, _>("revision_number")
            .map_err(map_sqlx_error)?;
        let revision_number = u32::try_from(number)
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

/// A Save keeps an exact pin owned by its expected Revision after the Question
/// lineage is archived. New Question IDs resolve only through current
/// Available selection, and PostgreSQL rechecks the pins while locked.
async fn resolve_revision_question_pins(
    transaction: &mut Transaction<'_, Postgres>,
    requested: Vec<QuestionId>,
    prior: &StoredBlueprintCourseContent,
) -> Result<BTreeMap<QuestionId, QuestionRevisionReference>, StoreError> {
    let requested = requested.into_iter().collect::<BTreeSet<_>>();
    let available = resolve_available_question_pins(transaction, &requested).await?;
    let retained = retained_question_pins(prior)?;
    requested
        .into_iter()
        .map(|question_id| {
            retained
                .get(&question_id)
                .or_else(|| available.get(&question_id))
                .cloned()
                .ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "Blueprint Course requires currently available Published Questions"
                            .to_string(),
                    )
                })
                .map(|pin| (question_id, pin))
        })
        .collect()
}

async fn resolve_available_question_pins(
    transaction: &mut Transaction<'_, Postgres>,
    requested: &BTreeSet<QuestionId>,
) -> Result<BTreeMap<QuestionId, QuestionRevisionReference>, StoreError> {
    let identifiers = requested
        .iter()
        .map(|question_id| question_id.as_compact_str().to_owned())
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
        let number = row
            .try_get::<i32, _>("revision_number")
            .map_err(map_sqlx_error)?;
        let revision_number = u32::try_from(number)
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
    Ok(pins)
}

fn retained_question_pins(
    content: &StoredBlueprintCourseContent,
) -> Result<BTreeMap<QuestionId, QuestionRevisionReference>, StoreError> {
    let mut pins = BTreeMap::new();
    for assignment in content
        .modules
        .iter()
        .flat_map(|module| &module.assignments)
    {
        for entry in &assignment.content.entries {
            let references: Box<dyn Iterator<Item = &QuestionRevisionReference> + '_> = match entry
            {
                crate::StoredBlueprintAssignmentEntry::Fixed {
                    question_revision, ..
                } => Box::new(std::iter::once(question_revision)),
                crate::StoredBlueprintAssignmentEntry::Pool {
                    question_revisions, ..
                } => Box::new(question_revisions.iter()),
            };
            for reference in references {
                match pins.entry(reference.question_id.clone()) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(reference.clone());
                    }
                    std::collections::btree_map::Entry::Occupied(entry)
                        if entry.get() == reference => {}
                    std::collections::btree_map::Entry::Occupied(_) => {
                        return Err(invalid("retained Question Revision pins"));
                    }
                }
            }
        }
    }
    Ok(pins)
}

fn decode_summary(row: &sqlx::postgres::PgRow) -> Result<StoredBlueprintCourseSummary, StoreError> {
    Ok(StoredBlueprintCourseSummary {
        total_adoptions: u64::try_from(
            row.try_get::<i64, _>("total_adoptions")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Blueprint adoption count"))?,
        total_students_ever_enrolled: u64::try_from(
            row.try_get::<i64, _>("total_students_ever_enrolled")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Blueprint enrollment count"))?,
        reference: reference(row.try_get("reference_number").map_err(map_sqlx_error)?)?,
        short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
        long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
        availability: availability_value(row.try_get("availability").map_err(map_sqlx_error)?)?,
        metadata_etag: metadata_etag(row.try_get("metadata_etag").map_err(map_sqlx_error)?),
        current_revision: revision(
            row.try_get("current_blueprint_revision_number")
                .map_err(map_sqlx_error)?,
        )?,
        read_access: read_access(row.try_get("is_owner").map_err(map_sqlx_error)?),
    })
}

fn decode_course(row: &sqlx::postgres::PgRow) -> Result<StoredBlueprintCourse, StoreError> {
    let Json(encoded): Json<Value> = row.try_get("content").map_err(map_sqlx_error)?;
    let content = decode_stored_content(encoded)?;
    verify_content_checksum(
        &content,
        &row.try_get::<Vec<u8>, _>("content_checksum")
            .map_err(map_sqlx_error)?,
    )?;
    Ok(StoredBlueprintCourse {
        reference: reference(row.try_get("reference_number").map_err(map_sqlx_error)?)?,
        short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
        long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
        availability: availability_value(row.try_get("availability").map_err(map_sqlx_error)?)?,
        metadata_etag: metadata_etag(row.try_get("metadata_etag").map_err(map_sqlx_error)?),
        current_revision: revision(
            row.try_get("current_blueprint_revision_number")
                .map_err(map_sqlx_error)?,
        )?,
        read_access: read_access(row.try_get("is_owner").map_err(map_sqlx_error)?),
        content,
    })
}

fn encode_content(content: &StoredBlueprintCourseContent) -> Result<Value, StoreError> {
    let mut encoded = serde_json::to_value(content).map_err(|_| invalid("Blueprint Content"))?;
    compact_question_ids(&mut encoded)?;
    Ok(encoded)
}

fn decode_stored_content(encoded: Value) -> Result<StoredBlueprintCourseContent, StoreError> {
    serde_json::from_value(encoded).map_err(|_| invalid("Blueprint Content"))
}

/// PostgreSQL JSON is a machine boundary, so it stores compact Question IDs.
/// Domain serialization remains grouped for browser-facing values.
fn compact_question_ids(value: &mut Value) -> Result<(), StoreError> {
    match value {
        Value::Array(values) => {
            for value in values {
                compact_question_ids(value)?;
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                compact_question_ids(value)?;
            }
            for key in ["questionId", "question_id"] {
                if let Some(Value::String(question_id)) = values.get_mut(key) {
                    let compact = question_id
                        .parse::<QuestionId>()
                        .map_err(|_| invalid("Blueprint Question ID"))?
                        .as_compact_str()
                        .to_owned();
                    *question_id = compact;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn verify_content_checksum(
    content: &StoredBlueprintCourseContent,
    expected: &[u8],
) -> Result<(), StoreError> {
    (expected == content.checksum()?.as_bytes())
        .then_some(())
        .ok_or_else(|| invalid("Blueprint Content Checksum"))
}

fn decode_metadata_state(
    row: &sqlx::postgres::PgRow,
) -> Result<BlueprintMetadataState, StoreError> {
    Ok(BlueprintMetadataState {
        short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
        long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
        availability: availability_value(row.try_get("availability").map_err(map_sqlx_error)?)?,
        metadata_etag: metadata_etag(row.try_get("metadata_etag").map_err(map_sqlx_error)?),
    })
}

fn read_access(is_owner: bool) -> BlueprintCourseReadAccess {
    if is_owner {
        BlueprintCourseReadAccess::BlueprintCourseOwner
    } else {
        BlueprintCourseReadAccess::ActiveInstructor
    }
}

fn availability_value(value: String) -> Result<BlueprintAvailability, StoreError> {
    match value.as_str() {
        "available" => Ok(BlueprintAvailability::Available),
        "archived" => Ok(BlueprintAvailability::Archived),
        _ => Err(invalid("Blueprint availability")),
    }
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
fn metadata_etag(value: uuid::Uuid) -> BlueprintMetadataEtag {
    BlueprintMetadataEtag::from_uuid(value)
}
fn reference_number(value: BlueprintCourseReference) -> Result<i64, StoreError> {
    Ok(i64::from(value.number()))
}
fn revision_number(value: BlueprintRevision) -> Result<i64, StoreError> {
    i64::try_from(value.value()).map_err(|_| invalid("Blueprint Revision"))
}
fn timestamp(value: i64) -> Result<Timestamp, StoreError> {
    Ok(Timestamp::from_unix_millis(value))
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::compact_question_ids;

    #[test]
    fn postgres_blueprint_json_uses_compact_question_ids() {
        let mut value = json!({
            "questionId": "ABC-DEFG",
            "nested": [{"question_id": "234-5678"}],
        });

        compact_question_ids(&mut value).expect("valid Question IDs");

        assert_eq!(value["questionId"], "ABCDEFG");
        assert_eq!(value["nested"][0]["question_id"], "2345678");
    }
}
