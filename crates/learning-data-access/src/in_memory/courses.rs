use crate::{CourseCreationAuthority, CreateCourseCommand};
use crate::{
    CourseGroupMembershipWarning, CourseGroupPurposePolicyRevision, CourseGroupView,
    StoredCourseGroupPurposePolicy, UpdateCourseGroupPurposePolicyCommand,
};
use async_trait::async_trait;

use super::*;

#[cfg(test)]
use std::sync::{Mutex, OnceLock};

/// A unit-test-only fault immediately after the reference and course rows
/// exist. It exercises the transaction boundary without exposing an
/// application/test-support capability in non-test builds.
#[cfg(test)]
static CREATE_COURSE_LATE_FAILURE: OnceLock<Mutex<Option<(TenantId, CourseId)>>> = OnceLock::new();

#[cfg(test)]
fn arm_create_course_late_failure(tenant: TenantId, course: CourseId) {
    *CREATE_COURSE_LATE_FAILURE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .expect("course-creation test fault lock is available") = Some((tenant, course));
}

#[cfg(test)]
fn consume_create_course_late_failure(tenant: TenantId, course: CourseId) -> bool {
    let mut armed = CREATE_COURSE_LATE_FAILURE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .expect("course-creation test fault lock is available");
    if *armed == Some((tenant, course)) {
        *armed = None;
        true
    } else {
        false
    }
}

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
        let initial_instructor = authorize_course_creation(&state, context, &command.authority)?;
        if state.courses.contains_key(&(tenant, course_id)) {
            return Err(StoreError::AlreadyExists);
        }
        // The lock serializes readers and writers, but does not make a series
        // of map updates transactional.  Keep an exact pre-provisioning
        // snapshot and publish only a complete aggregate.
        let snapshot = state.clone();
        let result = provision_course_locked(&mut state, course, initial_instructor).map(|_| ());
        if let Err(error) = result {
            *state = snapshot;
            return Err(error);
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
        let revision = match state.course_group_revisions.get(&key).copied() {
            Some(current) if command.expected_revision == Some(current) => {
                if state.course_groups.get(&key) == Some(&command.record) {
                    return Ok(StoredCourseGroup {
                        record: command.record,
                        revision: current,
                    });
                }
                current.next()?
            }
            Some(_) => return Err(StoreError::Conflict),
            None if command.expected_revision.is_none() => CourseGroupRevision::INITIAL,
            None => return Err(StoreError::Conflict),
        };
        let snapshot = state.clone();
        validate_group_purpose_transition(&state, tenant, &command.record)?;
        super::navigation_references::ensure_course_group_reference(
            &mut state,
            tenant,
            command.record.id,
        )?;
        state.course_groups.insert(key, command.record.clone());
        state.course_group_revisions.insert(key, revision);
        if let Err(error) = reresolve_group_affected_assignments(
            &mut state,
            tenant,
            command.record.course,
            command.record.id,
        ) {
            *state = snapshot;
            return Err(error);
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

/// Materializes one complete ordinary course while the caller holds the sole
/// Memory write lock. Callers retain a full `State` snapshot for rollback.
///
/// Keeping reference allocation, direct Instructor authority, defaults, and
/// the schedule revision in this transition prevents partial provisioning and
/// lock re-entry (ASVS 2.3.3, 15.4.2, 15.4.3).
pub(super) fn provision_course_locked(
    state: &mut State,
    course: CourseRecord,
    initial_instructor: UserId,
) -> Result<question_model::CourseReference, StoreError> {
    let tenant = course.tenant;
    let course_id = course.id;
    if state.courses.contains_key(&(tenant, course_id)) {
        return Err(StoreError::AlreadyExists);
    }
    validate_course(&course)?;
    let reference =
        super::navigation_references::ensure_course_reference(state, tenant, course_id)?;
    state.courses.insert((tenant, course_id), course);
    #[cfg(test)]
    if consume_create_course_late_failure(tenant, course_id) {
        return Err(StoreError::Unavailable(
            "injected late course-creation failure".to_string(),
        ));
    }
    super::entitlement::create_initial_instructor_membership(
        state,
        tenant,
        course_id,
        initial_instructor,
    )?;
    state.roster_policies.insert(
        (tenant, course_id),
        super::course_roster::initial_roster_policy(course_id),
    );
    state.course_grade_schemes.insert(
        (tenant, course_id),
        super::course_gradebook::initial_course_grade_scheme(course_id),
    );
    for purpose in ALL_GROUP_PURPOSES {
        state.course_group_purpose_policies.insert(
            (tenant, course_id, purpose),
            StoredCourseGroupPurposePolicy {
                policy: question_model::CourseGroupPurposePolicy::default_for_purpose(purpose),
                revision: CourseGroupPurposePolicyRevision::INITIAL,
            },
        );
    }
    state.course_appearances.insert(
        (tenant, course_id),
        question_model::CourseAppearance {
            theme: question_model::CourseThemeId::default(),
            revision: question_model::CourseAppearanceRevision::INITIAL,
            banner: None,
        },
    );
    state.course_schedule_revisions.insert(
        (tenant, course_id),
        question_model::CourseScheduleRevision::INITIAL,
    );
    Ok(reference)
}

fn authorize_course_creation(
    state: &State,
    context: TenantContext,
    authority: &CourseCreationAuthority,
) -> Result<UserId, StoreError> {
    match authority {
        CourseCreationAuthority::ApprovedInstructor { actor, session } => {
            let subject = super::sessions::active_subject(state, context, *session)
                .ok_or(StoreError::NotFound)?;
            if subject.user() != *actor
                || !subject
                    .roles()
                    .contains(&question_model::UserRole::Instructor)
            {
                return Err(StoreError::Forbidden);
            }
            let approval = state.instructor_approvals.get(actor).copied();
            let approval = approval.ok_or(StoreError::Forbidden)?;
            domain::teaching_authority::validate_instructor_approval(
                &approval.approval,
                state.authoritative_time,
            )
            .map_err(|error| {
                StoreError::InvalidRecord(format!("invalid instructor approval: {error:?}"))
            })?;
            (approval.approval.user == *actor && approval.approval.revoked_at.is_none())
                .then_some(*actor)
                .ok_or(StoreError::Forbidden)
        }
        CourseCreationAuthority::Sysadmin { actor, session } => {
            let subject = super::sessions::active_subject(state, context, *session)
                .ok_or(StoreError::NotFound)?;
            (subject.user() == *actor
                && subject
                    .roles()
                    .contains(&question_model::UserRole::Sysadmin))
            .then_some(*actor)
            .ok_or(StoreError::Forbidden)
        }
    }
}

#[async_trait]
impl crate::CourseGroupManagementStore for MemoryStore {
    async fn list_course_groups(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<CourseGroupView>, StoreError> {
        let state = self.read_state()?;
        require_group_manager(&state, context.tenant_id(), course, actor)?;
        let records = state
            .course_groups
            .iter()
            .filter(|((tenant, _), record)| {
                *tenant == context.tenant_id() && record.course == course
            })
            .map(|((tenant, id), record)| {
                let reference = state
                    .course_group_references
                    .get(&(*tenant, *id))
                    .copied()
                    .ok_or(StoreError::NotFound)?;
                let revision = state
                    .course_group_revisions
                    .get(&(*tenant, *id))
                    .copied()
                    .ok_or(StoreError::NotFound)?;
                Ok((
                    format!("{:010}", reference.number()),
                    CourseGroupView {
                        reference,
                        group: StoredCourseGroup {
                            record: record.clone(),
                            revision,
                        },
                    },
                ))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        Ok(page_records(records, &page))
    }

    async fn get_course_group_by_reference(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        reference: question_model::CourseGroupReference,
    ) -> Result<Option<CourseGroupView>, StoreError> {
        let state = self.read_state()?;
        require_group_manager(&state, context.tenant_id(), course, actor)?;
        let tenant = context.tenant_id();
        let Some(group) = state
            .course_groups_by_reference
            .get(&(tenant, reference))
            .copied()
        else {
            return Ok(None);
        };
        let Some(record) = state
            .course_groups
            .get(&(tenant, group))
            .filter(|record| record.course == course)
        else {
            return Ok(None);
        };
        let revision = state
            .course_group_revisions
            .get(&(tenant, group))
            .copied()
            .ok_or(StoreError::NotFound)?;
        Ok(Some(CourseGroupView {
            reference,
            group: StoredCourseGroup {
                record: record.clone(),
                revision,
            },
        }))
    }

    async fn get_course_group_by_id_for_instructor(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        group: CourseGroupId,
    ) -> Result<Option<CourseGroupView>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        require_group_manager(&state, tenant, course, actor)?;
        let Some(record) = state
            .course_groups
            .get(&(tenant, group))
            .filter(|record| record.course == course)
        else {
            return Ok(None);
        };
        let reference = state
            .course_group_references
            .get(&(tenant, group))
            .copied()
            .ok_or(StoreError::NotFound)?;
        let revision = state
            .course_group_revisions
            .get(&(tenant, group))
            .copied()
            .ok_or(StoreError::NotFound)?;
        Ok(Some(CourseGroupView {
            reference,
            group: StoredCourseGroup {
                record: record.clone(),
                revision,
            },
        }))
    }

    async fn delete_course_group(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        group: CourseGroupId,
        expected_revision: CourseGroupRevision,
    ) -> Result<bool, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        require_group_manager(&state, tenant, course, actor)?;
        let key = (tenant, group);
        let Some(record) = state.course_groups.get(&key) else {
            return Ok(false);
        };
        if record.course != course {
            return Ok(false);
        }
        if state.course_group_revisions.get(&key).copied() != Some(expected_revision) {
            return Err(StoreError::Conflict);
        }
        if group_is_referenced(&state, tenant, group) {
            return Err(StoreError::Conflict);
        }
        state.course_groups.remove(&key);
        state.course_group_revisions.remove(&key);
        let reference = state
            .course_group_references
            .remove(&key)
            .ok_or(StoreError::NotFound)?;
        state
            .course_groups_by_reference
            .remove(&(tenant, reference));
        Ok(true)
    }

    async fn get_course_group_purpose_policy(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        purpose: question_model::CourseGroupPurpose,
    ) -> Result<Option<StoredCourseGroupPurposePolicy>, StoreError> {
        let state = self.read_state()?;
        require_group_manager(&state, context.tenant_id(), course, actor)?;
        Ok(state
            .course_group_purpose_policies
            .get(&(context.tenant_id(), course, purpose))
            .copied())
    }

    async fn update_course_group_purpose_policy(
        &self,
        context: TenantContext,
        command: UpdateCourseGroupPurposePolicyCommand,
    ) -> Result<StoredCourseGroupPurposePolicy, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        // ASVS 8.2.1-8.2.3, 8.3.1-8.3.3, 15.4.2: resolve the live,
        // tenant-bound session and exact-course Instructor in the same write
        // critical section as the compare-and-swap. The command never carries
        // an actor identity that a caller could forge.
        require_session_group_policy_manager(&state, context, command.session, command.course)?;
        let key = (tenant, command.course, command.policy.purpose);
        let current = state
            .course_group_purpose_policies
            .get(&key)
            .copied()
            .ok_or(StoreError::NotFound)?;
        if current.revision != command.expected_revision {
            return Err(StoreError::Conflict);
        }
        let stored = StoredCourseGroupPurposePolicy {
            policy: command.policy,
            revision: current.revision.next()?,
        };
        state.course_group_purpose_policies.insert(key, stored);
        Ok(stored)
    }

    async fn course_group_membership_warnings(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
    ) -> Result<Vec<CourseGroupMembershipWarning>, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        require_group_manager(&state, tenant, course, actor)?;
        Ok(membership_warnings(&state, tenant, course))
    }
}

const ALL_GROUP_PURPOSES: [question_model::CourseGroupPurpose; 5] = [
    question_model::CourseGroupPurpose::Section,
    question_model::CourseGroupPurpose::Lab,
    question_model::CourseGroupPurpose::Cohort,
    question_model::CourseGroupPurpose::Accommodation,
    question_model::CourseGroupPurpose::Work,
];

fn require_group_manager(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    actor: UserId,
) -> Result<(), StoreError> {
    if !state.courses.contains_key(&(tenant, course))
        || super::entitlement::current_course_role(state, tenant, course, actor)
            != Some(CourseMembershipRole::Instructor)
    {
        return Err(StoreError::NotFound);
    }
    require_course_records_accessible(state, tenant, course)
}

fn require_session_group_policy_manager(
    state: &State,
    context: TenantContext,
    session: crate::SessionTokenHash,
    course: CourseId,
) -> Result<(), StoreError> {
    let subject =
        super::sessions::active_subject(state, context, session).ok_or(StoreError::NotFound)?;
    if !subject
        .roles()
        .contains(&question_model::UserRole::Instructor)
    {
        return Err(StoreError::NotFound);
    }
    super::teaching_authority::require_direct_instructor(
        state,
        context.tenant_id(),
        course,
        subject.user(),
    )?;
    require_course_records_accessible(state, context.tenant_id(), course)
}

fn group_is_referenced(state: &State, tenant: TenantId, group: CourseGroupId) -> bool {
    state.assignments.values().any(|assignment| {
        assignment.tenant == tenant && audience_mentions(&assignment.audience, group)
    }) || state
        .assignment_group_schedule_offsets
        .keys()
        .any(|(record_tenant, _, candidate)| *record_tenant == tenant && *candidate == group)
        || state
            .assignment_group_accommodations
            .keys()
            .any(|(record_tenant, _, candidate)| *record_tenant == tenant && *candidate == group)
}

fn validate_group_purpose_transition(
    state: &State,
    tenant: TenantId,
    proposed: &CourseGroupRecord,
) -> Result<(), StoreError> {
    let Some(existing) = state.course_groups.get(&(tenant, proposed.id)) else {
        return Ok(());
    };
    if existing.purpose == proposed.purpose {
        return Ok(());
    }
    let capabilities = question_model::GroupPurposeCapabilities::for_purpose(proposed.purpose);
    let audience_used = state.assignments.values().any(|assignment| {
        assignment.tenant == tenant
            && assignment.course_id == proposed.course
            && audience_mentions(&assignment.audience, proposed.id)
    });
    let schedule_used = state
        .assignment_group_schedule_offsets
        .keys()
        .any(|(record_tenant, _, group)| *record_tenant == tenant && *group == proposed.id);
    let accommodation_used = state
        .assignment_group_accommodations
        .keys()
        .any(|(record_tenant, _, group)| *record_tenant == tenant && *group == proposed.id);
    if (audience_used && !capabilities.assignment_audience)
        || (schedule_used && !capabilities.schedule_scope)
        || (accommodation_used && !capabilities.accommodation_scope)
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn membership_warnings(
    state: &State,
    tenant: TenantId,
    course: CourseId,
) -> Vec<CourseGroupMembershipWarning> {
    let mut counts = std::collections::BTreeMap::new();
    for group in state
        .course_groups
        .values()
        .filter(|group| group.tenant == tenant && group.course == course)
    {
        for membership in &group.members {
            *counts.entry((*membership, group.purpose)).or_insert(0_u32) += 1;
        }
    }
    counts
        .into_iter()
        .map(|((membership, purpose), membership_count)| {
            let policy = state
                .course_group_purpose_policies
                .get(&(tenant, course, purpose))
                .expect("course creation initializes all purpose policies");
            CourseGroupMembershipWarning {
                membership,
                purpose,
                membership_count,
                disposition: domain::teaching_authority::evaluate_multiple_membership(
                    policy.policy,
                    usize::try_from(membership_count).expect("u32 membership count fits usize"),
                ),
            }
        })
        .filter(|warning| {
            matches!(
                warning.disposition,
                question_model::MultipleMembershipDisposition::AllowedWithWarning
            )
        })
        .collect()
}

fn reresolve_group_affected_assignments(
    state: &mut State,
    tenant: TenantId,
    course: CourseId,
    group: CourseGroupId,
) -> Result<(), StoreError> {
    let assignments = state
        .assignments
        .values()
        .filter(|assignment| assignment.tenant == tenant && assignment.course_id == course)
        .filter(|assignment| {
            audience_mentions(&assignment.audience, group)
                || state.assignment_group_schedule_offsets.contains_key(&(
                    tenant,
                    assignment.id,
                    group,
                ))
                || state.assignment_group_accommodations.contains_key(&(
                    tenant,
                    assignment.id,
                    group,
                ))
        })
        .map(|assignment| assignment.id)
        .collect::<Vec<_>>();
    for assignment in assignments {
        super::course_policy::reresolve_active_assignment_attempts(
            state, tenant, course, assignment,
        )?;
    }
    Ok(())
}

fn audience_mentions(audience: &question_model::AssignmentAudience, group: CourseGroupId) -> bool {
    match audience {
        question_model::AssignmentAudience::CourseWide => false,
        question_model::AssignmentAudience::AnyOfGroups(groups) => {
            groups.iter().any(|candidate| candidate == group)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CourseRecord, InstructorApprovalRevision, SessionLifetime, SessionStore, SessionSubject,
        SessionTokenHash, Store, StoredInstructorApproval,
    };
    use question_model::{
        CourseMembershipRole, CourseTerm, InstructorApproval, TenantId, UserId, UserRole,
    };
    use uuid::Uuid;

    fn fixture_uuid(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn context(tenant: TenantId) -> TenantContext {
        TenantContext::from_authenticated_session(tenant)
    }

    fn course(tenant: TenantId, id: CourseId) -> CourseRecord {
        CourseRecord {
            id,
            tenant,
            title: "Memory aggregate fixture".to_string(),
            term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                .expect("fixed fixture term"),
        }
    }

    async fn sysadmin_authority(
        store: &MemoryStore,
        tenant: TenantId,
        user: UserId,
        token: &[u8],
    ) -> CourseCreationAuthority {
        let session = SessionTokenHash::compute(token);
        store
            .create_session(
                session,
                SessionSubject::new(tenant, user, "Course fixture", vec![UserRole::Sysadmin])
                    .expect("valid fixture subject"),
                SessionLifetime::from_seconds(3_600).expect("positive fixture lifetime"),
            )
            .await
            .expect("fixture session persists");
        CourseCreationAuthority::Sysadmin {
            actor: user,
            session,
        }
    }

    async fn approved_instructor_authority(
        store: &MemoryStore,
        tenant: TenantId,
        user: UserId,
        roles: Vec<UserRole>,
        token: &[u8],
    ) -> CourseCreationAuthority {
        let session = SessionTokenHash::compute(token);
        store
            .create_session(
                session,
                SessionSubject::new(tenant, user, "Course fixture", roles)
                    .expect("valid fixture subject"),
                SessionLifetime::from_seconds(3_600).expect("positive fixture lifetime"),
            )
            .await
            .expect("fixture session persists");
        let mut state = store.write_state().expect("Memory state is available");
        let approved_at = state.authoritative_time;
        state.instructor_approvals.insert(
            user,
            StoredInstructorApproval {
                approval: InstructorApproval {
                    user,
                    approved_by: user,
                    approved_at,
                    revoked_at: None,
                },
                revision: InstructorApprovalRevision::INITIAL,
            },
        );
        CourseCreationAuthority::ApprovedInstructor {
            actor: user,
            session,
        }
    }

    #[tokio::test]
    async fn approved_instructor_creation_requires_an_instructor_session_role() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(fixture_uuid(20));
        let instructor = UserId::from_uuid(fixture_uuid(21));
        let denied_course = CourseId::from_uuid(fixture_uuid(22));
        let student_authority = approved_instructor_authority(
            &store,
            tenant,
            instructor,
            vec![UserRole::Student],
            b"approved-student-session",
        )
        .await;

        assert_eq!(
            store
                .create_course(
                    context(tenant),
                    CreateCourseCommand {
                        course: course(tenant, denied_course),
                        authority: student_authority,
                    },
                )
                .await,
            Err(StoreError::Forbidden),
        );
        {
            let state = store.read_state().expect("Memory state is available");
            assert!(state.courses.is_empty());
            assert!(state.course_references.is_empty());
            assert!(state.course_memberships.is_empty());
            assert!(state.roster_policies.is_empty());
            assert!(state.course_grade_schemes.is_empty());
        }

        let instructor_authority = approved_instructor_authority(
            &store,
            tenant,
            instructor,
            vec![UserRole::Instructor],
            b"approved-instructor-session",
        )
        .await;
        store
            .create_course(
                context(tenant),
                CreateCourseCommand {
                    course: course(tenant, denied_course),
                    authority: instructor_authority,
                },
            )
            .await
            .expect("approved Instructor session creates a complete course");
    }

    #[tokio::test]
    async fn course_creation_physically_materializes_the_complete_initial_aggregate() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(fixture_uuid(1));
        let instructor = UserId::from_uuid(fixture_uuid(2));
        let course_id = CourseId::from_uuid(fixture_uuid(3));
        let authority =
            sysadmin_authority(&store, tenant, instructor, b"physical-course-aggregate").await;

        store
            .create_course(
                context(tenant),
                CreateCourseCommand {
                    course: course(tenant, course_id),
                    authority,
                },
            )
            .await
            .expect("course creation succeeds");

        let state = store.read_state().expect("Memory state is available");
        let membership_id = state
            .active_course_membership_by_user
            .get(&(tenant, course_id, instructor))
            .copied()
            .expect("initial instructor current-membership index is materialized");
        let membership = state
            .course_memberships
            .get(&(tenant, membership_id))
            .expect("initial instructor membership is materialized");
        assert_eq!(membership.user, instructor);
        assert_eq!(membership.course, course_id);
        assert_eq!(membership.role, CourseMembershipRole::Instructor);
        assert!(membership.student.is_none());
        assert!(membership.roster_id.is_none());
        assert_eq!(membership.status, crate::CourseMemberStatus::Active);
        assert!(
            state
                .course_membership_references
                .contains_key(&(tenant, membership_id))
        );

        assert_eq!(
            state.roster_policies.get(&(tenant, course_id)),
            Some(&super::super::course_roster::initial_roster_policy(
                course_id
            )),
            "the initial roster policy is persisted rather than synthesized on read",
        );
        assert!(
            state
                .roster_profiles
                .keys()
                .all(|(record_tenant, record_course, _)| {
                    *record_tenant != tenant || *record_course != course_id
                })
        );
        assert!(state.roster_member_by_roster_id.keys().all(
            |(record_tenant, record_course, _)| {
                *record_tenant != tenant || *record_course != course_id
            }
        ));

        assert_eq!(
            state.course_appearances.get(&(tenant, course_id)),
            Some(&question_model::CourseAppearance {
                theme: question_model::CourseThemeId::default(),
                revision: question_model::CourseAppearanceRevision::INITIAL,
                banner: None,
            }),
            "the initial appearance is persisted rather than synthesized on read",
        );
        assert_eq!(
            state.course_grade_schemes.get(&(tenant, course_id)),
            Some(&super::super::course_gradebook::initial_course_grade_scheme(course_id)),
            "the initial grade scheme is persisted rather than synthesized on read",
        );

        let policies = state
            .course_group_purpose_policies
            .iter()
            .filter(|((record_tenant, record_course, _), _)| {
                *record_tenant == tenant && *record_course == course_id
            })
            .collect::<Vec<_>>();
        assert_eq!(policies.len(), ALL_GROUP_PURPOSES.len());
        for purpose in ALL_GROUP_PURPOSES {
            assert_eq!(
                state
                    .course_group_purpose_policies
                    .get(&(tenant, course_id, purpose)),
                Some(&StoredCourseGroupPurposePolicy {
                    policy: question_model::CourseGroupPurposePolicy::default_for_purpose(purpose),
                    revision: CourseGroupPurposePolicyRevision::INITIAL,
                }),
                "initial group-purpose policy is stored for {purpose:?}",
            );
        }
    }

    #[tokio::test]
    async fn late_course_creation_failure_restores_the_entire_memory_state() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(fixture_uuid(10));
        let instructor = UserId::from_uuid(fixture_uuid(11));
        let course_id = CourseId::from_uuid(fixture_uuid(12));
        let authority =
            sysadmin_authority(&store, tenant, instructor, b"atomic-course-failure").await;
        let before = store
            .read_state()
            .expect("Memory state is available")
            .clone();
        arm_create_course_late_failure(tenant, course_id);

        let result = store
            .create_course(
                context(tenant),
                CreateCourseCommand {
                    course: course(tenant, course_id),
                    authority,
                },
            )
            .await;

        assert_eq!(
            result,
            Err(StoreError::Unavailable(
                "injected late course-creation failure".to_string()
            ))
        );
        let after = store.read_state().expect("Memory state is available");
        assert_eq!(
            format!("{before:#?}"),
            format!("{after:#?}"),
            "an error after reference and course insertion restores every private Memory collection"
        );
    }
}
