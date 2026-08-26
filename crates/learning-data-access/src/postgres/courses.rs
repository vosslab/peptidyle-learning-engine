use async_trait::async_trait;
use question_model::{ActivityTimestamp, CourseMembershipId, StudentId};

use super::*;
use crate::CourseMembershipRecord;
use crate::assignment_revision_from_stored;

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

pub(super) fn map_course_group_mutator_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database_error) = &error {
        match database_error.code().as_deref() {
            // The legacy Store path reported all group authority and active
            // member failures as an unavailable object, not as a privilege
            // leak to the caller.
            Some("42501") if database_error.message() == "course group is unavailable" => {
                return StoreError::Conflict;
            }
            Some("42501") => return StoreError::NotFound,
            // Purpose/reference guards and stale expected revisions were
            // conflicts before the broker owned this aggregate.
            Some("23514") => return StoreError::Conflict,
            Some("55000") if database_error.message() == "course group revision conflict" => {
                return StoreError::Conflict;
            }
            _ => {}
        }
    }
    map_sqlx_error(error)
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
                let (entry, actor, session) = match command.authority {
                    crate::CourseCreationAuthority::ApprovedInstructor { actor, session } => (
                        "SELECT course_id, instructor_membership_id \
                         FROM public.ple_create_course_as_instructor_v1(\
                             $1, $2, $3, $4::date, $5::date, $6, $7, $8::character(64))",
                        actor,
                        session,
                    ),
                    crate::CourseCreationAuthority::Sysadmin { actor, session } => (
                        "SELECT course_id, instructor_membership_id \
                         FROM public.ple_create_course_as_sysadmin_v1(\
                             $1, $2, $3, $4::date, $5::date, $6, $7, $8::character(64))",
                        actor,
                        session,
                    ),
                };
                // ASVS 1.2.4, 2.2.1, 2.3.1, 2.3.3: the typed server-owned
                // authority selects a fixed broker entry; every untrusted
                // scalar remains a bound parameter and PostgreSQL atomically
                // rechecks the session/approval before bootstrap writes.
                let row = sqlx::query(entry)
                    .bind(tenant.as_uuid())
                    .bind(course_id.as_uuid())
                    .bind(&course.title)
                    .bind(course.term.start_date().as_str())
                    .bind(course.term.end_date().as_str())
                    .bind(course.term.time_zone().as_str())
                    .bind(actor.as_uuid())
                    .bind(session.to_string())
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                let returned_course: Uuid = row.try_get("course_id").map_err(map_sqlx_error)?;
                let membership: Uuid = row
                    .try_get("instructor_membership_id")
                    .map_err(map_sqlx_error)?;
                if returned_course != course_id.as_uuid() || membership.is_nil() {
                    return Err(StoreError::Unavailable(
                        "course creation broker returned an invalid aggregate identity".to_string(),
                    ));
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
                let mut member_ids = command
                    .record
                    .members
                    .iter()
                    .map(CourseMembershipId::as_uuid)
                    .collect::<Vec<_>>();
                member_ids.sort_unstable();
                let row = sqlx::query(
                    "SELECT revision, affected_assignment_ids, affected_assignment_revisions \
                     FROM ple_put_course_group_v1($1,$2,$3,$4,$5,$6,$7,$8)",
                )
                .bind(tenant.as_uuid())
                .bind(command.actor.as_uuid())
                .bind(command.record.course.as_uuid())
                .bind(command.record.id.as_uuid())
                .bind(
                    command
                        .expected_revision
                        .map(|revision| i64::try_from(revision.value()))
                        .transpose()
                        .map_err(|_| StoreError::Conflict)?,
                )
                .bind(encode_course_group_purpose(command.record.purpose))
                .bind(&command.record.title)
                .bind(member_ids)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_course_group_mutator_error)?;
                let revision = CourseGroupRevision::from_stored(
                    row.try_get("revision").map_err(map_sqlx_error)?,
                )?;
                let assignment_ids: Vec<Uuid> = row
                    .try_get("affected_assignment_ids")
                    .map_err(map_sqlx_error)?;
                let assignment_revisions: Vec<i64> = row
                    .try_get("affected_assignment_revisions")
                    .map_err(map_sqlx_error)?;
                if assignment_ids.len() != assignment_revisions.len()
                    || assignment_ids.windows(2).any(|pair| pair[0] >= pair[1])
                {
                    return Err(StoreError::Conflict);
                }
                for (assignment, revision) in assignment_ids.into_iter().zip(assignment_revisions) {
                    super::course_policy::reresolve_post_mutation_active_attempts(
                        &mut transaction,
                        context,
                        command.actor,
                        command.record.course,
                        AssignmentId::from_uuid(assignment),
                        assignment_revision_from_stored(revision)?,
                    )
                    .await?;
                }
                let row = sqlx::query(
                    "SELECT course_id, purpose, title, revision FROM course_group \
                     WHERE tenant_id=$1 AND course_group_id=$2",
                )
                .bind(tenant.as_uuid())
                .bind(command.record.id.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?
                .ok_or(StoreError::NotFound)?;
                let stored_course =
                    CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?);
                if stored_course != command.record.course {
                    return Err(StoreError::Conflict);
                }
                let stored = StoredCourseGroup {
                    record: CourseGroupRecord {
                        id: command.record.id,
                        tenant,
                        course: stored_course,
                        purpose: decode_course_group_purpose(
                            row.try_get("purpose").map_err(map_sqlx_error)?,
                        )?,
                        title: row.try_get("title").map_err(map_sqlx_error)?,
                        members: assignment_timing::load_postgres_course_group_members(
                            &mut transaction,
                            tenant,
                            command.record.id,
                        )
                        .await?,
                    },
                    revision,
                };
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(stored)
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
