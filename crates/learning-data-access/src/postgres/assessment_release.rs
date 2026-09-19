//! PostgreSQL persistence for the Assessment Workspace and release boundary.

use async_trait::async_trait;
use question_model::{
    AccountTimeZone, AssessmentAuthoredContentField, AssessmentEditNumber, AssessmentEntry,
    AssessmentEntryAvailability, AssessmentEntryId, AssessmentEntryScoringRule, AssessmentId,
    AssessmentInstructions, AssessmentOrigin, AssessmentPointValue, AssessmentStatus,
    AssessmentTitle, AssessmentType, BlueprintAssessmentId, BlueprintAssessmentSource,
    BlueprintCourseId, BlueprintRevision, BlueprintRevisionReference, CourseInstanceId, CourseTerm,
    FixedQuestionAssessmentEntry, LateWorkRule, LocalDateAndTime, QuestionAttemptLimit,
    QuestionAttemptTimeLimit, QuestionId, QuestionPoolAssessmentEntry, QuestionPoolRevisionNumber,
    QuestionPoolRevisionReference, QuestionPoolSelectedQuestionOrder, QuestionPoolSelectionRule,
    QuestionRevisionNumber, QuestionRevisionReference, Timestamp,
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

pub use super::assessment_workspace_connection::PostgresLiveAssessmentStore;

#[async_trait]
impl LiveAssessmentStore for PostgresLiveAssessmentStore {
    async fn review_course_blueprint_update(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceId,
    ) -> Result<crate::CourseBlueprintUpdateReview, StoreError> {
        super::assessment_blueprint_update::review_course(self, token, course).await
    }

    async fn review_assessment_blueprint_update(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
    ) -> Result<crate::AssessmentBlueprintUpdateReview, StoreError> {
        super::assessment_blueprint_update::review(self, token, course, assessment).await
    }

    async fn apply_assessment_blueprint_update(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
        input: crate::ApplyAssessmentBlueprintUpdateInput,
        bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<LiveAssessmentWorkspace, StoreError> {
        super::assessment_blueprint_update::apply(
            self,
            token,
            course,
            assessment,
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
            "SELECT course_reference_number, course_long_name, assessment_reference_number, ",
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
                    course_reference: course_reference(
                        row.try_get("course_reference_number")
                            .map_err(map_sqlx_error)?,
                    )?,
                    course_long_name: course_name(
                        row.try_get("course_long_name").map_err(map_sqlx_error)?,
                    )?,
                    assessment_reference: assessment_reference(
                        row.try_get("assessment_reference_number")
                            .map_err(map_sqlx_error)?,
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
        course: CourseInstanceId,
    ) -> Result<Vec<CourseAssessmentSummary>, StoreError> {
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, &course).await?;
        // ASVS 1.2.3 and 8.2.2: bind the public Course Reference and let the
        // session-authorized database function enforce the exact Course owner.
        let rows = sqlx::query(
            "SELECT assessment_reference_number, assessment_type, assessment_title, due_at_millis, assessment_status, \
             assessment_edit_number FROM ple_api.list_course_assessments($1)",
        )
        .bind(course.as_string())
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let assessments = rows
            .iter()
            .map(|row| {
                Ok(CourseAssessmentSummary {
                    reference: assessment_reference(
                        row.try_get("assessment_reference_number")
                            .map_err(map_sqlx_error)?,
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
                    edit_number: edit(
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
        course: CourseInstanceId,
    ) -> Result<Vec<AssessmentQuestionPickerEntry>, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT question_id, question_revision_number, question_description, bloom_cognitive_process, bloom_knowledge_dimension, bloom_classification_edit_number FROM ple_api.list_assessment_question_picker($1)")
            .bind(course.as_string()).fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        let records = rows
            .iter()
            .map(|row| {
                Ok(AssessmentQuestionPickerEntry {
                    reference: question_revision_reference(
                        row.try_get("question_id").map_err(map_sqlx_error)?,
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
        course: CourseInstanceId,
        input: CreateLiveAssessmentInput,
    ) -> Result<LiveAssessmentWorkspace, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT * FROM ple_api.create_assessment($1, $2, $3, $4, $5)")
            .bind(random_uuid()?)
            .bind(course.as_string())
            .bind(input.assessment_type.as_str())
            .bind(input.title.as_str())
            .bind(input.instructions.as_str())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let assessment = assessment_reference(
            row.try_get("assessment_reference_number")
                .map_err(map_sqlx_error)?,
        )?;
        let context = schedule_context(&mut tx, &course).await?;
        let rows = workspace_rows(&mut tx, &course, &assessment).await?;
        let result = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn load_live_assessment(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
    ) -> Result<LiveAssessmentWorkspace, StoreError> {
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, &course).await?;
        let rows = workspace_rows(&mut tx, &course, &assessment).await?;
        let result = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn save_live_assessment(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
        input: SaveLiveAssessmentInput,
    ) -> Result<LiveAssessmentWorkspace, StoreError> {
        input.validate()?;
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, &course).await?;
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
        sqlx::query(
            "SELECT * FROM ple_api.save_assessment($1, $2, $3, \
             $4::jsonb || jsonb_build_object( \
                 'available_at', CASE WHEN $5 IS NULL THEN NULL::timestamptz ELSE to_timestamp($5::double precision / 1000) END, \
                 'due_at', CASE WHEN $6 IS NULL THEN NULL::timestamptz ELSE to_timestamp($6::double precision / 1000) END, \
                 'closes_at', CASE WHEN $7 IS NULL THEN NULL::timestamptz ELSE to_timestamp($7::double precision / 1000) END \
             ), $8)",
        )
        .bind(course.as_string())
        .bind(assessment.as_string())
        .bind(
            i64::try_from(input.expected_edit_number.value())
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
        let rows = workspace_rows(&mut tx, &course, &assessment).await?;
        let result = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn save_live_assessment_inline(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
        expected_edit_number: AssessmentEditNumber,
        input: SaveLiveAssessmentInlineInput,
    ) -> Result<CourseAssessmentSummary, StoreError> {
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, &course).await?;
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
            "SELECT assessment_reference_number, assessment_type, assessment_title, due_at_millis, assessment_status, assessment_edit_number \
             FROM ple_api.save_assessment_inline($1, $2, $3, $4, \
             CASE WHEN $5 IS NULL THEN NULL::timestamptz ELSE to_timestamp($5::double precision / 1000) END)",
        )
        .bind(course.as_string())
        .bind(assessment.as_string())
        .bind(i64::try_from(expected_edit_number.value()).map_err(|_| invalid("Assessment Edit Number"))?)
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
            reference: assessment_reference(
                row.try_get("assessment_reference_number")
                    .map_err(map_sqlx_error)?,
            )?,
            assessment_type: assessment_type(
                row.try_get("assessment_type").map_err(map_sqlx_error)?,
            )?,
            title: title(row.try_get("assessment_title").map_err(map_sqlx_error)?)?,
            due_at,
            display_time_zone: context.account_time_zone,
            status: status(row.try_get("assessment_status").map_err(map_sqlx_error)?)?,
            edit_number: edit(
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
        course: CourseInstanceId,
        assessment: AssessmentId,
        input: SaveBaseAssessmentPolicyInput,
    ) -> Result<LiveAssessmentWorkspace, StoreError> {
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, &course).await?;
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
            .bind(course.as_string()).bind(assessment.as_string())
            .bind(i64::try_from(input.expected_edit_number.value()).map_err(|_| invalid("Assessment Edit Number"))?)
            .bind(values).bind(available_at_millis).bind(due_at_millis).bind(closes_at_millis)
            .fetch_one(&mut *tx).await.map_err(map_sqlx_error)?;
        let rows = workspace_rows(&mut tx, &course, &assessment).await?;
        let result = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn validate_live_assessment_release(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
    ) -> Result<AssessmentReleaseValidation, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT issue FROM ple_api.validate_assessment_release($1, $2)")
            .bind(course.as_string())
            .bind(assessment.as_string())
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
        course: CourseInstanceId,
        assessment: AssessmentId,
        expected: AssessmentEditNumber,
    ) -> Result<LiveAssessmentWorkspace, StoreError> {
        let mut tx = self.begin(token).await?;
        let _row = sqlx::query("SELECT * FROM ple_api.release_assessment($1, $2, $3)")
            .bind(course.as_string())
            .bind(assessment.as_string())
            .bind(i64::try_from(expected.value()).map_err(|_| invalid("Assessment Edit Number"))?)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let context = schedule_context(&mut tx, &course).await?;
        let rows = workspace_rows(&mut tx, &course, &assessment).await?;
        let workspace = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(workspace)
    }

    async fn read_live_assessment_unrelease_impact(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
    ) -> Result<AssessmentUnreleaseImpact, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT * FROM ple_api.read_assessment_unrelease_impact($1, $2)")
            .bind(course.as_string())
            .bind(assessment.as_string())
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
        course: CourseInstanceId,
        assessment: AssessmentId,
        expected: AssessmentEditNumber,
        confirmation_title: AssessmentTitle,
    ) -> Result<UnreleasedLiveAssessment, StoreError> {
        let mut tx = self.begin(token).await?;
        // ASVS 1.2.3 and 2.3.1: all caller data is bound, while the
        // SECURITY DEFINER procedure owns authorization, lock ordering,
        // transition, closure deletion, retained anonymous totals, and audit receipt.
        let row = sqlx::query("SELECT * FROM ple_api.unrelease_assessment($1, $2, $3, $4)")
            .bind(course.as_string())
            .bind(assessment.as_string())
            .bind(i64::try_from(expected.value()).map_err(|_| invalid("Assessment Edit Number"))?)
            .bind(confirmation_title.as_str())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_unrelease_sqlx_error)?;
        let deleted = decode_unrelease_impact(&row)?;
        let context = schedule_context(&mut tx, &course).await?;
        let rows = workspace_rows(&mut tx, &course, &assessment).await?;
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
        edit_number: edit(
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
    course: &CourseInstanceId,
    assessment: &AssessmentId,
) -> Result<Vec<sqlx::postgres::PgRow>, StoreError> {
    sqlx::query("SELECT * FROM ple_api.load_assessment_workspace_rows($1, $2)")
        .bind(course.as_string())
        .bind(assessment.as_string())
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
    course: &CourseInstanceId,
) -> Result<AssessmentScheduleContext, StoreError> {
    let row = sqlx::query(
        "SELECT term_starts_on::text AS term_starts_on, term_ends_on::text AS term_ends_on \
         FROM ple_api.load_course_instance($1)",
    )
    .bind(course.as_string())
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
            row.try_get::<Option<String>, _>("question_id")
                .ok()
                .flatten()
                .map(|id| {
                    Ok(AuthoredAssessmentQuestion {
                        reference: question_revision_reference(
                            id,
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
        reference: assessment_reference(
            first
                .try_get("assessment_reference_number")
                .map_err(map_sqlx_error)?,
        )?,
        edit_number: edit(
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
fn question_id(value: String) -> Result<QuestionId, StoreError> {
    value.parse().map_err(|_| invalid("Question ID"))
}

fn question_revision_reference(
    question: String,
    revision: i32,
) -> Result<QuestionRevisionReference, StoreError> {
    Ok(QuestionRevisionReference {
        question_id: question_id(question)?,
        revision_number: QuestionRevisionNumber::new(
            u32::try_from(revision).map_err(|_| invalid("Question Revision Number"))?,
        )
        .map_err(|_| invalid("Question Revision Number"))?,
    })
}
#[rustfmt::skip]
fn decode_entries(rows: &[sqlx::postgres::PgRow]) -> Result<Vec<AssessmentEntry>, StoreError> {
    let mut entries = Vec::new(); let mut index = 0;
    while let Some(row) = rows.get(index) {
        let Some(raw_id) = row.try_get::<Option<uuid::Uuid>, _>("assessment_entry_id").map_err(map_sqlx_error)? else { break };
        let id = AssessmentEntryId::from_uuid(raw_id);
        let kind: String = row.try_get("entry_kind").map_err(map_sqlx_error)?;
        let availability = parse_entry_availability(row.try_get("entry_availability").map_err(map_sqlx_error)?)?;
        let scoring_rule = entry_scoring_rule(row.try_get("scoring_rule").map_err(map_sqlx_error)?)?;
        let policy = question_policy(row)?;
        if kind == "fixed_question" {
            entries.push(AssessmentEntry::FixedQuestion(FixedQuestionAssessmentEntry {
                id, reference: row_reference(row)?, points_possible: point_value(row, "points_possible")?, availability, scoring_rule,
                question_attempt_limit: policy.0, question_attempt_time_limit: policy.1,
            })); index += 1; continue;
        }
        if kind != "question_pool" { return Err(invalid("Assessment Entry kind")); }
        let selection_count = u32::try_from(row.try_get::<i32, _>("selection_count").map_err(map_sqlx_error)?).map_err(|_| invalid("Question Pool selection count"))?;
        let selection_rule = QuestionPoolSelectionRule { selected_question_order: selected_question_order_from_row(row.try_get("selected_question_order").map_err(map_sqlx_error)?)? };
        let question_pool_revision = QuestionPoolRevisionReference {
            question_pool_id: question_id(row.try_get("question_pool_public_id").map_err(map_sqlx_error)?)?,
            revision_number: QuestionPoolRevisionNumber::new(
                u64::try_from(row.try_get::<i64, _>("question_pool_revision_number").map_err(map_sqlx_error)?)
                    .map_err(|_| invalid("Question Pool Revision Number"))?,
            ).map_err(|_| invalid("Question Pool Revision Number"))?,
        };
        entries.push(AssessmentEntry::QuestionPool(QuestionPoolAssessmentEntry {
            id, question_pool_revision, availability, scoring_rule,
            selection_count: std::num::NonZeroU32::new(selection_count)
                .ok_or_else(|| invalid("Question Pool selection count"))?,
            points_per_item: point_value(row, "points_per_item")?, selection_rule,
            question_attempt_limit: policy.0, question_attempt_time_limit: policy.1,
        }));
        while let Some(member_row) = rows.get(index) {
            let member_entry_id = member_row
                .try_get::<Option<uuid::Uuid>, _>("assessment_entry_id")
                .map_err(map_sqlx_error)?;
            if member_entry_id != Some(raw_id) {
                break;
            }
            index += 1;
        }
    }
    Ok(entries)
}
#[rustfmt::skip]
fn row_reference(row: &sqlx::postgres::PgRow) -> Result<QuestionRevisionReference, StoreError> {
    question_revision_reference(row.try_get("question_id").map_err(map_sqlx_error)?, row.try_get("question_revision_number").map_err(map_sqlx_error)?)
}

#[rustfmt::skip]
fn point_value(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<AssessmentPointValue, StoreError> {
    row.try_get::<bigdecimal::BigDecimal, _>(column).map_err(map_sqlx_error)?.to_string().parse().map_err(|_| invalid("Assessment point value"))
}

#[rustfmt::skip]
fn question_policy(
    row: &sqlx::postgres::PgRow,
) -> Result<(QuestionAttemptLimit, QuestionAttemptTimeLimit), StoreError> {
    let limit = row.try_get::<Option<i32>, _>("question_attempt_limit").map_err(map_sqlx_error)?.map(|v| u32::try_from(v).map_err(|_| invalid("Question Attempt Limit"))).transpose()?;
    let time = match row.try_get::<Option<i32>, _>("question_attempt_time_limit_seconds").map_err(map_sqlx_error)? { None => QuestionAttemptTimeLimit::Unlimited, Some(v) => QuestionAttemptTimeLimit::Limited { seconds: u32::try_from(v).map_err(|_| invalid("Question Attempt Time Limit"))?, grace_seconds: u32::try_from(row.try_get::<Option<i32>, _>("question_attempt_grace_seconds").map_err(map_sqlx_error)?.ok_or_else(|| invalid("Question Attempt Grace"))?).map_err(|_| invalid("Question Attempt Grace"))? } };
    Ok((QuestionAttemptLimit { max_attempts: limit }, time))
}

#[rustfmt::skip]
fn parse_entry_availability(value: String) -> Result<AssessmentEntryAvailability, StoreError> {
    match value.as_str() { "available" => Ok(AssessmentEntryAvailability::Available), "retired" => Ok(AssessmentEntryAvailability::Retired), _ => Err(invalid("Assessment Entry availability")) }
}
fn assessment_type(value: String) -> Result<AssessmentType, StoreError> {
    AssessmentType::parse(&value).ok_or_else(|| invalid("Assessment Type"))
}
#[rustfmt::skip]
fn entry_scoring_rule(value: String) -> Result<AssessmentEntryScoringRule, StoreError> {
    match value.as_str() { "normal" => Ok(AssessmentEntryScoringRule::Normal), "full_credit" => Ok(AssessmentEntryScoringRule::FullCredit), "extra_credit" => Ok(AssessmentEntryScoringRule::ExtraCredit), "excluded" => Ok(AssessmentEntryScoringRule::Excluded), _ => Err(invalid("Assessment Entry scoring rule")) }
}
#[rustfmt::skip]
fn selected_question_order_from_row(value: String) -> Result<QuestionPoolSelectedQuestionOrder, StoreError> {
    match value.as_str() { "question_pool_order" => Ok(QuestionPoolSelectedQuestionOrder::QuestionPoolOrder), "random_order" => Ok(QuestionPoolSelectedQuestionOrder::RandomOrder), _ => Err(invalid("Question Pool selected question order")) }
}

fn assessment_origin(row: &sqlx::postgres::PgRow) -> Result<AssessmentOrigin, StoreError> {
    // ASVS 2.2.1 and 2.2.3: accept only the two complete persisted origin shapes.
    let kind: String = row.try_get("origin_kind").map_err(map_sqlx_error)?;
    let course_reference: Option<String> = row
        .try_get("source_blueprint_course_reference_number")
        .map_err(map_sqlx_error)?;
    let revision: Option<i64> = row
        .try_get("source_blueprint_revision_number")
        .map_err(map_sqlx_error)?;
    let assessment_reference: Option<uuid::Uuid> = row
        .try_get("source_blueprint_assessment_reference")
        .map_err(map_sqlx_error)?;
    match (
        kind.as_str(),
        course_reference,
        revision,
        assessment_reference,
    ) {
        ("direct", None, None, None) => Ok(AssessmentOrigin::Direct),
        ("adopted", Some(course_reference), Some(revision), Some(assessment_reference)) => {
            let course_reference = BlueprintCourseId::new(course_reference)
                .map_err(|_| invalid("Assessment Origin"))?;
            let revision = u64::try_from(revision)
                .ok()
                .and_then(BlueprintRevision::new)
                .ok_or_else(|| invalid("Assessment Origin"))?;
            Ok(AssessmentOrigin::Adopted {
                source: BlueprintAssessmentSource::new(
                    BlueprintRevisionReference {
                        reference: course_reference,
                        revision,
                    },
                    BlueprintAssessmentId::from_uuid(assessment_reference),
                ),
            })
        }
        _ => Err(invalid("Assessment Origin")),
    }
}

fn local_timestamp_from_row(
    row: &sqlx::postgres::PgRow,
    column: &str,
    context: &AssessmentScheduleContext,
    field: AssessmentAuthoredContentField,
) -> Result<Option<LocalDateAndTime>, StoreError> {
    row.try_get::<Option<i64>, _>(column)
        .map_err(map_sqlx_error)?
        .map(|millis| {
            LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
                Timestamp::from_unix_millis(millis),
                &context.term,
                &context.account_time_zone,
                field,
            )
            .map_err(|_| invalid("Account-local Assessment time"))
        })
        .transpose()
}
pub(super) fn assessment_reference(value: String) -> Result<AssessmentId, StoreError> {
    AssessmentId::new(value).map_err(|_| invalid("Assessment Reference"))
}
fn course_reference(value: String) -> Result<CourseInstanceId, StoreError> {
    CourseInstanceId::new(value).map_err(|_| invalid("Course Instance Reference"))
}
fn course_name(value: String) -> Result<String, StoreError> {
    if value.is_empty() || value != value.trim() || value.chars().count() > 200 {
        return Err(invalid("Course Name"));
    }
    Ok(value)
}
fn edit(value: i64) -> Result<AssessmentEditNumber, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(AssessmentEditNumber::new)
        .ok_or_else(|| invalid("Assessment Edit Number"))
}
fn title(value: String) -> Result<AssessmentTitle, StoreError> {
    value.try_into().map_err(|_| invalid("Assessment Title"))
}
fn instructions(value: String) -> Result<AssessmentInstructions, StoreError> {
    value
        .try_into()
        .map_err(|_| invalid("Assessment Instructions"))
}
fn status(value: String) -> Result<AssessmentStatus, StoreError> {
    match value.as_str() {
        "unreleased" => Ok(AssessmentStatus::Unreleased),
        "released" => Ok(AssessmentStatus::Released),
        "closed" => Ok(AssessmentStatus::Closed),
        "archived" => Ok(AssessmentStatus::Archived),
        _ => Err(invalid("Assessment Status")),
    }
}
fn parse_late_work_rule(value: String) -> Result<LateWorkRule, StoreError> {
    match value.as_str() {
        "accept" => Ok(LateWorkRule::Accept),
        "mark_late" => Ok(LateWorkRule::MarkLate),
        "reject" => Ok(LateWorkRule::Reject),
        _ => Err(invalid("Late Work Rule")),
    }
}
pub(super) fn late_work_rule(value: &LateWorkRule) -> &'static str {
    match value {
        LateWorkRule::Accept => "accept",
        LateWorkRule::MarkLate => "mark_late",
        LateWorkRule::Reject => "reject",
    }
}
fn nonzero_optional(
    value: Option<i32>,
    label: &str,
) -> Result<Option<std::num::NonZeroU32>, StoreError> {
    value
        .map(|value| {
            u32::try_from(value)
                .ok()
                .and_then(std::num::NonZeroU32::new)
                .ok_or_else(|| invalid(label))
        })
        .transpose()
}
fn resolve_local_timestamp(
    value: Option<&LocalDateAndTime>,
    context: &AssessmentScheduleContext,
    field: AssessmentAuthoredContentField,
) -> Result<Option<i64>, StoreError> {
    value
        .map(|value| {
            value
                .resolve_in_account_time_zone(&context.term, &context.account_time_zone, field)
                .map(|timestamp| timestamp.as_unix_millis())
                .map_err(|_| invalid("Account-local Assessment time"))
        })
        .transpose()
}

pub(super) fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}
fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Assessment Workspace UUID randomness unavailable".to_string())
    })
}
