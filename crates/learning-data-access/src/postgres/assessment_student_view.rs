//! PostgreSQL persistence for the read-only Instructor Student View.

use std::num::NonZeroU32;

use async_trait::async_trait;
use question_model::{
    AccountTimeZone, AssessmentEditNumber, AssessmentEntryAvailability, AssessmentEntryId,
    AssessmentEntryScoringRule, AssessmentInstructions, AssessmentPointValue,
    AssessmentQuestionOrderRule, AssessmentId, AssessmentStatus, AssessmentTitle,
    CourseInstanceId, DraftImathasQuestionBackendBinding, ImathasDeploymentReference,
    ImathasItemReference, InstructorStudentViewDelivery, LateWorkRule, ObjectId,
    PoolRevisionMemberReference, QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionId,
    QuestionPoolAssessmentEntry, QuestionPoolRevisionNumber, QuestionPoolRevisionReference,
    QuestionPoolSelectedItem, QuestionPoolSelectedQuestionOrder, QuestionPoolSelectionRule,
    QuestionRevisionNumber, QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference,
    Timestamp,
};
use sqlx::{Postgres, Row, Transaction};

use super::{
    Pool, assessment_delivery_source::ready_question_asset_renditions, connection::map_sqlx_error,
};
use crate::{
    InstructorStudentViewSnapshot, InstructorStudentViewSnapshotEntry, InstructorStudentViewSource,
    InstructorStudentViewStore, SessionTokenHash, StoreError,
};

/// PostgreSQL Store for answer-free, non-mutating Instructor Student View reads.
#[derive(Clone)]
pub struct PostgresInstructorStudentViewStore {
    pool: Pool,
}

impl PostgresInstructorStudentViewStore {
    /// Binds the attested API pool to the Student View read procedures.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin_read_only(
        &self,
        token: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        // ASVS 2.3.3: this is deliberately the first statement after BEGIN.
        // One repeatable-read snapshot contains authorization, Assessment pins,
        // source bindings, and ready Asset renditions, and PostgreSQL rejects writes.
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
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
impl InstructorStudentViewStore for PostgresInstructorStudentViewStore {
    async fn load_instructor_student_view_snapshot(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
    ) -> Result<InstructorStudentViewSnapshot, StoreError> {
        let mut transaction = self.begin_read_only(session_token_hash).await?;
        // ASVS 1.2.4 and 8.2.2: opaque references are bound parameters and the
        // definer function rechecks the current direct Course Instructor relationship.
        let rows = sqlx::query("SELECT * FROM ple_api.load_assessment_workspace_rows($1, $2)")
            .bind(course.as_string())
            .bind(assessment.as_string())
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let first = rows.first().ok_or(StoreError::NotFound)?;
        let duration_seconds: Option<i32> = sqlx::query_scalar(
            "SELECT ple_api.read_instructor_student_view_duration_seconds($1, $2)",
        )
        .bind(course.as_string())
        .bind(assessment.as_string())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut effective_delivery = delivery(first)?;
        effective_delivery.assessment_attempt_time_limit_seconds = duration_seconds
            .map(|seconds| u32::try_from(seconds).map_err(|_| invalid("Assessment duration")))
            .transpose()?;
        let display_time_zone: String =
            sqlx::query_scalar("SELECT ple_api.current_account_time_zone()")
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let snapshot = InstructorStudentViewSnapshot {
            edit_number: edit_number(first)?,
            status: assessment_status(first)?,
            title: AssessmentTitle::try_new(column(first, "assessment_title")?)
                .map_err(|_| invalid("Assessment Title"))?,
            instructions: AssessmentInstructions::try_new(column(
                first,
                "assessment_instructions",
            )?)
            .map_err(|_| invalid("Assessment Instructions"))?,
            display_time_zone: AccountTimeZone::parse(&display_time_zone)
                .map_err(|_| invalid("Account Time Zone"))?,
            delivery: effective_delivery,
            assessment_question_order_rule: assessment_question_order_rule(first)?,
            entries: snapshot_entries(&rows)?,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(snapshot)
    }

    async fn load_instructor_student_view_question_source(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
        expected_edit_number: AssessmentEditNumber,
        authored_position: u32,
        question_revision: QuestionRevisionReference,
    ) -> Result<InstructorStudentViewSource, StoreError> {
        let mut transaction = self.begin_read_only(session_token_hash).await?;
        let authored_position = i32::try_from(authored_position)
            .map_err(|_| invalid("Assessment authored position"))?;
        let expected_edit_number = i64::try_from(expected_edit_number.value())
            .map_err(|_| invalid("Assessment Edit Number"))?;
        let revision_number = i32::try_from(question_revision.revision_number.get())
            .map_err(|_| invalid("Question Revision Number"))?;
        // ASVS 1.2.4, 2.2.2, and 8.3.1: the typed definer boundary validates
        // the full current manifest precondition and exact authored source pin.
        let row = sqlx::query(
            "SELECT * FROM ple_api.load_instructor_student_view_question_source(\
             $1, $2, $3, $4, $5, $6)",
        )
        .bind(course.as_string())
        .bind(assessment.as_string())
        .bind(expected_edit_number)
        .bind(authored_position)
        .bind(question_revision.question_id.as_str())
        .bind(revision_number)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_student_view_source_error)?;
        let source = source_from_row(&row, question_revision)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(source)
    }
}

fn delivery(row: &sqlx::postgres::PgRow) -> Result<InstructorStudentViewDelivery, StoreError> {
    Ok(InstructorStudentViewDelivery {
        available_at: optional_timestamp(row, "available_at_millis")?,
        due_at: optional_timestamp(row, "due_at_millis")?,
        closes_at: optional_timestamp(row, "closes_at_millis")?,
        assessment_attempt_time_limit_seconds: optional_positive_i32(
            row,
            "assessment_attempt_time_limit_seconds",
            "Assessment Attempt Time Limit",
        )?,
        attempt_limit: optional_positive_i32(row, "attempt_limit", "Assessment Attempt Limit")?,
        late_work_rule: match column::<String>(row, "late_work_rule")?.as_str() {
            "accept" => LateWorkRule::Accept,
            "mark_late" => LateWorkRule::MarkLate,
            "reject" => LateWorkRule::Reject,
            _ => return Err(invalid("Late Work Rule")),
        },
    })
}

fn snapshot_entries(
    rows: &[sqlx::postgres::PgRow],
) -> Result<Vec<InstructorStudentViewSnapshotEntry>, StoreError> {
    let mut entries = Vec::new();
    let mut index = 0;
    let mut previous_authored_position = None;
    while let Some(row) = rows.get(index) {
        let Some(raw_entry_id) = row
            .try_get::<Option<uuid::Uuid>, _>("assessment_entry_id")
            .map_err(map_sqlx_error)?
        else {
            if rows.len() != 1 {
                return Err(invalid("Assessment Entry rows"));
            }
            break;
        };
        let authored_position = unsigned_i32(row, "authored_position", "Authored position")?;
        if previous_authored_position.is_some_and(|previous| previous >= authored_position) {
            return Err(invalid("Assessment Entry authored order"));
        }
        previous_authored_position = Some(authored_position);
        let availability = entry_availability(row)?;
        let entry_id = AssessmentEntryId::from_uuid(raw_entry_id);
        match column::<String>(row, "entry_kind")?.as_str() {
            "fixed_question" => {
                entries.push(InstructorStudentViewSnapshotEntry::Fixed {
                    authored_position,
                    availability,
                    question_revision: question_revision(row)?,
                });
                index += 1;
            }
            "question_pool" => {
                let assessment_entry = pool_assessment_entry(row, entry_id, availability)?;
                let expected_pool_revision = assessment_entry.question_pool_revision.clone();
                let mut members = Vec::new();
                let mut previous_member_position = None;
                while let Some(member_row) = rows.get(index) {
                    let member_entry_id = member_row
                        .try_get::<Option<uuid::Uuid>, _>("assessment_entry_id")
                        .map_err(map_sqlx_error)?;
                    if member_entry_id != Some(raw_entry_id) {
                        break;
                    }
                    if unsigned_i32(member_row, "authored_position", "Authored position")?
                        != authored_position
                    {
                        return Err(invalid("Question Pool authored position"));
                    }
                    let member_position = unsigned_i32(
                        member_row,
                        "member_position",
                        "Question Pool member position",
                    )?
                    .checked_sub(1)
                    .ok_or_else(|| invalid("Question Pool member position"))?;
                    if previous_member_position.is_some_and(|previous| previous >= member_position)
                    {
                        return Err(invalid("Question Pool member order"));
                    }
                    previous_member_position = Some(member_position);
                    members.push(QuestionPoolSelectedItem {
                        pool_revision_member: PoolRevisionMemberReference {
                            question_pool_revision: expected_pool_revision.clone(),
                            member_position,
                        },
                        reference: question_revision(member_row)?,
                    });
                    index += 1;
                }
                if members.is_empty() {
                    return Err(invalid("Question Pool members"));
                }
                entries.push(InstructorStudentViewSnapshotEntry::Pool {
                    authored_position,
                    availability,
                    assessment_entry,
                    members,
                });
            }
            _ => return Err(invalid("Assessment Entry kind")),
        }
    }
    Ok(entries)
}

fn pool_assessment_entry(
    row: &sqlx::postgres::PgRow,
    id: AssessmentEntryId,
    availability: AssessmentEntryAvailability,
) -> Result<QuestionPoolAssessmentEntry, StoreError> {
    let question_pool_revision = QuestionPoolRevisionReference {
        question_pool_id: column::<String>(row, "question_pool_public_id")?
            .parse::<QuestionId>()
            .map_err(|_| invalid("Question Pool ID"))?,
        revision_number: QuestionPoolRevisionNumber::new(unsigned_i64(
            row,
            "question_pool_revision_number",
            "Question Pool Revision Number",
        )?)
        .map_err(|_| invalid("Question Pool Revision Number"))?,
    };
    let selection_count = NonZeroU32::new(unsigned_i32(
        row,
        "selection_count",
        "Question Pool selection count",
    )?)
    .ok_or_else(|| invalid("Question Pool selection count"))?;
    Ok(QuestionPoolAssessmentEntry {
        id,
        question_pool_revision,
        availability,
        scoring_rule: entry_scoring_rule(row)?,
        selection_count,
        points_per_item: point_value(row, "points_per_item")?,
        selection_rule: QuestionPoolSelectionRule {
            selected_question_order: match column::<String>(row, "selected_question_order")?
                .as_str()
            {
                "question_pool_order" => QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
                "random_order" => QuestionPoolSelectedQuestionOrder::RandomOrder,
                _ => return Err(invalid("Question Pool selected Question order")),
            },
        },
        question_attempt_limit: QuestionAttemptLimit {
            max_attempts: optional_positive_i32(
                row,
                "question_attempt_limit",
                "Question Attempt Limit",
            )?,
        },
        question_attempt_time_limit: question_attempt_time_limit(row)?,
    })
}

fn source_from_row(
    row: &sqlx::postgres::PgRow,
    question_revision: QuestionRevisionReference,
) -> Result<InstructorStudentViewSource, StoreError> {
    let source_object_reference = SourceObjectReference {
        object: ObjectId::from_uuid(column(row, "source_object_id")?),
    };
    let source_object_checksum =
        SourceObjectChecksum::parse(column::<String>(row, "source_object_checksum")?)
            .map_err(|_| invalid("Question Source Object checksum"))?;
    let source_media_type = column(row, "source_media_type")?;
    let question_asset_renditions = ready_question_asset_renditions(row)?;
    let webwork_pg_path = optional_column(row, "webwork_pg_path")?;
    let imathas_deployment = optional_column(row, "imathas_deployment_reference")?;
    let imathas_item = optional_column(row, "imathas_item_reference")?;
    match (
        column::<String>(row, "backend")?.as_str(),
        webwork_pg_path,
        imathas_deployment,
        imathas_item,
    ) {
        ("ple", None, None, None) => Ok(InstructorStudentViewSource::Ple {
            question_revision,
            source_object_reference,
            source_object_checksum,
            source_media_type,
            question_asset_renditions,
        }),
        ("webwork", Some(webwork_pg_path), None, None) => {
            Ok(InstructorStudentViewSource::Webwork {
                question_revision,
                source_object_reference,
                source_object_checksum,
                source_media_type,
                webwork_pg_path,
                question_asset_renditions,
            })
        }
        ("imathas", None, Some(deployment), Some(item)) => {
            let deployment = ImathasDeploymentReference::new(deployment)
                .map_err(|_| invalid("iMathAS deployment reference"))?;
            let item =
                ImathasItemReference::new(item).map_err(|_| invalid("iMathAS item reference"))?;
            Ok(InstructorStudentViewSource::Imathas {
                question_revision,
                source_object_reference,
                source_object_checksum,
                source_media_type,
                imathas_question_backend_binding: DraftImathasQuestionBackendBinding::new(
                    deployment, item,
                ),
                question_asset_renditions,
            })
        }
        _ => Err(invalid("Question Source Backend binding")),
    }
}

fn edit_number(row: &sqlx::postgres::PgRow) -> Result<AssessmentEditNumber, StoreError> {
    AssessmentEditNumber::new(unsigned_i64(
        row,
        "assessment_edit_number",
        "Assessment Edit Number",
    )?)
    .ok_or_else(|| invalid("Assessment Edit Number"))
}

fn assessment_status(row: &sqlx::postgres::PgRow) -> Result<AssessmentStatus, StoreError> {
    match column::<String>(row, "assessment_status")?.as_str() {
        "unreleased" => Ok(AssessmentStatus::Unreleased),
        "released" => Ok(AssessmentStatus::Released),
        "closed" => Ok(AssessmentStatus::Closed),
        "archived" => Ok(AssessmentStatus::Archived),
        _ => Err(invalid("Assessment Status")),
    }
}

fn assessment_question_order_rule(
    row: &sqlx::postgres::PgRow,
) -> Result<AssessmentQuestionOrderRule, StoreError> {
    match column::<String>(row, "assessment_question_order_rule")?.as_str() {
        "authored_order" => Ok(AssessmentQuestionOrderRule::AuthoredOrder),
        "shuffled" => Ok(AssessmentQuestionOrderRule::Shuffled),
        _ => Err(invalid("Assessment Question Order Rule")),
    }
}

fn entry_availability(
    row: &sqlx::postgres::PgRow,
) -> Result<AssessmentEntryAvailability, StoreError> {
    match column::<String>(row, "entry_availability")?.as_str() {
        "available" => Ok(AssessmentEntryAvailability::Available),
        "retired" => Ok(AssessmentEntryAvailability::Retired),
        _ => Err(invalid("Assessment Entry availability")),
    }
}

fn entry_scoring_rule(
    row: &sqlx::postgres::PgRow,
) -> Result<AssessmentEntryScoringRule, StoreError> {
    match column::<String>(row, "scoring_rule")?.as_str() {
        "normal" => Ok(AssessmentEntryScoringRule::Normal),
        "full_credit" => Ok(AssessmentEntryScoringRule::FullCredit),
        "extra_credit" => Ok(AssessmentEntryScoringRule::ExtraCredit),
        "excluded" => Ok(AssessmentEntryScoringRule::Excluded),
        _ => Err(invalid("Assessment Entry scoring rule")),
    }
}

fn question_revision(row: &sqlx::postgres::PgRow) -> Result<QuestionRevisionReference, StoreError> {
    let question_id = column::<String>(row, "question_id")?
        .parse::<QuestionId>()
        .map_err(|_| invalid("Question ID"))?;
    let revision_number = QuestionRevisionNumber::new(unsigned_i32(
        row,
        "question_revision_number",
        "Question Revision Number",
    )?)
    .map_err(|_| invalid("Question Revision Number"))?;
    Ok(QuestionRevisionReference {
        question_id,
        revision_number,
    })
}

fn question_attempt_time_limit(
    row: &sqlx::postgres::PgRow,
) -> Result<QuestionAttemptTimeLimit, StoreError> {
    let seconds = optional_positive_i32(
        row,
        "question_attempt_time_limit_seconds",
        "Question Attempt Time Limit",
    )?;
    let grace = row
        .try_get::<Option<i32>, _>("question_attempt_grace_seconds")
        .map_err(map_sqlx_error)?;
    match (seconds, grace) {
        (None, None) => Ok(QuestionAttemptTimeLimit::Unlimited),
        (Some(seconds), Some(grace_seconds)) => Ok(QuestionAttemptTimeLimit::Limited {
            seconds,
            grace_seconds: u32::try_from(grace_seconds)
                .map_err(|_| invalid("Question Attempt grace period"))?,
        }),
        _ => Err(invalid("Question Attempt time limit")),
    }
}

fn point_value(
    row: &sqlx::postgres::PgRow,
    name: &str,
) -> Result<AssessmentPointValue, StoreError> {
    column::<bigdecimal::BigDecimal>(row, name)?
        .to_string()
        .parse()
        .map_err(|_| invalid("Assessment point value"))
}

fn optional_timestamp(
    row: &sqlx::postgres::PgRow,
    name: &str,
) -> Result<Option<Timestamp>, StoreError> {
    Ok(row
        .try_get::<Option<i64>, _>(name)
        .map_err(map_sqlx_error)?
        .map(Timestamp::from_unix_millis))
}

fn optional_positive_i32(
    row: &sqlx::postgres::PgRow,
    name: &str,
    label: &str,
) -> Result<Option<u32>, StoreError> {
    row.try_get::<Option<i32>, _>(name)
        .map_err(map_sqlx_error)?
        .map(|value| u32::try_from(value).map_err(|_| invalid(label)))
        .transpose()
        .and_then(|value| {
            if value == Some(0) {
                Err(invalid(label))
            } else {
                Ok(value)
            }
        })
}

fn unsigned_i32(row: &sqlx::postgres::PgRow, name: &str, label: &str) -> Result<u32, StoreError> {
    u32::try_from(column::<i32>(row, name)?).map_err(|_| invalid(label))
}

fn unsigned_i64(row: &sqlx::postgres::PgRow, name: &str, label: &str) -> Result<u64, StoreError> {
    u64::try_from(column::<i64>(row, name)?).map_err(|_| invalid(label))
}

fn column<T>(row: &sqlx::postgres::PgRow, name: &str) -> Result<T, StoreError>
where
    for<'r> T: sqlx::Decode<'r, Postgres> + sqlx::Type<Postgres>,
{
    row.try_get(name).map_err(map_sqlx_error)
}

fn optional_column(row: &sqlx::postgres::PgRow, name: &str) -> Result<Option<String>, StoreError> {
    row.try_get(name).map_err(map_sqlx_error)
}

fn map_student_view_source_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("40001")
    {
        return StoreError::Conflict;
    }
    map_sqlx_error(error)
}

fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("{label} is invalid"))
}
