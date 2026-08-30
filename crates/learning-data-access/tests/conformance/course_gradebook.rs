//! Public MemoryStore oracle for the protected course-grade capability.

use super::*;
use learning_data_access::{
    CalculatedGradebookRequest, CalculatedGradebookResult, CourseGradeAssignmentMembership,
    CourseGradebookStore, CourseRosterEntry, GradebookFilter, GradebookFilterRequest,
    GradebookSelectionRequest, GradebookSelectionResult, PageRequest, PageSize, SessionLifetime,
    SubmittedRunChoicesRequest, UpdateCourseGradeScheme,
};
use question_model::{
    CourseGradeMode, CourseGradeRoundingRule, CourseGradeScheme, GradeCategoryId,
    GradeCategoryTitle, GradePolicy, ScoringStatus, WeightedGradeCategory,
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
                disclosure_policy: question_model::StudentDisclosurePolicy::default(),
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

async fn add_gradebook_student(
    store: &MemoryStore,
    context: TenantContext,
    course: CourseId,
    instructor: UserId,
    seed: u128,
) {
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: UserId::from_uuid(uuid(seed)),
                display_name: format!("Gradebook Student {seed}"),
                roster_contact: None,
            },
        )
        .await
        .expect("active gradebook student");
}

fn first_gradebook_page(filter: GradebookFilter, size: u16) -> CalculatedGradebookRequest {
    CalculatedGradebookRequest {
        filter,
        page: PageRequest::first(PageSize::new(size).expect("bounded page size")),
    }
}

#[tokio::test]
async fn calculated_gradebook_continues_roster_order_and_reloads_for_structural_changes() {
    let store = MemoryStore::default();
    let (context, session, course, instructor) = claimed_gradebook_fixture(&store).await;
    add_gradebook_student(&store, context, course, instructor, 91_004).await;

    let CalculatedGradebookResult::Page(first) = store
        .calculated_gradebook_page(
            context,
            session,
            course,
            first_gradebook_page(GradebookFilter::All, 1),
        )
        .await
        .expect("first gradebook page")
    else {
        panic!("first page must be available");
    };
    let cursor = first.next_cursor.clone().expect("second roster page");
    let CalculatedGradebookResult::Page(second) = store
        .calculated_gradebook_page(
            context,
            session,
            course,
            CalculatedGradebookRequest {
                filter: GradebookFilter::All,
                page: PageRequest::after(cursor, PageSize::new(1).expect("page size")),
            },
        )
        .await
        .expect("second gradebook page")
    else {
        panic!("unchanged roster must continue");
    };
    assert_ne!(first.rows[0].membership, second.rows[0].membership);

    let cursor = first.next_cursor.expect("reusable cursor");
    let scheme = store
        .course_grade_scheme(context, session, course)
        .await
        .expect("scheme");
    store
        .update_course_grade_scheme(
            context,
            session,
            UpdateCourseGradeScheme {
                course,
                expected_revision: scheme.revision,
                scheme: scheme.scheme.clone(),
                assignments: memberships(&scheme),
            },
        )
        .await
        .expect("structural scheme revision");
    assert_eq!(
        store
            .calculated_gradebook_page(
                context,
                session,
                course,
                CalculatedGradebookRequest {
                    filter: GradebookFilter::All,
                    page: PageRequest::after(cursor, PageSize::new(1).expect("page size")),
                },
            )
            .await,
        Ok(CalculatedGradebookResult::ReloadRequired {
            reason: learning_data_access::GradebookReloadReason::SchemeChanged,
        })
    );

    let CalculatedGradebookResult::Page(after_scheme) = store
        .calculated_gradebook_page(
            context,
            session,
            course,
            first_gradebook_page(GradebookFilter::All, 1),
        )
        .await
        .expect("fresh page after scheme reload")
    else {
        panic!("fresh page must be available");
    };
    let roster_cursor = after_scheme.next_cursor.expect("second roster page");
    add_gradebook_student(&store, context, course, instructor, 91_006).await;
    assert_eq!(
        store
            .calculated_gradebook_page(
                context,
                session,
                course,
                CalculatedGradebookRequest {
                    filter: GradebookFilter::All,
                    page: PageRequest::after(roster_cursor, PageSize::new(1).expect("page size")),
                },
            )
            .await,
        Ok(CalculatedGradebookResult::ReloadRequired {
            reason: learning_data_access::GradebookReloadReason::RosterChanged,
        })
    );

    let CalculatedGradebookResult::Page(after_roster) = store
        .calculated_gradebook_page(
            context,
            session,
            course,
            first_gradebook_page(GradebookFilter::All, 1),
        )
        .await
        .expect("fresh page after roster reload")
    else {
        panic!("fresh page must be available");
    };
    let filter_cursor = after_roster.next_cursor.expect("second roster page");
    assert_eq!(
        store
            .calculated_gradebook_page(
                context,
                session,
                course,
                CalculatedGradebookRequest {
                    filter: GradebookFilter::Student(after_roster.rows[0].membership),
                    page: PageRequest::after(filter_cursor, PageSize::new(1).expect("page size")),
                },
            )
            .await,
        Ok(CalculatedGradebookResult::ReloadRequired {
            reason: learning_data_access::GradebookReloadReason::FilterChanged,
        })
    );
    let CalculatedGradebookResult::Page(student_page) = store
        .calculated_gradebook_page(
            context,
            session,
            course,
            first_gradebook_page(
                GradebookFilter::Student(after_roster.rows[0].membership),
                10,
            ),
        )
        .await
        .expect("named Student page")
    else {
        panic!("named Student page must be available");
    };
    assert_eq!(
        student_page.rows[0].membership,
        after_roster.rows[0].membership
    );
}

#[tokio::test]
async fn assignment_filter_keeps_every_active_student_and_marks_missing_enrollment() {
    let store = MemoryStore::default();
    let (context, session, course, instructor) = claimed_gradebook_fixture(&store).await;
    add_gradebook_student(&store, context, course, instructor, 91_005).await;
    let assignment = create_grade_assignment(
        &store,
        context,
        context.tenant_id(),
        instructor,
        course,
        91_060,
        PointValue::from_whole(1),
    )
    .await;
    let assignment_reference = store
        .assignment_reference(context, instructor, assignment)
        .await
        .expect("assignment reference lookup")
        .expect("assignment reference");

    let CalculatedGradebookResult::Page(page) = store
        .calculated_gradebook_page(
            context,
            session,
            course,
            first_gradebook_page(GradebookFilter::Assignment(assignment_reference), 10),
        )
        .await
        .expect("assignment gradebook page")
    else {
        panic!("assignment page must be available");
    };
    assert_eq!(page.rows.len(), 2);
    assert!(page.rows.iter().all(|row| {
        matches!(
            row.assignment_cells.as_slice(),
            [learning_data_access::CalculatedAssignmentCell {
                availability:
                    learning_data_access::CalculatedAssignmentCellAvailability::Unavailable,
                inspection_choice: learning_data_access::AssignmentInspectionChoice::NoSubmittedRun,
                ..
            }]
        )
    }));
}

#[tokio::test]
async fn calculated_gradebook_reports_live_scoring_and_exact_run_choices() {
    let store = MemoryStore::default();
    let fixture = exercise_run_api_receipts_with_grade_policy(
        &store,
        StudentDisclosurePolicy::default(),
        93_000,
        GradePolicy::Highest,
    )
    .await;
    let session = SessionTokenHash::compute(b"gradebook-run-choice-instructor");
    store
        .create_session(
            session,
            SessionSubject::new(
                fixture.tenant,
                fixture.publisher,
                "Gradebook run choice instructor",
                vec![UserRole::Instructor],
            )
            .expect("session subject"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("instructor session");
    let assignment_reference = store
        .assignment_reference(fixture.context, fixture.publisher, fixture.assignment)
        .await
        .expect("assignment reference lookup")
        .expect("assignment reference");
    let assignment = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment read")
        .expect("assignment exists");
    let CalculatedGradebookResult::Page(page) = store
        .calculated_gradebook_page(
            fixture.context,
            session,
            fixture.course,
            first_gradebook_page(GradebookFilter::All, 10),
        )
        .await
        .expect("calculated gradebook page")
    else {
        panic!("calculated gradebook must produce a page");
    };
    assert_eq!(
        page.scoring_witnesses,
        vec![learning_data_access::AssignmentScoringWitness {
            assignment: assignment_reference,
            generation: assignment.scoring_generation,
            status: assignment.scoring_status,
        }]
    );
    assert_eq!(
        page.rows[0].assignment_cells[0].inspection_choice,
        learning_data_access::AssignmentInspectionChoice::SelectedRun {
            basis: learning_data_access::AssignmentRunSelectionBasis::Highest,
            run: fixture.run.reference,
            submitted_at: fixture.run.completed_at.expect("completed fixture run"),
        }
    );
    let debug = format!("{page:?}");
    assert!(
        !debug.contains("Run learner")
            && !debug.contains("selected_score")
            && !debug.contains("outcome"),
        "calculated Gradebook debug output is structural and excludes Student labels and scores"
    );

    let mut recalculating_items = assignment.record.items.clone();
    recalculating_items[0].points_possible = PointValue::from_whole(2);
    let recalculating = store
        .replace_assignment(
            fixture.context,
            ReplaceAssignmentCommand {
                actor: fixture.publisher,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: assignment.revision,
                update: AssignmentUpdate {
                    title: assignment.record.title.clone(),
                    audience: assignment.record.audience.clone(),
                    items: recalculating_items,
                    selection_groups: assignment.record.selection_groups.clone(),
                    disclosure_policy: assignment.record.disclosure_policy,
                    policies: assignment.record.policies,
                },
            },
        )
        .await
        .expect("assignment point update queues recalculation");
    assert_eq!(recalculating.scoring_status, ScoringStatus::Recalculating);
    let CalculatedGradebookResult::Page(recalculating_page) = store
        .calculated_gradebook_page(
            fixture.context,
            session,
            fixture.course,
            first_gradebook_page(GradebookFilter::All, 10),
        )
        .await
        .expect("recalculating gradebook page")
    else {
        panic!("recalculating Gradebook must produce a page");
    };
    assert_eq!(
        recalculating_page.scoring_witnesses,
        vec![learning_data_access::AssignmentScoringWitness {
            assignment: assignment_reference,
            generation: recalculating.scoring_generation,
            status: ScoringStatus::Recalculating,
        }]
    );
    assert_eq!(
        recalculating_page.rows[0].assignment_cells[0].selected_score, None,
        "a non-current assignment never exposes a stale selected score"
    );

    let choose_fixture = exercise_run_api_receipts_with_grade_policy(
        &store,
        StudentDisclosurePolicy::default(),
        94_000,
        GradePolicy::InstructorSelected,
    )
    .await;
    let choose_session = SessionTokenHash::compute(b"gradebook-choose-run-instructor");
    store
        .create_session(
            choose_session,
            SessionSubject::new(
                choose_fixture.tenant,
                choose_fixture.publisher,
                "Gradebook choose-run instructor",
                vec![UserRole::Instructor],
            )
            .expect("session subject"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("Instructor-selected session");
    let CalculatedGradebookResult::Page(choose_page) = store
        .calculated_gradebook_page(
            choose_fixture.context,
            choose_session,
            choose_fixture.course,
            first_gradebook_page(GradebookFilter::All, 10),
        )
        .await
        .expect("Instructor-selected gradebook page")
    else {
        panic!("Instructor-selected gradebook must produce a page");
    };
    assert_eq!(
        choose_page.rows[0].assignment_cells[0].inspection_choice,
        learning_data_access::AssignmentInspectionChoice::ChooseRun {
            completed_run_count: 2,
        }
    );
    assert_eq!(
        choose_page.rows[0].assignment_cells[0].scoring_status,
        ScoringStatus::Current
    );
}

#[tokio::test]
async fn student_filter_requires_an_active_student_in_the_selected_course() {
    let store = MemoryStore::default();
    let (context, session, course, instructor) = claimed_gradebook_fixture(&store).await;
    let page_size = PageSize::new(10).expect("page size");
    let CalculatedGradebookResult::Page(page) = store
        .calculated_gradebook_page(
            context,
            session,
            course,
            first_gradebook_page(GradebookFilter::All, 10),
        )
        .await
        .expect("active gradebook page")
    else {
        panic!("active roster must produce a gradebook page");
    };
    let active_student = page.rows[0].membership;

    let roster = store
        .list_course_roster(context, session, course, PageRequest::first(page_size))
        .await
        .expect("course roster");
    let member = roster
        .entries
        .items
        .into_iter()
        .find_map(|entry| match entry {
            CourseRosterEntry::Member(member) => Some(member),
            CourseRosterEntry::Invitation(_) => None,
        })
        .expect("active Student roster entry");
    store
        .revoke_course_member(
            context,
            session,
            RevokeCourseMember {
                course,
                member: member.id,
                expected_revision: roster.policy.revision,
            },
        )
        .await
        .expect("revoke Student membership");
    assert_eq!(
        store
            .calculated_gradebook_page(
                context,
                session,
                course,
                first_gradebook_page(GradebookFilter::Student(active_student), 10),
            )
            .await,
        Err(StoreError::NotFound)
    );

    let foreign_course = CourseId::from_uuid(uuid(91_070));
    let authority =
        sysadmin_course_creation_authority(&store, context.tenant_id(), foreign_course, instructor)
            .await;
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: foreign_course,
                    title: "Foreign gradebook course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("fixture term"),
                },
                authority,
            },
        )
        .await
        .expect("foreign course");
    add_gradebook_student(&store, context, foreign_course, instructor, 91_071).await;
    let CalculatedGradebookResult::Page(foreign_page) = store
        .calculated_gradebook_page(
            context,
            session,
            foreign_course,
            first_gradebook_page(GradebookFilter::All, 10),
        )
        .await
        .expect("foreign gradebook page")
    else {
        panic!("foreign roster must produce a gradebook page");
    };
    assert_eq!(
        store
            .calculated_gradebook_page(
                context,
                session,
                course,
                first_gradebook_page(
                    GradebookFilter::Student(foreign_page.rows[0].membership),
                    10
                ),
            )
            .await,
        Err(StoreError::NotFound)
    );
}

#[tokio::test]
async fn gradebook_selection_is_roster_ordered_and_cursor_bound() {
    let store = MemoryStore::default();
    let (context, session, course, instructor) = claimed_gradebook_fixture(&store).await;
    add_gradebook_student(&store, context, course, instructor, 95_001).await;
    let assignment = create_grade_assignment(
        &store,
        context,
        context.tenant_id(),
        instructor,
        course,
        95_010,
        PointValue::from_whole(1),
    )
    .await;
    let assignment = store
        .assignment_reference(context, instructor, assignment)
        .await
        .expect("assignment reference lookup")
        .expect("assignment reference");
    let request = GradebookSelectionRequest {
        filter: GradebookFilterRequest::Assignment(assignment),
        page: PageRequest::first(PageSize::new(1).expect("bounded page")),
    };
    let GradebookSelectionResult::StudentSelection {
        rows: first,
        next_cursor,
    } = store
        .gradebook_selection(context, session, course, request.clone())
        .await
        .expect("first named Student selection")
    else {
        panic!("assignment scope selects a Student");
    };
    let cursor = next_cursor.expect("selection continues");
    let GradebookSelectionResult::StudentSelection { rows: second, .. } = store
        .gradebook_selection(
            context,
            session,
            course,
            GradebookSelectionRequest {
                filter: GradebookFilterRequest::Assignment(assignment),
                page: PageRequest::after(cursor.clone(), PageSize::new(1).expect("bounded page")),
            },
        )
        .await
        .expect("continued named Student selection")
    else {
        panic!("continued selection remains a list");
    };
    assert!(first[0].membership < second[0].membership);
    assert_eq!(
        store
            .gradebook_selection(
                context,
                session,
                course,
                GradebookSelectionRequest {
                    filter: GradebookFilterRequest::All,
                    page: PageRequest::after(cursor, PageSize::new(1).expect("bounded page")),
                }
            )
            .await,
        Err(StoreError::NotFound),
        "a selection cursor cannot be replayed into a different scope",
    );
}

#[tokio::test]
async fn submitted_run_choices_are_bounded_and_mark_the_score_selected_run() {
    let store = MemoryStore::default();
    let fixture = exercise_run_api_receipts_with_grade_policy(
        &store,
        StudentDisclosurePolicy::default(),
        96_000,
        GradePolicy::Highest,
    )
    .await;
    let session = SessionTokenHash::compute(b"submitted-run-choices-instructor");
    store
        .create_session(
            session,
            SessionSubject::new(
                fixture.tenant,
                fixture.publisher,
                "Run choices instructor",
                vec![UserRole::Instructor],
            )
            .expect("session subject"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("instructor session");
    let assignment = store
        .assignment_reference(fixture.context, fixture.publisher, fixture.assignment)
        .await
        .expect("assignment reference lookup")
        .expect("assignment reference");
    let CalculatedGradebookResult::Page(page) = store
        .calculated_gradebook_page(
            fixture.context,
            session,
            fixture.course,
            first_gradebook_page(GradebookFilter::All, 10),
        )
        .await
        .expect("Gradebook page")
    else {
        panic!("Gradebook page available");
    };
    let membership = page.rows[0].membership;
    let first = store
        .submitted_run_choices(
            fixture.context,
            session,
            fixture.course,
            SubmittedRunChoicesRequest {
                membership,
                assignment,
                operation: None,
                page: PageRequest::first(PageSize::new(1).expect("bounded page")),
            },
        )
        .await
        .expect("first submitted-run chooser page");
    assert_eq!(first.rows.len(), 1);
    assert!(
        first.next_cursor.is_some(),
        "two completed runs require a bounded continuation"
    );
    let second = store
        .submitted_run_choices(
            fixture.context,
            session,
            fixture.course,
            SubmittedRunChoicesRequest {
                membership,
                assignment,
                operation: None,
                page: PageRequest::after(
                    first.next_cursor.expect("chooser cursor"),
                    PageSize::new(1).expect("bounded page"),
                ),
            },
        )
        .await
        .expect("continued submitted-run chooser page");
    assert_eq!(second.rows.len(), 1);
    assert_ne!(first.rows[0].run, second.rows[0].run);
    assert_eq!(
        first
            .rows
            .iter()
            .chain(second.rows.iter())
            .filter(|choice| choice.score_selected)
            .count(),
        1,
        "one completed run remains selected by the current score policy",
    );
}

#[path = "course_gradebook/totals.rs"]
mod totals;
