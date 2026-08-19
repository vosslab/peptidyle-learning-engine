use crate::CreateCourseCommand;
use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::CourseStore for MemoryStore {
    async fn create_course_impl(
        &self,
        context: TenantContext,
        command: CreateCourseCommand,
    ) -> Result<(), StoreError> {
        let course = command.course;
        ensure_tenant(context, course.tenant)?;
        validate_course(&course)?;
        let tenant = course.tenant;
        let course_id = course.id;
        let mut state = self.write_state()?;
        if state.courses.contains_key(&(tenant, course_id)) {
            return Err(StoreError::AlreadyExists);
        }
        super::navigation_references::ensure_course_reference(&mut state, tenant, course_id)?;
        state.courses.insert((tenant, course_id), course);
        super::entitlement::create_initial_instructor_membership(
            &mut state,
            tenant,
            course_id,
            command.initial_instructor,
        )?;
        state
            .course_appearances
            .entry((tenant, course_id))
            .or_insert_with(|| question_model::CourseAppearance {
                theme: question_model::CourseThemeId::default(),
                revision: question_model::CourseAppearanceRevision::INITIAL,
                banner: None,
            });
        Ok(())
    }
    async fn get_course_impl(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<Option<CourseRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state.courses.get(&(context.tenant_id(), course)).cloned())
    }
    async fn get_current_course_membership_impl(
        &self,
        context: TenantContext,
        course: CourseId,
        user: UserId,
    ) -> Result<Option<CourseMembershipRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(
            super::entitlement::active_membership_for(&state, context.tenant_id(), course, user)
                .cloned(),
        )
    }
    async fn list_courses_impl(
        &self,
        context: TenantContext,
        scope: CourseListScope,
        page: PageRequest,
    ) -> Result<Page<CourseSummary>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .courses
            .iter()
            .filter_map(|((tenant, course_id), record)| {
                if *tenant != context.tenant_id() {
                    return None;
                }
                let role = match scope {
                    CourseListScope::Member(user) => {
                        super::entitlement::active_membership_for(&state, *tenant, *course_id, user)
                            .map(|membership| membership.role)?
                    }
                };
                if role == CourseMembershipRole::Student
                    && !course_records_accessible(&state, context.tenant_id(), *course_id)
                {
                    return None;
                }
                let public_id = state
                    .course_references
                    .get(&(*tenant, *course_id))
                    .copied()?;
                Some((course_id.to_string(), record.summary(role, public_id)))
            })
            .collect();
        Ok(page_records(records, &page))
    }
    async fn put_course_group_impl(
        &self,
        context: TenantContext,
        command: PutCourseGroupCommand,
    ) -> Result<StoredCourseGroup, StoreError> {
        ensure_tenant(context, command.record.tenant)?;
        validate_course_group(&command.record)?;
        let tenant = context.tenant_id();
        let key = (tenant, command.record.id);
        let mut state = self.write_state()?;
        require_course_records_accessible(&state, tenant, command.record.course)?;
        if !state.courses.contains_key(&(tenant, command.record.course))
            || super::entitlement::active_membership_for(
                &state,
                tenant,
                command.record.course,
                command.actor,
            )
            .is_none_or(|membership| membership.role != CourseMembershipRole::Instructor)
            || command.record.members.iter().any(|membership| {
                super::entitlement::active_membership_by_id(&state, tenant, *membership).is_none_or(
                    |record| {
                        record.course != command.record.course
                            || record.role != CourseMembershipRole::Student
                    },
                )
            })
        {
            return Err(StoreError::NotFound);
        }
        if state
            .course_groups
            .get(&key)
            .is_some_and(|existing| existing.course != command.record.course)
        {
            return Err(StoreError::Conflict);
        }
        if let Some(existing) = state.course_groups.get(&key)
            && existing == &command.record
        {
            return Ok(StoredCourseGroup {
                record: existing.clone(),
                revision: state
                    .course_group_revisions
                    .get(&key)
                    .copied()
                    .ok_or(StoreError::NotFound)?,
            });
        }
        let revision = match state.course_group_revisions.get(&key).copied() {
            Some(current) if command.expected_revision == Some(current) => current.next()?,
            Some(_) => return Err(StoreError::Conflict),
            None if command.expected_revision.is_none() => CourseGroupRevision::INITIAL,
            None => return Err(StoreError::Conflict),
        };
        state.course_groups.insert(key, command.record.clone());
        state.course_group_revisions.insert(key, revision);
        Ok(StoredCourseGroup {
            record: command.record,
            revision,
        })
    }
    async fn get_course_group_impl(
        &self,
        context: TenantContext,
        group: CourseGroupId,
    ) -> Result<Option<StoredCourseGroup>, StoreError> {
        let state = self.read_state()?;
        let key = (context.tenant_id(), group);
        let Some(record) = state.course_groups.get(&key).cloned() else {
            return Ok(None);
        };
        Ok(Some(StoredCourseGroup {
            record,
            revision: state
                .course_group_revisions
                .get(&key)
                .copied()
                .ok_or(StoreError::NotFound)?,
        }))
    }
}
