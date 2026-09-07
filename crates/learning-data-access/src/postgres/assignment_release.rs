//! PostgreSQL persistence for the M10 Assignment Workspace and release boundary.

use async_trait::async_trait;
use question_model::{
    AssignmentAuthoredContentField, AssignmentEditNumber, AssignmentInstructions,
    AssignmentReference, AssignmentStatus, AssignmentTitle, CourseInstanceReference,
    CourseLocalDateAndTime, CourseTerm, LateWorkRule, QuestionId, Timestamp,
};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    AssignmentPreview, AssignmentQuestionPickerEntry, AssignmentReleaseIssue,
    AssignmentReleaseValidation, AuthoredAssignmentQuestion, CreateLiveAssignmentInput,
    LiveAssignmentStore, LiveAssignmentWorkspace, ReleasedLiveAssignment, SaveLiveAssignmentInput,
    SessionTokenHash, StoreError,
};

/// PostgreSQL Store for the direct-Instructor Assignment Workspace.
#[derive(Clone)]
pub struct PostgresLiveAssignmentStore {
    pool: Pool,
}

impl PostgresLiveAssignmentStore {
    /// Binds the attested API pool to M10 procedures.
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
        let term = course_term(&mut tx, course).await?;
        let rows = workspace_rows(&mut tx, course, assignment).await?;
        let result = decode_workspace(&rows, &term)?.ok_or(StoreError::NotFound)?;
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
        let term = course_term(&mut tx, course).await?;
        let due_at_millis = input
            .due_at
            .as_ref()
            .map(|value| {
                value
                    .resolve_for_course(&term, AssignmentAuthoredContentField::DueAt)
                    .map(|timestamp| timestamp.as_unix_millis())
                    .map_err(|_| invalid("Course-local Due at"))
            })
            .transpose()?;
        let ids = input
            .question_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        sqlx::query(
            "SELECT * FROM ple_api.save_live_demo_assignment($1, $2, $3, $4, $5, $6, $7, $8)",
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
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let rows = workspace_rows(&mut tx, course, assignment).await?;
        let result = decode_workspace(&rows, &term)?.ok_or(StoreError::NotFound)?;
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
        let term = course_term(&mut tx, course).await?;
        let rows = workspace_rows(&mut tx, course, assignment).await?;
        let workspace = decode_workspace(&rows, &term)?.ok_or(StoreError::NotFound)?;
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

async fn course_term(
    tx: &mut Transaction<'_, Postgres>,
    course: CourseInstanceReference,
) -> Result<CourseTerm, StoreError> {
    let row = sqlx::query(
        "SELECT term_starts_on::text AS term_starts_on, term_ends_on::text AS term_ends_on, \
         course_time_zone FROM ple_api.load_live_demo_assignment_course_term($1)",
    )
    .bind(i64::from(course.number()))
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let starts_on: String = row.try_get("term_starts_on").map_err(map_sqlx_error)?;
    let ends_on: String = row.try_get("term_ends_on").map_err(map_sqlx_error)?;
    let time_zone: String = row.try_get("course_time_zone").map_err(map_sqlx_error)?;
    CourseTerm::from_parts(&starts_on, &ends_on, &time_zone).map_err(|_| invalid("Course Term"))
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
        late_work_rule: LateWorkRule::Accept,
        questions: vec![],
    })
}
fn decode_workspace(
    rows: &[sqlx::postgres::PgRow],
    term: &CourseTerm,
) -> Result<Option<LiveAssignmentWorkspace>, StoreError> {
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let mut workspace = workspace_without_questions(first)?;
    workspace.due_at = first
        .try_get::<Option<i64>, _>("due_at_millis")
        .map_err(map_sqlx_error)?
        .map(|millis| {
            CourseLocalDateAndTime::from_activity_timestamp(
                Timestamp::from_unix_millis(millis),
                term,
                AssignmentAuthoredContentField::DueAt,
            )
            .map_err(|_| invalid("Course-local Due at"))
        })
        .transpose()?;
    workspace.late_work_rule =
        parse_late_work_rule(first.try_get("late_work_rule").map_err(map_sqlx_error)?)?;
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
fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}
fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Assignment Workspace UUID randomness unavailable".to_string())
    })
}
