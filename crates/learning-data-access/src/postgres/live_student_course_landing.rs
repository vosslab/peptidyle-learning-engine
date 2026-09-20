//! PostgreSQL adapter for the Student Course landing projections.

use async_trait::async_trait;
use question_model::{
    AssessmentAttemptCompletion, AssessmentId, AssessmentType, CourseInstanceId, CourseTerm,
};
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    LiveAssessmentGradeContribution, LiveStudentAssessmentLandingSummary,
    LiveStudentCourseInvitationSummary, LiveStudentCourseLandingStore,
    LiveStudentCourseLandingSummary, SessionTokenHash, StoreError,
};

/// PostgreSQL Store for the active Student Course landing.
#[derive(Clone)]
pub struct PostgresLiveStudentCourseLandingStore {
    pool: Pool,
}

impl PostgresLiveStudentCourseLandingStore {
    /// Binds the attested API pool to the Student landing procedures.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(
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
impl LiveStudentCourseLandingStore for PostgresLiveStudentCourseLandingStore {
    async fn list_live_student_courses(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<LiveStudentCourseLandingSummary>, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let rows = sqlx::query(
            "SELECT course_instance_id, course_short_name, course_long_name \
             FROM ple_api.list_live_student_course_landing()",
        )
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let courses = rows
            .iter()
            .map(decode_course)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(courses)
    }

    async fn list_pending_live_student_course_invitations(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<LiveStudentCourseInvitationSummary>, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let rows = sqlx::query(
            "SELECT course_instance_id, course_short_name, course_long_name, \
             instructor_display_name, term_starts_on::text AS term_starts_on, \
             term_ends_on::text AS term_ends_on \
             FROM ple_api.list_pending_student_course_invitations()",
        )
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let invitations = rows
            .iter()
            .map(decode_invitation)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(invitations)
    }

    async fn list_released_live_student_assessments(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
    ) -> Result<Vec<LiveStudentAssessmentLandingSummary>, StoreError> {
        let mut transaction = self.begin(session_token_hash).await?;
        let rows = sqlx::query(
            "SELECT assessment_id, assessment_title, assessment_type, start_decision, \
             time_limit_seconds, assessment_attempt_limit AS attempt_limit, \
             late_work_rule, display_time_zone, \
             CASE WHEN available_at IS NULL THEN NULL ELSE \
                 floor(extract(epoch FROM available_at) * 1000)::bigint END AS available_at_millis, \
             CASE WHEN due_at IS NULL THEN NULL ELSE \
                 floor(extract(epoch FROM due_at) * 1000)::bigint END AS due_at_millis, \
             CASE WHEN closes_at IS NULL THEN NULL ELSE \
                 floor(extract(epoch FROM closes_at) * 1000)::bigint END AS closes_at_millis, \
             floor(extract(epoch FROM evaluated_at) * 1000)::bigint AS evaluated_at_millis, \
             assessment_attempt_number, \
             assessment_attempt_completion, can_resume_assessment_attempt, \
             graded_question_count, saved_question_count, question_count, \
             assessment_score_points_earned, assessment_score_points_possible \
             FROM ple_api.list_released_live_student_assessments($1)",
        )
        .bind(course_instance_id.as_string())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let assessments = rows
            .iter()
            .map(decode_assessment)
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(assessments)
    }
}

fn decode_course(
    row: &sqlx::postgres::PgRow,
) -> Result<LiveStudentCourseLandingSummary, StoreError> {
    Ok(LiveStudentCourseLandingSummary {
        course_instance_id: course_instance_id(
            row.try_get("course_instance_id").map_err(map_sqlx_error)?,
        )?,
        short_name: name(
            row.try_get("course_short_name").map_err(map_sqlx_error)?,
            "Course short name",
        )?,
        long_name: name(
            row.try_get("course_long_name").map_err(map_sqlx_error)?,
            "Course long name",
        )?,
    })
}

fn decode_assessment(
    row: &sqlx::postgres::PgRow,
) -> Result<LiveStudentAssessmentLandingSummary, StoreError> {
    let decision = super::student_assessment_decision::decode(row)?;
    let assessment_type = serde_json::from_value::<AssessmentType>(serde_json::Value::String(
        row.try_get("assessment_type").map_err(map_sqlx_error)?,
    ))
    .map_err(|_| invalid("Assessment Type"))?;
    let assessment_attempt_number = row
        .try_get::<Option<i32>, _>("assessment_attempt_number")
        .map_err(map_sqlx_error)?
        .map(u32::try_from)
        .transpose()
        .map_err(|_| invalid("Assessment Attempt number"))?;
    let assessment_attempt_completion = match row
        .try_get::<Option<String>, _>("assessment_attempt_completion")
        .map_err(map_sqlx_error)?
        .as_deref()
    {
        None => None,
        Some("in_progress") => Some(AssessmentAttemptCompletion::InProgress),
        Some("completed") => Some(AssessmentAttemptCompletion::Completed),
        Some(_) => return Err(invalid("Assessment Attempt completion")),
    };
    let can_resume_assessment_attempt = row
        .try_get("can_resume_assessment_attempt")
        .map_err(map_sqlx_error)?;
    let graded_question_count = count(row, "graded_question_count")?;
    let saved_question_count = count(row, "saved_question_count")?;
    let question_count = count(row, "question_count")?;
    // ASVS 2.2.1: validate the complete database contribution pair at the adapter boundary.
    let points_earned = optional_finite_nonnegative(row, "assessment_score_points_earned")?;
    let points_possible = optional_finite_nonnegative(row, "assessment_score_points_possible")?;
    let assessment_score = match (points_earned, points_possible) {
        (Some(points_earned), Some(points_possible)) => Some(LiveAssessmentGradeContribution {
            points_earned,
            points_possible,
        }),
        (None, None) => None,
        _ => return Err(invalid("Assessment grade contribution")),
    };
    if question_count == 0
        || graded_question_count > question_count
        || saved_question_count > question_count
        || (assessment_attempt_completion.is_none()
            && (assessment_attempt_number.is_some()
                || graded_question_count != 0
                || saved_question_count != 0))
        || (assessment_attempt_completion.is_some() && assessment_attempt_number.is_none())
    {
        return Err(invalid("Assessment progress"));
    }
    Ok(LiveStudentAssessmentLandingSummary {
        assessment_id: assessment_id(row.try_get("assessment_id").map_err(map_sqlx_error)?)?,
        title: row.try_get("assessment_title").map_err(map_sqlx_error)?,
        assessment_type,
        decision,
        assessment_attempt_number,
        assessment_attempt_completion,
        can_resume_assessment_attempt,
        graded_question_count,
        saved_question_count,
        question_count,
        assessment_score,
    })
}

fn decode_invitation(
    row: &sqlx::postgres::PgRow,
) -> Result<LiveStudentCourseInvitationSummary, StoreError> {
    let instructor_display_name = name(
        row.try_get("instructor_display_name")
            .map_err(map_sqlx_error)?,
        "Instructor display name",
    )?;
    if instructor_display_name.chars().any(char::is_control) {
        return Err(invalid("Instructor display name"));
    }
    let start_date: String = row.try_get("term_starts_on").map_err(map_sqlx_error)?;
    let end_date: String = row.try_get("term_ends_on").map_err(map_sqlx_error)?;
    let term =
        CourseTerm::from_parts(&start_date, &end_date).map_err(|_| invalid("Course term"))?;
    Ok(LiveStudentCourseInvitationSummary {
        course_instance_id: course_instance_id(
            row.try_get("course_instance_id").map_err(map_sqlx_error)?,
        )?,
        short_name: name(
            row.try_get("course_short_name").map_err(map_sqlx_error)?,
            "Course short name",
        )?,
        long_name: name(
            row.try_get("course_long_name").map_err(map_sqlx_error)?,
            "Course long name",
        )?,
        instructor_display_name,
        term,
    })
}

fn count(row: &sqlx::postgres::PgRow, column: &str) -> Result<u32, StoreError> {
    u32::try_from(row.try_get::<i64, _>(column).map_err(map_sqlx_error)?)
        .map_err(|_| invalid(column))
}

fn optional_finite_nonnegative(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<Option<f64>, StoreError> {
    let value = row
        .try_get::<Option<f64>, _>(column)
        .map_err(map_sqlx_error)?;
    match value {
        Some(value) if value.is_finite() && value >= 0.0 => Ok(Some(value)),
        None => Ok(None),
        Some(_) => Err(invalid(column)),
    }
}

fn invalid(name: &str) -> StoreError {
    StoreError::InvalidRecord(format!("{name} is invalid"))
}

fn name(value: String, label: &str) -> Result<String, StoreError> {
    (value == value.trim() && !value.is_empty() && value.chars().count() <= 200)
        .then_some(value)
        .ok_or_else(|| invalid(label))
}

fn course_instance_id(value: String) -> Result<CourseInstanceId, StoreError> {
    CourseInstanceId::new(value).map_err(|_| invalid("Course Instance ID"))
}

fn assessment_id(value: String) -> Result<AssessmentId, StoreError> {
    AssessmentId::new(value)
        .map_err(|_| StoreError::InvalidRecord("Assessment ID is invalid".to_string()))
}
