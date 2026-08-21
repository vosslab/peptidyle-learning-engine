use async_trait::async_trait;
use question_model::{ActivityTimestamp, CourseMembershipId, StudentId};

use super::*;
use crate::CourseMembershipRecord;

fn random_membership_id() -> Result<Uuid, StoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        StoreError::Unavailable(format!(
            "course membership ID randomness unavailable: {error}"
        ))
    })?;
    Ok(Uuid::from_bytes(bytes))
}

pub(super) fn encode_course_group_purpose(
    purpose: question_model::CourseGroupPurpose,
) -> &'static str {
    match purpose {
        question_model::CourseGroupPurpose::Section => "section",
        question_model::CourseGroupPurpose::Lab => "lab",
        question_model::CourseGroupPurpose::Cohort => "cohort",
        question_model::CourseGroupPurpose::Accommodation => "accommodation",
        question_model::CourseGroupPurpose::Work => "work",
    }
}

pub(super) fn decode_course_group_purpose(
    value: String,
) -> Result<question_model::CourseGroupPurpose, StoreError> {
    match value.as_str() {
        "section" => Ok(question_model::CourseGroupPurpose::Section),
        "lab" => Ok(question_model::CourseGroupPurpose::Lab),
        "cohort" => Ok(question_model::CourseGroupPurpose::Cohort),
        "accommodation" => Ok(question_model::CourseGroupPurpose::Accommodation),
        "work" => Ok(question_model::CourseGroupPurpose::Work),
        _ => Err(StoreError::Unavailable(
            "stored course group purpose is invalid".to_string(),
        )),
    }
}

#[async_trait]
impl crate::CourseStore for PostgresStore {
    async fn create_course_impl(
        &self,
        context: TenantContext,
        command: CreateCourseCommand,
    ) -> Result<(), StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
                let course = command.course;
                ensure_tenant(context, course.tenant)?;
                validate_course(&course)?;
                let tenant = course.tenant;
                let course_id = course.id;
                let mut transaction = self.begin_tenant(context).await?;
                sqlx::query(
                    "INSERT INTO course (tenant_id, course_id, title, term_start_date, \
                     term_end_date, time_zone) VALUES ($1, $2, $3, $4::date, $5::date, $6)",
                )
                .bind(tenant.as_uuid())
                .bind(course_id.as_uuid())
                .bind(&course.title)
                .bind(course.term.start_date().as_str())
                .bind(course.term.end_date().as_str())
                .bind(course.term.time_zone().as_str())
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)
                .map_err(|error| match error {
                    StoreError::AlreadyExists => StoreError::AlreadyExists,
                    other => other,
                })?;
                super::course_roster::ensure_roster_state(&mut transaction, tenant, course_id)
                    .await?;
                sqlx::query(
                    "INSERT INTO course_member \
                     (tenant_id, course_id, course_membership_id, user_id, role, student_id, status, joined_at) \
                     VALUES ($1, $2, $3, $4, 'instructor', NULL, 'active', transaction_timestamp())",
                )
                .bind(tenant.as_uuid())
                .bind(course_id.as_uuid())
                .bind(random_membership_id()?)
                .bind(command.initial_instructor.as_uuid())
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
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
        let row = sqlx::query(
            "SELECT title, term_start_date::text AS term_start_date, \
             term_end_date::text AS term_end_date, time_zone FROM course \
             WHERE tenant_id = $1 AND course_id = $2",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let record = CourseRecord {
            id: course,
            tenant: context.tenant_id(),
            title: row.try_get("title").map_err(map_sqlx_error)?,
            term: decode_course_term(&row)?,
        };
        validate_course(&record).map_err(|error| {
            StoreError::Unavailable(format!("stored course is invalid: {error}"))
        })?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(record))
    }
    async fn get_current_course_membership_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        user: UserId,
    ) -> Result<Option<CourseMembershipRecord>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT course_membership_id, student_id, role, roster_id, status, \
             floor(extract(epoch FROM joined_at) * 1000)::bigint AS joined_at_millis, \
             floor(extract(epoch FROM revoked_at) * 1000)::bigint AS revoked_at_millis \
             FROM course_member WHERE tenant_id = $1 AND course_id = $2 AND user_id = $3 \
             AND status = 'active'",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(course.as_uuid())
        .bind(user.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = row
            .map(|row| {
                Ok::<CourseMembershipRecord, StoreError>(CourseMembershipRecord {
                    id: CourseMembershipId::from_uuid(
                        row.try_get("course_membership_id")
                            .map_err(map_sqlx_error)?,
                    ),
                    tenant: context.tenant_id(),
                    course,
                    user,
                    student: row
                        .try_get::<Option<Uuid>, _>("student_id")
                        .map_err(map_sqlx_error)?
                        .map(StudentId::from_uuid),
                    role: parse_course_membership_role(
                        &row.try_get::<String, _>("role").map_err(map_sqlx_error)?,
                    )?,
                    roster_id: row
                        .try_get::<Option<String>, _>("roster_id")
                        .map_err(map_sqlx_error)?
                        .map(|value| crate::CourseRosterId::parse(&value))
                        .transpose()
                        .map_err(|error| StoreError::Unavailable(error.to_string()))?,
                    status: crate::CourseMemberStatus::Active,
                    joined_at: ActivityTimestamp::from_unix_millis(
                        row.try_get("joined_at_millis").map_err(map_sqlx_error)?,
                    ),
                    revoked_at: row
                        .try_get::<Option<i64>, _>("revoked_at_millis")
                        .map_err(map_sqlx_error)?
                        .map(ActivityTimestamp::from_unix_millis),
                })
            })
            .transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
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
        };
        let mut records = rows
            .iter()
            .map(|row| {
                let key: String = row.try_get("stable_key").map_err(map_sqlx_error)?;
                let id = CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?);
                let public_number: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
                let reference = question_model::CourseReference::new(public_number as u64)
                    .ok_or_else(|| {
                        StoreError::Unavailable("stored course route number is invalid".to_string())
                    })?;
                let title = row.try_get("title").map_err(map_sqlx_error)?;
                let term = decode_course_term(row)?;
                let role: String = row.try_get("role").map_err(map_sqlx_error)?;
                Ok((
                    key,
                    CourseSummary {
                        id,
                        reference,
                        tenant: context.tenant_id(),
                        title,
                        term,
                        role: parse_course_membership_role(&role)?,
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
                    .map(CourseMembershipId::as_uuid)
                    .collect::<Vec<_>>();
                let valid_members: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM course_member WHERE tenant_id = $1 AND course_id = $2 \
             AND role = 'student' AND status = 'active' AND course_membership_id = ANY($3::uuid[])",
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
                    "SELECT course_id, purpose, title, revision FROM course_group \
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
                            purpose: decode_course_group_purpose(
                                row.try_get("purpose").map_err(map_sqlx_error)?,
                            )?,
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
                if existing
                    .as_ref()
                    .is_some_and(|stored| stored.record.purpose != command.record.purpose)
                {
                    super::course_groups::validate_group_purpose_transition(
                        &mut transaction,
                        tenant,
                        command.record.course,
                        command.record.id,
                        command.record.purpose,
                    )
                    .await?;
                }
                let affected = sqlx::query_scalar::<_, Uuid>(
                    "SELECT assignment_id FROM assignment_audience_group \
             WHERE tenant_id = $1 AND course_id = $2 AND course_group_id = $3 UNION \
             SELECT assignment_id FROM assignment_group_schedule_offset \
             WHERE tenant_id = $1 AND course_id = $2 AND course_group_id = $3 UNION \
             SELECT assignment_id FROM assignment_group_accommodation \
             WHERE tenant_id = $1 AND course_id = $2 AND course_group_id = $3 ORDER BY assignment_id",
                )
                .bind(tenant.as_uuid())
                .bind(command.record.course.as_uuid())
                .bind(command.record.id.as_uuid())
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?
                .into_iter()
                .map(AssignmentId::from_uuid)
                .collect::<BTreeSet<_>>();
                // BTreeSet iteration gives every concurrent group edit the same
                // assignment lock order before any active attempt/timing row lock.
                for assignment in &affected {
                    assignment_timing::lock_postgres_assignment_policy(
                        &mut transaction,
                        tenant,
                        *assignment,
                    )
                    .await?;
                }
                let revision_i64 =
                    i64::try_from(revision.value()).map_err(|_| StoreError::Conflict)?;
                if existing.is_some() {
                    let updated = sqlx::query(
                        "UPDATE course_group SET purpose = $3, title = $4, revision = $5, \
                 updated_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND course_group_id = $2",
                    )
                    .bind(tenant.as_uuid())
                    .bind(command.record.id.as_uuid())
                    .bind(encode_course_group_purpose(command.record.purpose))
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
                 (tenant_id, course_id, course_group_id, purpose, title, revision) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
                    )
                    .bind(tenant.as_uuid())
                    .bind(command.record.course.as_uuid())
                    .bind(command.record.id.as_uuid())
                    .bind(encode_course_group_purpose(command.record.purpose))
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
                for membership in &command.record.members {
                    let inserted = sqlx::query(
                        "INSERT INTO course_group_member \
                 (tenant_id, course_id, course_group_id, course_membership_id) \
                 VALUES ($1, $2, $3, $4)",
                    )
                    .bind(tenant.as_uuid())
                    .bind(command.record.course.as_uuid())
                    .bind(command.record.id.as_uuid())
                    .bind(membership.as_uuid())
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                    if inserted.rows_affected() != 1 {
                        return Err(StoreError::NotFound);
                    }
                }
                // A course-group edit changes the S5 facts from which S3
                // derives applicable scopes. Re-resolve only assignments that
                // actually reference this group, in the lock order above.
                for assignment in affected {
                    let revision: i64 = sqlx::query_scalar(
                        "SELECT revision FROM assignment WHERE tenant_id=$1 AND assignment_id=$2 FOR UPDATE",
                    )
                    .bind(tenant.as_uuid())
                    .bind(assignment.as_uuid())
                    .fetch_optional(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?
                    .ok_or(StoreError::NotFound)?;
                    super::course_policy::reresolve_active_attempts(
                        &mut transaction,
                        tenant,
                        command.record.course,
                        assignment,
                        AssignmentRevision::from_stored(revision)?,
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
            "SELECT course_id, purpose, title, revision FROM course_group \
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
                    purpose: decode_course_group_purpose(
                        row.try_get("purpose").map_err(map_sqlx_error)?,
                    )?,
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

fn decode_course_term(
    row: &sqlx::postgres::PgRow,
) -> Result<question_model::CourseTerm, StoreError> {
    let start_date: String = row.try_get("term_start_date").map_err(map_sqlx_error)?;
    let end_date: String = row.try_get("term_end_date").map_err(map_sqlx_error)?;
    let time_zone: String = row.try_get("time_zone").map_err(map_sqlx_error)?;
    question_model::CourseTerm::from_parts(&start_date, &end_date, &time_zone)
        .map_err(|error| StoreError::Unavailable(format!("stored course term is invalid: {error}")))
}
