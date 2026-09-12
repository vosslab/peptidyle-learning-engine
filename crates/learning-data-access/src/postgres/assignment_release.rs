//! PostgreSQL persistence for the Assignment Workspace and release boundary.

use async_trait::async_trait;
use question_model::{
    AccountTimeZone, AssignmentAuthoredContentField, AssignmentEditNumber, AssignmentEntry,
    AssignmentEntryAvailability, AssignmentEntryId, AssignmentEntryScoringRule,
    AssignmentInstructions, AssignmentPointValue, AssignmentReference, AssignmentStatus,
    AssignmentTitle, BlueprintAssignmentReference, BlueprintAssignmentSource,
    BlueprintCourseReference, BlueprintRevision, BlueprintRevisionReference,
    CourseInstanceReference, CourseTerm, FixedQuestionAssignmentEntry, LateWorkRule,
    LocalDateAndTime, QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionId,
    QuestionPoolAssignmentEntry, QuestionPoolItem, QuestionPoolItemAvailability,
    QuestionPoolItemId, QuestionPoolSelectedQuestionOrder, QuestionPoolSelectionRule,
    QuestionRevisionNumber, QuestionRevisionReference, Timestamp,
};
use sqlx::{Postgres, Row, Transaction};

use super::{
    Pool,
    assignment_workspace_policy::{activity_rules, feedback_rules},
    assignment_workspace_save::{assignment_entries_json, assignment_values_json},
    connection::map_sqlx_error,
};
use crate::{
    AssignmentPreview, AssignmentQuestionPickerEntry, AssignmentReleaseIssue,
    AssignmentReleaseValidation, AssignmentUnreleaseImpact, AuthoredAssignmentQuestion,
    CourseAssignmentSourceChoice, CourseAssignmentSummary, CreateLiveAssignmentInput,
    DueSoonAssignmentSummary, DueSoonAssignments, LiveAssignmentStore, LiveAssignmentWorkspace,
    SaveLiveAssignmentInlineInput, SaveLiveAssignmentInput, SessionTokenHash, StoreError,
    UnreleasedLiveAssignment,
};

/// PostgreSQL Store for the direct-Instructor Assignment Workspace.
#[derive(Clone)]
pub struct PostgresLiveAssignmentStore {
    pool: Pool,
}

impl PostgresLiveAssignmentStore {
    /// Binds the attested API pool to Assignment Workspace procedures.
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
impl LiveAssignmentStore for PostgresLiveAssignmentStore {
    async fn list_assignments_due_soon(
        &self,
        token: SessionTokenHash,
    ) -> Result<DueSoonAssignments, StoreError> {
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
            "SELECT course_reference_number, course_long_name, assignment_reference_number, ",
            "assignment_title, assignment_status, due_at_millis ",
            "FROM ple_api.list_assignments_due_soon()",
        ))
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let items = rows
            .iter()
            .map(|row| {
                Ok(DueSoonAssignmentSummary {
                    course_reference: course_reference(
                        row.try_get("course_reference_number")
                            .map_err(map_sqlx_error)?,
                    )?,
                    course_long_name: course_name(
                        row.try_get("course_long_name").map_err(map_sqlx_error)?,
                    )?,
                    assignment_reference: assignment_reference(
                        row.try_get("assignment_reference_number")
                            .map_err(map_sqlx_error)?,
                    )?,
                    assignment_title: title(
                        row.try_get("assignment_title").map_err(map_sqlx_error)?,
                    )?,
                    assignment_status: status(
                        row.try_get("assignment_status").map_err(map_sqlx_error)?,
                    )?,
                    due_at_millis: row.try_get("due_at_millis").map_err(map_sqlx_error)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(DueSoonAssignments {
            items,
            next_cursor: None,
            display_time_zone,
        })
    }

    async fn list_course_assignments(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<CourseAssignmentSummary>, StoreError> {
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, course).await?;
        // ASVS 1.2.3 and 8.2.2: bind the public Course Reference and let the
        // session-authorized database function enforce the exact Course owner.
        let rows = sqlx::query(
            "SELECT assignment_reference_number, assignment_title, due_at_millis, assignment_status, \
             assignment_edit_number FROM ple_api.list_course_assignments($1)",
        )
        .bind(i64::from(course.number()))
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let assignments = rows
            .iter()
            .map(|row| {
                Ok(CourseAssignmentSummary {
                    reference: assignment_reference(
                        row.try_get("assignment_reference_number")
                            .map_err(map_sqlx_error)?,
                    )?,
                    title: title(row.try_get("assignment_title").map_err(map_sqlx_error)?)?,
                    due_at: row
                        .try_get::<Option<i64>, _>("due_at_millis")
                        .map_err(map_sqlx_error)?
                        .map(|milliseconds| {
                            LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
                                Timestamp::from_unix_millis(milliseconds),
                                &context.term,
                                &context.account_time_zone,
                                AssignmentAuthoredContentField::DueAt,
                            )
                        })
                        .transpose()
                        .map_err(|_| invalid("Due at"))?,
                    display_time_zone: context.account_time_zone.clone(),
                    status: status(row.try_get("assignment_status").map_err(map_sqlx_error)?)?,
                    edit_number: edit(
                        row.try_get("assignment_edit_number")
                            .map_err(map_sqlx_error)?,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(assignments)
    }

    async fn list_assignment_question_picker(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<AssignmentQuestionPickerEntry>, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT question_id, question_revision_number, question_description FROM ple_api.list_assignment_question_picker($1)")
            .bind(i64::from(course.number())).fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        let records = rows
            .iter()
            .map(|row| {
                Ok(AssignmentQuestionPickerEntry {
                    reference: question_revision_reference(
                        row.try_get("question_id").map_err(map_sqlx_error)?,
                        row.try_get("question_revision_number")
                            .map_err(map_sqlx_error)?,
                    )?,
                    description: row
                        .try_get("question_description")
                        .map_err(map_sqlx_error)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(records)
    }

    async fn list_course_assignment_source_choices(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<CourseAssignmentSourceChoice>, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query(
            "SELECT source_blueprint_course_reference_number, source_blueprint_revision_number, \
             source_blueprint_assignment_reference, source_label \
             FROM ple_api.list_course_assignment_source_choices($1)",
        )
        .bind(i64::from(course.number()))
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let records = rows
            .iter()
            .map(|row| {
                Ok(CourseAssignmentSourceChoice {
                    source: blueprint_assignment_source(row)?,
                    label: row.try_get("source_label").map_err(map_sqlx_error)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(records)
    }

    async fn create_live_assignment(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        input: CreateLiveAssignmentInput,
    ) -> Result<LiveAssignmentWorkspace, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT * FROM ple_api.create_assignment($1, $2, $3, $4, $5)")
            .bind(random_uuid()?)
            .bind(i64::from(course.number()))
            .bind(input.blueprint_assignment_reference)
            .bind(input.title.as_str())
            .bind(input.instructions.as_str())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let assignment = assignment_reference(
            row.try_get("assignment_reference_number")
                .map_err(map_sqlx_error)?,
        )?;
        let context = schedule_context(&mut tx, course).await?;
        let rows = workspace_rows(&mut tx, course, assignment).await?;
        let result = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn load_live_assignment(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<LiveAssignmentWorkspace, StoreError> {
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, course).await?;
        let rows = workspace_rows(&mut tx, course, assignment).await?;
        let result = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn save_live_assignment(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        input: SaveLiveAssignmentInput,
    ) -> Result<LiveAssignmentWorkspace, StoreError> {
        input.validate()?;
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, course).await?;
        let due_at_millis = input
            .due_at
            .as_ref()
            .map(|value| {
                value
                    .resolve_in_account_time_zone(
                        &context.term,
                        &context.account_time_zone,
                        AssignmentAuthoredContentField::DueAt,
                    )
                    .map(|timestamp| timestamp.as_unix_millis())
                    .map_err(|_| invalid("Account-local Due at"))
            })
            .transpose()?;
        let available_at_millis = resolve_local_timestamp(
            input.available_at.as_ref(),
            &context,
            AssignmentAuthoredContentField::AvailableAt,
        )?;
        let closes_at_millis = resolve_local_timestamp(
            input.closes_at.as_ref(),
            &context,
            AssignmentAuthoredContentField::ClosesAt,
        )?;
        let values = assignment_values_json(&input)?;
        let entries = assignment_entries_json(&input.entries)?;
        sqlx::query(
            "SELECT * FROM ple_api.save_assignment($1, $2, $3, \
             $4::jsonb || jsonb_build_object( \
                 'available_at', CASE WHEN $5 IS NULL THEN NULL::timestamptz ELSE to_timestamp($5::double precision / 1000) END, \
                 'due_at', CASE WHEN $6 IS NULL THEN NULL::timestamptz ELSE to_timestamp($6::double precision / 1000) END, \
                 'closes_at', CASE WHEN $7 IS NULL THEN NULL::timestamptz ELSE to_timestamp($7::double precision / 1000) END \
             ), $8)",
        )
        .bind(i64::from(course.number()))
        .bind(i64::from(assignment.number()))
        .bind(
            i64::try_from(input.expected_edit_number.value())
                .map_err(|_| invalid("Assignment Edit Number"))?,
        )
        .bind(values)
        .bind(available_at_millis)
        .bind(due_at_millis)
        .bind(closes_at_millis)
        .bind(entries)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let rows = workspace_rows(&mut tx, course, assignment).await?;
        let result = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn save_live_assignment_inline(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        expected_edit_number: AssignmentEditNumber,
        input: SaveLiveAssignmentInlineInput,
    ) -> Result<CourseAssignmentSummary, StoreError> {
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, course).await?;
        let due_at_millis = input
            .due_at
            .as_ref()
            .map(|value| {
                value
                    .resolve_in_account_time_zone(
                        &context.term,
                        &context.account_time_zone,
                        AssignmentAuthoredContentField::DueAt,
                    )
                    .map(|timestamp| timestamp.as_unix_millis())
                    .map_err(|_| invalid("Account-local Due at"))
            })
            .transpose()?;
        let row = sqlx::query(
            "SELECT assignment_reference_number, assignment_title, due_at_millis, assignment_status, assignment_edit_number \
             FROM ple_api.save_assignment_inline($1, $2, $3, $4, \
             CASE WHEN $5 IS NULL THEN NULL::timestamptz ELSE to_timestamp($5::double precision / 1000) END)",
        )
        .bind(i64::from(course.number()))
        .bind(i64::from(assignment.number()))
        .bind(i64::try_from(expected_edit_number.value()).map_err(|_| invalid("Assignment Edit Number"))?)
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
                    AssignmentAuthoredContentField::DueAt,
                )
            })
            .transpose()
            .map_err(|_| invalid("Due at"))?;
        let result = CourseAssignmentSummary {
            reference: assignment_reference(
                row.try_get("assignment_reference_number")
                    .map_err(map_sqlx_error)?,
            )?,
            title: title(row.try_get("assignment_title").map_err(map_sqlx_error)?)?,
            due_at,
            display_time_zone: context.account_time_zone,
            status: status(row.try_get("assignment_status").map_err(map_sqlx_error)?)?,
            edit_number: edit(
                row.try_get("assignment_edit_number")
                    .map_err(map_sqlx_error)?,
            )?,
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn validate_live_assignment_release(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<AssignmentReleaseValidation, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT issue FROM ple_api.validate_assignment_release($1, $2)")
            .bind(i64::from(course.number()))
            .bind(i64::from(assignment.number()))
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
                    "attempt_time_limit_required" => Ok(AssignmentReleaseIssue::TimeLimitRequired),
                    "questions_required" => Ok(AssignmentReleaseIssue::NoPublishedQuestions),
                    "question_pool_insufficient_items" => {
                        Ok(AssignmentReleaseIssue::QuestionUnavailable)
                    }
                    _ => Err(invalid("Assignment Release Issue")),
                }
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(AssignmentReleaseValidation {
            can_release: issues.is_empty(),
            issues,
        })
    }

    async fn load_live_assignment_preview(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<AssignmentPreview, StoreError> {
        let mut tx = self.begin(token).await?;
        let context = schedule_context(&mut tx, course).await?;
        let rows = workspace_rows(&mut tx, course, assignment).await?;
        let workspace = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(AssignmentPreview {
            title: workspace.title,
            instructions: workspace.instructions,
            questions: workspace.questions,
        })
    }

    async fn release_live_assignment(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        expected: AssignmentEditNumber,
    ) -> Result<LiveAssignmentWorkspace, StoreError> {
        let mut tx = self.begin(token).await?;
        let _row = sqlx::query("SELECT * FROM ple_api.release_assignment($1, $2, $3)")
            .bind(i64::from(course.number()))
            .bind(i64::from(assignment.number()))
            .bind(i64::try_from(expected.value()).map_err(|_| invalid("Assignment Edit Number"))?)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let context = schedule_context(&mut tx, course).await?;
        let rows = workspace_rows(&mut tx, course, assignment).await?;
        let workspace = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(workspace)
    }

    async fn read_live_assignment_unrelease_impact(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<AssignmentUnreleaseImpact, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT * FROM ple_api.read_assignment_unrelease_impact($1, $2)")
            .bind(i64::from(course.number()))
            .bind(i64::from(assignment.number()))
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_unrelease_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let impact = decode_unrelease_impact(&row)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(impact)
    }

    async fn unrelease_live_assignment(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        expected: AssignmentEditNumber,
        confirmation_title: AssignmentTitle,
    ) -> Result<UnreleasedLiveAssignment, StoreError> {
        let mut tx = self.begin(token).await?;
        // ASVS 1.2.3 and 2.3.1: all caller data is bound, while the
        // SECURITY DEFINER procedure owns authorization, lock ordering,
        // transition, closure deletion, statistics rebuild, and audit receipt.
        let row = sqlx::query("SELECT * FROM ple_api.unrelease_assignment($1, $2, $3, $4)")
            .bind(i64::from(course.number()))
            .bind(i64::from(assignment.number()))
            .bind(i64::try_from(expected.value()).map_err(|_| invalid("Assignment Edit Number"))?)
            .bind(confirmation_title.as_str())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_unrelease_sqlx_error)?;
        let deleted = decode_unrelease_impact(&row)?;
        let context = schedule_context(&mut tx, course).await?;
        let rows = workspace_rows(&mut tx, course, assignment).await?;
        let assignment = decode_workspace(&rows, &context)?.ok_or(StoreError::NotFound)?;
        if assignment.status != AssignmentStatus::Unreleased {
            return Err(invalid("Unreleased Assignment status"));
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(UnreleasedLiveAssignment {
            assignment,
            deleted,
        })
    }
}

fn decode_unrelease_impact(
    row: &sqlx::postgres::PgRow,
) -> Result<AssignmentUnreleaseImpact, StoreError> {
    let question_submissions = count(row, "question_submission_count")?;
    let assignment_submissions = count(row, "assignment_submission_count")?;
    let submission_count = question_submissions
        .checked_add(assignment_submissions)
        .ok_or_else(|| invalid("Assignment Unrelease submission count"))?;
    Ok(AssignmentUnreleaseImpact {
        confirmation_title: title(row.try_get("assignment_title").map_err(map_sqlx_error)?)?,
        edit_number: edit(
            row.try_get("assignment_edit_number")
                .map_err(map_sqlx_error)?,
        )?,
        attempt_count: count(row, "assignment_attempt_count")?,
        submission_count,
        grade_count: count(row, "grading_result_count")?,
    })
}

fn count(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64, StoreError> {
    u64::try_from(row.try_get::<i64, _>(column).map_err(map_sqlx_error)?)
        .map_err(|_| invalid("Assignment Unrelease count"))
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

async fn workspace_rows(
    tx: &mut Transaction<'_, Postgres>,
    course: CourseInstanceReference,
    assignment: AssignmentReference,
) -> Result<Vec<sqlx::postgres::PgRow>, StoreError> {
    sqlx::query("SELECT * FROM ple_api.load_assignment_workspace_rows($1, $2)")
        .bind(i64::from(course.number()))
        .bind(i64::from(assignment.number()))
        .fetch_all(&mut **tx)
        .await
        .map_err(map_sqlx_error)
}

struct AssignmentScheduleContext {
    term: CourseTerm,
    account_time_zone: AccountTimeZone,
}

async fn schedule_context(
    tx: &mut Transaction<'_, Postgres>,
    course: CourseInstanceReference,
) -> Result<AssignmentScheduleContext, StoreError> {
    let row = sqlx::query(
        "SELECT term_starts_on::text AS term_starts_on, term_ends_on::text AS term_ends_on \
         FROM ple_api.load_course_instance($1)",
    )
    .bind(i64::from(course.number()))
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
    Ok(AssignmentScheduleContext {
        term,
        account_time_zone,
    })
}

fn decode_workspace(
    rows: &[sqlx::postgres::PgRow],
    context: &AssignmentScheduleContext,
) -> Result<Option<LiveAssignmentWorkspace>, StoreError> {
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
                    Ok(AuthoredAssignmentQuestion {
                        reference: question_revision_reference(
                            id,
                            row.try_get("question_revision_number")
                                .map_err(map_sqlx_error)?,
                        )?,
                        description: row
                            .try_get("question_description")
                            .map_err(map_sqlx_error)?,
                    })
                })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(Some(LiveAssignmentWorkspace {
        reference: assignment_reference(
            first
                .try_get("assignment_reference_number")
                .map_err(map_sqlx_error)?,
        )?,
        edit_number: edit(
            first
                .try_get("assignment_edit_number")
                .map_err(map_sqlx_error)?,
        )?,
        status: status(first.try_get("assignment_status").map_err(map_sqlx_error)?)?,
        source: blueprint_assignment_source(first)?,
        title: title(first.try_get("assignment_title").map_err(map_sqlx_error)?)?,
        instructions: instructions(
            first
                .try_get("assignment_instructions")
                .map_err(map_sqlx_error)?,
        )?,
        due_at: local_timestamp_from_row(
            first,
            "due_at_millis",
            context,
            AssignmentAuthoredContentField::DueAt,
        )?,
        available_at: local_timestamp_from_row(
            first,
            "available_at_millis",
            context,
            AssignmentAuthoredContentField::AvailableAt,
        )?,
        closes_at: local_timestamp_from_row(
            first,
            "closes_at_millis",
            context,
            AssignmentAuthoredContentField::ClosesAt,
        )?,
        late_work_rule: parse_late_work_rule(
            first.try_get("late_work_rule").map_err(map_sqlx_error)?,
        )?,
        assignment_attempt_time_limit_seconds: nonzero_optional(
            first
                .try_get("assignment_attempt_time_limit_seconds")
                .map_err(map_sqlx_error)?,
            "Assignment Attempt Time Limit",
        )?,
        attempt_limit: nonzero_optional(
            first.try_get("attempt_limit").map_err(map_sqlx_error)?,
            "Assignment Attempt Limit",
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
fn decode_entries(rows: &[sqlx::postgres::PgRow]) -> Result<Vec<AssignmentEntry>, StoreError> {
    let mut entries = Vec::new(); let mut index = 0;
    while let Some(row) = rows.get(index) {
        let Some(raw_id) = row.try_get::<Option<uuid::Uuid>, _>("assignment_entry_id").map_err(map_sqlx_error)? else { break };
        let id = AssignmentEntryId::from_uuid(raw_id);
        let kind: String = row.try_get("entry_kind").map_err(map_sqlx_error)?;
        let availability = parse_entry_availability(row.try_get("entry_availability").map_err(map_sqlx_error)?)?;
        let scoring_rule = entry_scoring_rule(row.try_get("scoring_rule").map_err(map_sqlx_error)?)?;
        let policy = question_policy(row)?;
        if kind == "fixed_question" {
            entries.push(AssignmentEntry::FixedQuestion(FixedQuestionAssignmentEntry {
                id, reference: row_reference(row)?, points_possible: point_value(row, "points_possible")?, availability, scoring_rule,
                question_attempt_limit: policy.0, question_attempt_time_limit: policy.1,
            })); index += 1; continue;
        }
        if kind != "question_pool" { return Err(invalid("Assignment Entry kind")); }
        let selection_count = u32::try_from(row.try_get::<i32, _>("selection_count").map_err(map_sqlx_error)?).map_err(|_| invalid("Question Pool selection count"))?;
        let selection_rule = QuestionPoolSelectionRule { selected_question_order: selected_question_order_from_row(row.try_get("selected_question_order").map_err(map_sqlx_error)?)? };
        let mut items = Vec::new();
        while let Some(item_row) = rows.get(index) {
            let item_entry: Option<uuid::Uuid> = item_row.try_get("assignment_entry_id").map_err(map_sqlx_error)?;
            if item_entry != Some(raw_id) { break; }
            if let Some(item_id) = item_row.try_get::<Option<uuid::Uuid>, _>("question_pool_item_id").map_err(map_sqlx_error)? {
                items.push(QuestionPoolItem { id: QuestionPoolItemId::from_uuid(item_id), reference: row_reference(item_row)?, availability: pool_item_availability_from_row(item_row.try_get("item_availability").map_err(map_sqlx_error)?)? });
            }
            index += 1;
        }
        entries.push(AssignmentEntry::QuestionPool(QuestionPoolAssignmentEntry {
            id, availability, scoring_rule, selection_count, points_per_item: point_value(row, "points_per_item")?, selection_rule,
            question_attempt_limit: policy.0, question_attempt_time_limit: policy.1, items,
        }));
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
) -> Result<AssignmentPointValue, StoreError> {
    row.try_get::<bigdecimal::BigDecimal, _>(column).map_err(map_sqlx_error)?.to_string().parse().map_err(|_| invalid("Assignment point value"))
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
fn parse_entry_availability(value: String) -> Result<AssignmentEntryAvailability, StoreError> {
    match value.as_str() { "available" => Ok(AssignmentEntryAvailability::Available), "retired" => Ok(AssignmentEntryAvailability::Retired), _ => Err(invalid("Assignment Entry availability")) }
}
#[rustfmt::skip]
fn pool_item_availability_from_row(value: Option<String>) -> Result<QuestionPoolItemAvailability, StoreError> {
    match value.as_deref() { Some("available") => Ok(QuestionPoolItemAvailability::Available), Some("retired") => Ok(QuestionPoolItemAvailability::Retired), _ => Err(invalid("Question Pool Item availability")) }
}
#[rustfmt::skip]
fn entry_scoring_rule(value: String) -> Result<AssignmentEntryScoringRule, StoreError> {
    match value.as_str() { "normal" => Ok(AssignmentEntryScoringRule::Normal), "full_credit" => Ok(AssignmentEntryScoringRule::FullCredit), "extra_credit" => Ok(AssignmentEntryScoringRule::ExtraCredit), "excluded" => Ok(AssignmentEntryScoringRule::Excluded), _ => Err(invalid("Assignment Entry scoring rule")) }
}
#[rustfmt::skip]
fn selected_question_order_from_row(value: String) -> Result<QuestionPoolSelectedQuestionOrder, StoreError> {
    match value.as_str() { "question_pool_order" => Ok(QuestionPoolSelectedQuestionOrder::QuestionPoolOrder), "random_order" => Ok(QuestionPoolSelectedQuestionOrder::RandomOrder), _ => Err(invalid("Question Pool selected question order")) }
}

fn blueprint_assignment_source(
    row: &sqlx::postgres::PgRow,
) -> Result<BlueprintAssignmentSource, StoreError> {
    let course_reference = u64::try_from(
        row.try_get::<i64, _>("source_blueprint_course_reference_number")
            .map_err(map_sqlx_error)?,
    )
    .ok()
    .and_then(BlueprintCourseReference::new)
    .ok_or_else(|| invalid("Blueprint Course Reference"))?;
    let revision = u64::try_from(
        row.try_get::<i64, _>("source_blueprint_revision_number")
            .map_err(map_sqlx_error)?,
    )
    .ok()
    .and_then(BlueprintRevision::new)
    .ok_or_else(|| invalid("Blueprint Revision"))?;
    let assignment_reference = BlueprintAssignmentReference::from_uuid(
        row.try_get("source_blueprint_assignment_reference")
            .map_err(map_sqlx_error)?,
    );
    Ok(BlueprintAssignmentSource::new(
        BlueprintRevisionReference {
            reference: course_reference,
            revision,
        },
        assignment_reference,
    ))
}

fn local_timestamp_from_row(
    row: &sqlx::postgres::PgRow,
    column: &str,
    context: &AssignmentScheduleContext,
    field: AssignmentAuthoredContentField,
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
            .map_err(|_| invalid("Account-local Assignment time"))
        })
        .transpose()
}
fn assignment_reference(value: i64) -> Result<AssignmentReference, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(AssignmentReference::new)
        .ok_or_else(|| invalid("Assignment Reference"))
}
fn course_reference(value: i64) -> Result<CourseInstanceReference, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(CourseInstanceReference::new)
        .ok_or_else(|| invalid("Course Instance Reference"))
}
fn course_name(value: String) -> Result<String, StoreError> {
    if value.is_empty() || value != value.trim() || value.chars().count() > 200 {
        return Err(invalid("Course Name"));
    }
    Ok(value)
}
fn edit(value: i64) -> Result<AssignmentEditNumber, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(AssignmentEditNumber::new)
        .ok_or_else(|| invalid("Assignment Edit Number"))
}
fn title(value: String) -> Result<AssignmentTitle, StoreError> {
    value.try_into().map_err(|_| invalid("Assignment Title"))
}
fn instructions(value: String) -> Result<AssignmentInstructions, StoreError> {
    value
        .try_into()
        .map_err(|_| invalid("Assignment Instructions"))
}
fn status(value: String) -> Result<AssignmentStatus, StoreError> {
    match value.as_str() {
        "unreleased" => Ok(AssignmentStatus::Unreleased),
        "released" => Ok(AssignmentStatus::Released),
        "closed" => Ok(AssignmentStatus::Closed),
        "archived" => Ok(AssignmentStatus::Archived),
        _ => Err(invalid("Assignment Status")),
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
    context: &AssignmentScheduleContext,
    field: AssignmentAuthoredContentField,
) -> Result<Option<i64>, StoreError> {
    value
        .map(|value| {
            value
                .resolve_in_account_time_zone(&context.term, &context.account_time_zone, field)
                .map(|timestamp| timestamp.as_unix_millis())
                .map_err(|_| invalid("Account-local Assignment time"))
        })
        .transpose()
}

pub(super) fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}
fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Assignment Workspace UUID randomness unavailable".to_string())
    })
}
