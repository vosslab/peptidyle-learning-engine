//! PostgreSQL persistence for the current published Blueprint Course lifecycle.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use question_model::{
    BlueprintCourseReadAccess, BlueprintCourseReference, BlueprintRevision,
    CreateBlueprintCourseContentInput, QuestionId, QuestionRevisionNumber,
    QuestionRevisionReference, ReplaceBlueprintCourseContentInput,
};
use serde_json::Value;
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
    let Json(encoded_content): Json<Value> = row
        .try_get("blueprint_course_content")
        .map_err(map_sqlx_error)?;
    let encoding_version = row
        .try_get::<i16, _>("blueprint_content_encoding_version")
        .map_err(map_sqlx_error)
        .and_then(|value| {
            u8::try_from(value).map_err(|_| invalid("Blueprint Content encoding version"))
        })?;
    let content = decode_stored_content(encoded_content, encoding_version)?;
    let title: String = row.try_get("title").map_err(map_sqlx_error)?;
    if content.title != title {
        return Err(invalid("Blueprint Revision title"));
    }
    let expected_checksum: Vec<u8> = row
        .try_get("blueprint_content_checksum")
        .map_err(map_sqlx_error)?;
    verify_content_checksum(&content, encoding_version, &expected_checksum)?;
    Ok(StoredBlueprintCourse {
        reference: reference(row.try_get("reference_number").map_err(map_sqlx_error)?)?,
        revision: revision(row.try_get("revision_number").map_err(map_sqlx_error)?)?,
        read_access: read_access(row.try_get("is_owner").map_err(map_sqlx_error)?),
        content,
    })
}

fn verify_content_checksum(
    content: &StoredBlueprintCourseContent,
    encoding_version: u8,
    expected_checksum: &[u8],
) -> Result<(), StoreError> {
    // ASVS 2.3.1: declared encoding selects the only checksum schema that
    // can authorize this immutable revision's stored meaning.
    let actual_checksum = content
        .checksum_for_encoding_version(encoding_version)?
        .ok_or_else(|| invalid("Blueprint Content encoding version"))?;
    (expected_checksum == actual_checksum.as_bytes())
        .then_some(())
        .ok_or_else(|| invalid("Blueprint Content Checksum"))
}

fn decode_stored_content(
    mut encoded_content: Value,
    encoding_version: u8,
) -> Result<StoredBlueprintCourseContent, StoreError> {
    // ASVS 1.5.2: a closed storage version selects strict deserialization;
    // no public DTO receives a compatibility default.
    match encoding_version {
        2 => add_legacy_submitted_response_default(&mut encoded_content)?,
        3 => {}
        _ => return Err(invalid("Blueprint Content encoding version")),
    }
    serde_json::from_value(encoded_content).map_err(|_| invalid("Blueprint Revision Content"))
}

fn add_legacy_submitted_response_default(content: &mut Value) -> Result<(), StoreError> {
    let modules = content
        .get_mut("modules")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| invalid("Blueprint Revision Content"))?;
    for module in modules {
        let assignments = module
            .get_mut("assignments")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| invalid("Blueprint Revision Content"))?;
        for assignment in assignments {
            let feedback = assignment
                .get_mut("content")
                .and_then(|value| value.get_mut("defaults"))
                .and_then(|value| value.get_mut("student_feedback_release_rule"))
                .and_then(Value::as_object_mut)
                .ok_or_else(|| invalid("Blueprint Revision Content"))?;
            if feedback.contains_key("submitted_response") {
                return Err(invalid("Blueprint Revision Content"));
            }
            feedback.insert(
                "submitted_response".to_string(),
                Value::String("after_submit".to_string()),
            );
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{decode_stored_content, verify_content_checksum};

    const LEGACY_V2_CHECKSUM: [u8; 32] = [
        254, 41, 113, 160, 76, 235, 129, 101, 175, 65, 213, 142, 249, 126, 46, 238, 159, 36, 250,
        101, 107, 33, 203, 226, 18, 64, 74, 185, 200, 46, 116, 62,
    ];

    fn legacy_v2_json() -> Value {
        serde_json::from_str(
            r#"{
                "title":"Legacy blueprint",
                "modules":[{
                    "blueprint_module_reference":"00000000-0000-0000-0000-000000000001",
                    "label":"Week 1",
                    "assignments":[{
                        "blueprint_assignment_reference":"00000000-0000-0000-0000-000000000002",
                        "content":{
                            "title":"Legacy assignment",
                            "instructions":"Explain each choice.",
                            "entries":[{
                                "kind":"fixed",
                                "question_revision":{"questionId":"7K3-M9QX","revisionNumber":1},
                                "points_possible":"3",
                                "scoring_rule":"normal",
                                "question_attempt_limit":{"maxAttempts":null},
                                "question_attempt_time_limit":{"kind":"unlimited"}
                            }],
                            "defaults":{
                                "assignment_attempt_time_limit_seconds":null,
                                "attempt_limit":null,
                                "late_work_rule":"mark_late",
                                "assignment_deadline_rule":"auto_submit",
                                "activity_rules":{
                                    "assignmentCompletionRule":{"kind":"allCorrect"},
                                    "assignmentAttemptGradeRule":"first",
                                    "assignmentAttemptContinuationRule":{"kind":"closed"},
                                    "questionPoolReuseRule":"selectAgain",
                                    "questionVariationRule":"reuseVariation",
                                    "assignmentAttemptResumeRule":"singleSession",
                                    "assignmentQuestionDisplayRule":"allQuestions",
                                    "assignmentNavigationRule":"forwardOnly",
                                    "assignmentQuestionOrderRule":"authoredOrder"
                                },
                                "student_feedback_release_rule":{
                                    "score":"after_due",
                                    "per_item_correctness":"never",
                                    "question_feedback":"after_close",
                                    "question_answer":"after_submit",
                                    "question_answer_explanation":"never",
                                    "class_statistics":"never"
                                }
                            },
                            "schedule":{"available_at":null,"due_at":null,"closes_at":null}
                        }
                    }]
                }]
            }"#,
        )
        .expect("literal legacy v2 Blueprint Content JSON")
    }

    #[test]
    fn literal_v2_content_adapts_only_the_missing_response_timing_and_preserves_its_checksum() {
        let content = decode_stored_content(legacy_v2_json(), 2).expect("legacy v2 content");
        assert_eq!(
            content.modules[0].assignments[0]
                .content
                .defaults
                .student_feedback_release_rule
                .submitted_response,
            question_model::StudentFeedbackReleaseTiming::AfterSubmit
        );
        verify_content_checksum(&content, 2, &LEGACY_V2_CHECKSUM)
            .expect("independently precomputed v2 checksum");

        let mut policy_tampered = legacy_v2_json();
        *policy_tampered
            .pointer_mut("/modules/0/assignments/0/content/defaults/late_work_rule")
            .expect("legacy late-work field") = json!("accept");
        let policy_tampered = decode_stored_content(policy_tampered, 2).expect("valid shape");
        assert!(verify_content_checksum(&policy_tampered, 2, &LEGACY_V2_CHECKSUM).is_err());

        let mut field_tampered = legacy_v2_json();
        field_tampered
            .pointer_mut("/modules/0/assignments/0/content/defaults/student_feedback_release_rule")
            .and_then(Value::as_object_mut)
            .expect("legacy feedback rule")
            .insert("submitted_response".to_string(), json!("never"));
        assert!(decode_stored_content(field_tampered, 2).is_err());

        assert!(decode_stored_content(legacy_v2_json(), 3).is_err());

        let mut v3_after_submit = legacy_v2_json();
        v3_after_submit
            .pointer_mut("/modules/0/assignments/0/content/defaults/student_feedback_release_rule")
            .and_then(Value::as_object_mut)
            .expect("legacy feedback rule")
            .insert("submitted_response".to_string(), json!("after_submit"));
        let v3_after_submit = decode_stored_content(v3_after_submit, 3).expect("strict v3 content");
        let v3_after_submit_checksum = v3_after_submit
            .checksum_for_encoding_version(3)
            .expect("v3 checksum encoding")
            .expect("recognized v3 encoding");

        let mut v3_never = legacy_v2_json();
        v3_never
            .pointer_mut("/modules/0/assignments/0/content/defaults/student_feedback_release_rule")
            .and_then(Value::as_object_mut)
            .expect("legacy feedback rule")
            .insert("submitted_response".to_string(), json!("never"));
        let v3_never = decode_stored_content(v3_never, 3).expect("strict v3 content");
        assert_ne!(
            v3_after_submit_checksum,
            v3_never
                .checksum_for_encoding_version(3)
                .expect("v3 checksum encoding")
                .expect("recognized v3 encoding")
        );
    }
}
