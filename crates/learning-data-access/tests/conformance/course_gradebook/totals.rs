//! Course-total and grade-scheme conformance for the Gradebook Store.

use super::*;

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

    let page = store
        .calculated_gradebook_page(
            context,
            session,
            course,
            CalculatedGradebookRequest {
                filter: GradebookFilter::All,
                page: PageRequest::first(PageSize::new(10).expect("bounded page size")),
            },
        )
        .await
        .expect("calculated gradebook page");
    let CalculatedGradebookResult::Page(page) = page else {
        panic!("first calculated gradebook request must not require reload");
    };
    assert_eq!(page.rows.len(), 1);
    assert_eq!(page.rows[0].display_label, "Numeric Student");
    assert_eq!(page.rows[0].assignment_cells.len(), 2);
    assert!(page.rows[0].assignment_cells.iter().all(|cell| matches!(
        cell.inspection_choice,
        learning_data_access::AssignmentInspectionChoice::NoSubmittedRun
    )));
    assert_eq!(page.scoring_witnesses.len(), 2);

    let export = store
        .create_course_grade_export(context, session, course)
        .await
        .expect("course export");
    assert_eq!(export.rows.len(), totals.rows.len());
    for (export_row, total_row) in export.rows.iter().zip(&totals.rows) {
        assert_eq!(export_row.display_name, total_row.display_name);
        assert_eq!(export_row.outcome, total_row.outcome);
    }
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
    let CalculatedGradebookResult::Page(weighted_page) = store
        .calculated_gradebook_page(
            context,
            session,
            course,
            first_gradebook_page(GradebookFilter::All, 10),
        )
        .await
        .expect("weighted calculated page")
    else {
        panic!("weighted page must be available");
    };
    assert_eq!(
        weighted_page.rows[0].outcome.dropped_assignment_ids,
        vec![second]
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

#[tokio::test]
async fn memory_course_grade_export_includes_active_student_without_roster_contact() {
    let store = MemoryStore::default();
    let (context, session, course, instructor) = claimed_gradebook_fixture(&store).await;
    let tenant = context.tenant_id();
    create_grade_assignment(
        &store,
        context,
        tenant,
        instructor,
        course,
        91_050,
        PointValue::from_whole(1),
    )
    .await;
    add_gradebook_student(&store, context, course, instructor, 91_051).await;

    let totals = store
        .course_gradebook_totals(context, session, course)
        .await
        .expect("calculated totals include the uncontacted active student");
    let total = totals
        .rows
        .iter()
        .find(|row| row.display_name == "Gradebook Student 91051")
        .expect("uncontacted active student total");
    assert_eq!(total.outcome.rounded_score, Some(0.0));
    assert_eq!(total.outcome.total_possible, Some(1.0));

    let CalculatedGradebookResult::Page(page) = store
        .calculated_gradebook_page(
            context,
            session,
            course,
            first_gradebook_page(GradebookFilter::All, 10),
        )
        .await
        .expect("calculated Gradebook page includes the uncontacted active student")
    else {
        panic!("first calculated Gradebook request must not require reload");
    };
    let page_row = page
        .rows
        .iter()
        .find(|row| row.display_label == "Gradebook Student 91051")
        .expect("uncontacted active student Gradebook row");
    assert_eq!(page_row.outcome, total.outcome);

    let export = store
        .create_course_grade_export(context, session, course)
        .await
        .expect("course export includes the uncontacted active student");
    let export_row = export
        .rows
        .iter()
        .find(|row| row.display_name == "Gradebook Student 91051")
        .expect("uncontacted active student export row");
    assert!(export_row.roster_id.is_none());
    assert!(export_row.roster_email.is_none());
    assert_eq!(export_row.outcome, total.outcome);
    assert_eq!(export.audit.row_count, export.rows.len());
    assert_eq!(export.rows.len(), totals.rows.len());
}
