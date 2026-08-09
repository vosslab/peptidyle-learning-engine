//! PostgreSQL retention scheduling and cleanup persistence.

use question_model::ObjectId;

use super::*;
use crate::SessionTokenHash;

#[cfg(feature = "postgres")]
#[async_trait]
impl RetentionStore for PostgresStore {
    async fn configure_retention_policy(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        policy: InstitutionRetentionPolicy,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let changed: bool =
            sqlx::query_scalar("SELECT ple_configure_retention_policy($1, $2, $3, $4)")
                .bind(session.to_string())
                .bind(i32::from(policy.notify_after().get()))
                .bind(i32::from(policy.archive_after().get()))
                .bind(i32::from(policy.delete_after().get()))
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !changed {
            return Err(StoreError::Forbidden);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn end_course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseRetentionRecord, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let ended: bool = sqlx::query_scalar("SELECT ple_end_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !ended {
            return Err(StoreError::Forbidden);
        }
        let row = sqlx::query("SELECT * FROM ple_read_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let record = decode_retention_record(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseRetentionRecord>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query("SELECT * FROM ple_read_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_retention_record).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl RetentionScheduleStore for PostgresStore {
    async fn dispatch_due_retention_stages(
        &self,
        batch: RetentionDispatchBatch,
    ) -> Result<u16, StoreError> {
        let mut transaction = self.begin_app().await?;
        let count: i64 = sqlx::query_scalar("SELECT ple_dispatch_due_retention_stages($1)")
            .bind(i32::from(batch.get()))
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let count = u16::try_from(count).map_err(|_| {
            StoreError::Unavailable("retention broker returned invalid dispatch count".to_string())
        })?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(count)
    }

    async fn extend_course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        additional_days: RetentionDays,
    ) -> Result<CourseRetentionRecord, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let changed: bool = sqlx::query_scalar("SELECT ple_extend_course_retention($1, $2, $3)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .bind(i32::from(additional_days.get()))
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !changed {
            return Err(StoreError::Conflict);
        }
        let row = sqlx::query("SELECT * FROM ple_read_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let record = decode_retention_record(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn set_archive_disposition(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<CourseRetentionRecord, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let disposition = match disposition {
            AssignmentDefinitionDisposition::Retain => "retain",
            AssignmentDefinitionDisposition::Delete => "delete",
        };
        let changed: bool = sqlx::query_scalar("SELECT ple_set_archive_disposition($1, $2, $3)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .bind(disposition)
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if !changed {
            return Err(StoreError::Conflict);
        }
        let row = sqlx::query("SELECT * FROM ple_read_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let record = decode_retention_record(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl RetentionApiStore for PostgresStore {
    async fn retention_view(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseRetentionView>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query("SELECT * FROM ple_read_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let view = row
            .as_ref()
            .map(decode_retention_record)
            .transpose()?
            .map(|record| {
                record
                    .safe_view()
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(view)
    }

    async fn retention_notification(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<crate::RetentionNotificationView>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query("SELECT * FROM ple_read_retention_notification($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let intent: String = row.try_get("intent").map_err(map_sqlx_error)?;
        let created_at_millis: i64 = row.try_get("created_at_millis").map_err(map_sqlx_error)?;
        match intent.as_str() {
            "archive" => Ok(Some(crate::RetentionNotificationView {
                intent: crate::RetentionNotificationIntent::Archive,
                created_at: ActivityTimestamp::from_unix_millis(created_at_millis),
            })),
            "delete" => Ok(Some(crate::RetentionNotificationView {
                intent: crate::RetentionNotificationIntent::Delete,
                created_at: ActivityTimestamp::from_unix_millis(created_at_millis),
            })),
            "extend" => Ok(Some(crate::RetentionNotificationView {
                intent: crate::RetentionNotificationIntent::Extend,
                created_at: ActivityTimestamp::from_unix_millis(created_at_millis),
            })),
            _ => Err(StoreError::InvalidRecord(
                "invalid retention notification intent".to_string(),
            )),
        }
    }

    async fn extend_retention_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        additional_days: RetentionDays,
    ) -> Result<CourseRetentionView, StoreError> {
        let (retention, outcome) = self
            .apply_retention_api_action(
                context,
                session,
                course,
                expected,
                RetentionApiAction::Extend(additional_days),
            )
            .await?;
        let _ = outcome;
        Ok(retention)
    }

    async fn request_retention_archive_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<crate::RetentionRequestResult, StoreError> {
        let (retention, outcome) = self
            .apply_retention_api_action(
                context,
                session,
                course,
                expected,
                RetentionApiAction::Archive(disposition),
            )
            .await?;
        Ok(crate::RetentionRequestResult { retention, outcome })
    }

    async fn request_retention_delete_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
    ) -> Result<crate::RetentionRequestResult, StoreError> {
        let (retention, outcome) = self
            .apply_retention_api_action(
                context,
                session,
                course,
                expected,
                RetentionApiAction::Delete,
            )
            .await?;
        Ok(crate::RetentionRequestResult { retention, outcome })
    }
}

#[cfg(feature = "postgres")]
impl PostgresStore {
    async fn apply_retention_api_action(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        action: RetentionApiAction,
    ) -> Result<(CourseRetentionView, crate::RetentionRequestOutcome), StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let (action, additional_days, disposition) = match action {
            RetentionApiAction::Extend(days) => ("extend", Some(i32::from(days.get())), None),
            RetentionApiAction::Archive(AssignmentDefinitionDisposition::Retain) => {
                ("archive", None, Some("retain"))
            }
            RetentionApiAction::Archive(AssignmentDefinitionDisposition::Delete) => {
                ("archive", None, Some("delete"))
            }
            RetentionApiAction::Delete => ("delete", None, None),
        };
        let outcome: Option<String> =
            sqlx::query_scalar("SELECT ple_apply_retention_api_action($1, $2, $3, $4, $5, $6)")
                .bind(session.to_string())
                .bind(course.as_uuid())
                .bind(i64::try_from(expected.value()).map_err(|_| StoreError::Conflict)?)
                .bind(action)
                .bind(additional_days)
                .bind(disposition)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let Some(outcome) = outcome else {
            return Err(StoreError::Conflict);
        };
        let row = sqlx::query("SELECT * FROM ple_read_course_retention($1, $2)")
            .bind(session.to_string())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        let view = decode_retention_record(&row)?
            .safe_view()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        let outcome = match outcome.as_str() {
            "scheduled" | "changed" => crate::RetentionRequestOutcome::Scheduled,
            "inProgress" => crate::RetentionRequestOutcome::InProgress,
            "completed" => crate::RetentionRequestOutcome::Completed,
            _ => {
                return Err(StoreError::InvalidRecord(
                    "invalid retention API outcome".to_string(),
                ));
            }
        };
        Ok((view, outcome))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl RetentionWorkerStore for PostgresStore {
    async fn prepare_retention_work(
        &self,
        command: RetentionWorkerCommand,
    ) -> Result<RetentionWork, StoreError> {
        let mut transaction = self
            .begin_tenant(TenantContext::from_authenticated_session(command.tenant))
            .await?;
        let value: Option<Value> =
            sqlx::query_scalar("SELECT ple_prepare_retention_work($1,$2,$3,$4,$5,$6)")
                .bind(command.tenant.as_uuid())
                .bind(command.job.as_uuid())
                .bind(command.lease.as_uuid())
                .bind(command.course.as_uuid())
                .bind(retention_stage_db(command.stage))
                .bind(i64::try_from(command.generation).map_err(|_| {
                    StoreError::InvalidRecord(
                        "retention generation exceeds database range".to_string(),
                    )
                })?)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let Some(value) = value else {
            return Err(StoreError::Conflict);
        };
        let kind = value.get("kind").and_then(Value::as_str).ok_or_else(|| {
            StoreError::Unavailable("stored retention work is invalid".to_string())
        })?;
        let work = match kind {
            "notify" => RetentionWork::Notify,
            "cleanup" => {
                let values = value
                    .get("objects")
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        StoreError::Unavailable(
                            "stored retention object manifest is invalid".to_string(),
                        )
                    })?;
                let mut objects = Vec::with_capacity(values.len());
                for value in values {
                    let raw = value.as_str().ok_or_else(|| {
                        StoreError::Unavailable(
                            "stored retention object manifest is invalid".to_string(),
                        )
                    })?;
                    let object = uuid::Uuid::parse_str(raw).map_err(|_| {
                        StoreError::Unavailable(
                            "stored retention object manifest is invalid".to_string(),
                        )
                    })?;
                    objects.push(objects::ObjectKey::StudentRecord {
                        tenant: command.tenant,
                        object: ObjectId::from_uuid(object),
                    });
                }
                RetentionWork::Cleanup(RetentionCleanupManifest { objects })
            }
            _ => {
                return Err(StoreError::Unavailable(
                    "stored retention work kind is invalid".to_string(),
                ));
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(work)
    }

    async fn commit_retention_work(
        &self,
        command: RetentionWorkerCommand,
    ) -> Result<(), StoreError> {
        let mut transaction = self
            .begin_tenant(TenantContext::from_authenticated_session(command.tenant))
            .await?;
        let committed: bool =
            sqlx::query_scalar("SELECT ple_commit_retention_work($1,$2,$3,$4,$5,$6)")
                .bind(command.tenant.as_uuid())
                .bind(command.job.as_uuid())
                .bind(command.lease.as_uuid())
                .bind(command.course.as_uuid())
                .bind(retention_stage_db(command.stage))
                .bind(i64::try_from(command.generation).map_err(|_| {
                    StoreError::InvalidRecord(
                        "retention generation exceeds database range".to_string(),
                    )
                })?)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !committed {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)
    }
}

#[cfg(feature = "postgres")]
fn retention_stage_db(stage: crate::RetentionStage) -> &'static str {
    match stage {
        crate::RetentionStage::Notify => "notify",
        crate::RetentionStage::ArchiveStudentRecords => "archiveStudentRecords",
        crate::RetentionStage::DeleteStudentRecords => "deleteStudentRecords",
    }
}

#[cfg(feature = "postgres")]
fn decode_retention_policy(row: &PgRow) -> Result<InstitutionRetentionPolicy, StoreError> {
    let notify: i32 = row.try_get("notify_days").map_err(map_sqlx_error)?;
    let archive: i32 = row.try_get("archive_days").map_err(map_sqlx_error)?;
    let delete: i32 = row.try_get("delete_days").map_err(map_sqlx_error)?;
    let days = |value| {
        RetentionDays::new(u16::try_from(value).map_err(|_| {
            StoreError::Unavailable("stored retention policy is invalid".to_string())
        })?)
        .map_err(|error| StoreError::Unavailable(error.to_string()))
    };
    InstitutionRetentionPolicy::new(days(notify)?, days(archive)?, days(delete)?)
        .map_err(|error| StoreError::Unavailable(error.to_string()))
}

#[cfg(feature = "postgres")]
fn decode_retention_record(row: &PgRow) -> Result<CourseRetentionRecord, StoreError> {
    let ended_at: i64 = row.try_get("ended_at_millis").map_err(map_sqlx_error)?;
    let generation: i64 = row.try_get("generation").map_err(map_sqlx_error)?;
    let lifecycle: String = row.try_get("lifecycle").map_err(map_sqlx_error)?;
    let disposition: String = row
        .try_get("assignment_disposition")
        .map_err(map_sqlx_error)?;
    let disposition = match disposition.as_str() {
        "retain" => AssignmentDefinitionDisposition::Retain,
        "delete" => AssignmentDefinitionDisposition::Delete,
        _ => {
            return Err(StoreError::Unavailable(
                "stored retention disposition is invalid".to_string(),
            ));
        }
    };
    let state = match lifecycle.as_str() {
        "active" => CourseRetentionState::Active,
        "archived" => CourseRetentionState::StudentRecordsArchived,
        "deleted" => CourseRetentionState::StudentRecordsDeleted,
        _ => {
            return Err(StoreError::Unavailable(
                "stored retention lifecycle is invalid".to_string(),
            ));
        }
    };
    Ok(CourseRetentionRecord {
        snapshot: CourseRetentionSnapshot::new(
            ActivityTimestamp::from_unix_millis(ended_at),
            decode_retention_policy(row)?,
            disposition,
            u64::try_from(generation).map_err(|_| {
                StoreError::Unavailable("stored retention generation is invalid".to_string())
            })?,
        )
        .map_err(|error| StoreError::Unavailable(error.to_string()))?,
        status: crate::CourseRetentionStatus::from_persisted(state, disposition),
    })
}
