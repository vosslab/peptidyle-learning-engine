use crate::{ActorContext, CourseCreationAuthority, CreateCourseCommand};
use crate::{
    CourseGroupMembershipWarning, CourseGroupPurposePolicyRevision, CourseGroupView,
    StoredCourseGroupPurposePolicy, UpdateCourseGroupPurposePolicyCommand,
};
use async_trait::async_trait;

use super::*;

#[async_trait]
impl crate::CourseStore for MemoryStore {
    async fn create_course_impl(
        &self,
        context: ActorContext,
        command: CreateCourseCommand,
    ) -> Result<(), StoreError> {
        let course = command.course;
        validate_course(&course)?;
        let course_id = course.id;
        let mut state = self.write_state()?;
        let initial_instructor = authorize_course_creation(&state, context, &command.authority)?;
        if state.courses.contains_key(&course_id) {
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
        _context: ActorContext,
        course: CourseId,
    ) -> Result<Option<CourseRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(state.courses.get(&course).cloned())
    }
    async fn get_current_course_membership_impl(
        &self,
        _context: ActorContext,
        course: CourseId,
        user: UserId,
    ) -> Result<Option<CourseMembershipRecord>, StoreError> {
        let state = self.read_state()?;
        Ok(super::entitlement::active_membership_for(&state, course, user).cloned())
    }
    async fn list_courses_impl(
        &self,
        context: ActorContext,
        page: PageRequest,
    ) -> Result<Page<CourseSummary>, StoreError> {
        let state = self.read_state()?;
        let records = state
            .courses
            .iter()
            .filter_map(|(course_id, record)| {
                let role = super::entitlement::active_membership_for(
                    &state,
                    *course_id,
                    context.user_id(),
                )
                .map(|membership| membership.role)?;
                if role == CourseMembershipRole::Student
                    && !course_records_accessible(&state, *course_id)
                {
                    return None;
                }
                let public_id = state.course_references.get(course_id).copied()?;
                Some((course_id.to_string(), record.summary(role, public_id)))
            })
            .collect();
        Ok(page_records(records, &page))
    }
    async fn put_course_group_impl(
        &self,
        _context: ActorContext,
        command: PutCourseGroupCommand,
    ) -> Result<StoredCourseGroup, StoreError> {
        validate_course_group(&command.record)?;
        let key = command.record.id;
        let mut state = self.write_state()?;
        require_course_records_accessible(&state, command.record.course)?;
        if !state.courses.contains_key(&command.record.course)
            || super::entitlement::active_membership_for(
                &state,
                command.record.course,
                command.actor,
            )
            .is_none_or(|membership| membership.role != CourseMembershipRole::Instructor)
            || command.record.members.iter().any(|membership| {
                super::entitlement::active_membership_by_id(&state, *membership).is_none_or(
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
        validate_group_purpose_transition(&state, &command.record)?;
        super::navigation_references::ensure_course_group_reference(&mut state, command.record.id)?;
        state.course_groups.insert(key, command.record.clone());
        state.course_group_revisions.insert(key, revision);
        if let Err(error) = reresolve_group_affected_assignments(
            &mut state,
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
        _context: ActorContext,
        group: CourseGroupId,
    ) -> Result<Option<StoredCourseGroup>, StoreError> {
        let state = self.read_state()?;
        let key = group;
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
    let course_id = course.id;
    if state.courses.contains_key(&course_id) {
        return Err(StoreError::AlreadyExists);
    }
    validate_course(&course)?;
    let reference = super::navigation_references::ensure_course_reference(state, course_id)?;
    state.courses.insert(course_id, course);
    super::entitlement::create_initial_instructor_membership(state, course_id, initial_instructor)?;
    state.roster_policies.insert(
        course_id,
        super::course_roster::initial_roster_policy(course_id),
    );
    state.course_grade_schemes.insert(
        course_id,
        super::course_gradebook::initial_course_grade_scheme(course_id),
    );
    for purpose in ALL_GROUP_PURPOSES {
        state.course_group_purpose_policies.insert(
            (course_id, purpose),
            StoredCourseGroupPurposePolicy {
                policy: question_model::CourseGroupPurposePolicy::default_for_purpose(purpose),
                revision: CourseGroupPurposePolicyRevision::INITIAL,
            },
        );
    }
    state.course_appearances.insert(
        course_id,
        question_model::CourseAppearance {
            theme: question_model::CourseThemeId::default(),
            revision: question_model::CourseAppearanceRevision::INITIAL,
            banner: None,
        },
    );
    state
        .course_schedule_revisions
        .insert(course_id, question_model::CourseScheduleRevision::INITIAL);
    Ok(reference)
}

fn authorize_course_creation(
    state: &State,
    context: ActorContext,
    authority: &CourseCreationAuthority,
) -> Result<UserId, StoreError> {
    match authority {
        CourseCreationAuthority::ApprovedInstructor { actor, session } => {
            let subject = super::sessions::active_subject(state, context, *session)
                .ok_or(StoreError::NotFound)?;
            if subject.user() != *actor || subject.role() != question_model::UserRole::Instructor {
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
            (subject.user() == *actor && subject.role() == question_model::UserRole::Sysadmin)
                .then_some(*actor)
                .ok_or(StoreError::Forbidden)
        }
    }
}

#[async_trait]
impl crate::CourseGroupManagementStore for MemoryStore {
    async fn list_course_groups(
        &self,
        _context: ActorContext,
        actor: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<CourseGroupView>, StoreError> {
        let state = self.read_state()?;
        require_group_manager(&state, course, actor)?;
        let records = state
            .course_groups
            .iter()
            .filter(|(_, record)| record.course == course)
            .map(|(id, record)| {
                let reference = state
                    .course_group_references
                    .get(id)
                    .copied()
                    .ok_or(StoreError::NotFound)?;
                let revision = state
                    .course_group_revisions
                    .get(id)
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
        _context: ActorContext,
        actor: UserId,
        course: CourseId,
        reference: question_model::CourseGroupReference,
    ) -> Result<Option<CourseGroupView>, StoreError> {
        let state = self.read_state()?;
        require_group_manager(&state, course, actor)?;
        let Some(group) = state.course_groups_by_reference.get(&reference).copied() else {
            return Ok(None);
        };
        let Some(record) = state
            .course_groups
            .get(&group)
            .filter(|record| record.course == course)
        else {
            return Ok(None);
        };
        let revision = state
            .course_group_revisions
            .get(&group)
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
        _context: ActorContext,
        actor: UserId,
        course: CourseId,
        group: CourseGroupId,
    ) -> Result<Option<CourseGroupView>, StoreError> {
        let state = self.read_state()?;
        require_group_manager(&state, course, actor)?;
        let Some(record) = state
            .course_groups
            .get(&group)
            .filter(|record| record.course == course)
        else {
            return Ok(None);
        };
        let reference = state
            .course_group_references
            .get(&group)
            .copied()
            .ok_or(StoreError::NotFound)?;
        let revision = state
            .course_group_revisions
            .get(&group)
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
        _context: ActorContext,
        actor: UserId,
        course: CourseId,
        group: CourseGroupId,
        expected_revision: CourseGroupRevision,
    ) -> Result<bool, StoreError> {
        let mut state = self.write_state()?;
        require_group_manager(&state, course, actor)?;
        let key = group;
        let Some(record) = state.course_groups.get(&key) else {
            return Ok(false);
        };
        if record.course != course {
            return Ok(false);
        }
        if state.course_group_revisions.get(&key).copied() != Some(expected_revision) {
            return Err(StoreError::Conflict);
        }
        if group_is_referenced(&state, group) {
            return Err(StoreError::Conflict);
        }
        state.course_groups.remove(&key);
        state.course_group_revisions.remove(&key);
        let reference = state
            .course_group_references
            .remove(&group)
            .ok_or(StoreError::NotFound)?;
        state.course_groups_by_reference.remove(&reference);
        Ok(true)
    }

    async fn get_course_group_purpose_policy(
        &self,
        _context: ActorContext,
        actor: UserId,
        course: CourseId,
        purpose: question_model::CourseGroupPurpose,
    ) -> Result<Option<StoredCourseGroupPurposePolicy>, StoreError> {
        let state = self.read_state()?;
        require_group_manager(&state, course, actor)?;
        Ok(state
            .course_group_purpose_policies
            .get(&(course, purpose))
            .copied())
    }

    async fn update_course_group_purpose_policy(
        &self,
        context: ActorContext,
        command: UpdateCourseGroupPurposePolicyCommand,
    ) -> Result<StoredCourseGroupPurposePolicy, StoreError> {
        let mut state = self.write_state()?;
        // ASVS 8.2.1-8.2.3, 8.3.1-8.3.3, 15.4.2: resolve the live,
        // session-bound direct Instructor and exact course in the same write
        // critical section as the compare-and-swap. The command never carries
        // an actor identity that a caller could forge.
        require_session_group_policy_manager(&state, context, command.session, command.course)?;
        let key = (command.course, command.policy.purpose);
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
        _context: ActorContext,
        actor: UserId,
        course: CourseId,
    ) -> Result<Vec<CourseGroupMembershipWarning>, StoreError> {
        let state = self.read_state()?;
        require_group_manager(&state, course, actor)?;
        Ok(membership_warnings(&state, course))
    }
}

const ALL_GROUP_PURPOSES: [question_model::CourseGroupPurpose; 5] = [
    question_model::CourseGroupPurpose::Section,
    question_model::CourseGroupPurpose::Lab,
    question_model::CourseGroupPurpose::Cohort,
    question_model::CourseGroupPurpose::Accommodation,
    question_model::CourseGroupPurpose::Work,
];

fn require_group_manager(state: &State, course: CourseId, actor: UserId) -> Result<(), StoreError> {
    if !state.courses.contains_key(&course)
        || super::entitlement::current_course_role(state, course, actor)
            != Some(CourseMembershipRole::Instructor)
    {
        return Err(StoreError::NotFound);
    }
    require_course_records_accessible(state, course)
}

fn require_session_group_policy_manager(
    state: &State,
    context: ActorContext,
    session: crate::SessionTokenHash,
    course: CourseId,
) -> Result<(), StoreError> {
    let subject =
        super::sessions::active_subject(state, context, session).ok_or(StoreError::NotFound)?;
    if subject.role() != question_model::UserRole::Instructor {
        return Err(StoreError::NotFound);
    }
    super::teaching_authority::require_direct_instructor(state, course, subject.user())?;
    require_course_records_accessible(state, course)
}

fn group_is_referenced(state: &State, group: CourseGroupId) -> bool {
    state
        .assignments
        .values()
        .any(|assignment| audience_mentions(&assignment.audience, group))
        || state
            .assignment_group_schedule_offsets
            .keys()
            .any(|(_, candidate)| *candidate == group)
        || state
            .assignment_group_accommodations
            .keys()
            .any(|(_, candidate)| *candidate == group)
}

fn validate_group_purpose_transition(
    state: &State,
    proposed: &CourseGroupRecord,
) -> Result<(), StoreError> {
    let Some(existing) = state.course_groups.get(&proposed.id) else {
        return Ok(());
    };
    if existing.purpose == proposed.purpose {
        return Ok(());
    }
    let capabilities = question_model::GroupPurposeCapabilities::for_purpose(proposed.purpose);
    let audience_used = state.assignments.values().any(|assignment| {
        assignment.course_id == proposed.course
            && audience_mentions(&assignment.audience, proposed.id)
    });
    let schedule_used = state
        .assignment_group_schedule_offsets
        .keys()
        .any(|(_, group)| *group == proposed.id);
    let accommodation_used = state
        .assignment_group_accommodations
        .keys()
        .any(|(_, group)| *group == proposed.id);
    if (audience_used && !capabilities.assignment_audience)
        || (schedule_used && !capabilities.schedule_scope)
        || (accommodation_used && !capabilities.accommodation_scope)
    {
        return Err(StoreError::Conflict);
    }
    Ok(())
}

fn membership_warnings(state: &State, course: CourseId) -> Vec<CourseGroupMembershipWarning> {
    let mut counts = std::collections::BTreeMap::new();
    for group in state
        .course_groups
        .values()
        .filter(|group| group.course == course)
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
                .get(&(course, purpose))
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
    course: CourseId,
    group: CourseGroupId,
) -> Result<(), StoreError> {
    let assignments = state
        .assignments
        .values()
        .filter(|assignment| assignment.course_id == course)
        .filter(|assignment| {
            audience_mentions(&assignment.audience, group)
                || state
                    .assignment_group_schedule_offsets
                    .contains_key(&(assignment.id, group))
                || state
                    .assignment_group_accommodations
                    .contains_key(&(assignment.id, group))
        })
        .map(|assignment| assignment.id)
        .collect::<Vec<_>>();
    for assignment in assignments {
        super::course_policy::reresolve_active_assignment_attempts(state, course, assignment)?;
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
