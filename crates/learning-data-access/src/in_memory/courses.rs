use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::CourseStore for MemoryStore {
    async fn upsert_course_impl(
        &self,
        context: TenantContext,
        course: CourseRecord,
    ) -> Result<(), StoreError> {
        ensure_tenant(context, course.tenant)?;
        validate_course(&course)?;
        let tenant = course.tenant;
        let course_id = course.id;
        let student_members = course
            .members
            .iter()
            .filter_map(|membership| {
                (membership.role == question_model::CourseMembershipRole::Student)
                    .then_some(membership.user)
            })
            .collect::<BTreeSet<_>>();
        let mut state = self.write_state()?;
        let affected_groups = state
            .course_groups
            .iter()
            .filter_map(|((record_tenant, group), record)| {
                (*record_tenant == tenant && record.course == course_id).then_some(*group)
            })
            .collect::<BTreeSet<_>>();
        let affected_assignments = state
            .assignment_policy_exceptions
            .iter()
            .filter_map(|((record_tenant, assignment, target), _)| {
                (*record_tenant == tenant
                    && matches!(
                        target,
                        AssignmentPolicyExceptionTarget::CourseGroup(group)
                            if affected_groups.contains(group)
                    ))
                .then_some(*assignment)
            })
            .collect::<BTreeSet<_>>();
        let snapshot = state.clone();
        state.courses.insert((tenant, course_id), course);
        state
            .course_appearances
            .entry((tenant, course_id))
            .or_insert_with(|| question_model::CourseAppearance {
                theme: question_model::CourseThemeId::default(),
                revision: question_model::CourseAppearanceRevision::INITIAL,
                banner: None,
            });
        for group in &affected_groups {
            if let Some(record) = state.course_groups.get_mut(&(tenant, *group)) {
                record
                    .members
                    .retain(|member| student_members.contains(member));
            }
        }
        for assignment in affected_assignments {
            if let Err(error) =
                apply_memory_assignment_timing_update(&mut state, tenant, assignment, None)
            {
                *state = snapshot;
                return Err(error);
            }
        }
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
                    CourseListScope::Member(user) => record.role_for(user)?,
                };
                if role == CourseMembershipRole::Student
                    && !course_records_accessible(&state, context.tenant_id(), *course_id)
                {
                    return None;
                }
                Some((course_id.to_string(), record.summary(role)))
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
        let course = state
            .courses
            .get(&(tenant, command.record.course))
            .ok_or(StoreError::NotFound)?;
        if course.role_for(command.actor) != Some(CourseMembershipRole::Instructor)
            || command
                .record
                .members
                .iter()
                .any(|user| course.role_for(*user) != Some(CourseMembershipRole::Student))
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
        let affected = state
            .assignment_policy_exceptions
            .iter()
            .filter_map(|((record_tenant, assignment, target), _)| {
                (*record_tenant == tenant
                    && *target == AssignmentPolicyExceptionTarget::CourseGroup(command.record.id))
                .then_some(*assignment)
            })
            .collect::<BTreeSet<_>>();
        let snapshot = state.clone();
        state.course_groups.insert(key, command.record.clone());
        state.course_group_revisions.insert(key, revision);
        for assignment in affected {
            if let Err(error) =
                apply_memory_assignment_timing_update(&mut state, tenant, assignment, None)
            {
                *state = snapshot;
                return Err(error);
            }
        }
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
