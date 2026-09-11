//! PostgreSQL persistence for Course Instance creation and initial teaching team.

use async_trait::async_trait;
use question_model::{
    AccountReference, CourseId, CourseInstanceReference, CourseMembershipRole, CourseSummary,
    CourseTerm, CourseTheme,
};
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::{
    CourseCreationInstructor, CourseInstanceStore, CourseInstanceSummary, CourseInstanceView,
    CreateCourseInstanceInput, CreatedCourseInstance, SessionTokenHash, StoreError,
};

/// PostgreSQL Store for Course Instance creation and current Teaching Team reads.
#[derive(Clone)]
pub struct PostgresCourseInstanceStore {
    pool: Pool,
}

impl PostgresCourseInstanceStore {
    /// Binds the attested API pool to Course Instance procedures.
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
impl CourseInstanceStore for PostgresCourseInstanceStore {
    async fn resolve_course_navigation(
        &self,
        session_token_hash: SessionTokenHash,
        reference: CourseInstanceReference,
    ) -> Result<CourseId, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 1.2.3, 8.2.2, and 8.3.1: the procedure resolves an opaque
        // reference only after binding it to the installed active membership.
        let row = sqlx::query("SELECT course_id FROM ple_api.resolve_course_navigation($1)")
            .bind(i64::from(reference.number()))
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let course = row
            .map(|row| {
                row.try_get("course_id")
                    .map(CourseId::from_uuid)
                    .map_err(map_sqlx_error)
            })
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(course)
    }

    async fn read_course_summary(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseSummary, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        // ASVS 1.2.3, 8.2.2, and 8.3.1: the SECURITY DEFINER procedure binds
        // this opaque Course ID to the installed session's active membership.
        let row = sqlx::query(
            "SELECT course_id, reference_number, title, term_starts_on::text AS term_starts_on, \
             term_ends_on::text AS term_ends_on, membership_role \
             FROM ple_api.read_course_summary($1)",
        )
        .bind(course.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let summary = row
            .as_ref()
            .map(decode_course_summary)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(summary)
    }

    async fn list_course_instances(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<CourseInstanceSummary>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let rows = sqlx::query(
            "SELECT reference_number, title, term_starts_on::text AS term_starts_on, \
             term_ends_on::text AS term_ends_on, course_theme \
             FROM ple_api.list_live_demo_course_instances()",
        )
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

    async fn create_course_instance(
        &self,
        session_token_hash: SessionTokenHash,
        input: CreateCourseInstanceInput,
    ) -> Result<CreatedCourseInstance, StoreError> {
        input.validate()?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row = sqlx::query(
            "SELECT reference_number, title, term_starts_on::text AS term_starts_on, \
             term_ends_on::text AS term_ends_on, creator_is_assigned_instructor \
             FROM ple_api.create_live_demo_course_instance(\
             $1, $2, $3, $4, $5, $6, $7, $8, $9::date, $10::date, $11)",
        )
        .bind(random_uuid()?)
        .bind(random_uuid()?)
        .bind(random_uuid()?)
        .bind(random_uuid()?)
        .bind(random_uuid()?)
        .bind(i64::from(input.blueprint_course.number()))
        .bind(
            i64::try_from(input.blueprint_revision.value())
                .map_err(|_| invalid("Blueprint Revision"))?,
        )
        .bind(&input.title)
        .bind(input.term.start_date().to_string())
        .bind(input.term.end_date().to_string())
        .bind(
            input
                .assigned_instructor
                .map(|reference| i64::from(reference.number())),
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = CreatedCourseInstance {
            course: CourseInstanceSummary {
                reference: course_reference(
                    row.try_get("reference_number").map_err(map_sqlx_error)?,
                )?,
                title: row.try_get("title").map_err(map_sqlx_error)?,
                term: term(
                    row.try_get("term_starts_on").map_err(map_sqlx_error)?,
                    row.try_get("term_ends_on").map_err(map_sqlx_error)?,
                )?,
                theme: CourseTheme::default(),
            },
            creator_is_assigned_instructor: row
                .try_get("creator_is_assigned_instructor")
                .map_err(map_sqlx_error)?,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn load_course_instance(
        &self,
        session_token_hash: SessionTokenHash,
        reference: CourseInstanceReference,
    ) -> Result<CourseInstanceView, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row = sqlx::query(
            "SELECT reference_number, title, term_starts_on::text AS term_starts_on, \
             term_ends_on::text AS term_ends_on, course_theme, is_assigned_instructor, \
             active_instructor_count FROM ple_api.load_live_demo_course_instance($1)",
        )
        .bind(i64::from(reference.number()))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row
            .as_ref()
            .map(decode_view)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn list_course_creation_instructors(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<CourseCreationInstructor>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let rows =
            sqlx::query("SELECT * FROM ple_api.list_live_demo_course_creation_instructors()")
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let records = rows
            .iter()
            .map(|row| {
                Ok(CourseCreationInstructor {
                    reference: account_reference(
                        row.try_get("reference_number").map_err(map_sqlx_error)?,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(records)
    }
}

fn decode_summary(row: &sqlx::postgres::PgRow) -> Result<CourseInstanceSummary, StoreError> {
    Ok(CourseInstanceSummary {
        reference: course_reference(row.try_get("reference_number").map_err(map_sqlx_error)?)?,
        title: row.try_get("title").map_err(map_sqlx_error)?,
        term: term(
            row.try_get("term_starts_on").map_err(map_sqlx_error)?,
            row.try_get("term_ends_on").map_err(map_sqlx_error)?,
        )?,
        theme: theme(row.try_get("course_theme").map_err(map_sqlx_error)?)?,
    })
}

fn decode_course_summary(row: &sqlx::postgres::PgRow) -> Result<CourseSummary, StoreError> {
    let course_id = row.try_get("course_id").map_err(map_sqlx_error)?;
    let stored_membership_role: String = row.try_get("membership_role").map_err(map_sqlx_error)?;
    Ok(CourseSummary {
        id: CourseId::from_uuid(course_id),
        reference: course_reference(row.try_get("reference_number").map_err(map_sqlx_error)?)?,
        title: row.try_get("title").map_err(map_sqlx_error)?,
        term: term(
            row.try_get("term_starts_on").map_err(map_sqlx_error)?,
            row.try_get("term_ends_on").map_err(map_sqlx_error)?,
        )?,
        role: membership_role(&stored_membership_role)?,
    })
}

fn decode_view(row: &sqlx::postgres::PgRow) -> Result<CourseInstanceView, StoreError> {
    let active_instructor_count: i64 = row
        .try_get("active_instructor_count")
        .map_err(map_sqlx_error)?;
    Ok(CourseInstanceView {
        course: decode_summary(row)?,
        is_assigned_instructor: row
            .try_get("is_assigned_instructor")
            .map_err(map_sqlx_error)?,
        active_instructor_count: u32::try_from(active_instructor_count)
            .map_err(|_| invalid("Teaching Team size"))?,
    })
}

fn course_reference(value: i64) -> Result<CourseInstanceReference, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(CourseInstanceReference::new)
        .ok_or_else(|| invalid("Course Instance Reference"))
}

fn account_reference(value: i64) -> Result<AccountReference, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(AccountReference::new)
        .ok_or_else(|| invalid("Account Reference"))
}

fn term(start_date: String, end_date: String) -> Result<CourseTerm, StoreError> {
    CourseTerm::from_parts(&start_date, &end_date).map_err(|_| invalid("Course Term"))
}

fn membership_role(value: &str) -> Result<CourseMembershipRole, StoreError> {
    match value {
        "student" => Ok(CourseMembershipRole::Student),
        "instructor" => Ok(CourseMembershipRole::Instructor),
        _ => Err(invalid("Course Membership Role")),
    }
}

fn theme(value: String) -> Result<CourseTheme, StoreError> {
    value.parse().map_err(|_| invalid("Course Theme"))
}

fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}

fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Course Instance UUID randomness unavailable".to_string())
    })
}
