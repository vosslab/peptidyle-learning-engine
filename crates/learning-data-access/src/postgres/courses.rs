use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::CourseStore for PostgresStore {
    async fn upsert_course_impl(
        &self,
        context: TenantContext,
        course: CourseRecord,
    ) -> Result<(), StoreError> {
        retry_transaction(|| {
            let course = course.clone();
            async move {
                ensure_tenant(context, course.tenant)?;
                validate_course(&course)?;
                let tenant = course.tenant;
                let course_id = course.id;
                let mut transaction = self.begin_tenant(context).await?;
                sqlx::query(
                    "INSERT INTO course (tenant_id, course_id, title) VALUES ($1, $2, $3) \
             ON CONFLICT (tenant_id, course_id) DO UPDATE SET \
             title = EXCLUDED.title, updated_at = transaction_timestamp()",
                )
                .bind(tenant.as_uuid())
                .bind(course_id.as_uuid())
                .bind(&course.title)
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                super::course_roster::ensure_roster_state(&mut transaction, tenant, course_id)
                    .await?;
                let affected = sqlx::query_scalar::<_, Uuid>(
                    "SELECT DISTINCT assignment_id FROM assignment_policy_exception \
             WHERE tenant_id = $1 AND course_id = $2 AND course_group_id IS NOT NULL \
             ORDER BY assignment_id",
                )
                .bind(tenant.as_uuid())
                .bind(course_id.as_uuid())
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?
                .into_iter()
                .map(AssignmentId::from_uuid)
                .collect::<Vec<_>>();
                // The sorted query fixes the multi-assignment lock order. Each policy
                // advisory lock precedes its active attempt/timing row locks.
                let mut locked = Vec::with_capacity(affected.len());
                for assignment in affected {
                    assignment_timing::lock_postgres_assignment_policy(
                        &mut transaction,
                        tenant,
                        assignment,
                    )
                    .await?;
                    locked.push((
                        assignment,
                        assignment_timing::lock_postgres_active_timing_rows(
                            &mut transaction,
                            tenant,
                            assignment,
                        )
                        .await?,
                    ));
                }
                for membership in &course.members {
                    sqlx::query(
                        "INSERT INTO course_member (tenant_id, course_id, user_id, role) \
                 VALUES ($1, $2, $3, $4) \
                 ON CONFLICT (tenant_id, course_id, user_id) DO UPDATE SET role = EXCLUDED.role",
                    )
                    .bind(tenant.as_uuid())
                    .bind(course_id.as_uuid())
                    .bind(membership.user.as_uuid())
                    .bind(course_membership_role_name(membership.role))
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                }
                let member_ids = course
                    .members
                    .iter()
                    .map(|membership| membership.user.as_uuid())
                    .collect::<Vec<_>>();
                sqlx::query(
                    "DELETE FROM course_member WHERE tenant_id = $1 AND course_id = $2 \
             AND NOT (user_id = ANY($3::uuid[]))",
                )
                .bind(tenant.as_uuid())
                .bind(course_id.as_uuid())
                .bind(&member_ids)
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                super::course_roster::reconcile_legacy_course_members(&mut transaction, &course)
                    .await?;
                sqlx::query(
                    "DELETE FROM course_group_member AS grouped USING course_member AS member \
             WHERE grouped.tenant_id = $1 AND grouped.course_id = $2 \
               AND member.tenant_id = grouped.tenant_id AND member.course_id = grouped.course_id \
               AND member.user_id = grouped.user_id AND member.role <> 'student'",
                )
                .bind(tenant.as_uuid())
                .bind(course_id.as_uuid())
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                let now = database_timestamp(&mut transaction).await?;
                for (assignment, rows) in locked {
                    assignment_timing::apply_postgres_locked_timing_rows(
                        &mut transaction,
                        tenant,
                        assignment,
                        None,
                        now,
                        rows,
                    )
                    .await?;
                }
                transaction.commit().await.map_err(map_sqlx_error)
            }
        })
        .await
    }
    async fn get_course_impl(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<Option<CourseRecord>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query("SELECT title FROM course WHERE tenant_id = $1 AND course_id = $2")
            .bind(context.tenant_id().as_uuid())
            .bind(course.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let member_rows = sqlx::query(
            "SELECT user_id, role FROM course_member \
             WHERE tenant_id = $1 AND course_id = $2 ORDER BY user_id",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let members = member_rows
            .iter()
            .map(|member| {
                let user = member.try_get("user_id").map_err(map_sqlx_error)?;
                let role: String = member.try_get("role").map_err(map_sqlx_error)?;
                Ok(CourseMembership {
                    user: UserId::from_uuid(user),
                    role: parse_course_membership_role(&role)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let record = CourseRecord {
            id: course,
            tenant: context.tenant_id(),
            title: row.try_get("title").map_err(map_sqlx_error)?,
            members,
        };
        validate_course(&record).map_err(|error| {
            StoreError::Unavailable(format!("stored course is invalid: {error}"))
        })?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(record))
    }
    async fn list_courses_impl(
        &self,
        context: TenantContext,
        scope: CourseListScope,
        page: PageRequest,
    ) -> Result<Page<CourseSummary>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let rows = match scope {
            CourseListScope::Member(user) => sqlx::query(MEMBER_COURSE_PAGE_SQL)
                .bind(context.tenant_id().as_uuid())
                .bind(user.as_uuid())
                .bind(cursor)
                .bind(limit)
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?,
            CourseListScope::TenantAdministrator => sqlx::query(
                "SELECT course_id::text AS stable_key, course_id, title, \
                        'administrator'::text AS role \
                 FROM course WHERE tenant_id = $1 \
                   AND ($2::text IS NULL OR course_id::text > $2) \
                 ORDER BY course_id::text LIMIT $3",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(cursor)
            .bind(limit)
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?,
        };
        let mut records = rows
            .iter()
            .map(|row| {
                let key: String = row.try_get("stable_key").map_err(map_sqlx_error)?;
                let id = CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?);
                let title = row.try_get("title").map_err(map_sqlx_error)?;
                let role: String = row.try_get("role").map_err(map_sqlx_error)?;
                Ok((
                    key,
                    CourseSummary {
                        id,
                        tenant: context.tenant_id(),
                        title,
                        role: parse_course_role(&role)?,
                    },
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let result = page_from_keyed_records(&mut records, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn put_course_group_impl(
        &self,
        context: TenantContext,
        command: PutCourseGroupCommand,
    ) -> Result<StoredCourseGroup, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
                ensure_tenant(context, command.record.tenant)?;
                validate_course_group(&command.record)?;
                let tenant = context.tenant_id();
                let mut transaction = self.begin_tenant(context).await?;
                let authorized = postgres_is_course_instructor(
                    &mut transaction,
                    tenant,
                    command.record.course,
                    command.actor,
                )
                .await?;
                let accessible: bool =
                    sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
                        .bind(tenant.as_uuid())
                        .bind(command.record.course.as_uuid())
                        .fetch_one(&mut *transaction)
                        .await
                        .map_err(map_sqlx_error)?;
                if !authorized || !accessible {
                    return Err(StoreError::NotFound);
                }
                let member_ids = command
                    .record
                    .members
                    .iter()
                    .map(UserId::as_uuid)
                    .collect::<Vec<_>>();
                let valid_members: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM course_member WHERE tenant_id = $1 AND course_id = $2 \
             AND role = 'student' AND user_id = ANY($3::uuid[])",
                )
                .bind(tenant.as_uuid())
                .bind(command.record.course.as_uuid())
                .bind(&member_ids)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                if valid_members
                    != i64::try_from(member_ids.len()).map_err(|_| {
                        StoreError::InvalidRecord("course group has too many members".to_string())
                    })?
                {
                    return Err(StoreError::NotFound);
                }

                let row = sqlx::query(
                    "SELECT course_id, title, revision FROM course_group \
             WHERE tenant_id = $1 AND course_group_id = $2 FOR UPDATE",
                )
                .bind(tenant.as_uuid())
                .bind(command.record.id.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                let existing = if let Some(row) = &row {
                    let members = assignment_timing::load_postgres_course_group_members(
                        &mut transaction,
                        tenant,
                        command.record.id,
                    )
                    .await?;
                    Some(StoredCourseGroup {
                        record: CourseGroupRecord {
                            id: command.record.id,
                            tenant,
                            course: CourseId::from_uuid(
                                row.try_get("course_id").map_err(map_sqlx_error)?,
                            ),
                            title: row.try_get("title").map_err(map_sqlx_error)?,
                            members,
                        },
                        revision: CourseGroupRevision::from_stored(
                            row.try_get("revision").map_err(map_sqlx_error)?,
                        )?,
                    })
                } else {
                    None
                };
                if let Some(existing) = &existing
                    && existing.record == command.record
                {
                    transaction.commit().await.map_err(map_sqlx_error)?;
                    return Ok(existing.clone());
                }
                let revision = match &existing {
                    Some(existing) if command.expected_revision == Some(existing.revision) => {
                        existing.revision.next()?
                    }
                    Some(_) => return Err(StoreError::Conflict),
                    None if command.expected_revision.is_none() => CourseGroupRevision::INITIAL,
                    None => return Err(StoreError::Conflict),
                };
                if existing
                    .as_ref()
                    .is_some_and(|record| record.record.course != command.record.course)
                {
                    return Err(StoreError::Conflict);
                }
                let affected = sqlx::query_scalar::<_, Uuid>(
                    "SELECT assignment_id FROM assignment_policy_exception \
             WHERE tenant_id = $1 AND course_group_id = $2 ORDER BY assignment_id",
                )
                .bind(tenant.as_uuid())
                .bind(command.record.id.as_uuid())
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?
                .into_iter()
                .map(AssignmentId::from_uuid)
                .collect::<BTreeSet<_>>();
                // BTreeSet iteration gives every concurrent group edit the same
                // assignment lock order before any active attempt/timing row lock.
                let mut locked = Vec::with_capacity(affected.len());
                for assignment in &affected {
                    assignment_timing::lock_postgres_assignment_policy(
                        &mut transaction,
                        tenant,
                        *assignment,
                    )
                    .await?;
                    locked.push((
                        *assignment,
                        assignment_timing::lock_postgres_active_timing_rows(
                            &mut transaction,
                            tenant,
                            *assignment,
                        )
                        .await?,
                    ));
                }
                let revision_i64 =
                    i64::try_from(revision.value()).map_err(|_| StoreError::Conflict)?;
                if existing.is_some() {
                    let updated = sqlx::query(
                        "UPDATE course_group SET title = $3, revision = $4, \
                 updated_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND course_group_id = $2",
                    )
                    .bind(tenant.as_uuid())
                    .bind(command.record.id.as_uuid())
                    .bind(&command.record.title)
                    .bind(revision_i64)
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                    if updated.rows_affected() != 1 {
                        return Err(StoreError::Conflict);
                    }
                } else {
                    sqlx::query(
                        "INSERT INTO course_group \
                 (tenant_id, course_id, course_group_id, title, revision) \
                 VALUES ($1, $2, $3, $4, $5)",
                    )
                    .bind(tenant.as_uuid())
                    .bind(command.record.course.as_uuid())
                    .bind(command.record.id.as_uuid())
                    .bind(&command.record.title)
                    .bind(revision_i64)
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                }
                sqlx::query(
                    "DELETE FROM course_group_member WHERE tenant_id = $1 AND course_group_id = $2",
                )
                .bind(tenant.as_uuid())
                .bind(command.record.id.as_uuid())
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                for user in &command.record.members {
                    sqlx::query(
                        "INSERT INTO course_group_member \
                 (tenant_id, course_id, course_group_id, user_id) VALUES ($1, $2, $3, $4)",
                    )
                    .bind(tenant.as_uuid())
                    .bind(command.record.course.as_uuid())
                    .bind(command.record.id.as_uuid())
                    .bind(user.as_uuid())
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                }
                let now = database_timestamp(&mut transaction).await?;
                for (assignment, rows) in locked {
                    assignment_timing::apply_postgres_locked_timing_rows(
                        &mut transaction,
                        tenant,
                        assignment,
                        None,
                        now,
                        rows,
                    )
                    .await?;
                }
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(StoredCourseGroup {
                    record: command.record,
                    revision,
                })
            }
        })
        .await
    }
    async fn get_course_group_impl(
        &self,
        context: TenantContext,
        group: CourseGroupId,
    ) -> Result<Option<StoredCourseGroup>, StoreError> {
        let tenant = context.tenant_id();
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT course_id, title, revision FROM course_group \
             WHERE tenant_id = $1 AND course_group_id = $2",
        )
        .bind(tenant.as_uuid())
        .bind(group.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = if let Some(row) = row {
            Some(StoredCourseGroup {
                record: CourseGroupRecord {
                    id: group,
                    tenant,
                    course: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
                    title: row.try_get("title").map_err(map_sqlx_error)?,
                    members: assignment_timing::load_postgres_course_group_members(
                        &mut transaction,
                        tenant,
                        group,
                    )
                    .await?,
                },
                revision: CourseGroupRevision::from_stored(
                    row.try_get("revision").map_err(map_sqlx_error)?,
                )?,
            })
        } else {
            None
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}
