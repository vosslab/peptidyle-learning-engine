//! Session-bound course-group-purpose policy conformance.

use super::*;
use learning_data_access::{
    CourseGroupPurposePolicyRevision, SessionStore, SessionTokenHash,
    UpdateCourseGroupPurposePolicyCommand,
};
use question_model::{CourseGroupPurpose as Purpose, MultipleMembershipPolicy};

pub(crate) async fn assert_all_default_policies<S>(
    store: &S,
    context: TenantContext,
    instructor: UserId,
    instructor_session: SessionTokenHash,
    outsider: UserId,
    course: CourseId,
) where
    S: CourseGroupManagementStore + SessionStore,
{
    for purpose in [
        Purpose::Section,
        Purpose::Lab,
        Purpose::Cohort,
        Purpose::Accommodation,
        Purpose::Work,
    ] {
        let current = store
            .get_course_group_purpose_policy(context, instructor, course, purpose)
            .await
            .expect("lookup")
            .expect("explicit policy");
        assert_eq!(
            current.policy,
            question_model::CourseGroupPurposePolicy::default_for_purpose(purpose)
        );
        assert_eq!(current.revision, CourseGroupPurposePolicyRevision::INITIAL);
    }
    assert_eq!(
        store
            .get_course_group_purpose_policy(context, outsider, course, Purpose::Section)
            .await,
        Err(StoreError::NotFound)
    );
    let section = store
        .get_course_group_purpose_policy(context, instructor, course, Purpose::Section)
        .await
        .expect("section")
        .expect("policy");
    let changed = store
        .update_course_group_purpose_policy(
            context,
            UpdateCourseGroupPurposePolicyCommand {
                session: instructor_session,
                course,
                expected_revision: section.revision,
                policy: question_model::CourseGroupPurposePolicy {
                    purpose: Purpose::Section,
                    multiple_membership: MultipleMembershipPolicy::Allow,
                },
            },
        )
        .await
        .expect("policy CAS");
    assert_eq!(changed.revision.value(), 2);
    assert_eq!(
        store
            .update_course_group_purpose_policy(
                context,
                UpdateCourseGroupPurposePolicyCommand {
                    session: instructor_session,
                    course,
                    expected_revision: section.revision,
                    policy: changed.policy,
                },
            )
            .await,
        Err(StoreError::Conflict)
    );
    store
        .revoke_session(instructor_session)
        .await
        .expect("revoke persisted instructor session");
    assert_eq!(
        store
            .update_course_group_purpose_policy(
                context,
                UpdateCourseGroupPurposePolicyCommand {
                    session: instructor_session,
                    course,
                    expected_revision: changed.revision,
                    policy: changed.policy,
                },
            )
            .await,
        Err(StoreError::NotFound)
    );
    let current = store
        .get_course_group_purpose_policy(context, instructor, course, Purpose::Section)
        .await
        .expect("policy lookup after revoked session")
        .expect("current policy after revoked session");
    assert_eq!(current, changed);
}
