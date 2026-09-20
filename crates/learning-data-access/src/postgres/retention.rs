//! PostgreSQL adapter for the Course-retention executor capability.

use async_trait::async_trait;
use question_model::{CourseInstanceId, Timestamp};
use sqlx::Row;

use super::{
    Pool,
    connection::{map_sqlx_error, parse_course_instance_id},
};
use crate::{
    CourseRetentionDueAction, CourseRetentionDueActionKind, CourseRetentionStore, StoreError,
};

/// Binds the attested retention-executor pool to its four exact procedures.
#[derive(Clone)]
pub struct PostgresCourseRetentionStore {
    pool: Pool,
}

impl PostgresCourseRetentionStore {
    /// Uses a pool whose sole capability is the retention executor.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(&self) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        // ASVS 2.3.1/2.3.3: each action uses the isolated executor capability
        // inside one transaction; callers cannot reach protected tables.
        sqlx::query("SET LOCAL ROLE ple_course_retention_executor")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[async_trait]
impl CourseRetentionStore for PostgresCourseRetentionStore {
    async fn list_due_course_retention_actions(
        &self,
        evaluated_at: Timestamp,
    ) -> Result<Vec<CourseRetentionDueAction>, StoreError> {
        let mut transaction = self.begin().await?;
        // ASVS 1.2.4: the explicit instant remains a bound query value.
        sqlx::query("SELECT ple_api.sweep_expired_authentication_growth(to_timestamp($1::double precision / 1000.0))")
            .bind(evaluated_at.as_unix_millis())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let rows = sqlx::query(
            "SELECT course_instance_id, due_action, \
                    (extract(epoch FROM due_at) * 1000)::bigint AS due_at_millis, \
                    CASE WHEN archive_marked_at IS NULL THEN NULL \
                         ELSE (extract(epoch FROM archive_marked_at) * 1000)::bigint END \
                         AS archive_marked_at_millis \
             FROM ple_data.course_retention_due_actions(to_timestamp($1::double precision / 1000.0))",
        )
        .bind(evaluated_at.as_unix_millis())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        rows.iter().map(decode_due_action).collect()
    }

    async fn mark_course_instance_inactive(
        &self,
        course_instance_id: CourseInstanceId,
        evaluated_at: Timestamp,
    ) -> Result<bool, StoreError> {
        self.commit_transition(
            "ple_api.mark_course_instance_inactive",
            course_instance_id,
            evaluated_at,
        )
        .await
    }

    async fn archive_course_student_records(
        &self,
        course_instance_id: CourseInstanceId,
        evaluated_at: Timestamp,
    ) -> Result<bool, StoreError> {
        self.commit_transition(
            "ple_api.archive_course_student_records",
            course_instance_id,
            evaluated_at,
        )
        .await
    }

    async fn delete_course_student_records(
        &self,
        course_instance_id: CourseInstanceId,
        evaluated_at: Timestamp,
    ) -> Result<bool, StoreError> {
        self.commit_transition(
            "ple_api.delete_course_student_records",
            course_instance_id,
            evaluated_at,
        )
        .await
    }
}

impl PostgresCourseRetentionStore {
    async fn commit_transition(
        &self,
        procedure: &'static str,
        course_instance_id: CourseInstanceId,
        evaluated_at: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin().await?;
        // The procedure name is a closed internal selection, never caller input.
        let statement = match procedure {
            "ple_api.mark_course_instance_inactive" => {
                "SELECT ple_api.mark_course_instance_inactive(\
                 $1, to_timestamp($2::double precision / 1000.0))"
            }
            "ple_api.archive_course_student_records" => {
                "SELECT ple_api.archive_course_student_records(\
                 $1, to_timestamp($2::double precision / 1000.0))"
            }
            "ple_api.delete_course_student_records" => {
                "SELECT ple_api.delete_course_student_records(\
                 $1, to_timestamp($2::double precision / 1000.0))"
            }
            _ => unreachable!("closed retention transition selection"),
        };
        // ASVS 1.2.4: Course identity and evaluated instant are bound values.
        let committed = sqlx::query_scalar(statement)
            .bind(course_instance_id.as_str())
            .bind(evaluated_at.as_unix_millis())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(committed)
    }
}

fn decode_due_action(row: &sqlx::postgres::PgRow) -> Result<CourseRetentionDueAction, StoreError> {
    let action = match row
        .try_get::<String, _>("due_action")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "mark_inactive" => CourseRetentionDueActionKind::MarkInactive,
        "warn_inactive" => CourseRetentionDueActionKind::WarnInactive,
        "notify_archive" => CourseRetentionDueActionKind::NotifyArchive,
        "archive" => CourseRetentionDueActionKind::Archive,
        "delete" => CourseRetentionDueActionKind::Delete,
        _ => {
            return Err(StoreError::InvalidRecord(
                "Course retention action is invalid".to_string(),
            ));
        }
    };
    Ok(CourseRetentionDueAction {
        course_instance_id: parse_course_instance_id(
            row.try_get("course_instance_id").map_err(map_sqlx_error)?,
        )?,
        action,
        due_at: Timestamp::from_unix_millis(row.try_get("due_at_millis").map_err(map_sqlx_error)?),
        archive_marked_at: row
            .try_get::<Option<i64>, _>("archive_marked_at_millis")
            .map_err(map_sqlx_error)?
            .map(Timestamp::from_unix_millis),
    })
}
