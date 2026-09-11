//! PostgreSQL persistence for the Assignment Workspace and release boundary.

use async_trait::async_trait;
use question_model::{
    AccountTimeZone, AssignmentActivityRules, AssignmentAttemptContinuationRule,
    AssignmentAttemptGradeRule, AssignmentAttemptResumeRule, AssignmentAuthoredContentField,
    AssignmentCompletionRule, AssignmentEditNumber, AssignmentInstructions,
    AssignmentNavigationRule, AssignmentQuestionDisplayRule, AssignmentQuestionOrderRule,
    AssignmentQuestionVariationRule, AssignmentReference, AssignmentStatus, AssignmentTitle,
    CourseInstanceReference, CourseTerm, LateWorkRule, LocalDateAndTime, QuestionId,
    QuestionPoolReuseRule, StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming, Timestamp,
};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    AssignmentPreview, AssignmentQuestionPickerEntry, AssignmentReleaseIssue,
    AssignmentReleaseValidation, AuthoredAssignmentQuestion, CourseAssignmentSummary,
    CreateLiveAssignmentInput, DueSoonAssignmentSummary, DueSoonAssignments, LiveAssignmentStore,
    LiveAssignmentWorkspace, ReleasedLiveAssignment, SaveLiveAssignmentInlineInput,
    SaveLiveAssignmentInput, SessionTokenHash, StoreError,
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
        let rows = sqlx::query("SELECT question_id, question_description FROM ple_api.list_live_demo_assignment_picker($1)")
            .bind(i64::from(course.number())).fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        let records = rows
            .iter()
            .map(|row| {
                Ok(AssignmentQuestionPickerEntry {
                    question_id: question_id(row.try_get("question_id").map_err(map_sqlx_error)?)?,
                    description: row
                        .try_get("question_description")
                        .map_err(map_sqlx_error)?,
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
        let row = sqlx::query("SELECT * FROM ple_api.create_live_demo_assignment($1, $2, $3, $4)")
            .bind(random_uuid()?)
            .bind(i64::from(course.number()))
            .bind(input.title.as_str())
            .bind(input.instructions.as_str())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let result = workspace_without_questions(&row)?;
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
        let ids = input
            .question_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        sqlx::query(
            "SELECT * FROM ple_api.save_live_demo_assignment($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28)",
        )
        .bind(i64::from(course.number()))
        .bind(i64::from(assignment.number()))
        .bind(
            i64::try_from(input.expected_edit_number.value())
                .map_err(|_| invalid("Assignment Edit Number"))?,
        )
        .bind(input.title.as_str())
        .bind(input.instructions.as_str())
        .bind(ids)
        .bind(due_at_millis)
        .bind(late_work_rule(&input.late_work_rule))
        .bind(input.assignment_attempt_time_limit_seconds.map(|value| i32::try_from(value.get())).transpose().map_err(|_| invalid("Assignment Attempt Time Limit"))?)
        .bind(input.attempt_limit.map(|value| i32::try_from(value.get())).transpose().map_err(|_| invalid("Assignment Attempt Limit"))?)
        .bind(activity_rule_values(&input.activity_rules)[0])
        .bind(activity_rule_values(&input.activity_rules)[1])
        .bind(activity_rule_values(&input.activity_rules)[2])
        .bind(activity_rule_values(&input.activity_rules)[3])
        .bind(activity_rule_values(&input.activity_rules)[4])
        .bind(activity_rule_values(&input.activity_rules)[5])
        .bind(activity_rule_values(&input.activity_rules)[6])
        .bind(activity_rule_values(&input.activity_rules)[7])
        .bind(activity_rule_values(&input.activity_rules)[8])
        .bind(activity_rule_extras(&input.activity_rules).0)
        .bind(activity_rule_extras(&input.activity_rules).1)
        .bind(feedback_rule_values(&input.student_feedback_release_rule)[0])
        .bind(feedback_rule_values(&input.student_feedback_release_rule)[1])
        .bind(feedback_rule_values(&input.student_feedback_release_rule)[2])
        .bind(feedback_rule_values(&input.student_feedback_release_rule)[3])
        .bind(feedback_rule_values(&input.student_feedback_release_rule)[4])
        .bind(feedback_rule_values(&input.student_feedback_release_rule)[5])
        .bind(feedback_rule_values(&input.student_feedback_release_rule)[6])
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
        let row = sqlx::query("SELECT assignment_reference_number, assignment_title, due_at_millis, assignment_status, assignment_edit_number FROM ple_api.save_live_demo_assignment_inline($1,$2,$3,$4,$5)")
            .bind(i64::from(course.number())).bind(i64::from(assignment.number()))
            .bind(i64::try_from(expected_edit_number.value()).map_err(|_| invalid("Assignment Edit Number"))?)
            .bind(input.title.as_str()).bind(due_at_millis).fetch_one(&mut *tx).await.map_err(map_sqlx_error)?;
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
        let rows =
            sqlx::query("SELECT issue FROM ple_api.validate_live_demo_assignment_release($1, $2)")
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
                    "time_limit_required" => Ok(AssignmentReleaseIssue::TimeLimitRequired),
                    "no_published_questions" => Ok(AssignmentReleaseIssue::NoPublishedQuestions),
                    "question_unavailable" => Ok(AssignmentReleaseIssue::QuestionUnavailable),
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
    ) -> Result<ReleasedLiveAssignment, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query("SELECT * FROM ple_api.release_live_demo_assignment($1, $2, $3, $4)")
            .bind(random_uuid()?)
            .bind(i64::from(course.number()))
            .bind(i64::from(assignment.number()))
            .bind(i64::try_from(expected.value()).map_err(|_| invalid("Assignment Edit Number"))?)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let revision: i64 = row.try_get("revision_number").map_err(map_sqlx_error)?;
        let result = ReleasedLiveAssignment {
            reference: assignment_reference(
                row.try_get("reference_number").map_err(map_sqlx_error)?,
            )?,
            revision_number: u64::try_from(revision)
                .map_err(|_| invalid("Assignment Revision Number"))?,
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

async fn workspace_rows(
    tx: &mut Transaction<'_, Postgres>,
    course: CourseInstanceReference,
    assignment: AssignmentReference,
) -> Result<Vec<sqlx::postgres::PgRow>, StoreError> {
    sqlx::query("SELECT * FROM ple_api.live_demo_assignment_workspace_rows($1, $2)")
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
        "SELECT term_starts_on::text AS term_starts_on, term_ends_on::text AS term_ends_on, \
         account_time_zone FROM ple_api.load_live_demo_assignment_schedule_context($1)",
    )
    .bind(i64::from(course.number()))
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let starts_on: String = row.try_get("term_starts_on").map_err(map_sqlx_error)?;
    let ends_on: String = row.try_get("term_ends_on").map_err(map_sqlx_error)?;
    let account_time_zone: String = row.try_get("account_time_zone").map_err(map_sqlx_error)?;
    let account_time_zone =
        AccountTimeZone::parse(&account_time_zone).map_err(|_| invalid("Account Time Zone"))?;
    // CourseTerm carries only inclusive calendar dates; the Account zone resolves wall-clock input.
    let term = CourseTerm::from_parts(&starts_on, &ends_on).map_err(|_| invalid("Course Term"))?;
    Ok(AssignmentScheduleContext {
        term,
        account_time_zone,
    })
}

fn workspace_without_questions(
    row: &sqlx::postgres::PgRow,
) -> Result<LiveAssignmentWorkspace, StoreError> {
    Ok(LiveAssignmentWorkspace {
        reference: assignment_reference(row.try_get("reference_number").map_err(map_sqlx_error)?)?,
        edit_number: edit(
            row.try_get("assignment_edit_number")
                .map_err(map_sqlx_error)?,
        )?,
        status: status(row.try_get("assignment_status").map_err(map_sqlx_error)?)?,
        title: title(row.try_get("assignment_title").map_err(map_sqlx_error)?)?,
        instructions: instructions(
            row.try_get("assignment_instructions")
                .map_err(map_sqlx_error)?,
        )?,
        due_at: None,
        late_work_rule: LateWorkRule::Reject,
        assignment_attempt_time_limit_seconds: None,
        attempt_limit: None,
        activity_rules: AssignmentActivityRules::default(),
        student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
        display_time_zone: AccountTimeZone::parse("UTC")
            .map_err(|_| invalid("Account Time Zone"))?,
        questions: vec![],
    })
}
fn decode_workspace(
    rows: &[sqlx::postgres::PgRow],
    context: &AssignmentScheduleContext,
) -> Result<Option<LiveAssignmentWorkspace>, StoreError> {
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let mut workspace = workspace_without_questions(first)?;
    workspace.due_at = first
        .try_get::<Option<i64>, _>("due_at_millis")
        .map_err(map_sqlx_error)?
        .map(|millis| {
            LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
                Timestamp::from_unix_millis(millis),
                &context.term,
                &context.account_time_zone,
                AssignmentAuthoredContentField::DueAt,
            )
            .map_err(|_| invalid("Account-local Due at"))
        })
        .transpose()?;
    workspace.late_work_rule =
        parse_late_work_rule(first.try_get("late_work_rule").map_err(map_sqlx_error)?)?;
    workspace.assignment_attempt_time_limit_seconds = nonzero_optional(
        first
            .try_get("assignment_attempt_time_limit_seconds")
            .map_err(map_sqlx_error)?,
        "Assignment Attempt Time Limit",
    )?;
    workspace.attempt_limit = nonzero_optional(
        first.try_get("attempt_limit").map_err(map_sqlx_error)?,
        "Assignment Attempt Limit",
    )?;
    workspace.activity_rules = activity_rules(first)?;
    workspace.student_feedback_release_rule = feedback_rules(first)?;
    workspace.display_time_zone = context.account_time_zone.clone();
    workspace.questions = rows
        .iter()
        .filter_map(|row| {
            row.try_get::<Option<String>, _>("question_id")
                .ok()
                .flatten()
                .map(|id| {
                    Ok(AuthoredAssignmentQuestion {
                        question_id: question_id(id)?,
                        description: row
                            .try_get("question_description")
                            .map_err(map_sqlx_error)?,
                    })
                })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(Some(workspace))
}
fn question_id(value: String) -> Result<QuestionId, StoreError> {
    value.parse().map_err(|_| invalid("Question ID"))
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
fn late_work_rule(value: &LateWorkRule) -> &'static str {
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
fn activity_rules(row: &sqlx::postgres::PgRow) -> Result<AssignmentActivityRules, StoreError> {
    let value = |column| row.try_get::<String, _>(column).map_err(map_sqlx_error);
    Ok(AssignmentActivityRules {
        assignment_completion_rule: match value("assignment_completion_rule")?.as_str() {
            "answer_all" => AssignmentCompletionRule::AnswerAll,
            "all_correct" => AssignmentCompletionRule::AllCorrect,
            "score_at_least" => AssignmentCompletionRule::ScoreAtLeast {
                fraction: row
                    .try_get::<Option<f64>, _>("assignment_completion_score_threshold")
                    .map_err(map_sqlx_error)?
                    .ok_or_else(|| invalid("Assignment Completion Score Threshold"))?,
            },
            _ => return Err(invalid("Assignment Completion Rule")),
        },
        assignment_attempt_grade_rule: match value("assignment_attempt_grade_rule")?.as_str() {
            "first" => AssignmentAttemptGradeRule::First,
            "latest" => AssignmentAttemptGradeRule::Latest,
            "highest" => AssignmentAttemptGradeRule::Highest,
            "instructor_selected" => AssignmentAttemptGradeRule::InstructorSelected,
            _ => return Err(invalid("Assignment Attempt Grade Rule")),
        },
        assignment_attempt_continuation_rule: match value("assignment_attempt_continuation_rule")?
            .as_str()
        {
            "unlimited" => AssignmentAttemptContinuationRule::Unlimited,
            "closed" => AssignmentAttemptContinuationRule::Closed,
            "capped" => AssignmentAttemptContinuationRule::Capped {
                max_additional_assignment_attempts: u32::try_from(
                    row.try_get::<Option<i32>, _>("max_additional_assignment_attempts")
                        .map_err(map_sqlx_error)?
                        .ok_or_else(|| invalid("Maximum Additional Assignment Attempts"))?,
                )
                .map_err(|_| invalid("Maximum Additional Assignment Attempts"))?,
            },
            _ => return Err(invalid("Assignment Attempt Continuation Rule")),
        },
        question_pool_reuse_rule: match value("question_pool_reuse_rule")?.as_str() {
            "reuse_selection" => QuestionPoolReuseRule::ReuseSelection,
            "select_again" => QuestionPoolReuseRule::SelectAgain,
            _ => return Err(invalid("Question Pool Reuse Rule")),
        },
        question_variation_rule: match value("question_variation_rule")?.as_str() {
            "reuse_variation" => AssignmentQuestionVariationRule::ReuseVariation,
            "new_variation" => AssignmentQuestionVariationRule::NewVariation,
            _ => return Err(invalid("Question Variation Rule")),
        },
        assignment_attempt_resume_rule: match value("assignment_attempt_resume_rule")?.as_str() {
            "resumable" => AssignmentAttemptResumeRule::Resumable,
            "single_session" => AssignmentAttemptResumeRule::SingleSession,
            _ => return Err(invalid("Assignment Attempt Resume Rule")),
        },
        assignment_question_display_rule: match value("assignment_question_display_rule")?.as_str()
        {
            "one_question_at_a_time" => AssignmentQuestionDisplayRule::OneQuestionAtATime,
            "all_questions" => AssignmentQuestionDisplayRule::AllQuestions,
            _ => return Err(invalid("Assignment Question Display Rule")),
        },
        assignment_navigation_rule: match value("assignment_navigation_rule")?.as_str() {
            "free_navigation" => AssignmentNavigationRule::FreeNavigation,
            "forward_only" => AssignmentNavigationRule::ForwardOnly,
            _ => return Err(invalid("Assignment Navigation Rule")),
        },
        assignment_question_order_rule: match value("assignment_question_order_rule")?.as_str() {
            "authored_order" => AssignmentQuestionOrderRule::AuthoredOrder,
            "shuffled" => AssignmentQuestionOrderRule::Shuffled,
            _ => return Err(invalid("Assignment Question Order Rule")),
        },
    })
}
fn feedback_timing(value: String) -> Result<StudentFeedbackReleaseTiming, StoreError> {
    match value.as_str() {
        "during_attempt" => Ok(StudentFeedbackReleaseTiming::DuringAttempt),
        "after_submit" => Ok(StudentFeedbackReleaseTiming::AfterSubmit),
        "after_due" => Ok(StudentFeedbackReleaseTiming::AfterDue),
        "after_close" => Ok(StudentFeedbackReleaseTiming::AfterClose),
        "never" => Ok(StudentFeedbackReleaseTiming::Never),
        _ => Err(invalid("Student Feedback Release Timing")),
    }
}
fn feedback_rules(row: &sqlx::postgres::PgRow) -> Result<StudentFeedbackReleaseRule, StoreError> {
    Ok(StudentFeedbackReleaseRule {
        score: feedback_timing(row.try_get("feedback_score").map_err(map_sqlx_error)?)?,
        per_item_correctness: feedback_timing(
            row.try_get("feedback_per_item_correctness")
                .map_err(map_sqlx_error)?,
        )?,
        submitted_response: feedback_timing(
            row.try_get("feedback_submitted_response")
                .map_err(map_sqlx_error)?,
        )?,
        question_feedback: feedback_timing(
            row.try_get("feedback_question_feedback")
                .map_err(map_sqlx_error)?,
        )?,
        question_answer: feedback_timing(
            row.try_get("feedback_question_answer")
                .map_err(map_sqlx_error)?,
        )?,
        question_answer_explanation: feedback_timing(
            row.try_get("feedback_question_answer_explanation")
                .map_err(map_sqlx_error)?,
        )?,
        class_statistics: feedback_timing(
            row.try_get("feedback_class_statistics")
                .map_err(map_sqlx_error)?,
        )?,
    })
}
fn activity_rule_values(rules: &AssignmentActivityRules) -> [&'static str; 9] {
    [
        match rules.assignment_completion_rule {
            AssignmentCompletionRule::AnswerAll => "answer_all",
            AssignmentCompletionRule::AllCorrect => "all_correct",
            AssignmentCompletionRule::ScoreAtLeast { .. } => "score_at_least",
        },
        match rules.assignment_attempt_grade_rule {
            AssignmentAttemptGradeRule::First => "first",
            AssignmentAttemptGradeRule::Latest => "latest",
            AssignmentAttemptGradeRule::Highest => "highest",
            AssignmentAttemptGradeRule::InstructorSelected => "instructor_selected",
        },
        match rules.assignment_attempt_continuation_rule {
            AssignmentAttemptContinuationRule::Unlimited => "unlimited",
            AssignmentAttemptContinuationRule::Capped { .. } => "capped",
            AssignmentAttemptContinuationRule::Closed => "closed",
        },
        match rules.question_pool_reuse_rule {
            QuestionPoolReuseRule::ReuseSelection => "reuse_selection",
            QuestionPoolReuseRule::SelectAgain => "select_again",
        },
        match rules.question_variation_rule {
            AssignmentQuestionVariationRule::ReuseVariation => "reuse_variation",
            AssignmentQuestionVariationRule::NewVariation => "new_variation",
        },
        match rules.assignment_attempt_resume_rule {
            AssignmentAttemptResumeRule::Resumable => "resumable",
            AssignmentAttemptResumeRule::SingleSession => "single_session",
        },
        match rules.assignment_question_display_rule {
            AssignmentQuestionDisplayRule::AllQuestions => "all_questions",
            AssignmentQuestionDisplayRule::OneQuestionAtATime => "one_question_at_a_time",
        },
        match rules.assignment_navigation_rule {
            AssignmentNavigationRule::FreeNavigation => "free_navigation",
            AssignmentNavigationRule::ForwardOnly => "forward_only",
        },
        match rules.assignment_question_order_rule {
            AssignmentQuestionOrderRule::AuthoredOrder => "authored_order",
            AssignmentQuestionOrderRule::Shuffled => "shuffled",
        },
    ]
}
fn activity_rule_extras(rules: &AssignmentActivityRules) -> (Option<f64>, Option<i32>) {
    (
        match rules.assignment_completion_rule {
            AssignmentCompletionRule::ScoreAtLeast { fraction } => Some(fraction),
            _ => None,
        },
        match rules.assignment_attempt_continuation_rule {
            AssignmentAttemptContinuationRule::Capped {
                max_additional_assignment_attempts,
            } => i32::try_from(max_additional_assignment_attempts).ok(),
            _ => None,
        },
    )
}
fn feedback_rule_values(rule: &StudentFeedbackReleaseRule) -> [&'static str; 7] {
    [
        feedback_value(rule.score),
        feedback_value(rule.per_item_correctness),
        feedback_value(rule.submitted_response),
        feedback_value(rule.question_feedback),
        feedback_value(rule.question_answer),
        feedback_value(rule.question_answer_explanation),
        feedback_value(rule.class_statistics),
    ]
}
fn feedback_value(value: StudentFeedbackReleaseTiming) -> &'static str {
    match value {
        StudentFeedbackReleaseTiming::DuringAttempt => "during_attempt",
        StudentFeedbackReleaseTiming::AfterSubmit => "after_submit",
        StudentFeedbackReleaseTiming::AfterDue => "after_due",
        StudentFeedbackReleaseTiming::AfterClose => "after_close",
        StudentFeedbackReleaseTiming::Never => "never",
    }
}
fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}
fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Assignment Workspace UUID randomness unavailable".to_string())
    })
}
