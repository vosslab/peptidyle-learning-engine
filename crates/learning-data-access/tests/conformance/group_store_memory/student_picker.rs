use super::*;

pub(super) async fn assert_student_picker_reference_contract<S>(
    store: &S,
    context: TenantContext,
    instructor: UserId,
    outsider: UserId,
    course: CourseId,
) where
    S: Store + CourseRosterStore + TeachingAuthorityReferenceStore,
{
    for (offset, display_name) in [
        (720_040, "Picker student one"),
        (720_041, "Picker student two"),
        (720_042, "Picker student three"),
    ] {
        store
            .upsert_course_member(
                context,
                instructor,
                learning_data_access::UpsertCourseMember {
                    course,
                    user: UserId::from_uuid(uuid(offset)),
                    display_name: display_name.to_owned(),
                    roster_contact: None,
                },
            )
            .await
            .expect("active Student membership");
    }
    let page = PageRequest::first(PageSize::new(2).expect("bounded picker page"));
    assert_eq!(
        store
            .list_course_active_student_membership_reference_views(
                context,
                outsider,
                course,
                page.clone(),
            )
            .await,
        Err(StoreError::NotFound),
        "a non-Instructor cannot enumerate student picker references"
    );
    let first = store
        .list_course_active_student_membership_reference_views(context, instructor, course, page)
        .await
        .expect("direct Instructor receives the first bounded Student picker page");
    let cursor = first
        .next_cursor
        .clone()
        .expect("a second Student picker page remains");
    assert!(first.items.iter().all(|view| {
        view.role == question_model::CourseMembershipRole::Student
            && view.status == learning_data_access::CourseMemberStatus::Active
    }));
    let second = store
        .list_course_active_student_membership_reference_views(
            context,
            instructor,
            course,
            PageRequest::after(cursor, PageSize::new(2).expect("bounded picker page")),
        )
        .await
        .expect("stable numeric Student picker cursor");
    assert!(
        second.items.iter().all(|view| {
            view.role == question_model::CourseMembershipRole::Student
                && view.status == learning_data_access::CourseMemberStatus::Active
        }) && second.next_cursor.is_none(),
        "the bounded continuation contains only active Students and reaches its terminal cursor"
    );
}
