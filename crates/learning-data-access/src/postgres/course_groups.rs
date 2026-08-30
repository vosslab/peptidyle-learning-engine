//! PostgreSQL implementation of the T2 course-group management capability.

use async_trait::async_trait;
use question_model::{
    CourseGroupPurpose, CourseGroupPurposePolicy, CourseGroupReference, CourseMembershipId,
    MultipleMembershipPolicy,
};

use super::*;
use crate::{
    CourseGroupMembershipWarning, CourseGroupPurposePolicyRevision, CourseGroupView,
    StoredCourseGroupPurposePolicy, UpdateCourseGroupPurposePolicyCommand,
};

const PURPOSES: [CourseGroupPurpose; 5] = [
    CourseGroupPurpose::Section,
    CourseGroupPurpose::Lab,
    CourseGroupPurpose::Cohort,
    CourseGroupPurpose::Accommodation,
    CourseGroupPurpose::Work,
];

fn encode_multiple_membership(policy: MultipleMembershipPolicy) -> &'static str {
    match policy {
        MultipleMembershipPolicy::Allow => "allow",
        MultipleMembershipPolicy::Warn => "warn",
    }
}

fn decode_multiple_membership(value: String) -> Result<MultipleMembershipPolicy, StoreError> {
    match value.as_str() {
        "allow" => Ok(MultipleMembershipPolicy::Allow),
        "warn" => Ok(MultipleMembershipPolicy::Warn),
        _ => Err(StoreError::Unavailable(
            "stored course group membership policy is invalid".into(),
        )),
    }
}

fn map_purpose_policy_broker_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database_error) = &error {
        match database_error.code().as_deref() {
            Some("55000")
                if database_error.message() == "course group purpose policy revision conflict" =>
            {
                return StoreError::Conflict;
            }
            Some("42501") => return StoreError::NotFound,
            Some("55000") => {
                return StoreError::Unavailable(
                    "course group purpose policy aggregate is unavailable".to_string(),
                );
            }
            _ => {}
        }
    }
    map_sqlx_error(error)
}

async fn require_group_manager(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> Result<(), StoreError> {
    let accessible: bool =
        sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
            .bind(tenant.as_uuid())
            .bind(course.as_uuid())
            .fetch_one(&mut **tx)
            .await
            .map_err(map_sqlx_error)?;
    if !accessible || !postgres_is_course_instructor(tx, course, actor).await? {
        return Err(StoreError::NotFound);
    }
    Ok(())
}

async fn load_group(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    row: &PgRow,
) -> Result<CourseGroupView, StoreError> {
    let id = CourseGroupId::from_uuid(row.try_get("course_group_id").map_err(map_sqlx_error)?);
    let course = CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?);
    let public_id: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
    let reference = CourseGroupReference::new(public_id as u64).ok_or_else(|| {
        StoreError::Unavailable("stored course group route number is invalid".into())
    })?;
    Ok(CourseGroupView {
        reference,
        group: StoredCourseGroup {
            record: CourseGroupRecord {
                id,
                course,
                purpose: super::courses::decode_course_group_purpose(
                    row.try_get("purpose").map_err(map_sqlx_error)?,
                )?,
                title: row.try_get("title").map_err(map_sqlx_error)?,
                members: assignment_timing::load_postgres_course_group_members(tx, tenant, id)
                    .await?,
            },
            revision: CourseGroupRevision::from_stored(
                row.try_get("revision").map_err(map_sqlx_error)?,
            )?,
        },
    })
}

async fn ensure_complete_policy_rows(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<(), StoreError> {
    let rows = sqlx::query(
        "SELECT purpose, multiple_membership, revision FROM course_group_membership_policy \
         WHERE tenant_id=$1 AND course_id=$2 ORDER BY purpose",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    if rows.len() != PURPOSES.len() {
        return Err(StoreError::NotFound);
    }
    let mut found = BTreeSet::new();
    for row in rows {
        let purpose = super::courses::decode_course_group_purpose(
            row.try_get("purpose").map_err(map_sqlx_error)?,
        )?;
        decode_multiple_membership(row.try_get("multiple_membership").map_err(map_sqlx_error)?)?;
        CourseGroupPurposePolicyRevision::from_stored(
            row.try_get("revision").map_err(map_sqlx_error)?,
        )?;
        found.insert(purpose);
    }
    if found.len() != PURPOSES.len() || PURPOSES.iter().any(|purpose| !found.contains(purpose)) {
        return Err(StoreError::NotFound);
    }
    Ok(())
}

async fn load_purpose_policy(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    purpose: CourseGroupPurpose,
) -> Result<StoredCourseGroupPurposePolicy, StoreError> {
    let row = sqlx::query(
        "SELECT purpose, multiple_membership, revision FROM course_group_membership_policy \
         WHERE tenant_id=$1 AND course_id=$2 AND purpose=$3",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(super::courses::encode_course_group_purpose(purpose))
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let stored_purpose = super::courses::decode_course_group_purpose(
        row.try_get("purpose").map_err(map_sqlx_error)?,
    )?;
    if stored_purpose != purpose {
        return Err(StoreError::Unavailable(
            "stored course group policy purpose is invalid".into(),
        ));
    }
    Ok(StoredCourseGroupPurposePolicy {
        policy: CourseGroupPurposePolicy {
            purpose,
            multiple_membership: decode_multiple_membership(
                row.try_get("multiple_membership").map_err(map_sqlx_error)?,
            )?,
        },
        revision: CourseGroupPurposePolicyRevision::from_stored(
            row.try_get("revision").map_err(map_sqlx_error)?,
        )?,
    })
}

#[async_trait]
impl crate::CourseGroupManagementStore for PostgresStore {
    async fn list_course_groups(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<CourseGroupView>, StoreError> {
        let tenant = context.tenant_id();
        let cursor = page.after.as_ref().map(|value| value.as_str().to_owned());
        let limit = i64::from(page.size.get()) + 1;
        let mut tx = self.begin_tenant(context).await?;
        require_group_manager(&mut tx, tenant, course, actor).await?;
        ensure_complete_policy_rows(&mut tx, tenant, course).await?;
        let rows = sqlx::query(
            "SELECT course_group_id, course_id, public_id, purpose, title, revision, \
             lpad(public_id::text, 10, '0') AS stable_key \
             FROM course_group WHERE tenant_id=$1 AND course_id=$2 AND \
             ($3::text IS NULL OR lpad(public_id::text, 10, '0') > $3) \
             ORDER BY public_id LIMIT $4",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let key: String = row.try_get("stable_key").map_err(map_sqlx_error)?;
            records.push((key, load_group(&mut tx, tenant, &row).await?));
        }
        let result = page_from_keyed_records(&mut records, page.size.get())?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn get_course_group_by_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        reference: CourseGroupReference,
    ) -> Result<Option<CourseGroupView>, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        require_group_manager(&mut tx, tenant, course, actor).await?;
        ensure_complete_policy_rows(&mut tx, tenant, course).await?;
        let row = sqlx::query(
            "SELECT course_group_id, course_id, public_id, purpose, title, revision \
             FROM course_group WHERE tenant_id=$1 AND course_id=$2 AND public_id=$3",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(i64::from(reference.number()))
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let result = match row {
            Some(row) => Some(load_group(&mut tx, tenant, &row).await?),
            None => None,
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn get_course_group_by_id_for_instructor(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        group: CourseGroupId,
    ) -> Result<Option<CourseGroupView>, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        require_group_manager(&mut tx, tenant, course, actor).await?;
        ensure_complete_policy_rows(&mut tx, tenant, course).await?;
        let row = sqlx::query(
            "SELECT course_group_id, course_id, public_id, purpose, title, revision \
             FROM course_group WHERE tenant_id=$1 AND course_id=$2 AND course_group_id=$3",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(group.as_uuid())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let result = match row {
            Some(row) => Some(load_group(&mut tx, tenant, &row).await?),
            None => None,
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn delete_course_group(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        group: CourseGroupId,
        expected_revision: CourseGroupRevision,
    ) -> Result<bool, StoreError> {
        retry_transaction(|| async move {
            let tenant = context.tenant_id();
            let mut tx = self.begin_tenant(context).await?;
            let deleted: bool =
                sqlx::query_scalar("SELECT ple_delete_course_group_v1($1,$2,$3,$4,$5)")
                    .bind(tenant.as_uuid())
                    .bind(actor.as_uuid())
                    .bind(course.as_uuid())
                    .bind(group.as_uuid())
                    .bind(
                        i64::try_from(expected_revision.value())
                            .map_err(|_| StoreError::Conflict)?,
                    )
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(super::courses::map_course_group_mutator_error)?;
            tx.commit().await.map_err(map_sqlx_error)?;
            Ok(deleted)
        })
        .await
    }

    async fn get_course_group_purpose_policy(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        purpose: CourseGroupPurpose,
    ) -> Result<Option<StoredCourseGroupPurposePolicy>, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        require_group_manager(&mut tx, tenant, course, actor).await?;
        ensure_complete_policy_rows(&mut tx, tenant, course).await?;
        let policy = load_purpose_policy(&mut tx, tenant, course, purpose).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(policy))
    }

    async fn update_course_group_purpose_policy(
        &self,
        context: TenantContext,
        command: UpdateCourseGroupPurposePolicyCommand,
    ) -> Result<StoredCourseGroupPurposePolicy, StoreError> {
        retry_transaction(|| async move {
            let tenant = context.tenant_id();
            let mut tx = self.begin_tenant(context).await?;
            // ASVS 2.2.1-2.2.3, 2.3.1-2.3.4, 8.2.1-8.2.3, 8.3.1-8.3.3,
            // 15.4.2-15.4.3: PostgreSQL locks the presented live session,
            // exact course authority, and closed five-row policy aggregate in
            // one CAS.  The public command carries no caller-selected actor.
            let row = sqlx::query(
                "SELECT tenant_id, actor_id, course_id, purpose, multiple_membership, revision \
                 FROM public.ple_replace_course_group_purpose_policy_v1($1,$2::character(64),$3,$4,$5,$6)",
            )
            .bind(tenant.as_uuid())
            .bind(command.session.to_string())
            .bind(command.course.as_uuid())
            .bind(super::courses::encode_course_group_purpose(
                command.policy.purpose,
            ))
            .bind(encode_multiple_membership(
                command.policy.multiple_membership,
            ))
            .bind(
                i64::try_from(command.expected_revision.value())
                    .map_err(|_| StoreError::Conflict)?,
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(map_purpose_policy_broker_error)?;
            let returned_tenant: Uuid = row.try_get("tenant_id").map_err(map_sqlx_error)?;
            let returned_actor: Uuid = row.try_get("actor_id").map_err(map_sqlx_error)?;
            let returned_course: Uuid = row.try_get("course_id").map_err(map_sqlx_error)?;
            let returned_purpose = super::courses::decode_course_group_purpose(
                row.try_get("purpose").map_err(map_sqlx_error)?,
            )?;
            let returned_membership = decode_multiple_membership(
                row.try_get("multiple_membership").map_err(map_sqlx_error)?,
            )?;
            let revision = CourseGroupPurposePolicyRevision::from_stored(
                row.try_get("revision").map_err(map_sqlx_error)?,
            )?;
            if returned_tenant != tenant.as_uuid()
                || returned_actor.is_nil()
                || returned_course != command.course.as_uuid()
                || returned_purpose != command.policy.purpose
                || returned_membership != command.policy.multiple_membership
                || revision.value() != command.expected_revision.value().saturating_add(1)
            {
                return Err(StoreError::Unavailable(
                    "course group purpose policy broker returned an invalid witness".to_string(),
                ));
            }
            tx.commit().await.map_err(map_sqlx_error)?;
            Ok(StoredCourseGroupPurposePolicy {
                policy: CourseGroupPurposePolicy {
                    purpose: returned_purpose,
                    multiple_membership: returned_membership,
                },
                revision,
            })
        })
        .await
    }

    async fn course_group_membership_warnings(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
    ) -> Result<Vec<CourseGroupMembershipWarning>, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        require_group_manager(&mut tx, tenant, course, actor).await?;
        ensure_complete_policy_rows(&mut tx, tenant, course).await?;
        let rows = sqlx::query(
            "SELECT member.course_membership_id, groups.purpose, \
             count(*)::bigint AS membership_count \
             FROM course_group_member AS member JOIN course_group AS groups \
             ON groups.tenant_id=member.tenant_id AND groups.course_id=member.course_id \
             AND groups.course_group_id=member.course_group_id \
             JOIN course_group_membership_policy AS policy \
             ON policy.tenant_id=groups.tenant_id AND policy.course_id=groups.course_id \
             AND policy.purpose=groups.purpose \
             WHERE groups.tenant_id=$1 AND groups.course_id=$2 \
             AND policy.multiple_membership='warn' \
             GROUP BY member.course_membership_id, groups.purpose HAVING count(*) > 1 \
             ORDER BY groups.purpose, member.course_membership_id",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let mut warnings = Vec::with_capacity(rows.len());
        for row in rows {
            let count: i64 = row.try_get("membership_count").map_err(map_sqlx_error)?;
            warnings.push(CourseGroupMembershipWarning {
                membership: CourseMembershipId::from_uuid(
                    row.try_get("course_membership_id")
                        .map_err(map_sqlx_error)?,
                ),
                purpose: super::courses::decode_course_group_purpose(
                    row.try_get("purpose").map_err(map_sqlx_error)?,
                )?,
                membership_count: u32::try_from(count).map_err(|_| {
                    StoreError::Unavailable("stored group membership count is invalid".into())
                })?,
                disposition: question_model::MultipleMembershipDisposition::AllowedWithWarning,
            });
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn membership_policy_decode_is_closed() {
        assert_eq!(
            decode_multiple_membership("allow".into()).unwrap(),
            MultipleMembershipPolicy::Allow
        );
        assert_eq!(
            decode_multiple_membership("warn".into()).unwrap(),
            MultipleMembershipPolicy::Warn
        );
        assert!(decode_multiple_membership("default".into()).is_err());
    }
}
