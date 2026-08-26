use question_model::{
    CourseTerm, CourseTermShiftIneligibility, CourseTermShiftPreviewOutcome,
    CourseTermShiftPreviewRequest, CourseTermShiftRecoveryAction,
};

use super::*;

#[tokio::test]
async fn missing_course_schedule_revision_fails_closed_without_repairing_state() {
    let fixture = Fixture::new().await;
    let applied = fixture.instantiate("missing-schedule").await;
    let intact_witness = witness(&fixture, applied.course);
    let before = b2_snapshot(&fixture);
    {
        let mut state = fixture.store.write_state().expect("state");
        let course = resolve_course(&state, fixture.tenant, applied.course).expect("course");
        state
            .course_schedule_revisions
            .remove(&(fixture.tenant, course));
    }
    let corrupted = b2_snapshot(&fixture);
    assert!(matches!(
        fixture
            .store
            .preview_course_term_shift(
                fixture.context,
                fixture.session,
                CourseTermShiftPreviewRequest {
                    witness: intact_witness,
                    target_term: CourseTerm::from_parts(
                        "2027-01-11",
                        "2027-05-08",
                        "America/Chicago"
                    )
                    .expect("term"),
                },
            )
            .await,
        Err(StoreError::Unavailable(_)) | Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(b2_snapshot(&fixture), corrupted);
    assert_ne!(
        before, corrupted,
        "fault injection is the only state mutation"
    );
}

/// Issued learner work changes the visible preview into an explicit recovery
/// path before a term-shift command can exist.  Apply retains its separate
/// optimistic fence for work issued after an eligible preview.
#[tokio::test]
async fn issued_course_term_shift_preview_requires_rollover_recovery() {
    let fixture = Fixture::new().await;
    let applied = fixture.instantiate("issued-before-preview").await;
    issue_run(&fixture, applied.course);

    let outcome = fixture
        .store
        .preview_course_term_shift(
            fixture.context,
            fixture.session,
            CourseTermShiftPreviewRequest {
                witness: witness(&fixture, applied.course),
                target_term: CourseTerm::from_parts("2027-01-11", "2027-05-08", "America/Chicago")
                    .expect("term"),
            },
        )
        .await
        .expect("issued work returns a typed preview outcome");

    assert!(matches!(
        outcome,
        CourseTermShiftPreviewOutcome::Ineligible {
            course,
            reason: CourseTermShiftIneligibility::IssuedWork,
            recovery: CourseTermShiftRecoveryAction::RolloverCourse,
        } if course == applied.course
    ));
}

/// A course created by an Alpha adoption has no ordinary-origin fallback when
/// its immutable whole-course record is detached from otherwise current
/// assignment projections.
#[tokio::test]
async fn detached_whole_course_adoption_refuses_inspection_without_mutation() {
    let fixture = Fixture::new().await;
    let applied = fixture.instantiate("detached-whole-course").await;
    let before = b2_snapshot(&fixture);
    {
        let mut state = fixture.store.write_state().expect("state");
        let course = resolve_course(&state, fixture.tenant, applied.course).expect("course");
        state
            .curriculum_adoption
            .whole_course_adoptions
            .remove(&(fixture.tenant, course));
    }
    let corrupted = b2_snapshot(&fixture);

    assert!(matches!(
        fixture
            .store
            .inspect_curriculum_imports(fixture.context, fixture.session, applied.course)
            .await,
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(b2_snapshot(&fixture), corrupted);
    assert_ne!(
        before, corrupted,
        "fault injection is the only state mutation"
    );
}
