//! Public MemoryStore oracle for the protected course-grade capability.

use super::*;
use learning_data_access::{
    CourseGradeAssignmentMembership, CourseGradebookStore, SessionLifetime, UpdateCourseGradeScheme,
};
use question_model::{
    CourseGradeMode, CourseGradeRoundingRule, CourseGradeScheme, GradeCategoryId,
    GradeCategoryTitle, WeightedGradeCategory,
};

async fn gradebook_session(store: &MemoryStore, user: UserId, token: &[u8]) -> SessionTokenHash {
    let token = SessionTokenHash::compute(token);
    store
        .create_session(
            token,
            SessionSubject::new(
                TenantId::from_uuid(uuid(1)),
                user,
                "Course grade fixture",
                vec![UserRole::Instructor],
            )
            .expect("session subject"),
            SessionLifetime::from_seconds(3600).expect("session lifetime"),
        )
        .await
        .expect("session persists");
    token
}

#[tokio::test]
async fn memory_course_grade_scheme_is_revisioned_authorized_and_merges_new_assignments() {
    let store = MemoryStore::default();
    exercise_store(&store).await;
    let tenant = TenantId::from_uuid(uuid(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let course = CourseId::from_uuid(uuid(17));
    let instructor = UserId::from_uuid(uuid(18));
    let student = UserId::from_uuid(uuid(14));
    let outsider = UserId::from_uuid(uuid(99_991));
    let instructor_session =
        gradebook_session(&store, instructor, b"course-grade-instructor").await;
    let student_session = gradebook_session(&store, student, b"course-grade-student").await;
    let outsider_session = gradebook_session(&store, outsider, b"course-grade-outsider").await;

    let implicit = store
        .course_grade_scheme(context, instructor_session, course)
        .await
        .expect("instructor reads implicit scheme");
    assert_eq!(implicit.scheme.mode, CourseGradeMode::TotalPoints);
    assert!(implicit.assignments.iter().all(|entry| entry.included));
    assert!(
        implicit
            .assignments
            .iter()
            .all(|entry| entry.category.is_none() && entry.position.is_none())
    );

    assert_eq!(
        store
            .course_grade_scheme(context, student_session, course)
            .await,
        Err(StoreError::Forbidden)
    );
    assert_eq!(
        store
            .course_grade_scheme(context, outsider_session, course)
            .await,
        Err(StoreError::NotFound)
    );

    let updated = store
        .update_course_grade_scheme(
            context,
            instructor_session,
            UpdateCourseGradeScheme {
                course,
                expected_revision: implicit.revision,
                scheme: implicit.scheme.clone(),
                assignments: memberships(&implicit),
            },
        )
        .await
        .expect("scheme update");
    assert_eq!(
        updated.revision,
        implicit.revision.next().expect("revision advances")
    );
    assert_eq!(
        store
            .update_course_grade_scheme(
                context,
                instructor_session,
                UpdateCourseGradeScheme {
                    course,
                    expected_revision: implicit.revision,
                    scheme: implicit.scheme.clone(),
                    assignments: memberships(&implicit),
                },
            )
            .await,
        Err(StoreError::Conflict)
    );

    let mut new_assignment = store
        .get_assignment_for_edit(context, AssignmentId::from_uuid(uuid(8)))
        .await
        .expect("assignment read")
        .expect("assignment exists")
        .record;
    new_assignment.id = AssignmentId::from_uuid(uuid(90_001));
    new_assignment.title = "New course-grade assignment".to_string();
    store
        .create_assignment_with_default_policy(context, instructor, new_assignment.clone())
        .await
        .expect("new assignment");
    let merged = store
        .course_grade_scheme(context, instructor_session, course)
        .await
        .expect("merged scheme read");
    assert_eq!(
        merged.revision,
        updated
            .revision
            .next()
            .expect("assignment projection advances")
    );
    let new_entry = merged
        .assignments
        .iter()
        .find(|entry| entry.assignment == new_assignment.id)
        .expect("read includes current new assignment");
    assert!(new_entry.included);
    assert_eq!(new_entry.category, None);
    assert_eq!(new_entry.position, None);
    assert_eq!(
        store
            .update_course_grade_scheme(
                context,
                instructor_session,
                UpdateCourseGradeScheme {
                    course,
                    expected_revision: updated.revision,
                    scheme: merged.scheme.clone(),
                    assignments: memberships(&merged),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "an assignment projection change invalidates the prior strong token"
    );

    let invalid = CourseGradeAssignmentMembership {
        assignment: new_assignment.id,
        included: true,
        category: None,
        position: Some(0),
    };
    let mut invalid_assignments = memberships(&merged);
    invalid_assignments.retain(|entry| entry.assignment != invalid.assignment);
    invalid_assignments.push(invalid);
    assert!(matches!(
        store
            .update_course_grade_scheme(
                context,
                instructor_session,
                UpdateCourseGradeScheme {
                    course,
                    expected_revision: merged.revision,
                    scheme: merged.scheme.clone(),
                    assignments: invalid_assignments,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}

fn memberships(
    record: &learning_data_access::CourseGradeSchemeRecord,
) -> Vec<CourseGradeAssignmentMembership> {
    record
        .assignments
        .iter()
        .map(|entry| CourseGradeAssignmentMembership {
            assignment: entry.assignment,
            included: entry.included,
            category: entry.category,
            position: entry.position,
        })
        .collect()
}

async fn create_grade_assignment(
    store: &MemoryStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
    course: CourseId,
    seed: u128,
    points: PointValue,
) -> AssignmentId {
    let reference = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        seed,
        PublicationScope::Public,
    )
    .await;
    let assignment = AssignmentId::from_uuid(uuid(seed + 1));
    let mut items = fixed_items(vec![reference]);
    items[0].points_possible = points;
    store
        .create_assignment_with_default_policy(
            context,
            instructor,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: format!("Grade fixture {seed}"),
                lifecycle: question_model::AssignmentLifecycle::Published,
                instructions: question_model::AssignmentInstructions::default(),
                audience: question_model::AssignmentAudience::CourseWide,
                items,
                selection_groups: Vec::new(),
                disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                policies: policies(),
            },
        )
        .await
        .expect("grade fixture assignment");
    assignment
}

async fn claimed_gradebook_fixture(
    store: &MemoryStore,
) -> (TenantContext, SessionTokenHash, CourseId, UserId) {
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(91_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(uuid(91_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(91_001));
    let student = UserId::from_uuid(uuid(91_002));
    let course = CourseId::from_uuid(uuid(91_003));
    let course_creation_authority =
        sysadmin_course_creation_authority(store, tenant, course, instructor).await;
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Course grade numeric oracle".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("fixture term"),
                },
                authority: course_creation_authority,
            },
        )
        .await
        .expect("grade fixture course");
    let session = SessionTokenHash::compute(b"gradebook-numeric-instructor");
    store
        .create_session(
            session,
            SessionSubject::new(
                tenant,
                instructor,
                "Course grade numeric instructor",
                vec![UserRole::Instructor],
            )
            .expect("session subject"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("instructor session");
    let invitation = CourseInvitationSecretHash::compute(b"gradebook-numeric-invitation");
    store
        .create_course_invitation(
            context,
            session,
            CreateCourseInvitation {
                course,
                email: AuthenticationEmail::parse("numeric.student@mail.roosevelt.edu")
                    .expect("fixture email"),
                roster_id: CourseRosterId::parse("900123457").expect("fixture roster ID"),
                token_hash: invitation,
                idempotency_key: RosterIdempotencyKey::parse("gradebook-numeric-invite")
                    .expect("fixture key"),
                lifetime: CourseInvitationLifetime::from_seconds(3_600)
                    .expect("fixture invitation lifetime"),
            },
        )
        .await
        .expect("contact-bearing invitation");
    store
        .claim_course_invitation(ClaimCourseInvitation {
            token_hash: invitation,
            user: student,
            verified_email: AuthenticationEmail::parse("numeric.student@mail.roosevelt.edu")
                .expect("fixture verified email"),
            display_name: "Numeric Student".to_string(),
        })
        .await
        .expect("active student claim");
    (context, session, course, instructor)
}

#[tokio::test]
async fn memory_course_grade_totals_use_public_roster_and_scheme_transitions() {
    let store = MemoryStore::default();
    let (context, session, course, instructor) = claimed_gradebook_fixture(&store).await;
    let tenant = context.tenant_id();
    let first = create_grade_assignment(
        &store,
        context,
        tenant,
        instructor,
        course,
        91_010,
        PointValue::from_whole(1),
    )
    .await;
    let second = create_grade_assignment(
        &store,
        context,
        tenant,
        instructor,
        course,
        91_020,
        PointValue::from_whole(2),
    )
    .await;

    let totals = store
        .course_gradebook_totals(context, session, course)
        .await
        .expect("contact-bearing student total");
    let outcome = &totals.rows[0].outcome;
    assert_eq!(outcome.rounded_score, Some(0.0));
    assert_eq!(outcome.total_possible, Some(3.0));

    let export = store
        .create_course_grade_export(context, session, course)
        .await
        .expect("course export");
    assert_eq!(export.rows, totals.rows);
    assert_eq!(export.audit.row_count, export.rows.len());
    assert_eq!(export.audit.course, course);
    assert_eq!(export.audit.requested_by, instructor);
    assert_eq!(export.audit.mode, CourseGradeMode::TotalPoints);

    let initial = store
        .course_grade_scheme(context, session, course)
        .await
        .expect("initial scheme");
    let category = GradeCategoryId::from_uuid(uuid(91_030));
    let weighted = CourseGradeScheme {
        mode: CourseGradeMode::WeightedCategories,
        rounding: CourseGradeRoundingRule::FourDecimalPlacesHalfAwayFromZero,
        categories: vec![WeightedGradeCategory {
            id: category,
            title: GradeCategoryTitle::new("Practice").expect("category title"),
            position: 0,
            weight_basis_points: 10_000,
            drop_lowest: 1,
        }],
        letter_bands: Vec::new(),
    };
    let weighted_assignments = vec![
        CourseGradeAssignmentMembership {
            assignment: first,
            included: true,
            category: Some(category),
            position: Some(0),
        },
        CourseGradeAssignmentMembership {
            assignment: second,
            included: true,
            category: Some(category),
            position: Some(1),
        },
    ];
    let weighted_record = store
        .update_course_grade_scheme(
            context,
            session,
            UpdateCourseGradeScheme {
                course,
                expected_revision: initial.revision,
                scheme: weighted,
                assignments: weighted_assignments,
            },
        )
        .await
        .expect("weighted scheme");
    let weighted_totals = store
        .course_gradebook_totals(context, session, course)
        .await
        .expect("weighted missing summaries are zero");
    assert_eq!(weighted_totals.rows[0].outcome.rounded_score, Some(0.0));
    assert_eq!(
        weighted_totals.rows[0].outcome.dropped_assignment_ids,
        vec![second],
        "equal missing summaries deterministically drop the later category position"
    );

    let third = create_grade_assignment(
        &store,
        context,
        tenant,
        instructor,
        course,
        91_040,
        PointValue::ZERO,
    )
    .await;
    assert!(matches!(
        store.course_gradebook_totals(context, session, course).await,
        Err(StoreError::Unavailable(message)) if message.contains("mapping")
    ));

    let remapped = store
        .course_grade_scheme(context, session, course)
        .await
        .expect("new assignment appears for remapping");
    let mut remapped_assignments = memberships(&remapped);
    remapped_assignments
        .iter_mut()
        .find(|membership| membership.assignment == third)
        .expect("new assignment membership")
        .category = Some(category);
    remapped_assignments
        .iter_mut()
        .find(|membership| membership.assignment == third)
        .expect("new assignment membership")
        .position = Some(2);
    store
        .update_course_grade_scheme(
            context,
            session,
            UpdateCourseGradeScheme {
                course,
                expected_revision: remapped.revision,
                scheme: weighted_record.scheme,
                assignments: remapped_assignments,
            },
        )
        .await
        .expect("new assignment remapping");
    let remapped_totals = store
        .course_gradebook_totals(context, session, course)
        .await
        .expect("remapped weighted totals");
    assert_eq!(remapped_totals.rows[0].outcome.rounded_score, Some(0.0));
}
