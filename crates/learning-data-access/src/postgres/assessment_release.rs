//! PostgreSQL persistence for the Assessment Workspace and release boundary.

use async_trait::async_trait;
use question_model::{
    AccountTimeZone, AssessmentAuthoredContentField, AssessmentEditNumber, AssessmentId,
    AssessmentStatus, AssessmentTitle, CourseInstanceId, CourseTerm, LocalDateAndTime, Timestamp,
};
use sqlx::{Postgres, Row, Transaction};

use super::{
    assessment_workspace_policy::{activity_rules, feedback_rules},
    assessment_workspace_save::{assessment_entries_json, assessment_values_json},
    connection::map_sqlx_error,
};
use crate::{
    AssessmentQuestionPickerEntry, AssessmentReleaseIssue, AssessmentReleaseValidation,
    AssessmentUnreleaseImpact, AuthoredAssessmentQuestion, CourseAssessmentSummary,
    CreateLiveAssessmentInput, DueSoonAssessmentSummary, DueSoonAssessments, LiveAssessmentStore,
    LiveAssessmentWorkspace, SaveBaseAssessmentPolicyInput, SaveLiveAssessmentInlineInput,
    SaveLiveAssessmentInput, SessionTokenHash, StoreError, UnreleasedLiveAssessment,
};

const SAVE_ASSESSMENT_SQL: &str = "SELECT * FROM ple_api.save_assessment($1, $2, $3, \
             $4::jsonb || jsonb_build_object( \
                 'available_at', CASE WHEN $5 IS NULL THEN NULL::timestamptz ELSE to_timestamp($5::double precision / 1000) END, \
                 'due_at', CASE WHEN $6 IS NULL THEN NULL::timestamptz ELSE to_timestamp($6::double precision / 1000) END, \
                 'closes_at', CASE WHEN $7 IS NULL THEN NULL::timestamptz ELSE to_timestamp($7::double precision / 1000) END \
             ), $8)";

pub use super::assessment_workspace_connection::PostgresLiveAssessmentStore;

#[path = "assessment_release_decode.rs"]
mod assessment_release_decode;
use assessment_release_decode::*;
pub(in crate::postgres) use assessment_release_decode::{assessment_id, invalid, late_work_rule};

#[async_trait]
impl LiveAssessmentStore for PostgresLiveAssessmentStore {
    async fn review_course_blueprint_update(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
    ) -> Result<crate::CourseBlueprintUpdateReview, StoreError> {
        super::assessment_blueprint_update::review_course(self, token, course_instance_id).await
    }

    async fn review_assessment_blueprint_update(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
    ) -> Result<crate::AssessmentBlueprintUpdateReview, StoreError> {
        super::assessment_blueprint_update::review(self, token, course_instance_id, assessment_id)
            .await
    }

    async fn apply_assessment_blueprint_update(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
        input: crate::ApplyAssessmentBlueprintUpdateInput,
        bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<LiveAssessmentWorkspace, StoreError> {
        super::assessment_blueprint_update::apply(
            self,
            token,
            course_instance_id,
            assessment_id,
            input,
            bloom_receipts,
        )
        .await
    }
    async fn list_assessments_due_soon(
        &self,
        token: SessionTokenHash,
    ) -> Result<DueSoonAssessments, StoreError> {
        let mut tx = self.begin(token).await?;
        // ASVS 1.2.4: the function accepts no caller-controlled identifiers;
        // it derives both the Account and Course authority from this transaction's session.
        let display_time_zone =
            sqlx::query_scalar::<_, Option<String>>("SELECT ple_api.current_account_time_zone()")
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?
                .ok_or(StoreError::NotFound)
                .and_then(|value| {
                    AccountTimeZone::parse(&value).map_err(|_| invalid("Account Time Zone"))
                })?;
        let rows = sqlx::query(concat!(
            "SELECT course_instance_id, course_long_name, assessment_id, ",
            "assessment_type, assessment_title, assessment_status, due_at_millis ",
            "FROM ple_api.list_assessments_due_soon()",
        ))
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let items = rows
            .iter()
            .map(|row| {
                Ok(DueSoonAssessmentSummary {
                    course_instance_id: assessment_release_decode::course_instance_id(
                        row.try_get("course_instance_id").map_err(map_sqlx_error)?,
                    )?,
                    course_long_name: course_name(
                        row.try_get("course_long_name").map_err(map_sqlx_error)?,
                    )?,
                    assessment_id: assessment_release_decode::assessment_id(
                        row.try_get("assessment_id").map_err(map_sqlx_error)?,
                    )?,
                    assessment_type: assessment_type(
                        row.try_get("assessment_type").map_err(map_sqlx_error)?,
                    )?,
                    assessment_title: title(
                        row.try_get("assessment_title").map_err(map_sqlx_error)?,
                    )?,
                    assessment_status: status(
                        row.try_get("assessment_status").map_err(map_sqlx_error)?,
                    )?,
                    due_at_millis: row.try_get("due_at_millis").map_err(map_sqlx_error)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(DueSoonAssessments {
            items,
            next_cursor: None,
            display_time_zone,
        })
    }

    async fn list_course_assessments(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
    ) -> Result<Vec<CourseAssessmentSummary>, StoreError> {
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, &course_instance_id).await?;
        // ASVS 1.2.3 and 8.2.2: bind the public Course ID and let the
        // session-authorized database function enforce the exact Course owner.
        let rows = sqlx::query(
            "SELECT assessment_id, assessment_type, assessment_title, due_at_millis, assessment_status, \
             assessment_edit_number FROM ple_api.list_course_assessments($1)",
        )
        .bind(course_instance_id.as_string())
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let assessments = rows
            .iter()
            .map(|row| {
                Ok(CourseAssessmentSummary {
                    id: assessment_release_decode::assessment_id(
                        row.try_get("assessment_id").map_err(map_sqlx_error)?,
                    )?,
                    assessment_type: assessment_type(
                        row.try_get("assessment_type").map_err(map_sqlx_error)?,
                    )?,
                    title: title(row.try_get("assessment_title").map_err(map_sqlx_error)?)?,
                    due_at: row
                        .try_get::<Option<i64>, _>("due_at_millis")
                        .map_err(map_sqlx_error)?
                        .map(|milliseconds| {
                            LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
                                Timestamp::from_unix_millis(milliseconds),
                                &context.term,
                                &context.account_time_zone,
                                AssessmentAuthoredContentField::DueAt,
                            )
                        })
                        .transpose()
                        .map_err(|_| invalid("Due at"))?,
                    display_time_zone: context.account_time_zone.clone(),
                    status: status(row.try_get("assessment_status").map_err(map_sqlx_error)?)?,
                    assessment_edit_number: edit(
                        row.try_get("assessment_edit_number")
                            .map_err(map_sqlx_error)?,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(assessments)
    }

    async fn list_assessment_question_picker(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
    ) -> Result<Vec<AssessmentQuestionPickerEntry>, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT published_question_id, question_revision_number, question_description, bloom_cognitive_process, bloom_knowledge_dimension, bloom_classification_edit_number FROM ple_api.list_assessment_question_picker($1)")
            .bind(course_instance_id.as_string()).fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        let records = rows
            .iter()
            .map(|row| {
                Ok(AssessmentQuestionPickerEntry {
                    published_question_revision_tuple: published_question_revision_tuple(
                        row.try_get("published_question_id")
                            .map_err(map_sqlx_error)?,
                        row.try_get("question_revision_number")
                            .map_err(map_sqlx_error)?,
                    )?,
                    description: row
                        .try_get("question_description")
                        .map_err(map_sqlx_error)?,
                    bloom: super::question_pool_library::decode_bloom(row)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(records)
    }

    async fn create_live_assessment(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        input: CreateLiveAssessmentInput,
    ) -> Result<LiveAssessmentWorkspace, StoreError> {
        let mut tx = self.begin(token).await?;
        // The database owns public Assessment-ID minting. This canonical
        // placeholder passes the domain check and is replaced by the
        // assessment INSERT trigger with a reserved public ID.
        let row = sqlx::query("SELECT * FROM ple_api.create_assessment($1::text, $2, $3, $4, $5)")
            .bind("A0000000A")
            .bind(course_instance_id.as_string())
            .bind(input.assessment_type.as_str())
            .bind(input.title.as_str())
            .bind(input.instructions.as_str())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let assessment = assessment_release_decode::assessment_id(
            row.try_get("assessment_id").map_err(map_sqlx_error)?,
        )?;
        let context = schedule_context(&mut tx, &course_instance_id).await?;
        let rows = workspace_rows(&mut tx, &course_instance_id, &assessment).await?;
        let result = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn load_live_assessment(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
    ) -> Result<LiveAssessmentWorkspace, StoreError> {
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, &course_instance_id).await?;
        let rows = workspace_rows(&mut tx, &course_instance_id, &assessment_id).await?;
        let result = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn save_live_assessment(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
        input: SaveLiveAssessmentInput,
    ) -> Result<LiveAssessmentWorkspace, StoreError> {
        input.validate()?;
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, &course_instance_id).await?;
        let due_at_millis = input
            .due_at
            .as_ref()
            .map(|value| {
                value
                    .resolve_in_account_time_zone(
                        &context.term,
                        &context.account_time_zone,
                        AssessmentAuthoredContentField::DueAt,
                    )
                    .map(|timestamp| timestamp.as_unix_millis())
                    .map_err(|_| invalid("Account-local Due at"))
            })
            .transpose()?;
        let available_at_millis = resolve_local_timestamp(
            input.available_at.as_ref(),
            &context,
            AssessmentAuthoredContentField::AvailableAt,
        )?;
        let closes_at_millis = resolve_local_timestamp(
            input.closes_at.as_ref(),
            &context,
            AssessmentAuthoredContentField::ClosesAt,
        )?;
        let values = assessment_values_json(&input)?;
        let entries = assessment_entries_json(&input.entries)?;
        sqlx::query(SAVE_ASSESSMENT_SQL)
            .bind(course_instance_id.as_string())
            .bind(assessment_id.as_string())
            .bind(
                i64::try_from(input.expected_assessment_edit_number.value())
                    .map_err(|_| invalid("Assessment Edit Number"))?,
            )
            .bind(values)
            .bind(available_at_millis)
            .bind(due_at_millis)
            .bind(closes_at_millis)
            .bind(entries)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let rows = workspace_rows(&mut tx, &course_instance_id, &assessment_id).await?;
        let result = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn save_live_assessment_inline(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
        expected_assessment_edit_number: AssessmentEditNumber,
        input: SaveLiveAssessmentInlineInput,
    ) -> Result<CourseAssessmentSummary, StoreError> {
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, &course_instance_id).await?;
        let due_at_millis = input
            .due_at
            .as_ref()
            .map(|value| {
                value
                    .resolve_in_account_time_zone(
                        &context.term,
                        &context.account_time_zone,
                        AssessmentAuthoredContentField::DueAt,
                    )
                    .map(|timestamp| timestamp.as_unix_millis())
                    .map_err(|_| invalid("Account-local Due at"))
            })
            .transpose()?;
        let row = sqlx::query(
            "SELECT assessment_id, assessment_type, assessment_title, due_at_millis, assessment_status, assessment_edit_number \
             FROM ple_api.save_assessment_inline($1, $2, $3, $4, \
             CASE WHEN $5 IS NULL THEN NULL::timestamptz ELSE to_timestamp($5::double precision / 1000) END)",
        )
        .bind(course_instance_id.as_string())
        .bind(assessment_id.as_string())
        .bind(i64::try_from(expected_assessment_edit_number.value()).map_err(|_| invalid("Assessment Edit Number"))?)
        .bind(input.title.as_str())
        .bind(due_at_millis)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let due_at = row
            .try_get::<Option<i64>, _>("due_at_millis")
            .map_err(map_sqlx_error)?
            .map(|milliseconds| {
                LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
                    Timestamp::from_unix_millis(milliseconds),
                    &context.term,
                    &context.account_time_zone,
                    AssessmentAuthoredContentField::DueAt,
                )
            })
            .transpose()
            .map_err(|_| invalid("Due at"))?;
        let result = CourseAssessmentSummary {
            id: assessment_release_decode::assessment_id(
                row.try_get("assessment_id").map_err(map_sqlx_error)?,
            )?,
            assessment_type: assessment_type(
                row.try_get("assessment_type").map_err(map_sqlx_error)?,
            )?,
            title: title(row.try_get("assessment_title").map_err(map_sqlx_error)?)?,
            due_at,
            display_time_zone: context.account_time_zone,
            status: status(row.try_get("assessment_status").map_err(map_sqlx_error)?)?,
            assessment_edit_number: edit(
                row.try_get("assessment_edit_number")
                    .map_err(map_sqlx_error)?,
            )?,
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn save_base_assessment_policy(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
        input: SaveBaseAssessmentPolicyInput,
    ) -> Result<LiveAssessmentWorkspace, StoreError> {
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, &course_instance_id).await?;
        let resolve = |value: Option<&LocalDateAndTime>, field| {
            resolve_local_timestamp(value, &context, field)
        };
        let available_at_millis = resolve(
            input.available_at.as_ref(),
            AssessmentAuthoredContentField::AvailableAt,
        )?;
        let due_at_millis = resolve(input.due_at.as_ref(), AssessmentAuthoredContentField::DueAt)?;
        let closes_at_millis = resolve(
            input.closes_at.as_ref(),
            AssessmentAuthoredContentField::ClosesAt,
        )?;
        let values = super::assessment_workspace_save::base_assessment_policy_values_json(&input);
        sqlx::query("SELECT * FROM ple_api.save_assessment_policies($1, $2, $3, $4::jsonb || jsonb_build_object('available_at', CASE WHEN $5 IS NULL THEN NULL::timestamptz ELSE to_timestamp($5::double precision / 1000) END, 'due_at', CASE WHEN $6 IS NULL THEN NULL::timestamptz ELSE to_timestamp($6::double precision / 1000) END, 'closes_at', CASE WHEN $7 IS NULL THEN NULL::timestamptz ELSE to_timestamp($7::double precision / 1000) END))")
            .bind(course_instance_id.as_string()).bind(assessment_id.as_string())
            .bind(i64::try_from(input.expected_assessment_edit_number.value()).map_err(|_| invalid("Assessment Edit Number"))?)
            .bind(values).bind(available_at_millis).bind(due_at_millis).bind(closes_at_millis)
            .fetch_one(&mut *tx).await.map_err(map_sqlx_error)?;
        let rows = workspace_rows(&mut tx, &course_instance_id, &assessment_id).await?;
        let result = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn validate_live_assessment_release(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
    ) -> Result<AssessmentReleaseValidation, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT issue FROM ple_api.validate_assessment_release($1, $2)")
            .bind(course_instance_id.as_string())
            .bind(assessment_id.as_string())
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let issues = rows
            .iter()
            .map(|row| {
                match row
                    .try_get::<String, _>("issue")
                    .map_err(map_sqlx_error)?
                    .as_str()
                {
                    "questions_required" => Ok(AssessmentReleaseIssue::NoPublishedQuestions),
                    "question_count_exceeded" => Ok(AssessmentReleaseIssue::QuestionCountExceeded),
                    "question_pool_insufficient_items" => {
                        Ok(AssessmentReleaseIssue::QuestionUnavailable)
                    }
                    "due_date_required" => Ok(AssessmentReleaseIssue::DueDateRequired),
                    "due_date_less_than_24_hours_ahead" => {
                        Ok(AssessmentReleaseIssue::DueDateLessThan24HoursAhead)
                    }
                    "due_date_after_course_active_until" => {
                        Ok(AssessmentReleaseIssue::DueDateAfterCourseActiveUntil)
                    }
                    "availability_after_due_date" => {
                        Ok(AssessmentReleaseIssue::AvailabilityAfterDueDate)
                    }
                    "due_date_after_close" => Ok(AssessmentReleaseIssue::DueDateAfterClose),
                    _ => Err(invalid("Assessment Release Issue")),
                }
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(AssessmentReleaseValidation {
            can_release: issues.is_empty(),
            issues,
        })
    }

    async fn release_live_assessment(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
        expected_assessment_edit_number: AssessmentEditNumber,
    ) -> Result<LiveAssessmentWorkspace, StoreError> {
        let mut tx = self.begin(token).await?;
        let _row = sqlx::query("SELECT * FROM ple_api.release_assessment($1, $2, $3)")
            .bind(course_instance_id.as_string())
            .bind(assessment_id.as_string())
            .bind(
                i64::try_from(expected_assessment_edit_number.value())
                    .map_err(|_| invalid("Assessment Edit Number"))?,
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let context = schedule_context(&mut tx, &course_instance_id).await?;
        let rows = workspace_rows(&mut tx, &course_instance_id, &assessment_id).await?;
        let workspace = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(workspace)
    }

    async fn read_live_assessment_unrelease_impact(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
    ) -> Result<AssessmentUnreleaseImpact, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT * FROM ple_api.read_assessment_unrelease_impact($1, $2)")
            .bind(course_instance_id.as_string())
            .bind(assessment_id.as_string())
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_unrelease_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let impact = decode_unrelease_impact(&row)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(impact)
    }

    async fn unrelease_live_assessment(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
        expected: AssessmentEditNumber,
        confirmation_title: AssessmentTitle,
    ) -> Result<UnreleasedLiveAssessment, StoreError> {
        let mut tx = self.begin(token).await?;
        // ASVS 1.2.3 and 2.3.1: all caller data is bound, while the
        // SECURITY DEFINER procedure owns authorization, lock ordering,
        // transition, closure deletion, retained anonymous totals, and audit receipt.
        let row = sqlx::query("SELECT * FROM ple_api.unrelease_assessment($1, $2, $3, $4)")
            .bind(course_instance_id.as_string())
            .bind(assessment_id.as_string())
            .bind(i64::try_from(expected.value()).map_err(|_| invalid("Assessment Edit Number"))?)
            .bind(confirmation_title.as_str())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_unrelease_sqlx_error)?;
        let deleted = decode_unrelease_impact(&row)?;
        let context = schedule_context(&mut tx, &course_instance_id).await?;
        let rows = workspace_rows(&mut tx, &course_instance_id, &assessment_id).await?;
        let assessment = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        if assessment.status != AssessmentStatus::Unreleased {
            return Err(invalid("Unreleased Assessment status"));
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(UnreleasedLiveAssessment {
            assessment,
            deleted,
        })
    }
}

fn decode_unrelease_impact(
    row: &sqlx::postgres::PgRow,
) -> Result<AssessmentUnreleaseImpact, StoreError> {
    let submission_count = count(row, "assessment_submission_count")?;
    Ok(AssessmentUnreleaseImpact {
        confirmation_title: title(row.try_get("assessment_title").map_err(map_sqlx_error)?)?,
        assessment_edit_number: edit(
            row.try_get("assessment_edit_number")
                .map_err(map_sqlx_error)?,
        )?,
        attempt_count: count(row, "assessment_attempt_count")?,
        submission_count,
        grade_count: count(row, "grading_result_count")?,
    })
}

fn count(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64, StoreError> {
    u64::try_from(row.try_get::<i64, _>(column).map_err(map_sqlx_error)?)
        .map_err(|_| invalid("Assessment Unrelease count"))
}

fn map_unrelease_sqlx_error(error: sqlx::Error) -> StoreError {
    // The procedure reserves 40001 for its compare-and-swap precondition.
    // Other Store paths retain generic serialization retry handling.
    if let sqlx::Error::Database(database_error) = &error {
        match database_error.code().as_deref() {
            Some("40001") => return StoreError::Conflict,
            Some("55000") => return StoreError::LifecycleConflict,
            _ => {}
        }
    }
    map_sqlx_error(error)
}

pub(super) async fn workspace_rows(
    tx: &mut Transaction<'_, Postgres>,
    course_instance_id: &CourseInstanceId,
    assessment_id: &AssessmentId,
) -> Result<Vec<sqlx::postgres::PgRow>, StoreError> {
    sqlx::query("SELECT * FROM ple_api.load_assessment_workspace_rows($1, $2)")
        .bind(course_instance_id.as_string())
        .bind(assessment_id.as_string())
        .fetch_all(&mut **tx)
        .await
        .map_err(map_sqlx_error)
}

pub(super) struct AssessmentScheduleContext {
    term: CourseTerm,
    account_time_zone: AccountTimeZone,
}

pub(super) async fn schedule_context(
    tx: &mut Transaction<'_, Postgres>,
    course_instance_id: &CourseInstanceId,
) -> Result<AssessmentScheduleContext, StoreError> {
    let row = sqlx::query(
        "SELECT term_starts_on::text AS term_starts_on, term_ends_on::text AS term_ends_on \
         FROM ple_api.load_course_instance($1)",
    )
    .bind(course_instance_id.as_string())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let starts_on: String = row.try_get("term_starts_on").map_err(map_sqlx_error)?;
    let ends_on: String = row.try_get("term_ends_on").map_err(map_sqlx_error)?;
    let account_time_zone: String =
        sqlx::query_scalar("SELECT ple_api.current_account_time_zone()")
            .fetch_one(&mut **tx)
            .await
            .map_err(map_sqlx_error)?;
    let account_time_zone =
        AccountTimeZone::parse(&account_time_zone).map_err(|_| invalid("Account Time Zone"))?;
    // CourseTerm carries only inclusive calendar dates; the Account zone resolves wall-clock input.
    let term = CourseTerm::from_parts(&starts_on, &ends_on).map_err(|_| invalid("Course Term"))?;
    Ok(AssessmentScheduleContext {
        term,
        account_time_zone,
    })
}

pub(super) fn decode_workspace(
    rows: &[sqlx::postgres::PgRow],
    context: &AssessmentScheduleContext,
) -> Result<Option<LiveAssessmentWorkspace>, StoreError> {
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let questions = rows
        .iter()
        .filter_map(|row| {
            row.try_get::<Option<String>, _>("published_question_id")
                .ok()
                .flatten()
                .map(|published_question_id| {
                    Ok(AuthoredAssessmentQuestion {
                        published_question_revision_tuple: published_question_revision_tuple(
                            published_question_id,
                            row.try_get("question_revision_number")
                                .map_err(map_sqlx_error)?,
                        )?,
                        description: row
                            .try_get("question_description")
                            .map_err(map_sqlx_error)?,
                        bloom: super::question_pool_library::decode_bloom(row)?,
                    })
                })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(Some(LiveAssessmentWorkspace {
        id: assessment_release_decode::assessment_id(
            first.try_get("assessment_id").map_err(map_sqlx_error)?,
        )?,
        assessment_edit_number: edit(
            first
                .try_get("assessment_edit_number")
                .map_err(map_sqlx_error)?,
        )?,
        status: status(first.try_get("assessment_status").map_err(map_sqlx_error)?)?,
        origin: assessment_origin(first)?,
        assessment_type: assessment_type(
            first.try_get("assessment_type").map_err(map_sqlx_error)?,
        )?,
        title: title(first.try_get("assessment_title").map_err(map_sqlx_error)?)?,
        instructions: instructions(
            first
                .try_get("assessment_instructions")
                .map_err(map_sqlx_error)?,
        )?,
        due_at: local_timestamp_from_row(
            first,
            "due_at_millis",
            context,
            AssessmentAuthoredContentField::DueAt,
        )?,
        available_at: local_timestamp_from_row(
            first,
            "available_at_millis",
            context,
            AssessmentAuthoredContentField::AvailableAt,
        )?,
        closes_at: local_timestamp_from_row(
            first,
            "closes_at_millis",
            context,
            AssessmentAuthoredContentField::ClosesAt,
        )?,
        late_work_rule: parse_late_work_rule(
            first.try_get("late_work_rule").map_err(map_sqlx_error)?,
        )?,
        assessment_attempt_time_limit_seconds: nonzero_optional(
            first
                .try_get("assessment_attempt_time_limit_seconds")
                .map_err(map_sqlx_error)?,
            "Assessment Attempt Time Limit",
        )?,
        attempt_limit: nonzero_optional(
            first.try_get("attempt_limit").map_err(map_sqlx_error)?,
            "Assessment Attempt Limit",
        )?,
        activity_rules: activity_rules(first)?,
        student_feedback_release_rule: feedback_rules(first)?,
        display_time_zone: context.account_time_zone.clone(),
        entries: decode_entries(rows)?,
        questions,
    }))
}

#[cfg(test)]
mod tests {
    use super::SAVE_ASSESSMENT_SQL;

    #[test]
    fn save_assessment_query_names_the_canonical_api() {
        assert!(SAVE_ASSESSMENT_SQL.contains("ple_api.save_assessment"));
    }
}
