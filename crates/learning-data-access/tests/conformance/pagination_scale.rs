//! Store-generic cursor traversal invoked by MemoryStore conformance.

use std::collections::BTreeSet;

use super::*;

const ROW_COUNT: u128 = 51;
const DEFAULT_PAGE_SIZE: u16 = 50;
const SMALL_PAGE_SIZE: u16 = 17;

/// Exercises the assignment and gradebook lists beyond their ordinary first
/// page. Both lists must enumerate exactly the tenant-owned records once,
/// make progress at every continuation, and conceal the course from another
/// tenant.
pub(super) async fn exercise_course_pagination_scale<S>(store: &S)
where
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(80_000));
    let foreign_tenant = TenantId::from_uuid(uuid(80_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let instructor = UserId::from_uuid(uuid(80_002));
    let student = UserId::from_uuid(uuid(80_003));
    let course = CourseId::from_uuid(uuid(80_004));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Pagination scale course".to_string(),
                members: vec![
                    CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: student,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("pagination scale course");
    let reference = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        80_100,
        PublicationScope::Public,
    )
    .await;

    let mut expected_assignments = BTreeSet::new();
    let mut expected_gradebook_rows = BTreeSet::new();
    for offset in 0..ROW_COUNT {
        let assignment = AssignmentId::from_uuid(uuid(80_200 + offset));
        let enrollment = EnrollmentId::from_uuid(uuid(80_300 + offset));
        store
            .create_assignment(
                context,
                AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    title: format!("Pagination assignment {offset}"),
                    items: fixed_items(vec![reference]),
                    selection_groups: Vec::new(),
                    policies: policies(),
                },
            )
            .await
            .expect("pagination scale assignment");
        store
            .create_enrollment(
                context,
                AssignmentEnrollment {
                    id: enrollment,
                    tenant,
                    assignment,
                    user: student,
                    student: StudentId::from_uuid(uuid(80_400 + offset)),
                    first_completed_at: None,
                    current_grade_run: None,
                    best_grade_run: None,
                },
            )
            .await
            .expect("pagination scale enrollment");
        expected_assignments.insert(assignment.to_string());
        expected_gradebook_rows.insert((assignment.to_string(), enrollment.to_string()));
    }

    for page_size in [DEFAULT_PAGE_SIZE, SMALL_PAGE_SIZE] {
        let assignments = collect_assignments(store, context, course, page_size).await;
        assert_eq!(
            assignments, expected_assignments,
            "assignment traversal must return every record once at page size {page_size}"
        );

        let gradebook_rows = collect_gradebook_rows(store, context, course, page_size).await;
        assert_eq!(
            gradebook_rows, expected_gradebook_rows,
            "gradebook traversal must return every record once at page size {page_size}"
        );
    }

    let page = PageRequest::first(PageSize::new(DEFAULT_PAGE_SIZE).expect("valid page size"));
    assert_eq!(
        store
            .list_assignments(foreign_context, course, page.clone())
            .await,
        Err(StoreError::NotFound),
        "a foreign tenant cannot discover course assignments"
    );
    assert_eq!(
        store
            .list_gradebook_rows(foreign_context, course, page)
            .await,
        Err(StoreError::NotFound),
        "a foreign tenant cannot discover gradebook summaries"
    );
}

async fn collect_assignments<S>(
    store: &S,
    context: TenantContext,
    course: CourseId,
    page_size: u16,
) -> BTreeSet<String>
where
    S: Store + CatalogStore,
{
    let size = PageSize::new(page_size).expect("valid page size");
    let mut cursor = None;
    let mut seen_cursors = BTreeSet::new();
    let mut seen_assignments = BTreeSet::new();
    let mut previous = None;
    loop {
        let page = match cursor {
            Some(cursor) => {
                store
                    .list_assignments(context, course, PageRequest::after(cursor, size))
                    .await
            }
            None => {
                store
                    .list_assignments(context, course, PageRequest::first(size))
                    .await
            }
        }
        .expect("assignment page");
        assert!(page.items.len() <= usize::from(page_size));
        for assignment in page.items {
            let identity = assignment.id.to_string();
            assert!(
                previous
                    .as_ref()
                    .is_none_or(|previous| identity > *previous),
                "assignment traversal must advance in stable key order"
            );
            previous = Some(identity.clone());
            assert!(
                seen_assignments.insert(identity),
                "assignment traversal must not duplicate an assignment"
            );
        }
        let Some(next) = page.next_cursor else {
            break;
        };
        assert!(
            seen_cursors.insert(next.as_str().to_string()),
            "assignment cursor must strictly progress"
        );
        cursor = Some(next);
    }
    seen_assignments
}

async fn collect_gradebook_rows<S>(
    store: &S,
    context: TenantContext,
    course: CourseId,
    page_size: u16,
) -> BTreeSet<(String, String)>
where
    S: Store + CatalogStore,
{
    let size = PageSize::new(page_size).expect("valid page size");
    let mut cursor = None;
    let mut seen_cursors = BTreeSet::new();
    let mut seen_rows = BTreeSet::new();
    let mut previous = None;
    loop {
        let page = match cursor {
            Some(cursor) => {
                store
                    .list_gradebook_rows(context, course, PageRequest::after(cursor, size))
                    .await
            }
            None => {
                store
                    .list_gradebook_rows(context, course, PageRequest::first(size))
                    .await
            }
        }
        .expect("gradebook page");
        assert!(page.items.len() <= usize::from(page_size));
        for row in page.items {
            let identity = (row.assignment_id.to_string(), row.enrollment_id.to_string());
            assert!(
                previous
                    .as_ref()
                    .is_none_or(|previous| identity > *previous),
                "gradebook traversal must advance in assignment/enrollment key order"
            );
            previous = Some(identity.clone());
            assert!(
                seen_rows.insert(identity),
                "gradebook traversal must not duplicate an assignment/enrollment row"
            );
        }
        let Some(next) = page.next_cursor else {
            break;
        };
        assert!(
            seen_cursors.insert(next.as_str().to_string()),
            "gradebook cursor must strictly progress"
        );
        cursor = Some(next);
    }
    seen_rows
}
